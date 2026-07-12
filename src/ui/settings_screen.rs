use crate::config::{Config, ProviderType};
use crate::ui::colors::*;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs,
};
use ratatui::Frame;

/// Settings tab variants
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsTab {
    Providers,
    General,
    Developer,
}

impl SettingsTab {
    pub fn all() -> Vec<Self> {
        vec![
            SettingsTab::Providers,
            SettingsTab::General,
            SettingsTab::Developer,
        ]
    }

    pub fn title(&self) -> &str {
        match self {
            SettingsTab::Providers => "🔌 Providers",
            SettingsTab::General => "⚙ General",
            SettingsTab::Developer => "🔧 Developer",
        }
    }
}

/// Render the settings screen
pub fn render_settings_screen(
    frame: &mut Frame,
    area: Rect,
    config: &Config,
    active_tab: &SettingsTab,
    selected_provider: &ProviderType,
    dev_mode: bool,
    editing: bool,
    focus_field: usize,
    field_buffer: &str,
    model_selecting: bool,
    discovered_models: &Option<Vec<String>>,
    fetching_models: bool,
    models_fetch_error: &Option<String>,
    model_scroll_offset: u16,
) {
    // Dark overlay for settings screen
    let settings_area = centered_rect(80, 85, area);
    frame.render_widget(Clear, settings_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(MAUVE))
        .style(Style::default().bg(BASE))
        .title(Line::from(vec![
            Span::styled(" ⚙ ", Style::default().fg(MAUVE)),
            Span::styled(
                "Settings",
                Style::default().fg(MAUVE).add_modifier(Modifier::BOLD),
            ),
        ]))
        .title_alignment(Alignment::Center);

    let inner = block.inner(settings_area);
    frame.render_widget(block, settings_area);

    // Tab bar
    let tab_titles: Vec<Line> = SettingsTab::all()
        .iter()
        .map(|tab| {
            let is_active = tab == active_tab;
            Line::from(vec![Span::styled(
                format!(" {} ", tab.title()),
                if is_active {
                    Style::default()
                        .fg(BASE)
                        .bg(MAUVE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(SUBTEXT_0)
                },
            )])
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(SURFACE_2))
                .style(Style::default().bg(MANTLE)),
        )
        .highlight_style(
            Style::default()
                .fg(BASE)
                .bg(MAUVE)
                .add_modifier(Modifier::BOLD),
        )
        .select(match active_tab {
            SettingsTab::Providers => 0,
            SettingsTab::General => 1,
            SettingsTab::Developer => 2,
        });

    let tab_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(inner);

    frame.render_widget(tabs, tab_area[0]);

    // Tab content
    match active_tab {
        SettingsTab::Providers => render_providers_settings(
            frame, tab_area[1], config, selected_provider,
            editing, focus_field, field_buffer, model_selecting,
            discovered_models, fetching_models, models_fetch_error,
            model_scroll_offset,
        ),
        SettingsTab::General => render_general_settings(frame, tab_area[1], config),
        SettingsTab::Developer => render_developer_settings(frame, tab_area[1], dev_mode),
    }

    // Bottom keybind hints
    let hint_text = if editing && !model_selecting {
        Paragraph::new(Line::from(vec![
            Span::styled(" Tab ", Style::default().fg(OVERLAY_1)),
            Span::styled("next field  ", Style::default().fg(SUBTEXT_0)),
            Span::styled("Enter ", Style::default().fg(OVERLAY_1)),
            Span::styled("edit model  ", Style::default().fg(SUBTEXT_0)),
            Span::styled("Esc ", Style::default().fg(OVERLAY_1)),
            Span::styled("save & exit  ", Style::default().fg(SUBTEXT_0)),
            Span::styled("Ctrl+S ", Style::default().fg(OVERLAY_1)),
            Span::styled("save config", Style::default().fg(SUBTEXT_0)),
        ]))
        .style(Style::default().bg(CRUST))
        .alignment(Alignment::Center)
    } else if model_selecting {
        Paragraph::new(Line::from(vec![
            Span::styled(" ↑/↓ ", Style::default().fg(OVERLAY_1)),
            Span::styled("select model  ", Style::default().fg(SUBTEXT_0)),
            Span::styled("Enter ", Style::default().fg(OVERLAY_1)),
            Span::styled("confirm  ", Style::default().fg(SUBTEXT_0)),
            Span::styled("Esc ", Style::default().fg(OVERLAY_1)),
            Span::styled("cancel", Style::default().fg(SUBTEXT_0)),
        ]))
        .style(Style::default().bg(CRUST))
        .alignment(Alignment::Center)
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" ←/→ ", Style::default().fg(OVERLAY_1)),
            Span::styled("tabs  ", Style::default().fg(SUBTEXT_0)),
            Span::styled("↑/↓ ", Style::default().fg(OVERLAY_1)),
            Span::styled("navigate  ", Style::default().fg(SUBTEXT_0)),
            Span::styled("Enter ", Style::default().fg(OVERLAY_1)),
            Span::styled("edit config  ", Style::default().fg(SUBTEXT_0)),
            Span::styled("Esc ", Style::default().fg(OVERLAY_1)),
            Span::styled("close  ", Style::default().fg(SUBTEXT_0)),
            Span::styled("Ctrl+S ", Style::default().fg(OVERLAY_1)),
            Span::styled("save", Style::default().fg(SUBTEXT_0)),
        ]))
        .style(Style::default().bg(CRUST))
        .alignment(Alignment::Center)
    };

    let hint_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    frame.render_widget(hint_text, hint_area[1]);
}

