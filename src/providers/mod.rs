pub mod anthropic;
pub mod deepseek;
pub mod google;
pub mod groq;
pub mod mistral;
pub mod nvidia;
pub mod openai;

use crate::chat::Message;
use crate::config::{Config, ProviderType};
use crate::dev_mode::Metrics;
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// A chunk of streaming response from an AI provider
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub finish_reason: Option<String>,
    pub metrics: Option<Metrics>,
}

/// Trait that all AI providers must implement
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider display name
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Get the provider type
    #[allow(dead_code)]
    fn provider_type(&self) -> ProviderType;

    /// Get the current model
    fn model(&self) -> &str;

    /// Set the model
    #[allow(dead_code)]
    fn set_model(&mut self, model: &str);

    /// Get available models for this provider
    #[allow(dead_code)]
    fn available_models(&self) -> Vec<&str>;

    /// Send a non-streaming chat completion request
    async fn send_message(
        &self,
        messages: &[Message],
    ) -> Result<(String, Metrics)>;

    /// Stream a chat completion response
    async fn stream_message(
        &self,
        messages: &[Message],
    ) -> Result<mpsc::Receiver<Result<StreamChunk>>>;

    /// Check if the provider is properly configured (has API key)
    #[allow(dead_code)]
    fn is_configured(&self) -> bool;
}

/// Create a provider instance from config
pub fn create_provider(
    provider_type: &ProviderType,
    config: &Config,
) -> Result<Box<dyn Provider>> {
    let provider_config = config.get_provider_config(provider_type);

    match provider_type {
        ProviderType::OpenAI => Ok(Box::new(openai::OpenAIProvider::new(
            provider_config,
            config,
        )?)),
        ProviderType::Groq => Ok(Box::new(groq::GroqProvider::new(
            provider_config,
            config,
        )?)),
        ProviderType::Nvidia => Ok(Box::new(nvidia::NvidiaProvider::new(
            provider_config,
            config,
        )?)),
        ProviderType::Anthropic => Ok(Box::new(anthropic::AnthropicProvider::new(
            provider_config,
            config,
        )?)),
        ProviderType::Google => Ok(Box::new(google::GoogleProvider::new(
            provider_config,
            config,
        )?)),
        ProviderType::DeepSeek => Ok(Box::new(deepseek::DeepSeekProvider::new(
            provider_config,
            config,
        )?)),
        ProviderType::Mistral => Ok(Box::new(mistral::MistralProvider::new(
            provider_config,
            config,
        )?)),
    }
}
