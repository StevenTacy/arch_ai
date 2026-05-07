use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Provider API error: {0}")]
    ProviderError(String),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Session error: {0}")]
    Session(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("{}", &self);

        let (status, message) = match self {
            AppError::ProviderError(msg) => (StatusCode::BAD_GATEWAY, msg),
            AppError::HttpClient(_) => (
                StatusCode::BAD_GATEWAY,
                "upstream request to AI provider failed".to_string(),
            ),
            AppError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Serialization(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "response serialization failed".to_string(),
            ),
            AppError::Session(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "session store unavailable".to_string(),
            ),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
