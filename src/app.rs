use crate::chat::{ChatState, Message};
use crate::config::{Config, ProviderType};
use crate::dev_mode::Metrics;
use crate::providers::{create_provider, Provider, StreamChunk};
use crate::ui::settings_screen::SettingsTab;
use crate::ui::Screen;
use anyhow::Result;
use std::time::Instant;
use tokio::sync::mpsc;

/// Application state
pub struct App {
    /// Current screen being displayed
    pub screen: Screen,
    /// Chat interface state
    pub chat: ChatState,
    /// Application configuration
    pub config: Config,
    /// Current AI provider
    pub current_provider: ProviderType,
    /// Current model name
    pub current_model: String,
    /// Whether developer mode is enabled
    pub dev_mode: bool,
    /// Last received metrics (for dev mode display)
    pub last_metrics: Option<Metrics>,
    /// Whether we are currently receiving a stream
    pub is_streaming: bool,
    /// Stream receiver for the current request
    pub stream_rx: Option<mpsc::Receiver<Result<StreamChunk>>>,
    /// Accumulated streaming content
    pub stream_buffer: String,
    /// Provider instance (None if no API key configured)
    pub provider: Option<Box<dyn Provider>>,
    /// Settings tab
    pub settings_tab: SettingsTab,
    /// Selected provider in settings
    pub settings_selected_provider: ProviderType,
    #[allow(dead_code)]
    /// Last tick time
    pub last_tick: Instant,
    /// Scroll offset for messages
    pub scroll_offset: usize,
    /// Whether we're editing provider config in settings
    pub settings_editing: bool,
    /// Which field is focused (0=API Key, 1=Base URL, 2=Temperature, 3=Model)
    pub settings_field_index: usize,
    /// Input buffer for the active text field
    pub settings_field_buffer: String,
    /// Whether we're in model selection sub-mode
    pub settings_model_selecting: bool,
    /// Models discovered from the provider's /models endpoint
    pub discovered_models: Option<Vec<String>>,
    /// Whether we're currently fetching models
    pub fetching_models: bool,
    /// Error message from model discovery
    pub models_fetch_error: Option<String>,
    /// Scroll offset for the model selection list
    pub model_scroll_offset: usize,
}

impl App {
    pub fn new(config: Config) -> Result<Self> {
        let provider_type = config.get_default_provider();
        let dev_mode = config.ui.developer_mode;

        // Try to create the provider, but don't fail if no API key is configured
        let (provider, model) = match create_provider(&provider_type, &config) {
            Ok(p) => {
                let model = p.model().to_string();
                (Some(p), model)
            }
            Err(_) => {
                let model = provider_type.default_model().to_string();
                (None, model)
            }
        };

        Ok(Self {
            screen: Screen::Chat,
            chat: ChatState::new(),
            config,
            current_provider: provider_type.clone(),
            current_model: model,
            dev_mode,
            last_metrics: None,
            is_streaming: false,
            stream_rx: None,
            stream_buffer: String::new(),
            provider,
            settings_tab: SettingsTab::Providers,
            settings_selected_provider: provider_type.clone(),
            last_tick: Instant::now(),
            scroll_offset: 0,
            settings_editing: false,
            settings_field_index: 0,
            settings_field_buffer: String::new(),
            settings_model_selecting: false,
            discovered_models: None,
            fetching_models: false,
            models_fetch_error: None,
            model_scroll_offset: 0,
        })
    }

    /// Switch to a different provider
    pub fn switch_provider(&mut self, provider_type: &ProviderType) -> Result<()> {
        self.current_provider = provider_type.clone();
        self.settings_selected_provider = provider_type.clone();

        match create_provider(provider_type, &self.config) {
            Ok(p) => {
                let model = p.model().to_string();
                self.current_model = model;
                self.provider = Some(p);
            }
            Err(_) => {
                self.current_model = provider_type.default_model().to_string();
                self.provider = None;
            }
        }
        Ok(())
    }

    /// Switch model for the current provider
    #[allow(dead_code)]
    pub fn switch_model(&mut self, model: &str) {
        self.current_model = model.to_string();
        if let Some(ref mut p) = self.provider {
            p.set_model(model);
        }
        // Also save to config
        let pc = self.config.get_provider_config_mut(&self.current_provider);
        pc.model = Some(model.to_string());
        let _ = self.config.save();
    }

    /// Send a message (non-streaming)
    #[allow(dead_code)]
    pub async fn send_message(&mut self, content: String) -> Result<()> {
        // Early check — borrow is released immediately
        if self.provider.is_none() {
            let msg = format!(
                "No API key configured for {}. Open settings (Ctrl+S) to set one up.",
                self.current_provider.display_name()
            );
            return Err(anyhow::anyhow!(msg));
        }

        let user_msg = Message::user(&content);
        self.chat.conversation.add_message(user_msg.clone());
        self.chat.streaming_content = String::new();
        self.chat.error_message = None;

        let messages = self.chat.conversation.get_messages();

        match self.provider.as_ref().unwrap().send_message(&messages).await {
            Ok((response, metrics)) => {
                self.last_metrics = Some(metrics.clone());
                let assistant_msg = Message::assistant(&response);
                self.chat.conversation.add_message(assistant_msg);
                Ok(())
            }
            Err(e) => {
                self.chat.error_message = Some(format!("Error: {}", e));
                Err(e)
            }
        }
    }

