use redis::{aio::ConnectionManager, AsyncCommands};
use uuid::Uuid;

use crate::{error::AppError, models::Message};

const MAX_TURNS: usize = 20;

pub fn new_session_id() -> String {
    Uuid::new_v4().to_string()
}

pub async fn get_session(
    conn: &mut ConnectionManager,
    session_id: &str,
) -> Result<Vec<Message>, AppError> {
    let key = format!("session:{session_id}");
    let data: Option<String> = conn
        .get(&key)
        .await
        .map_err(|e| AppError::Session(e.to_string()))?;

    match data {
        Some(json) => Ok(serde_json::from_str(&json)?),
        None => Ok(Vec::new()),
    }
}

pub async fn save_session(
    conn: &mut ConnectionManager,
    session_id: &str,
    messages: &[Message],
    ttl_secs: u64,
) -> Result<(), AppError> {
    let mut msgs = messages.to_vec();

    // Keep last MAX_TURNS messages to cap Redis value size
    if msgs.len() > MAX_TURNS {
        msgs.drain(0..msgs.len() - MAX_TURNS);
    }

    let json = serde_json::to_string(&msgs)?;
    let key = format!("session:{session_id}");

    // SET + EX in one command; resets TTL on every active turn
    conn.set_ex::<_, _, ()>(&key, &json, ttl_secs)
        .await
        .map_err(|e| AppError::Session(e.to_string()))?;

    Ok(())
}
