use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod handlers;
mod models;
mod provider;
mod ui_handlers;

use config::ProviderKind;
use handlers::AppState;
use provider::{claude::ClaudeProvider, gemini::GeminiProvider, openai::OpenAiProvider};

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

    let provider: AppState = match config.provider {
        ProviderKind::Claude => Arc::new(ClaudeProvider::new(config)),
        ProviderKind::Gemini => Arc::new(GeminiProvider::new(config)),
        ProviderKind::OpenAi => Arc::new(OpenAiProvider::new(config)),
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
        .with_state(provider)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
