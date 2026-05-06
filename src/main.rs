use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod error;
mod handlers;
mod models;
mod provider;
mod session;
mod ui_handlers;

use config::ProviderKind;
use handlers::AppState;
use provider::{
    claude::ClaudeProvider,
    gemini::GeminiProvider,
    mock::MockProvider,
    openai::OpenAiProvider,
    openrouter::{OpenRouterProvider, discover_free_model},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "arch_ai=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::from_env()?;
    let port = config.port;

    tracing::info!(provider = ?config.provider, model = %config.model, "starting arch_ai");

    let provider: Arc<dyn provider::AiProvider + Send + Sync> = match config.provider {
        ProviderKind::Claude => Arc::new(ClaudeProvider::new(config.clone())),
        ProviderKind::Gemini => Arc::new(GeminiProvider::new(config.clone())),
        ProviderKind::OpenAi => Arc::new(OpenAiProvider::new(config.clone())),
        ProviderKind::Mock => Arc::new(MockProvider::new()),
        ProviderKind::OpenRouter => {
            if config.api_key.is_empty() {
                tracing::warn!("OPENROUTER_API_KEY not set — running in demo mode (Mock)");
                Arc::new(MockProvider::new()) as Arc<dyn provider::AiProvider + Send + Sync>
            } else {
                let resolved_model = if config.model == "auto" {
                    match discover_free_model(&config).await {
                        Ok(m) => {
                            tracing::info!(model = %m, "OpenRouter: auto-selected free model");
                            m
                        }
                        Err(e) => {
                            let fallback = "meta-llama/llama-3.2-1b-instruct:free";
                            tracing::warn!(error = %e, fallback, "OpenRouter: model discovery failed, using fallback");
                            fallback.into()
                        }
                    }
                } else {
                    config.model.clone()
                };
                let mut cfg = config.clone();
                cfg.model = resolved_model;
                Arc::new(OpenRouterProvider::new(cfg))
                    as Arc<dyn provider::AiProvider + Send + Sync>
            }
        }
    };

    // PostgreSQL pool — optional; enables RAG when DATABASE_URL is set
    let db_pool = match &config.database_url {
        Some(url) => {
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(url)
                .await
                .map_err(|e| anyhow::anyhow!("PostgreSQL connection failed: {e}"))?;

            sqlx::migrate!()
                .run(&pool)
                .await
                .map_err(|e| anyhow::anyhow!("Migration failed: {e}"))?;

            tracing::info!("PostgreSQL connected, migrations applied");
            Some(pool)
        }
        None => {
            tracing::info!("DATABASE_URL not set — RAG disabled");
            None
        }
    };

    // Redis connection manager — optional; required for /v2/chat
    let redis_conn = match &config.redis_url {
        Some(url) => {
            let client = redis::Client::open(url.as_str())
                .map_err(|e| anyhow::anyhow!("Invalid Redis URL: {e}"))?;
            let manager = redis::aio::ConnectionManager::new(client)
                .await
                .map_err(|e| anyhow::anyhow!("Redis connection failed: {e}"))?;

            tracing::info!("Redis connected");
            Some(manager)
        }
        None => {
            tracing::info!("REDIS_URL not set — /v2/chat will return 503");
            None
        }
    };

    let app_state = AppState {
        provider,
        db: db_pool,
        redis: redis_conn,
        rag_top_k: config.rag_top_k,
        session_ttl_secs: config.session_ttl_secs,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .route("/", get(ui_handlers::index))
        .route("/ui/chat", post(ui_handlers::ui_chat))
        .route("/health", get(handlers::health))
        .route("/chat", post(handlers::chat))
        .route("/v2/chat", post(handlers::chat_v2))
        .with_state(app_state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
