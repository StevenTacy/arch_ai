use crate::provider::AiProvider;
use redis::aio::ConnectionManager;
use std::sync::Arc;

/// Shared application state injected into every axum handler via [`axum::extract::State`].
///
/// Constructed once at startup and cloned cheaply into each request task.
/// The AI provider is held behind an `Arc` so the same connection pool or client
/// is reused across all concurrent requests.
#[derive(Clone)]
pub struct AppState {
    provider: Arc<dyn AiProvider + Send + Sync>,
    /// Present when `REDIS_URL` is set; required for session-based chat.
    redis: Option<ConnectionManager>,
    session_ttl_secs: u64,
}

impl AppState {
    /// Constructs [`AppState`] from its components.
    ///
    /// - `provider` — boxed AI backend (Claude, Gemini, OpenAI, or OpenRouter).
    /// - `redis` — optional connection manager; `None` disables session storage.
    /// - `session_ttl_secs` — lifetime in seconds for each Redis session key.
    pub fn new(
        provider: Arc<dyn AiProvider + Send + Sync>,
        redis: Option<ConnectionManager>,
        session_ttl_secs: u64,
    ) -> Self {
        Self {
            provider,
            redis,
            session_ttl_secs,
        }
    }

    /// Returns a reference to the active AI provider.
    pub fn provider(&self) -> &dyn AiProvider {
        self.provider.as_ref()
    }

    /// Returns a cloned [`ConnectionManager`] handle, or `None` if Redis is not configured.
    ///
    /// `ConnectionManager` is designed to be cloned — each clone shares the underlying pool.
    pub fn redis(&self) -> Option<ConnectionManager> {
        self.redis.clone()
    }

    /// Returns the session TTL in seconds used when writing keys to Redis.
    pub fn session_ttl_secs(&self) -> u64 {
        self.session_ttl_secs
    }
}
