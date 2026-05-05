use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::AppError,
    models::{Message, Role},
    provider::{AiProvider, SYSTEM_PROMPT},
};

const API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAiProvider {
    http: reqwest::Client,
    config: Config,
}

impl OpenAiProvider {
    pub fn new(config: Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<String, AppError> {
        // Prepend the system message, then map conversation history.
        let mut oai_messages = Vec::with_capacity(messages.len() + 1);
        oai_messages.push(OpenAiMessage {
            role: "system",
            content: SYSTEM_PROMPT.into(),
        });
        for m in messages {
            oai_messages.push(OpenAiMessage {
                role: match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: m.content,
            });
        }

        let body = OpenAiRequest {
            model: &self.config.model,
            messages: oai_messages,
            max_tokens: self.config.max_tokens,
        };

        let response = self
            .http
            .post(API_URL)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
            return Err(AppError::ProviderError(format!("[OpenAI] HTTP {status}: {text}")));
        }

        let resp: OpenAiResponse = response.json().await?;
        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| AppError::ProviderError("[OpenAI] no choices in response".into()))
    }
}

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: OpenAiReply,
}

#[derive(Deserialize)]
struct OpenAiReply {
    content: String,
}
