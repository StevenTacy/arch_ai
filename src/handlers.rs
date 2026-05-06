use std::sync::Arc;

use axum::{Json, extract::State};
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::{
    db, session,
    error::AppError,
    models::{ChatRequest, ChatResponse, ChatRequestV2, ChatResponseV2, Message, Role},
    provider::AiProvider,
};

#[derive(Clone)]
pub struct AppState {
    pub provider: Arc<dyn AiProvider + Send + Sync>,
    /// Present when DATABASE_URL is set; enables law chunk retrieval.
    pub db: Option<PgPool>,
    /// Present when REDIS_URL is set; required for /v2/chat.
    pub redis: Option<ConnectionManager>,
    pub rag_top_k: i64,
    pub session_ttl_secs: u64,
}

/// Legacy stateless endpoint — client owns full conversation history.
pub async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, AppError> {
    let reply = state.provider.chat(req.messages).await?;
    Ok(Json(ChatResponse { message: reply }))
}

/// Stateful endpoint — server owns history in Redis, retrieves relevant law chunks per turn.
pub async fn chat_v2(
    State(state): State<AppState>,
    Json(req): Json<ChatRequestV2>,
) -> Result<Json<ChatResponseV2>, AppError> {
    let mut redis = state
        .redis
        .ok_or_else(|| AppError::Session("REDIS_URL not configured".into()))?;

    let session_id = req.session_id.clone().unwrap_or_else(session::new_session_id);
    let mut session_messages = session::get_session(&mut redis, &session_id).await?;

    // RAG: full-text search if DB is available; degrade gracefully on failure
    let rag_context = if let Some(ref pool) = state.db {
        match db::search_law(pool, &req.message, state.rag_top_k).await {
            Ok(chunks) if !chunks.is_empty() => Some(db::format_chunks(&chunks)),
            Ok(_) => None,
            Err(e) => {
                tracing::warn!(error = %e, "law search failed, proceeding without RAG");
                None
            }
        }
    } else {
        None
    };

    // Build the message list sent to the AI provider.
    // RAG context is injected as an ephemeral exchange at the front — not saved to session —
    // so retrieved chunks don't accumulate in history across turns.
    let mut api_messages: Vec<Message> = Vec::new();
    if let Some(ctx) = rag_context {
        api_messages.push(Message {
            role: Role::User,
            content: format!("[法條參考資料]\n{ctx}"),
        });
        api_messages.push(Message {
            role: Role::Assistant,
            content: "已閱讀法條參考資料，請提問。".into(),
        });
    }
    api_messages.extend(session_messages.clone());
    api_messages.push(Message {
        role: Role::User,
        content: req.message.clone(),
    });

    let reply = state.provider.chat(api_messages).await?;

    // Persist only real user/assistant turns to Redis
    session_messages.push(Message {
        role: Role::User,
        content: req.message,
    });
    session_messages.push(Message {
        role: Role::Assistant,
        content: reply.clone(),
    });
    session::save_session(&mut redis, &session_id, &session_messages, state.session_ttl_secs)
        .await?;

    Ok(Json(ChatResponseV2 {
        session_id,
        message: reply,
    }))
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
