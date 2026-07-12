mod app;
mod chat;
mod config;
mod dev_mode;
mod providers;
mod ui;

use crate::app::App;
use crate::config::{Config, ProviderType};
use crate::providers::create_provider;
use crate::ui::settings_screen::SettingsTab;
use crate::ui::Screen;
use anyhow::Result;
use clap::Parser;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
    KeyModifiers,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::time::Duration;

/// OpenTUI - A stunning terminal AI chat interface for multiple providers
#[derive(Parser, Debug)]
#[command(name = "opentui", version, about = "A stunning terminal AI chat interface")]
struct Cli {
    /// Provider to use (openai, groq, nvidia, anthropic, google, deepseek, mistral)
    #[arg(short, long)]
    provider: Option<String>,

    /// Model to use
    #[arg(short, long)]
    model: Option<String>,

    /// Enable dev mode
    #[arg(short, long)]
    dev: bool,

    /// Show config path and exit
    #[arg(long)]
    config_path: bool,

    /// Send a message and exit (non-interactive)
    #[arg(long)]
    message: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle non-interactive commands
    if cli.config_path {
        println!("Config path: {:?}", Config::config_path()?);
        return Ok(());
    }

    // Load config
    let mut config = Config::load()?;

    // Apply CLI overrides
    if let Some(provider_name) = &cli.provider {
        let provider = match provider_name.to_lowercase().as_str() {
            "openai" => ProviderType::OpenAI,
            "groq" => ProviderType::Groq,
            "nvidia" => ProviderType::Nvidia,
            "anthropic" => ProviderType::Anthropic,
            "google" => ProviderType::Google,
            "deepseek" => ProviderType::DeepSeek,
            "mistral" => ProviderType::Mistral,
            _ => anyhow::bail!("Unknown provider: {}. Valid providers: openai, groq, nvidia, anthropic, google, deepseek, mistral", provider_name),
        };
        config.default_provider = Some(provider);
    }

    if cli.dev {
        config.ui.developer_mode = true;
    }

    // Handle one-shot message mode
    if let Some(msg) = cli.message {
        return send_one_shot(&config, &msg).await;
    }

    // Interactive mode
    run_interactive(config).await
}

/// Run interactive TUI mode
async fn run_interactive(config: Config) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Create app
    let mut app = App::new(config)?;
    let tick_rate = Duration::from_millis(50);

    // Run event loop
    let result = run_event_loop(&mut terminal, &mut app, tick_rate).await;

    // Restore terminal
    let mut restore_result = Ok(());
    if let Err(e) = disable_raw_mode() {
        restore_result = Err(e);
    }
    if let Err(e) = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ) {
        restore_result = Err(e.into());
    }
    terminal.show_cursor()?;

    // Print any error from the event loop
    if let Err(e) = &result {
        eprintln!("Error: {}", e);
    }

    result?;
    restore_result.map_err(|e| anyhow::anyhow!("Failed to restore terminal: {}", e))
}

