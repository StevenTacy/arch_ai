use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    error::AppError,
    models::{Message, Role},
    provider::{AiProvider, SYSTEM_PROMPT},
};

pub struct GeminiProvider {
    http: reqwest::Client,
    config: Config,
}

impl GeminiProvider {
    pub fn new(config: Config) -> Self {
        Self {
            http: reqwest::Client::new(),
            config,
        }
    }

    fn api_url(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.config.model(),
            self.config.api_key()
        )
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<String, AppError> {
        let contents: Vec<GeminiContent> = messages
            .into_iter()
            .map(|m| GeminiContent {
                // Gemini uses "user" and "model" (not "assistant")
                role: match m.role() {
                    Role::User => "user",
                    Role::Assistant => "model",
                }
                .into(),
                parts: vec![Part { text: m.content().to_string() }],
            })
            .collect();

        let body = GeminiRequest {
            system_instruction: SystemInstruction {
                parts: vec![Part {
                    text: SYSTEM_PROMPT.into(),
                }],
            },
            contents,
            generation_config: GenerationConfig {
                max_output_tokens: self.config.max_tokens(),
            },
        };

        let response = self.http.post(self.api_url()).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable body>".into());
            return Err(AppError::ProviderError(format!("[Gemini] HTTP {status}: {text}")));
        }

        let resp: GeminiResponse = response.json().await?;
        resp.candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| AppError::ProviderError("[Gemini] no text part in response".into()))
    }
}

// ── Wire types ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GeminiRequest {
    system_instruction: SystemInstruction,
    contents: Vec<GeminiContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: GeminiResponseContent,
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    parts: Vec<Part>,
}
