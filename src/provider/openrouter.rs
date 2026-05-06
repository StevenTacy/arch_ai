use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::AppError,
    models::{Message, Role},
    provider::{AiProvider, SYSTEM_PROMPT},
};

const BASE_URL: &str = "https://openrouter.ai/api/v1";
const APP_TITLE: &str = "Taiwan Construction Law AI";

pub struct OpenRouterProvider {
    http: reqwest::Client,
    config: Config,
}

impl OpenRouterProvider {
    pub fn new(config: Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }
}

/// Queries OpenRouter's model list and returns the id of the best free model
/// (highest context_length where prompt and completion pricing are both "0").
pub async fn discover_free_model(config: &Config) -> Result<String, AppError> {
    let http = reqwest::Client::new();

    let response = http
        .get(format!("{BASE_URL}/models"))
        .bearer_auth(&config.api_key)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(AppError::ProviderError(format!(
            "[OpenRouter] model list HTTP {status}: {text}"
        )));
    }

    let list: ModelList = response.json().await?;

    let mut free: Vec<ModelInfo> = list
        .data
        .into_iter()
        .filter(|m| m.pricing.prompt == "0" && m.pricing.completion == "0")
        .collect();

    // Prefer bigger context windows for law Q&A (long articles/history)
    free.sort_by(|a, b| {
        b.context_length
            .unwrap_or(0)
            .cmp(&a.context_length.unwrap_or(0))
    });

    free.into_iter()
        .next()
        .map(|m| m.id)
        .ok_or_else(|| AppError::ProviderError("[OpenRouter] no free models found".into()))
}

#[async_trait]
impl AiProvider for OpenRouterProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<String, AppError> {
        let mut oai_messages = Vec::with_capacity(messages.len() + 1);
        oai_messages.push(OaiMessage {
            role: "system",
            content: SYSTEM_PROMPT.into(),
        });
        for m in messages {
            oai_messages.push(OaiMessage {
                role: match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                content: m.content,
            });
        }

        let body = OaiRequest {
            model: &self.config.model,
            messages: oai_messages,
            max_tokens: self.config.max_tokens,
        };

        let response = self
            .http
            .post(format!("{BASE_URL}/chat/completions"))
            .bearer_auth(&self.config.api_key)
            .header("HTTP-Referer", "https://github.com/arch-ai")
            .header("X-Title", APP_TITLE)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::ProviderError(format!(
                "[OpenRouter] HTTP {status}: {text}"
            )));
        }

        let resp: OaiResponse = response.json().await?;
        resp.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| AppError::ProviderError("[OpenRouter] no choices in response".into()))
    }
}

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OaiRequest<'a> {
    model: &'a str,
    messages: Vec<OaiMessage>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct OaiMessage {
    role: &'static str,
    content: String,
}

#[derive(Deserialize)]
struct OaiResponse {
    choices: Vec<OaiChoice>,
}

#[derive(Deserialize)]
struct OaiChoice {
    message: OaiReply,
}

#[derive(Deserialize)]
struct OaiReply {
    content: String,
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelInfo>,
}

#[derive(Deserialize)]
struct ModelInfo {
    id: String,
    pricing: ModelPricing,
    context_length: Option<u64>,
}

#[derive(Deserialize)]
struct ModelPricing {
    prompt: String,
    completion: String,
}