/// Run the main event loop
async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    tick_rate: Duration,
) -> Result<()> {
    loop {
        // Process stream data if streaming
        if app.is_streaming {
            app.process_stream().await?;
        }

        // Fetch models from API if requested
        if app.fetching_models && !app.is_streaming {
            let base_url = app.config
                .get_provider_config(&app.settings_selected_provider)
                .base_url
                .clone()
                .unwrap_or_default();

            if !base_url.is_empty() {
                let models_url = format!("{}/models", base_url.trim_end_matches('/'));
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()?;

                match client.get(&models_url).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            if let Ok(body) = resp.json::<serde_json::Value>().await {
                                if let Some(data) = body["data"].as_array() {
                                    let models: Vec<String> = data
                                        .iter()
                                        .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                                        .collect();
                                    if !models.is_empty() {
                                        app.discovered_models = Some(models);
                                        app.models_fetch_error = None;
                                    } else {
                                        app.models_fetch_error = Some("No models found at this endpoint".to_string());
                                    }
                                } else {
                                    app.models_fetch_error = Some("Unexpected API response format".to_string());
                                }
                            } else {
                                app.models_fetch_error = Some("Failed to parse API response".to_string());
                            }
                        } else {
                            let status = resp.status();
                            let body = resp.text().await.unwrap_or_default();
                            app.models_fetch_error = Some(format!("HTTP {}: {}", status, body.chars().take(100).collect::<String>()));
                        }
                    }
                    Err(e) => {
                        app.models_fetch_error = Some(format!("Connection failed: {}", e));
                    }
                }
            } else {
                app.models_fetch_error = Some("No base URL configured".to_string());
            }
            app.fetching_models = false;
        }

        // Render UI - extract all data first to avoid borrow issues
        {
            let screen = app.screen.clone();
            let chat = &app.chat;
            let provider = &app.current_provider;
            let model = &app.current_model;
            let dev_mode = app.dev_mode;
            let metrics = &app.last_metrics;
            let streaming = app.is_streaming;
            let config = &app.config;
            let settings_tab = &app.settings_tab.clone();
            let selected_provider = &app.settings_selected_provider.clone();
            let editing = app.settings_editing;
            let focus_field = app.settings_field_index;
            let field_buffer = app.settings_field_buffer.clone();
            let model_selecting = app.settings_model_selecting;
            let discovered_models = &app.discovered_models;
            let fetching_models = app.fetching_models;
            let models_fetch_error = &app.models_fetch_error;
            let model_scroll_offset = app.model_scroll_offset as u16;

            terminal.draw(|frame| {
                match screen {
                    Screen::Chat => {
                        ui::chat_screen::render_chat_screen(
                            frame, chat, provider, model, dev_mode, metrics, streaming,
                        );
                    }
                    Screen::Settings => {
                        ui::chat_screen::render_chat_screen(
                            frame, chat, provider, model, dev_mode, metrics, streaming,
                        );
                        ui::settings_screen::render_settings_screen(
                            frame, frame.area(), config, &settings_tab, &selected_provider, dev_mode,
                            editing, focus_field, &field_buffer, model_selecting,
                            discovered_models, fetching_models, models_fetch_error,
                            model_scroll_offset,
                        );
                    }
                    Screen::Quit => {}
                }
            })?;
        }

        // Handle input with timeout
        if event::poll(tick_rate)? {
            let event = event::read()?;
            handle_event(app, event).await?;
        }

        // Check if we should quit
        if app.screen == Screen::Quit {
            break;
        }
    }

    Ok(())
}

