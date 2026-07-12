use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Provider type enum for all supported AI providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderType {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "groq")]
    Groq,
    #[serde(rename = "nvidia")]
    Nvidia,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "google")]
    Google,
    #[serde(rename = "deepseek")]
    DeepSeek,
    #[serde(rename = "mistral")]
    Mistral,
}

impl ProviderType {
    pub fn all() -> Vec<ProviderType> {
        vec![
            ProviderType::OpenAI,
            ProviderType::Groq,
            ProviderType::Nvidia,
            ProviderType::Anthropic,
            ProviderType::Google,
            ProviderType::DeepSeek,
            ProviderType::Mistral,
        ]
    }

    pub fn display_name(&self) -> &str {
        match self {
            ProviderType::OpenAI => "OpenAI",
            ProviderType::Groq => "Groq",
            ProviderType::Nvidia => "NVIDIA NIM",
            ProviderType::Anthropic => "Anthropic",
            ProviderType::Google => "Google Gemini",
            ProviderType::DeepSeek => "DeepSeek",
            ProviderType::Mistral => "Mistral AI",
        }
    }

    pub fn default_model(&self) -> &str {
        match self {
            ProviderType::OpenAI => "gpt-4o",
            ProviderType::Groq => "llama-3.3-70b-versatile",
            ProviderType::Nvidia => "nvidia/llama-3.1-nemotron-70b-instruct",
            ProviderType::Anthropic => "claude-sonnet-4-20250514",
            ProviderType::Google => "gemini-2.0-flash",
            ProviderType::DeepSeek => "deepseek-chat",
            ProviderType::Mistral => "mistral-large-latest",
        }
    }

    pub fn models(&self) -> Vec<&str> {
        match self {
            ProviderType::OpenAI => vec![
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-4-turbo",
                "gpt-4",
                "gpt-3.5-turbo",
                "o1",
                "o3-mini",
            ],
            ProviderType::Groq => vec![
                "llama-3.3-70b-versatile",
                "llama-3.1-8b-instant",
                "mixtral-8x7b-32768",
                "gemma2-9b-it",
                "llama-3.2-90b-vision-preview",
                "llama-3.2-11b-vision-preview",
                "llama-guard-3-8b",
            ],
            ProviderType::Nvidia => vec![
                "nvidia/llama-3.1-nemotron-70b-instruct",
                "mistralai/mixtral-8x22b-instruct-v0.1",
                "meta/llama-3.1-405b-instruct",
                "google/gemma-2-27b-it",
                "nvidia/nemotron-4-340b-instruct",
            ],
            ProviderType::Anthropic => vec![
                "claude-sonnet-4-20250514",
                "claude-3-5-sonnet-20241022",
                "claude-3-5-haiku-20241022",
                "claude-opus-4-20250514",
                "claude-3-opus-20240229",
            ],
            ProviderType::Google => vec![
                "gemini-2.0-flash",
                "gemini-2.0-flash-lite",
                "gemini-1.5-pro",
                "gemini-1.5-flash",
                "gemini-2.0-pro-exp",
            ],
            ProviderType::DeepSeek => vec![
                "deepseek-chat",
                "deepseek-reasoner",
                "deepseek-coder",
            ],
            ProviderType::Mistral => vec![
                "mistral-large-latest",
                "mistral-small-latest",
                "codestral-latest",
                "open-mistral-nemo",
            ],
        }
    }
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: Option<u32>,
}

fn default_max_tokens() -> Option<u32> {
    Some(4096)
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            model: None,
            temperature: None,
            max_tokens: Some(4096),
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIConfig {
    #[serde(default = "default_dev_mode")]
    pub developer_mode: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_scrollback")]
    pub scrollback_lines: usize,
}

fn default_dev_mode() -> bool {
    false
}

fn default_theme() -> String {
    "catppuccin".to_string()
}

fn default_scrollback() -> usize {
    1000
}

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            developer_mode: false,
            theme: "catppuccin".to_string(),
            scrollback_lines: 1000,
        }
    }
}

