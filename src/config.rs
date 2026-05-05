use std::str::FromStr;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    Claude,
    Gemini,
    OpenAi,
}

impl FromStr for ProviderKind {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "anthropic" => Ok(Self::Claude),
            "gemini" | "google" => Ok(Self::Gemini),
            "openai" | "codex" | "gpt" => Ok(Self::OpenAi),
            other => Err(AppError::Config(format!(
                "unknown AI_PROVIDER '{other}'; valid values: claude, gemini, openai"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub provider: ProviderKind,
    /// API key for the selected provider.
    pub api_key: String,
    pub port: u16,
    /// Model ID forwarded to the provider API.
    pub model: String,
    pub max_tokens: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let provider: ProviderKind = std::env::var("AI_PROVIDER")
            .unwrap_or_else(|_| "claude".into())
            .parse()?;

        let (key_var, default_model) = match provider {
            ProviderKind::Claude => ("ANTHROPIC_API_KEY", "claude-sonnet-4-6"),
            ProviderKind::Gemini => ("GEMINI_API_KEY", "gemini-2.0-flash"),
            ProviderKind::OpenAi => ("OPENAI_API_KEY", "gpt-4o"),
        };

        let api_key = std::env::var(key_var)
            .map_err(|_| AppError::Config(format!("{key_var} is not set")))?;

        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| default_model.into());

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse::<u16>()
            .map_err(|_| AppError::Config("PORT must be a valid port number (0–65535)".into()))?;

        let max_tokens = std::env::var("MAX_TOKENS")
            .unwrap_or_else(|_| "4096".into())
            .parse::<u32>()
            .map_err(|_| AppError::Config("MAX_TOKENS must be a positive integer".into()))?;

        Ok(Self {
            provider,
            api_key,
            port,
            model,
            max_tokens,
        })
    }
}
