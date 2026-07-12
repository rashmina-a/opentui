use crate::chat::Message;
use crate::config::{Config, ProviderConfig, ProviderType};
use crate::dev_mode::Metrics;
use crate::providers::openai::OpenAIProvider;
use crate::providers::{Provider, StreamChunk};
use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::sync::mpsc;

pub struct GroqProvider {
    inner: OpenAIProvider,
}

impl GroqProvider {
    pub fn new(provider_config: &ProviderConfig, _config: &Config) -> Result<Self> {
        let api_key = provider_config
            .api_key
            .clone()
            .or_else(|| std::env::var("GROQ_API_KEY").ok())
            .context("Groq API key not configured. Set it in config or GROQ_API_KEY env var.")?;

        let model = provider_config
            .model
            .clone()
            .unwrap_or_else(|| ProviderType::Groq.default_model().to_string());

        let temperature = provider_config.temperature.unwrap_or(0.7);
        let max_tokens = provider_config.max_tokens.unwrap_or(4096);

        // Create an OpenAI-compatible config for the inner provider
        let openai_config = ProviderConfig {
            api_key: Some(api_key),
            base_url: Some("https://api.groq.com/openai/v1".to_string()),
            model: Some(model),
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
        };

        let inner = OpenAIProvider::new(&openai_config, _config)?;

        Ok(Self { inner })
    }
}

#[async_trait]
impl Provider for GroqProvider {
    fn name(&self) -> &str {
        "Groq"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Groq
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    fn set_model(&mut self, model: &str) {
        self.inner.set_model(model);
    }

    fn available_models(&self) -> Vec<&str> {
        ProviderType::Groq.models()
    }

    fn is_configured(&self) -> bool {
        self.inner.is_configured()
    }

    async fn send_message(&self, messages: &[Message]) -> Result<(String, Metrics)> {
        self.inner.send_message(messages).await
    }

    async fn stream_message(&self, messages: &[Message]) -> Result<mpsc::Receiver<Result<StreamChunk>>> {
        self.inner.stream_message(messages).await
    }
}
