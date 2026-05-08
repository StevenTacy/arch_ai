use std::str::FromStr;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    Claude,
    Gemini,
    OpenAi,
    /// OpenRouter gateway — free models available.
    OpenRouter,
}

impl FromStr for ProviderKind {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" | "anthropic" => Ok(Self::Claude),
            "gemini" | "google" => Ok(Self::Gemini),
            "openai" | "codex" | "gpt" => Ok(Self::OpenAi),
            "openrouter" | "or" => Ok(Self::OpenRouter),
            other => Err(AppError::Config(format!(
                "unknown AI_PROVIDER '{other}'; valid values: claude, gemini, openai, openrouter"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    provider: ProviderKind,
    api_key: String,
    port: u16,
    model: String,
    max_tokens: u32,
    redis_url: Option<String>,
    session_ttl_secs: u64,
}

impl Config {
    pub fn provider(&self) -> &ProviderKind {
        &self.provider
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    pub fn redis_url(&self) -> Option<&str> {
        self.redis_url.as_deref()
    }

    pub fn session_ttl_secs(&self) -> u64 {
        self.session_ttl_secs
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Constructs [`Config`] by reading all fields from the process environment.
    ///
    /// Reads `AI_PROVIDER` first to determine which provider's API key and default model
    /// to use, then resolves remaining variables with the defaults shown below.
    ///
    /// | Variable | Required | Default |
    /// |---|---|---|
    /// | `AI_PROVIDER` | no | `"claude"` |
    /// | `ANTHROPIC_API_KEY` | if provider is `claude` | — |
    /// | `GEMINI_API_KEY` | if provider is `gemini` | — |
    /// | `OPENAI_API_KEY` | if provider is `openai` | — |
    /// | `OPENROUTER_API_KEY` | if provider is `openrouter` | — |
    /// | `AI_MODEL` | no | provider-specific default |
    /// | `PORT` | no | `8080` |
    /// | `MAX_TOKENS` | no | `4096` |
    /// | `REDIS_URL` | no | `None` |
    /// | `SESSION_TTL_SECS` | no | `3600` |
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Config`] if:
    /// - `AI_PROVIDER` holds an unrecognised value.
    /// - The required API key for the resolved provider is absent.
    /// - `PORT` cannot be parsed as a `u16`.
    /// - `MAX_TOKENS` cannot be parsed as a `u32`.
    /// - `SESSION_TTL_SECS` cannot be parsed as a `u64`.
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
            ProviderKind::OpenRouter => (
                std::env::var("OPENROUTER_API_KEY")
                    .map_err(|_| AppError::Config("OPENROUTER_API_KEY is not set".into()))?,
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

        let redis_url = std::env::var("REDIS_URL").ok();

        let session_ttl_secs = std::env::var("SESSION_TTL_SECS")
            .unwrap_or_else(|_| "3600".into())
            .parse::<u64>()
            .map_err(|_| AppError::Config("SESSION_TTL_SECS must be a positive integer".into()))?;

        Ok(Self {
            provider,
            api_key,
            port,
            model,
            max_tokens,
            redis_url,
            session_ttl_secs,
        })
    }
}