fn render_providers_settings(
    frame: &mut Frame,
    area: Rect,
    config: &Config,
    selected_provider: &ProviderType,
    editing: bool,
    focus_field: usize,
    field_buffer: &str,
    model_selecting: bool,
    discovered_models: &Option<Vec<String>>,
    fetching_models: bool,
    models_fetch_error: &Option<String>,
    model_scroll_offset: u16,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // Left: Provider list
    let provider_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(SURFACE_2))
        .style(Style::default().bg(MANTLE))
        .title(" Providers ")
        .title_alignment(Alignment::Center);

    let provider_items: Vec<ListItem> = ProviderType::all()
        .iter()
        .map(|p| {
            let pc = config.get_provider_config(p);
            let is_configured = pc.api_key.is_some();
            let is_selected = p == selected_provider;

            let icon = if is_selected { "▸" } else { " " };
            let status = if is_configured { "✓" } else { "○" };
            let status_color = if is_configured { GREEN } else { OVERLAY_1 };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", icon),
                    if is_selected {
                        Style::default().fg(MAUVE)
                    } else {
                        Style::default().fg(MANTLE)
                    },
                ),
                Span::styled(p.display_name().to_string(), Style::default().fg(TEXT)),
                Span::raw(" "),
                Span::styled(status.to_string(), Style::default().fg(status_color)),
            ]))
            .style(if is_selected {
                Style::default().bg(SURFACE_0)
            } else {
                Style::default().bg(MANTLE)
            })
        })
        .collect();

    let provider_list = List::new(provider_items)
        .block(provider_block)
        .highlight_style(Style::default().bg(SURFACE_0));

    frame.render_widget(provider_list, chunks[0]);

    // Right: Provider details with editable fields
    render_provider_details(frame, chunks[1], config, selected_provider, editing, focus_field, field_buffer, model_selecting, discovered_models, fetching_models, models_fetch_error, model_scroll_offset);
}