/// Keys configuration section for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeysConfig {
    pub openai: Option<String>,
    pub groq: Option<String>,
    pub nvidia: Option<String>,
    pub anthropic: Option<String>,
    pub google: Option<String>,
    pub deepseek: Option<String>,
    pub mistral: Option<String>,
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub openai: ProviderConfig,
    #[serde(default)]
    pub groq: ProviderConfig,
    #[serde(default)]
    pub nvidia: ProviderConfig,
    #[serde(default)]
    pub anthropic: ProviderConfig,
    #[serde(default)]
    pub google: ProviderConfig,
    #[serde(default)]
    pub deepseek: ProviderConfig,
    #[serde(default)]
    pub mistral: ProviderConfig,
    #[serde(default)]
    pub ui: UIConfig,
    #[serde(default)]
    pub default_provider: Option<ProviderType>,
    #[serde(default)]
    pub keys: Option<KeysConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai: ProviderConfig::default(),
            groq: ProviderConfig::default(),
            nvidia: ProviderConfig::default(),
            anthropic: ProviderConfig::default(),
            google: ProviderConfig::default(),
            deepseek: ProviderConfig::default(),
            mistral: ProviderConfig::default(),
            ui: UIConfig::default(),
            default_provider: Some(ProviderType::OpenAI),
            keys: None,
        }
    }
}

impl Config {
    /// Get the config directory path
    pub fn config_dir() -> Result<PathBuf> {
        let dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("opentui");
        Ok(dir)
    }

    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Load config from file, creating default if not found
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let contents = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config from {:?}", config_path))?;

        let mut config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config from {:?}", config_path))?;

        // Migrate legacy keys section to provider configs
        if let Some(keys) = config.keys.take() {
            if keys.openai.is_some() && config.openai.api_key.is_none() {
                config.openai.api_key = keys.openai;
            }
            if keys.groq.is_some() && config.groq.api_key.is_none() {
                config.groq.api_key = keys.groq;
            }
            if keys.nvidia.is_some() && config.nvidia.api_key.is_none() {
                config.nvidia.api_key = keys.nvidia;
            }
            if keys.anthropic.is_some() && config.anthropic.api_key.is_none() {
                config.anthropic.api_key = keys.anthropic;
            }
            if keys.google.is_some() && config.google.api_key.is_none() {
                config.google.api_key = keys.google;
            }
            if keys.deepseek.is_some() && config.deepseek.api_key.is_none() {
                config.deepseek.api_key = keys.deepseek;
            }
            if keys.mistral.is_some() && config.mistral.api_key.is_none() {
                config.mistral.api_key = keys.mistral;
            }
            // Save migrated config
            let _ = config.save();
        }

        Ok(config)
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        std::fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config directory {:?}", config_dir))?;

        let config_path = Self::config_path()?;
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(&config_path, contents)
            .with_context(|| format!("Failed to write config to {:?}", config_path))?;

        Ok(())
    }

    /// Get provider config by type
    pub fn get_provider_config(&self, provider: &ProviderType) -> &ProviderConfig {
        match provider {
            ProviderType::OpenAI => &self.openai,
            ProviderType::Groq => &self.groq,
            ProviderType::Nvidia => &self.nvidia,
            ProviderType::Anthropic => &self.anthropic,
            ProviderType::Google => &self.google,
            ProviderType::DeepSeek => &self.deepseek,
            ProviderType::Mistral => &self.mistral,
        }
    }

    /// Get mutable provider config by type
    #[allow(dead_code)]
    pub fn get_provider_config_mut(&mut self, provider: &ProviderType) -> &mut ProviderConfig {
        match provider {
            ProviderType::OpenAI => &mut self.openai,
            ProviderType::Groq => &mut self.groq,
            ProviderType::Nvidia => &mut self.nvidia,
            ProviderType::Anthropic => &mut self.anthropic,
            ProviderType::Google => &mut self.google,
            ProviderType::DeepSeek => &mut self.deepseek,
            ProviderType::Mistral => &mut self.mistral,
        }
    }

    /// Get the default provider
    pub fn get_default_provider(&self) -> ProviderType {
        self.default_provider.clone().unwrap_or(ProviderType::OpenAI)
    }
}