/// Handle keyboard events
async fn handle_event(app: &mut App, event: Event) -> Result<()> {
    if let Event::Key(key) = event {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match app.screen {
            Screen::Chat => {
                match key.code {
                    KeyCode::Enter => {
                        if app.is_streaming {
                            app.cancel_stream();
                        } else if let Some(msg) = app.chat.submit() {
                            // Start streaming
                            if let Err(e) = app.start_stream(msg).await {
                                app.chat.error_message =
                                    Some(format!("Error: {}", e));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        if app.is_streaming {
                            app.cancel_stream();
                        }
                    }
                    KeyCode::Backspace => {
                        if !app.is_streaming {
                            app.chat.delete_char();
                        }
                    }
                    KeyCode::Left => {
                        if !app.is_streaming {
                            app.chat.move_cursor_left();
                        }
                    }
                    KeyCode::Right => {
                        if !app.is_streaming {
                            app.chat.move_cursor_right();
                        }
                    }
                    KeyCode::Home => {
                        if !app.is_streaming {
                            app.chat.cursor_position = 0;
                        }
                    }
                    KeyCode::End => {
                        if !app.is_streaming {
                            app.chat.cursor_position = app.chat.input.len();
                        }
                    }
                    KeyCode::Up => {
                        // No-op, can be used for history later
                    }
                    KeyCode::PageUp => {
                        app.scroll_offset = app.scroll_offset.saturating_add(10);
                    }
                    KeyCode::PageDown => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(10);
                    }
                    KeyCode::Char(c) => {
                        if key.modifiers == KeyModifiers::CONTROL {
                            match c {
                                's' | 'S' => {
                                    app.screen = Screen::Settings;
                                }
                                'c' | 'C' | 'q' | 'Q' => {
                                    app.screen = Screen::Quit;
                                }
                                'l' | 'L' => {
                                    app.clear_conversation();
                                }
                                _ => {}
                            }
                        } else if !app.is_streaming {
                            app.chat.insert_char(c);
                        }
                    }
                    _ => {}
                }
            }
            Screen::Settings => {
                // Save the currently editing provider config
                let save_current_edit = |app: &mut App| -> Result<()> {
                    if app.settings_editing || app.settings_model_selecting {
                        let pc = app.config.get_provider_config_mut(&app.settings_selected_provider);
                        if app.settings_field_index == 0 && !app.settings_field_buffer.is_empty() {
                            pc.api_key = Some(app.settings_field_buffer.clone());
                        }
                        if app.settings_field_index == 1 && !app.settings_field_buffer.is_empty() {
                            pc.base_url = Some(app.settings_field_buffer.clone());
                        }
                        if app.settings_field_index == 2 && !app.settings_field_buffer.is_empty() {
                            if let Ok(temp) = app.settings_field_buffer.parse::<f64>() {
                                pc.temperature = Some(temp.clamp(0.0, 2.0));
                            }
                        }
                        app.config.save()?;
                    }
                    Ok(())
                };

                // Model selection mode
                if app.settings_model_selecting {
                    let models: Vec<&str> = app.discovered_models.as_ref()
                        .map(|m| m.iter().map(|s| s.as_str()).collect())
                        .unwrap_or_else(|| app.settings_selected_provider.models());
                    match key.code {
                        KeyCode::Esc => {
                            app.settings_model_selecting = false;
                            app.settings_field_buffer.clear();
                        }
                        KeyCode::Up => {
                            let current = app.settings_field_index as isize;
                            if current > 0 {
                                app.settings_field_index = (current - 1) as usize;
                            }
                            // Auto-scroll: scroll up when cursor goes above offset
                            if app.settings_field_index < app.model_scroll_offset {
                                app.model_scroll_offset = app.model_scroll_offset.saturating_sub(1);
                            }
                        }
                        KeyCode::Down => {
                            let current = app.settings_field_index;
                            if current + 1 < models.len() {
                                app.settings_field_index = current + 1;
                            }
                            // Auto-scroll: scroll down when cursor goes past 10 items from offset
                            if app.settings_field_index >= app.model_scroll_offset + 10 {
                                app.model_scroll_offset += 1;
                            }
                        }
                        KeyCode::Enter => {
                            // Confirm model selection
                            if app.settings_field_index < models.len() {
                                let model = models[app.settings_field_index].to_string();
                                let pc = app.config.get_provider_config_mut(&app.settings_selected_provider);
                                pc.model = Some(model.clone());
                                // If this is the current active provider, update the provider instance too
                                if app.settings_selected_provider == app.current_provider {
                                    app.current_model = model.clone();
                                    if let Some(ref mut p) = app.provider {
                                        p.set_model(&model);
                                    }
                                }
                                app.config.save()?;
                            }
                            app.settings_model_selecting = false;
                            app.settings_field_index = 3;
                            app.settings_field_buffer.clear();
                        }
                        _ => {}
                    }
                }
                // Editing mode
                else if app.settings_editing {
                    match key.code {
                        KeyCode::Esc => {
                            save_current_edit(app)?;
                            app.settings_editing = false;
                            app.settings_field_index = 0;
                            app.settings_field_buffer.clear();
                            // Trigger model discovery if a custom base URL is set
                            let pc = app.config.get_provider_config(&app.settings_selected_provider);
                            if pc.base_url.as_ref().map(|u| !u.is_empty()).unwrap_or(false) {
                                app.fetching_models = true;
                                app.discovered_models = None;
                                app.models_fetch_error = None;
                            }
                        }
                        KeyCode::Tab | KeyCode::Down => {
                            // Save current field before switching
                            save_current_edit(app)?;
                            app.settings_field_index = (app.settings_field_index + 1) % 4;
                            app.settings_field_buffer.clear();
                        }
                        KeyCode::BackTab => {
                            save_current_edit(app)?;
                            app.settings_field_index = (app.settings_field_index + 3) % 4;
                            app.settings_field_buffer.clear();
                        }
                        KeyCode::Up => {
                            // Save current field before switching
                            save_current_edit(app)?;
                            app.settings_field_index = (app.settings_field_index + 3) % 4;
                            app.settings_field_buffer.clear();
                        }
                        KeyCode::Enter => {
                            if app.settings_field_index == 3 {
                                // Enter model selection mode
                                let pc = app.config.get_provider_config(&app.settings_selected_provider);
                                // If custom base URL and no discovered models yet, trigger fetch
                                if pc.base_url.as_ref().map(|u| !u.is_empty()).unwrap_or(false) && app.discovered_models.is_none() {
                                    app.fetching_models = true;
                                    app.models_fetch_error = None;
                                }
                                // Use discovered models if available, otherwise hardcoded
                                let models = app.discovered_models.as_ref()
                                    .map(|m| m.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                                    .unwrap_or_else(|| app.settings_selected_provider.models());
                                let current_model = pc.model.clone().unwrap_or_else(|| app.settings_selected_provider.default_model().to_string());
                                let model_idx = models.iter().position(|m| *m == current_model).unwrap_or(0);
                                app.settings_field_index = model_idx;
                                app.settings_model_selecting = true;
                                app.settings_field_buffer.clear();
                            } else {
                                // Save current field
                                save_current_edit(app)?;
                                app.settings_field_buffer.clear();
                            }
                        }
                        KeyCode::Backspace => {
                            app.settings_field_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            if app.settings_field_index < 3 {
                                app.settings_field_buffer.push(c);
                            }
                        }
                        _ => {}
                    }
                }
                // Normal settings navigation
                else {
                    match key.code {
                        KeyCode::Esc => {
                            app.screen = Screen::Chat;
                            app.config.save()?;
                        }
                        KeyCode::Left => {
                            let tabs = SettingsTab::all();
                            let current = tabs.iter().position(|t| t == &app.settings_tab).unwrap_or(0);
                            if current > 0 {
                                app.settings_tab = tabs[current - 1].clone();
                            }
                        }
                        KeyCode::Right => {
                            let tabs = SettingsTab::all();
                            let current = tabs.iter().position(|t| t == &app.settings_tab).unwrap_or(0);
                            if current < tabs.len() - 1 {
                                app.settings_tab = tabs[current + 1].clone();
                            }
                        }
                        KeyCode::Up => {
                            if app.settings_tab == SettingsTab::Providers {
                                let providers = ProviderType::all();
                                let current = providers
                                    .iter()
                                    .position(|p| p == &app.settings_selected_provider)
                                    .unwrap_or(0);
                                if current > 0 {
                                    app.settings_selected_provider = providers[current - 1].clone();
                                }
                            }
                        }
                        KeyCode::Down => {
                            if app.settings_tab == SettingsTab::Providers {
                                let providers = ProviderType::all();
                                let current = providers
                                    .iter()
                                    .position(|p| p == &app.settings_selected_provider)
                                    .unwrap_or(0);
                                if current < providers.len() - 1 {
                                    app.settings_selected_provider = providers[current + 1].clone();
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if app.settings_tab == SettingsTab::Developer {
                                app.toggle_dev_mode();
                            } else if app.settings_tab == SettingsTab::Providers {
                                // Switch to the selected provider AND enter editing mode
                                let selected = app.settings_selected_provider.clone();
                                app.switch_provider(&selected)?;
                                app.settings_editing = true;
                                app.settings_field_index = 0;
                                app.settings_field_buffer.clear();
                                app.settings_model_selecting = false;
                            }
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                app.config.save()?;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Screen::Quit => {}
        }
    }

    Ok(())
}

/// Send a one-shot message and print the response
async fn send_one_shot(config: &Config, message: &str) -> Result<()> {
    let provider_type = config.get_default_provider();
    let provider = create_provider(&provider_type, config)?;

    let messages = vec![crate::chat::Message::user(message)];
    let (response, metrics) = provider.send_message(&messages).await?;

    println!("\n{}", response);
    println!("\n--- Metrics ---");
    println!("{}", metrics);

    Ok(())
}
