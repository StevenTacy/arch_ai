use std::str::FromStr;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    Claude,
    Gemini,
    OpenAi,
    /// Hardcoded demo responses — no network required.
    Mock,
    /// OpenRouter gateway — free models available; API key optional (falls back to Mock if absent).
    OpenRouter,
}

impl FromStr for ProviderKind {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "anthropic" => Ok(Self::Claude),
            "gemini" | "google" => Ok(Self::Gemini),
            "openai" | "codex" | "gpt" => Ok(Self::OpenAi),
            "mock" | "demo" | "stub" => Ok(Self::Mock),
            "openrouter" | "or" => Ok(Self::OpenRouter),
            other => Err(AppError::Config(format!(
                "unknown AI_PROVIDER '{other}'; valid values: claude, gemini, openai"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub provider: ProviderKind,
    pub api_key: String,
    pub port: u16,
    /// Model ID forwarded to the provider API.
    pub model: String,
    pub max_tokens: u32,
    /// PostgreSQL connection string. If absent, RAG is disabled.
    pub database_url: Option<String>,
    /// Redis connection string. If absent, /v2/chat returns 503.
    pub redis_url: Option<String>,
    /// Number of law chunks to retrieve per query.
    pub rag_top_k: i64,
    /// Redis session TTL in seconds.
    pub session_ttl_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self, AppError> {
        let provider: ProviderKind = std::env::var("AI_PROVIDER")
            .unwrap_or_else(|_| "claude".into())
            .parse()?;

        let (api_key, default_model) = match provider {
            ProviderKind::Claude => (
                std::env::var("ANTHROPIC_API_KEY")
                    .map_err(|_| AppError::Config("ANTHROPIC_API_KEY is not set".into()))?,
                "claude-sonnet-4-6",
            ),
            ProviderKind::Gemini => (
                std::env::var("GEMINI_API_KEY")
                    .map_err(|_| AppError::Config("GEMINI_API_KEY is not set".into()))?,
                "gemini-2.0-flash",
            ),
            ProviderKind::OpenAi => (
                std::env::var("OPENAI_API_KEY")
                    .map_err(|_| AppError::Config("OPENAI_API_KEY is not set".into()))?,
                "gpt-4o",
            ),
            ProviderKind::Mock => (String::new(), "mock"),
            // Key is optional — empty string triggers Mock fallback in main.rs
            ProviderKind::OpenRouter => (
                std::env::var("OPENROUTER_API_KEY").unwrap_or_default(),
                "auto",
            ),
        };

        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| default_model.into());

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "8080".into())
            .parse::<u16>()
            .map_err(|_| AppError::Config("PORT must be a valid port number (0–65535)".into()))?;

        let max_tokens = std::env::var("MAX_TOKENS")
            .unwrap_or_else(|_| "4096".into())
            .parse::<u32>()
            .map_err(|_| AppError::Config("MAX_TOKENS must be a positive integer".into()))?;

        let database_url = std::env::var("DATABASE_URL").ok();
        let redis_url = std::env::var("REDIS_URL").ok();

        let rag_top_k = std::env::var("RAG_TOP_K")
            .unwrap_or_else(|_| "5".into())
            .parse::<i64>()
            .map_err(|_| AppError::Config("RAG_TOP_K must be a positive integer".into()))?;

        let session_ttl_secs = std::env::var("SESSION_TTL_SECS")
            .unwrap_or_else(|_| "3600".into())
            .parse::<u64>()
            .map_err(|_| AppError::Config("SESSION_TTL_SECS must be a positive integer".into()))?;

        let ollama_base_url =
            std::env::var("OLLAMA_BASE_URL").unwrap_or_else(|_| "http://localhost:11434/v1".into());

        let openai_base_url =
            std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".into());

        Ok(Self {
            provider,
            api_key,
            port,
            model,
            max_tokens,
            database_url,
            redis_url,
            rag_top_k,
            session_ttl_secs,
        })
    }
}