fn render_provider_details(
    frame: &mut Frame,
    area: Rect,
    config: &Config,
    selected_provider: &ProviderType,
    editing: bool,
    focus_field: usize,
    field_buffer: &str,
    model_selecting: bool,
    discovered_models: &Option<Vec<String>>,
    fetching_models: bool,
    models_fetch_error: &Option<String>,
    model_scroll_offset: u16,
) {
    let pc = config.get_provider_config(selected_provider);

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if editing {
            Style::default().fg(GREEN)
        } else {
            Style::default().fg(SURFACE_2)
        })
        .style(Style::default().bg(BASE))
        .title(format!(
            " {} {} ",
            if editing { "✏" } else { "🔌" },
            selected_provider.display_name()
        ))
        .title_alignment(Alignment::Center);

    let detail_inner = detail_block.inner(area);
    frame.render_widget(detail_block, area);

    // Fields
    let pc_api_key = pc.api_key.clone().unwrap_or_default();
    let pc_base_url = pc.base_url.clone().unwrap_or_default();
    let pc_temp = pc.temperature.map(|t| format!("{:.1}", t)).unwrap_or_else(|| "0.7".to_string());
    let current_model = pc.model.clone().unwrap_or_else(|| selected_provider.default_model().to_string());

    // For editable fields, use the buffer for the focused field, config value otherwise
    let display_api_key = if editing && focus_field == 0 {
        if field_buffer.is_empty() { pc_api_key.clone() } else { field_buffer.to_string() }
    } else {
        pc_api_key.clone()
    };
    let display_base_url = if editing && focus_field == 1 {
        if field_buffer.is_empty() { pc_base_url.clone() } else { field_buffer.to_string() }
    } else {
        pc_base_url.clone()
    };
    let display_temp = if editing && focus_field == 2 {
        if field_buffer.is_empty() { pc_temp.clone() } else { field_buffer.to_string() }
    } else {
        pc_temp.clone()
    };

    let api_key_display = if display_api_key.is_empty() {
        "Not set".to_string()
    } else if editing && focus_field == 0 {
        display_api_key.clone()
    } else {
        // Mask the key for display when not editing
        if display_api_key.len() > 8 {
            format!("{}…{}", &display_api_key[..4], &display_api_key[display_api_key.len()-4..])
        } else {
            "****".to_string()
        }
    };

    let edit_indicator = |field: usize| -> Vec<Span<'static>> {
        if editing && focus_field == field {
            vec![
                Span::styled(" ▏", Style::default().fg(MAUVE).add_modifier(Modifier::SLOW_BLINK)),
            ]
        } else {
            vec![]
        }
    };

    let mut detail_lines: Vec<Line> = Vec::new();

    // ═══ CONFIG FIELDS ═══

    // API Key field
    detail_lines.push(Line::from(vec![
        Span::styled(" 🔑 ", Style::default().fg(if editing && focus_field == 0 { MAUVE } else { TEAL })),
        Span::styled("API Key", Style::default().fg(SUBTEXT_0).add_modifier(Modifier::BOLD)),
    ]));
    let api_key_style = if editing && focus_field == 0 {
        Style::default().fg(TEXT).bg(SURFACE_0)
    } else {
        Style::default().fg(if pc.api_key.is_some() { GREEN } else { PEACH })
    };
    let mut api_line = vec![
        Span::raw("    "),
        Span::styled(api_key_display, api_key_style),
    ];
    api_line.extend(edit_indicator(0));
    detail_lines.push(Line::from(api_line));

    // Base URL field
    detail_lines.push(Line::from(String::new()));
    detail_lines.push(Line::from(vec![
        Span::styled(" 🌐 ", Style::default().fg(if editing && focus_field == 1 { MAUVE } else { SKY })),
        Span::styled("Base URL", Style::default().fg(SUBTEXT_0).add_modifier(Modifier::BOLD)),
    ]));
    let url_style = if editing && focus_field == 1 {
        Style::default().fg(TEXT).bg(SURFACE_0)
    } else {
        Style::default().fg(SUBTEXT_1)
    };
    let mut url_line = vec![
        Span::raw("    "),
        Span::styled(
            if display_base_url.is_empty() { "Default (provider's API)".to_string() } else { display_base_url.clone() },
            url_style,
        ),
    ];
    url_line.extend(edit_indicator(1));
    detail_lines.push(Line::from(url_line));

    // Temperature field
    detail_lines.push(Line::from(String::new()));
    detail_lines.push(Line::from(vec![
        Span::styled(" 🌡 ", Style::default().fg(if editing && focus_field == 2 { MAUVE } else { PEACH })),
        Span::styled("Temperature", Style::default().fg(SUBTEXT_0).add_modifier(Modifier::BOLD)),
    ]));
    let temp_style = if editing && focus_field == 2 {
        Style::default().fg(TEXT).bg(SURFACE_0)
    } else {
        Style::default().fg(SUBTEXT_1)
    };
    let mut temp_line = vec![
        Span::raw("    "),
        Span::styled(display_temp.clone(), temp_style),
    ];
    temp_line.extend(edit_indicator(2));
    detail_lines.push(Line::from(temp_line));

    // ═══ MODEL SELECTION ═══
    detail_lines.push(Line::from(String::new()));
    
    // Show source indicator for models
    let has_custom_url = pc.base_url.as_ref().map(|u| !u.is_empty()).unwrap_or(false);
    let model_source_label = if has_custom_url && discovered_models.is_some() {
        "Model (from API)"
    } else if has_custom_url && fetching_models {
        "Model (fetching...)"
    } else if has_custom_url && models_fetch_error.is_some() {
        "Model (fetch failed)"
    } else {
        "Model"
    };
    
    detail_lines.push(Line::from(vec![
        Span::styled(" 🤖 ", Style::default().fg(if editing && focus_field == 3 { MAUVE } else { GREEN })),
        Span::styled(model_source_label, Style::default().fg(SUBTEXT_0).add_modifier(Modifier::BOLD)),
    ]));

    // Determine which models to show
    let models: Vec<&str> = if let Some(ref discovered) = discovered_models {
        discovered.iter().map(|s| s.as_str()).collect()
    } else {
        selected_provider.models()
    };

    if fetching_models {
        detail_lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("⏳ Fetching models from API...", Style::default().fg(YELLOW)),
        ]));
    } else if let Some(ref error) = models_fetch_error {
        detail_lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("✗ ", Style::default().fg(RED)),
            Span::styled(
                if error.len() > 60 { format!("{}…", &error[..60]) } else { error.clone() },
                Style::default().fg(RED),
            ),
        ]));
        // Still show hardcoded models when fetch fails
        if discovered_models.is_none() && !model_selecting {
            detail_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("Using default model list", Style::default().fg(OVERLAY_1)),
            ]));
        }
    }

    if model_selecting && !fetching_models {
        // Show scrollable model list
        for (idx, model) in models.iter().enumerate() {
            let is_active = *model == current_model;
            let is_focused = idx == focus_field;
            detail_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    if is_focused { "▸ " } else { "  " },
                    if is_focused {
                        Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(OVERLAY_0)
                    },
                ),
                Span::styled(
                    *model,
                    if is_focused {
                        Style::default().fg(TEXT).bg(SURFACE_0).add_modifier(Modifier::BOLD)
                    } else if is_active {
                        Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(SUBTEXT_1)
                    },
                ),
                if is_active && !is_focused {
                    Span::styled(" ✓", Style::default().fg(GREEN))
                } else {
                    Span::raw("")
                },
                if is_focused {
                    Span::styled(" ◄", Style::default().fg(MAUVE).add_modifier(Modifier::SLOW_BLINK))
                } else {
                    Span::raw("")
                },
            ]));
        }
    } else if !fetching_models && models_fetch_error.is_none() {
        // Show current model only (not in selection mode)
        detail_lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                &current_model,
                if editing && focus_field == 3 {
                    Style::default().fg(TEXT).bg(SURFACE_0)
                } else {
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
                },
            ),
            Span::styled(" ✓", Style::default().fg(GREEN)),
            if editing && focus_field == 3 {
                Span::styled("  [Enter to change]", Style::default().fg(OVERLAY_1))
            } else {
                Span::raw("")
            },
        ]));
    }

    // Show model count if discovered
    if let Some(ref discovered) = discovered_models {
        if !model_selecting && !fetching_models {
            detail_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("{} models available", discovered.len()),
                    Style::default().fg(OVERLAY_1),
                ),
            ]));
        }
    }

    detail_lines.push(Line::from(String::new()));
    detail_lines.push(Line::from(vec![
        Span::styled(" ℹ ", Style::default().fg(SKY)),
        if editing && !model_selecting {
            Span::styled(
                "Tab to switch fields · Type to edit",
                Style::default().fg(OVERLAY_1),
            )
        } else if model_selecting {
            if fetching_models {
                Span::styled(
                    "Fetching models...",
                    Style::default().fg(YELLOW),
                )
            } else {
                Span::styled(
                    "↑↓ to browse · Enter to select",
                    Style::default().fg(OVERLAY_1),
                )
            }
        } else if has_custom_url && discovered_models.is_none() && !fetching_models {
            Span::styled(
                "Enter model list to fetch models from API",
                Style::default().fg(OVERLAY_1),
            )
        } else {
            Span::styled(
                "Press Enter to edit this provider's config",
                Style::default().fg(OVERLAY_1),
            )
        },
    ]));

    let detail_para = Paragraph::new(Text::from(detail_lines))
        .style(Style::default().bg(BASE))
        .scroll((model_scroll_offset, 0));

    frame.render_widget(detail_para, detail_inner);
}

