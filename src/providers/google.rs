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

const GEMINI_API: &str = "https://generativelanguage.googleapis.com/v1beta";

pub struct GoogleProvider {
    client: Client,
    api_key: String,
    model: String,
    temperature: f64,
    max_tokens: u32,
}

impl GoogleProvider {
    pub fn new(provider_config: &ProviderConfig, _config: &Config) -> Result<Self> {
        let api_key = provider_config
            .api_key
            .clone()
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .context("Google API key not configured. Set it in config or GOOGLE_API_KEY env var.")?;

        let model = provider_config
            .model
            .clone()
            .unwrap_or_else(|| ProviderType::Google.default_model().to_string());

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

    fn convert_messages(messages: &[Message]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role.as_str() {
                    "assistant" => "model",
                    r => r,
                };
                serde_json::json!({
                    "role": role,
                    "parts": [{"text": m.content}]
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for GoogleProvider {
    fn name(&self) -> &str {
        "Google Gemini"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Google
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    fn available_models(&self) -> Vec<&str> {
        ProviderType::Google.models()
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn send_message(&self, messages: &[Message]) -> Result<(String, Metrics)> {
        let start = Instant::now();
        let contents = Self::convert_messages(messages);

        let request_body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": self.temperature,
                "maxOutputTokens": self.max_tokens,
            }
        });

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            GEMINI_API, self.model, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Google Gemini")?;

        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Google Gemini response")?;

        if !status.is_success() {
            let error_msg = body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error");
            anyhow::bail!("Google Gemini API error ({}): {}", status, error_msg);
        }

        let content = body["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let elapsed = start.elapsed();
        let usage = &body["usageMetadata"];
        let prompt_tokens = usage["promptTokenCount"].as_u64().unwrap_or(0) as u32;
        let completion_tokens = usage["candidatesTokenCount"].as_u64().unwrap_or(0) as u32;
        let total_tokens = usage["totalTokenCount"].as_u64().unwrap_or(0) as u32;

        let metrics = Metrics::new(prompt_tokens, completion_tokens, total_tokens, elapsed);

        Ok((content, metrics))
    }

    async fn stream_message(&self, messages: &[Message]) -> Result<mpsc::Receiver<Result<StreamChunk>>> {
        let (tx, rx) = mpsc::channel::<Result<StreamChunk>>(64);
        let contents = Self::convert_messages(messages);

        let request_body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": self.temperature,
                "maxOutputTokens": self.max_tokens,
            }
        });

        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            GEMINI_API, self.model, self.api_key
        );

        let client = self.client.clone();

        tokio::spawn(async move {
            let start = Instant::now();
            let mut prompt_tokens: u32 = 0;
            let mut completion_tokens: u32 = 0;

            match client
                .post(&url)
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
                                "Google Gemini API error ({}): {}",
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

                                    if line.is_empty() {
                                        continue;
                                    }

                                    if let Some(data) = line.strip_prefix("data: ") {
                                        if let Ok(json) =
                                            serde_json::from_str::<serde_json::Value>(data)
                                        {
                                            if let Some(candidates) =
                                                json["candidates"].as_array()
                                            {
                                                if let Some(candidate) = candidates.first() {
                                                    let content = candidate["content"]["parts"]
                                                        [0]["text"]
                                                        .as_str()
                                                        .unwrap_or("")
                                                        .to_string();

                                                    let finish_reason =
                                                        candidate["finishReason"]
                                                            .as_str()
                                                            .map(|s| s.to_string());

                                                    if let Some(usage) =
                                                        json["usageMetadata"].as_object()
                                                    {
                                                        prompt_tokens = usage["promptTokenCount"]
                                                            .as_u64()
                                                            .unwrap_or(0)
                                                            as u32;
                                                        completion_tokens =
                                                            usage["candidatesTokenCount"]
                                                                .as_u64()
                                                                .unwrap_or(0)
                                                                as u32;
                                                    }

                                                    let metrics = if prompt_tokens > 0
                                                        || completion_tokens > 0
                                                    {
                                                        Some(Metrics::new(
                                                            prompt_tokens,
                                                            completion_tokens,
                                                            prompt_tokens + completion_tokens,
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
