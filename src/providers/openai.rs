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

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    temperature: f64,
    max_tokens: u32,
}

impl OpenAIProvider {
    pub fn new(provider_config: &ProviderConfig, _config: &Config) -> Result<Self> {
        let api_key = provider_config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .context("OpenAI API key not configured. Set it in config or OPENAI_API_KEY env var.")?;

        let base_url = provider_config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        let model = provider_config
            .model
            .clone()
            .unwrap_or_else(|| ProviderType::OpenAI.default_model().to_string());

        let temperature = provider_config.temperature.unwrap_or(0.7);
        let max_tokens = provider_config.max_tokens.unwrap_or(4096);

        Ok(Self {
            client: Client::new(),
            api_key,
            base_url,
            model,
            temperature,
            max_tokens,
        })
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    fn available_models(&self) -> Vec<&str> {
        ProviderType::OpenAI.models()
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn send_message(&self, messages: &[Message]) -> Result<(String, Metrics)> {
        let start = Instant::now();

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
        });

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        if !status.is_success() {
            let error_msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error");
            anyhow::bail!("OpenAI API error ({}): {}", status, error_msg);
        }

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let elapsed = start.elapsed();
        let usage = &body["usage"];
        let prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(0) as u32;
        let total_tokens = usage["total_tokens"].as_u64().unwrap_or(0) as u32;

        let metrics = Metrics::new(
            prompt_tokens,
            completion_tokens,
            total_tokens,
            elapsed,
        );

        Ok((content, metrics))
    }

    async fn stream_message(&self, messages: &[Message]) -> Result<mpsc::Receiver<Result<StreamChunk>>> {
        let (tx, rx) = mpsc::channel::<Result<StreamChunk>>(64);

        let request_body = serde_json::json!({
            "model": self.model,
            "messages": messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            }).collect::<Vec<_>>(),
            "temperature": self.temperature,
            "max_tokens": self.max_tokens,
            "stream": true,
        });

        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();

        tokio::spawn(async move {
            let start = Instant::now();
            let mut prompt_tokens: u32 = 0;
            let mut completion_tokens: u32 = 0;
            let mut total_tokens: u32 = 0;

            match client
                .post(format!("{}/chat/completions", base_url))
                .header("Authorization", format!("Bearer {}", api_key))
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
                                "OpenAI API error ({}): {}",
                                status,
                                body_text
                            )))
                            .await;
                        return;
                    }

                    let mut stream = response.bytes_stream();
                    let mut buffer = String::new();

                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(bytes) => {
                                let chunk_str = String::from_utf8_lossy(&bytes);
                                buffer.push_str(&chunk_str);

                                while let Some(line_end) = buffer.find('\n') {
                                    let line = buffer[..line_end].trim().to_string();
                                    buffer = buffer[line_end + 1..].to_string();

                                    if line.is_empty() || line == "data: [DONE]" {
                                        continue;
                                    }

                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if let Ok(json) =
                                            serde_json::from_str::<serde_json::Value>(data)
                                        {
                                            let content = json["choices"][0]["delta"]["content"]
                                                .as_str()
                                                .unwrap_or("")
                                                .to_string();

                                            if let Some(usage) = json["usage"].as_object() {
                                                prompt_tokens =
                                                    usage["prompt_tokens"].as_u64().unwrap_or(0)
                                                        as u32;
                                                completion_tokens = usage["completion_tokens"]
                                                    .as_u64()
                                                    .unwrap_or(0)
                                                    as u32;
                                                total_tokens = usage["total_tokens"]
                                                    .as_u64()
                                                    .unwrap_or(0)
                                                    as u32;
                                            }

                                            let finish_reason = json["choices"][0]["finish_reason"]
                                                .as_str()
                                                .map(|s| s.to_string());

                                            let metrics = if total_tokens > 0 {
                                                Some(Metrics::new(
                                                    prompt_tokens,
                                                    completion_tokens,
                                                    total_tokens,
                                                    start.elapsed(),
                                                ))
                                            } else {
                                                None
                                            };

                                            let _ = tx
                                                .send(Ok(StreamChunk {
                                                    content,
                                                    finish_reason,
                                                    metrics,
                                                }))
                                                .await;
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

                    // Send final metrics if not already sent
                    if total_tokens == 0 {
                        let metrics = Metrics::new(
                            prompt_tokens,
                            completion_tokens,
                            total_tokens,
                            start.elapsed(),
                        );
                        let _ = tx
                            .send(Ok(StreamChunk {
                                content: String::new(),
                                finish_reason: Some("stop".to_string()),
                                metrics: Some(metrics),
                            }))
                            .await;
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
