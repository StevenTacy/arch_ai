use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Inbound payload from the chat client.
/// The client owns conversation history and sends the full message list each turn.
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: String,
}

/// Inbound payload for the stateful /v2/chat endpoint.
/// Client sends only the current turn; server owns history via Redis.
#[derive(Debug, Deserialize)]
pub struct ChatRequestV2 {
    /// Omit to start a new session; include to continue an existing one.
    pub session_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponseV2 {
    /// Echo back so the client can persist it for subsequent turns.
    pub session_id: String,
    pub message: String,
}
