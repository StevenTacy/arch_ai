use std::sync::Arc;

use axum::Json;
use redis::aio::ConnectionManager;

use crate::provider::AiProvider;

#[derive(Clone)]
pub struct AppState {
    provider: Arc<dyn AiProvider + Send + Sync>,
    /// Present when REDIS_URL is set; required for session-based chat.
    redis: Option<ConnectionManager>,
    session_ttl_secs: u64,
}

impl AppState {
    pub fn new(
        provider: Arc<dyn AiProvider + Send + Sync>,
        redis: Option<ConnectionManager>,
        session_ttl_secs: u64,
    ) -> Self {
        Self { provider, redis, session_ttl_secs }
    }

    pub fn provider(&self) -> &dyn AiProvider {
        self.provider.as_ref()
    }

    /// Returns a cloned connection manager handle. `ConnectionManager` is designed to be cloned.
    pub fn redis(&self) -> Option<ConnectionManager> {
        self.redis.clone()
    }

    pub fn session_ttl_secs(&self) -> u64 {
        self.session_ttl_secs
    }
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
