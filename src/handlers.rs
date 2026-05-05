use std::sync::Arc;

use axum::{Json, extract::State};

use crate::{
    error::AppError,
    models::{ChatRequest, ChatResponse},
    provider::AiProvider,
};

pub type AppState = Arc<dyn AiProvider + Send + Sync>;

pub async fn chat(
    State(provider): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, AppError> {
    let reply = provider.chat(req.messages).await?;
    Ok(Json(ChatResponse { message: reply }))
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