    /// Start streaming a message
    pub async fn start_stream(&mut self, content: String) -> Result<()> {
        // Early check — borrow is released immediately
        if self.provider.is_none() {
            let msg = format!(
                "🔑 No API key configured for {}.\nOpen settings (Ctrl+S) → Providers → select provider → Enter → type API key → Esc to save.",
                self.current_provider.display_name(),
            );
            self.chat.error_message = Some(msg.clone());
            return Err(anyhow::anyhow!(msg));
        }

        let user_msg = Message::user(&content);
        self.chat.conversation.add_message(user_msg.clone());
        self.is_streaming = true;
        self.stream_buffer = String::new();
        self.chat.streaming_content = String::new();
        self.chat.error_message = None;

        let messages = self.chat.conversation.get_messages();

        // Fresh borrow — no conflict since mutations are done
        match self.provider.as_ref().unwrap().stream_message(&messages).await {
            Ok(rx) => {
                self.stream_rx = Some(rx);
                Ok(())
            }
            Err(e) => {
                self.chat.error_message = Some(format!("Stream error: {}", e));
                self.is_streaming = false;
                Err(e)
            }
        }
    }

    /// Process incoming stream chunks (call this in the event loop)
    /// Consumes ALL available chunks in a loop so no data is left sitting in the channel buffer.
    pub async fn process_stream(&mut self) -> Result<()> {
        // Take ownership of the receiver so we can freely drain it
        let mut rx = match self.stream_rx.take() {
            Some(rx) => rx,
            None => return Ok(()),
        };

        loop {
            match rx.try_recv() {
                Ok(Ok(chunk)) => {
                    self.stream_buffer.push_str(&chunk.content);
                    self.chat.streaming_content = self.stream_buffer.clone();

                    if let Some(metrics) = chunk.metrics {
                        self.last_metrics = Some(metrics);
                    }

                    // Check if stream is done
                    if chunk.finish_reason.is_some() {
                        // Done streaming — commit the full accumulated content
                        if !self.stream_buffer.is_empty() {
                            let assistant_msg =
                                Message::assistant(&self.stream_buffer);
                            self.chat.conversation.add_message(assistant_msg);
                        }
                        self.stream_buffer.clear();
                        self.chat.streaming_content.clear();
                        self.is_streaming = false;
                        // Drain any leftover chunks that might have been sent after finish_reason
                        while rx.try_recv().is_ok() {}
                        // stream_rx stays None (channel is dead)
                        return Ok(());
                    }
                }
                Ok(Err(e)) => {
                    self.chat.error_message = Some(format!("Stream error: {}", e));
                    self.is_streaming = false;
                    // Don't return Err — the event loop should keep running so the
                    // user can see the error message and try again.
                    return Ok(());
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    // No more data right now — put the receiver back for the next tick
                    self.stream_rx = Some(rx);
                    return Ok(());
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Stream ended without a finish_reason — commit whatever we've got
                    if !self.stream_buffer.is_empty() {
                        let assistant_msg =
                            Message::assistant(&self.stream_buffer);
                        self.chat.conversation.add_message(assistant_msg);
                    }
                    self.stream_buffer.clear();
                    self.chat.streaming_content.clear();
                    self.is_streaming = false;
                    // stream_rx stays None (channel is dead)
                    return Ok(());
                }
            }
        }
    }

    /// Cancel the current stream
    pub fn cancel_stream(&mut self) {
        self.stream_rx = None;
        if !self.stream_buffer.is_empty() {
            let assistant_msg = Message::assistant(&self.stream_buffer);
            self.chat.conversation.add_message(assistant_msg);
        }
        self.stream_buffer.clear();
        self.chat.streaming_content.clear();
        self.is_streaming = false;
    }

    /// Clear the current conversation
    pub fn clear_conversation(&mut self) {
        self.chat.conversation.clear();
        self.chat.streaming_content.clear();
        self.last_metrics = None;
    }

    /// Toggle developer mode
    pub fn toggle_dev_mode(&mut self) {
        self.dev_mode = !self.dev_mode;
        self.config.ui.developer_mode = self.dev_mode;
        let _ = self.config.save();
    }

    /// Update a provider's config
    #[allow(dead_code)]
    pub fn update_provider_config(
        &mut self,
        provider_type: &ProviderType,
        api_key: Option<String>,
        model: Option<String>,
        base_url: Option<String>,
        temperature: Option<f64>,
    ) -> Result<()> {
        let pc = self.config.get_provider_config_mut(provider_type);
        if let Some(key) = api_key {
            pc.api_key = Some(key);
        }
        if let Some(m) = model.clone() {
            pc.model = Some(m);
        }
        if let Some(url) = base_url {
            pc.base_url = Some(url);
        }
        if let Some(temp) = temperature {
            pc.temperature = Some(temp);
        }

        self.config.save()?;

        // If we just updated the current provider, recreate it
        if *provider_type == self.current_provider {
            self.switch_provider(provider_type)?;
        }

        Ok(())
    }

    /// Check if the current provider is configured (has API key)
    #[allow(dead_code)]
    pub fn is_provider_configured(&self) -> bool {
        self.provider.is_some()
    }
}
