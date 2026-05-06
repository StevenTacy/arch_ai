use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::AppError,
    models::Message,
    provider::{AiProvider, SYSTEM_PROMPT},
};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

pub struct ClaudeProvider {
    http: reqwest::Client,
    config: Config,
}

impl ClaudeProvider {
    pub fn new(config: Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }
}

#[async_trait]
impl AiProvider for ClaudeProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<String, AppError> {
        let body = ClaudeRequest {
            model: self.config.model(),
            max_tokens: self.config.max_tokens(),
            system: SYSTEM_PROMPT,
            messages,
        };

        let response = self
            .http
            .post(API_URL)
            .header("x-api-key", self.config.api_key())
            .header("anthropic-version", API_VERSION)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
            return Err(AppError::ProviderError(format!("[Claude] HTTP {status}: {text}")));
        }

        let resp: ClaudeResponse = response.json().await?;
        resp.content
            .into_iter()
            .find(|c| c.content_type == "text")
            .and_then(|c| c.text)
            .ok_or_else(|| AppError::ProviderError("[Claude] no text block in response".into()))
    }
}

// ── Wire types (private to this module) ──────────────────────────────────────

#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}
