use std::sync::Arc;

use axum::Json;
use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::provider::AiProvider;

#[derive(Clone)]
pub struct AppState {
    provider: Arc<dyn AiProvider + Send + Sync>,
    /// Present when DATABASE_URL is set; enables law chunk retrieval.
    db: Option<PgPool>,
    /// Present when REDIS_URL is set; required for session-based chat.
    redis: Option<ConnectionManager>,
    rag_top_k: i64,
    session_ttl_secs: u64,
}

impl AppState {
    pub fn new(
        provider: Arc<dyn AiProvider + Send + Sync>,
        db: Option<PgPool>,
        redis: Option<ConnectionManager>,
        rag_top_k: i64,
        session_ttl_secs: u64,
    ) -> Self {
        Self { provider, db, redis, rag_top_k, session_ttl_secs }
    }

    pub fn provider(&self) -> &dyn AiProvider {
        self.provider.as_ref()
    }

    pub fn db(&self) -> Option<&PgPool> {
        self.db.as_ref()
    }

    /// Returns a cloned connection manager handle. `ConnectionManager` is designed to be cloned.
    pub fn redis(&self) -> Option<ConnectionManager> {
        self.redis.clone()
    }

    pub fn rag_top_k(&self) -> i64 {
        self.rag_top_k
    }

    pub fn session_ttl_secs(&self) -> u64 {
        self.session_ttl_secs
    }
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
