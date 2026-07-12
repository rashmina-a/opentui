use crate::chat::Message;
use crate::config::{Config, ProviderConfig, ProviderType};
use crate::dev_mode::Metrics;
use crate::providers::{Provider, StreamChunk};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use std::time::Instant;
use tokio::sync::mpsc;

const ANTHROPIC_API: &str = "https://api.anthropic.com/v1";

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
    temperature: f64,
    max_tokens: u32,
}

impl AnthropicProvider {
    pub fn new(provider_config: &ProviderConfig, _config: &Config) -> Result<Self> {
        let api_key = provider_config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .context(
                "Anthropic API key not configured. Set it in config or ANTHROPIC_API_KEY env var.",
            )?;

        let model = provider_config
            .model
            .clone()
            .unwrap_or_else(|| ProviderType::Anthropic.default_model().to_string());

        let temperature = provider_config.temperature.unwrap_or(0.7);
        let max_tokens = provider_config.max_tokens.unwrap_or(4096);

        Ok(Self {
            client: Client::new(),
            api_key,
            model,
            temperature,
            max_tokens,
        })
    }

    fn convert_messages(messages: &[Message]) -> (Option<String>, Vec<serde_json::Value>) {
        let mut system: Option<String> = None;
        let mut converted = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    system = Some(msg.content.clone());
                }
                "user" | "assistant" => {
                    converted.push(serde_json::json!({
                        "role": msg.role,
                        "content": msg.content,
                    }));
                }
                _ => {
                    converted.push(serde_json::json!({
                        "role": "user",
                        "content": msg.content,
                    }));
                }
            }
        }

        (system, converted)
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    fn available_models(&self) -> Vec<&str> {
        ProviderType::Anthropic.models()
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn send_message(&self, messages: &[Message]) -> Result<(String, Metrics)> {
        let start = Instant::now();
        let (system, messages) = Self::convert_messages(messages);

        let mut request_body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
        });

        if let Some(sys) = system {
            request_body["system"] = serde_json::Value::String(sys);
        }

        let response = self
            .client
            .post(format!("{}/messages", ANTHROPIC_API))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        if !status.is_success() {
            let error_msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error");
            anyhow::bail!("Anthropic API error ({}): {}", status, error_msg);
        }

        let content = body["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let elapsed = start.elapsed();
        let usage = &body["usage"];
        let prompt_tokens = usage["input_tokens"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = usage["output_tokens"].as_u64().unwrap_or(0) as u32;
        let total_tokens = prompt_tokens + completion_tokens;

        let metrics = Metrics::new(prompt_tokens, completion_tokens, total_tokens, elapsed);

        Ok((content, metrics))
    }

    async fn stream_message(&self, messages: &[Message]) -> Result<mpsc::Receiver<Result<StreamChunk>>> {
        let (tx, rx) = mpsc::channel::<Result<StreamChunk>>(64);
        let (system, messages) = Self::convert_messages(messages);

        let mut request_body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "stream": true,
        });

        if let Some(sys) = system {
            request_body["system"] = serde_json::Value::String(sys);
        }

        let client = self.client.clone();
        let api_key = self.api_key.clone();

        tokio::spawn(async move {
            let start = Instant::now();
            let mut prompt_tokens: u32 = 0;
            let mut completion_tokens: u32 = 0;

            match client
                .post(format!("{}/messages", ANTHROPIC_API))
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        let body_text = response.text().await.unwrap_or_default();
                        let _ = tx
                            .send(Err(anyhow::anyhow!(
                                "Anthropic API error ({}): {}",
                                status,
                                body_text
                            )))
                            .await;
                        return;
                    }

                    let mut stream = response.bytes_stream();
                    let mut buffer = String::new();
                    let mut current_content = String::new();

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(bytes) => {
                                let chunk_str = String::from_utf8_lossy(&bytes);
                                buffer.push_str(&chunk_str);

                                while let Some(line_end) = buffer.find('\n') {
                                    let line = buffer[..line_end].trim().to_string();
                                    buffer = buffer[line_end + 1..].to_string();

                                    if line.is_empty() {
                                        continue;
                                    }

                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if let Ok(json) =
                                            serde_json::from_str::<serde_json::Value>(data)
                                        {
                                            let event_type = json["type"].as_str().unwrap_or("");

                                            match event_type {
                                                "content_block_delta" => {
                                                    if let Some(text) = json["delta"]["text"]
                                                        .as_str()
                                                    {
                                                        current_content.push_str(text);
                                                        let _ = tx
                                                            .send(Ok(StreamChunk {
                                                                content: text.to_string(),
                                                                finish_reason: None,
                                                                metrics: None,
                                                            }))
                                                            .await;
                                                    }
                                                }
                                                "message_delta" => {
                                                    if let Some(usage) =
                                                        json["usage"].as_object()
                                                    {
                                                        prompt_tokens = usage["input_tokens"]
                                                            .as_u64()
                                                            .unwrap_or(0)
                                                            as u32;
                                                        completion_tokens = usage["output_tokens"]
                                                            .as_u64()
                                                            .unwrap_or(0)
                                                            as u32;
                                                    }

                                                    let stop_reason =
                                                        json["delta"]["stop_reason"]
                                                            .as_str()
                                                            .map(|s| s.to_string());

                                                    let metrics = Metrics::new(
                                                        prompt_tokens,
                                                        completion_tokens,
                                                        prompt_tokens + completion_tokens,
                                                        start.elapsed(),
                                                    );

                                                    let _ = tx
                                                        .send(Ok(StreamChunk {
                                                            content: String::new(),
                                                            finish_reason: stop_reason,
                                                            metrics: Some(metrics),
                                                        }))
                                                        .await;
                                                }
                                                "error" => {
                                                    let error_msg =
                                                        json["error"]["message"]
                                                            .as_str()
                                                            .unwrap_or("Unknown error");
                                                    let _ = tx
                                                        .send(Err(anyhow::anyhow!(
                                                            "Anthropic error: {}",
                                                            error_msg
                                                        )))
                                                        .await;
                                                    return;
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(Err(anyhow::anyhow!("Stream error: {}", e))).await;
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(anyhow::anyhow!("Request failed: {}", e))).await;
                }
            }
        });

        Ok(rx)
    }
}