fn render_general_settings(frame: &mut Frame, area: Rect, config: &Config) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(SURFACE_2))
        .style(Style::default().bg(BASE))
        .title(" General Settings ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let default_provider = config.get_default_provider();

    let content = Text::from(vec![
        Line::from(vec![
            Span::styled(" 🌐 ", Style::default().fg(TEAL)),
            Span::styled("Default Provider", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(default_provider.display_name().to_string(), Style::default().fg(GREEN)),
        ]),
        Line::from(String::new()),
        Line::from(vec![
            Span::styled(" 📜 ", Style::default().fg(TEAL)),
            Span::styled("Scrollback Lines", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{}", config.ui.scrollback_lines),
                Style::default().fg(GREEN),
            ),
        ]),
        Line::from(String::new()),
        Line::from(vec![
            Span::styled(" 🎨 ", Style::default().fg(TEAL)),
            Span::styled("Theme", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(&config.ui.theme, Style::default().fg(GREEN)),
        ]),
        Line::from(String::new()),
        Line::from(vec![
            Span::styled(" ℹ ", Style::default().fg(SKY)),
            Span::styled(
                "Config file location: ~/.config/opentui/config.toml",
                Style::default().fg(OVERLAY_1),
            ),
        ]),
    ]);

    let para = Paragraph::new(content).style(Style::default().bg(BASE));
    frame.render_widget(para, inner);
}

fn render_developer_settings(frame: &mut Frame, area: Rect, dev_mode: bool) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(YELLOW))
        .style(Style::default().bg(BASE))
        .title(" Developer Mode ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let status_icon = if dev_mode { "✓" } else { "○" };
    let status_color = if dev_mode { GREEN } else { OVERLAY_1 };
    let status_text = if dev_mode { "Enabled" } else { "Disabled" };

    let content = Text::from(vec![
        Line::from(vec![
            Span::styled(" 🔧 ", Style::default().fg(YELLOW)),
            Span::styled("Developer Mode", Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(
                format!("{} {}", status_icon, status_text),
                Style::default().fg(status_color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(String::new()),
        Line::from(vec![
            Span::styled("   Press Enter to toggle", Style::default().fg(SUBTEXT_0)),
        ]),
        Line::from(String::new()),
        Line::from(vec![
            Span::styled(" 📊 ", Style::default().fg(SKY)),
            Span::styled("When enabled, shows:", Style::default().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::raw("     "),
            Span::styled("• Tokens per second", Style::default().fg(SUBTEXT_1)),
        ]),
        Line::from(vec![
            Span::raw("     "),
            Span::styled("• Input tokens count", Style::default().fg(SUBTEXT_1)),
        ]),
        Line::from(vec![
            Span::raw("     "),
            Span::styled("• Output tokens count", Style::default().fg(SUBTEXT_1)),
        ]),
        Line::from(vec![
            Span::raw("     "),
            Span::styled("• Total tokens", Style::default().fg(SUBTEXT_1)),
        ]),
        Line::from(vec![
            Span::raw("     "),
            Span::styled("• Response duration", Style::default().fg(SUBTEXT_1)),
        ]),
        Line::from(vec![
            Span::raw("     "),
            Span::styled("• Estimated cost", Style::default().fg(SUBTEXT_1)),
        ]),
        Line::from(String::new()),
        Line::from(vec![
            Span::styled(" ⚠ ", Style::default().fg(PEACH)),
            Span::styled(
                "Developer mode adds a metrics bar below the chat area",
                Style::default().fg(OVERLAY_1),
            ),
        ]),
    ]);

    let para = Paragraph::new(content).style(Style::default().bg(BASE));
    frame.render_widget(para, inner);
}

/// Helper to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
