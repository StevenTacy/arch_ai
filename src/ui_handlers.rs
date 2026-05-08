use axum::{
    extract::{Form, Path, State},
    response::Html,
};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use serde::Deserialize;

use crate::{
    handlers::AppState,
    models::{Message, Role},
    session,
};

const MAX_MESSAGE_CHARS: usize = 2000;

// ─────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────

pub async fn index() -> Html<String> {
    Html(page().into_string())
}

#[derive(Deserialize)]
pub struct UiChatForm {
    #[serde(default)]
    message: String,
    #[serde(default)]
    session_id: Option<String>,
}

/// Handler for `POST /chat` (HTMX form submission).
///
/// Receives a user message and an optional session ID from [`UiChatForm`], loads
/// the existing conversation from Redis, calls the configured AI provider, saves
/// the updated history, and returns an HTML fragment that HTMX appends to `#messages`.
///
/// # Parameters
/// - `state` — shared [`AppState`] carrying the AI provider, Redis pool, and TTL config.
/// - `form` — `message`: the user's input (max [`MAX_MESSAGE_CHARS`] chars);
///   `session_id`: opaque string identifying the conversation (empty → new session generated).
///
/// # Returns
///
/// `Html<String>` — one of two possible fragments:
///
/// - **Success** — [`chat_fragment`]: two `div.msg-row` blocks (user bubble + AI bubble)
///   plus two OOB swaps (`#session-id-input` and `#message-input`) so the client
///   persists the session ID and clears the textarea.
/// - **Error** — [`error_fragment`]: a `div.msg-error` with a bilingual message when
///   input is empty/too long, Redis is unavailable, session load fails, or the AI
///   provider returns an error or a `[SCOPE_REJECT]` prefix.
pub async fn chat(State(state): State<AppState>, Form(form): Form<UiChatForm>) -> Html<String> {
    let message = form.message.trim().to_string();

    if message.is_empty() {
        return Html(error_fragment("請輸入問題。/ Please enter your question.").into_string());
    }

    if message.chars().count() > MAX_MESSAGE_CHARS {
        return Html(
            error_fragment(&format!(
                "訊息過長（最多 {MAX_MESSAGE_CHARS} 字）。/ Message too long (max {MAX_MESSAGE_CHARS} chars)."
            ))
            .into_string(),
        );
    }

    let mut redis = match state.redis() {
        Some(r) => r,
        None => {
            return Html(
                error_fragment(
                    "對話記錄服務未啟用，請聯繫管理員。/ Session storage not configured.",
                )
                .into_string(),
            );
        }
    };

    let session_id = form
        .session_id
        .filter(|s| !s.is_empty())
        .unwrap_or_else(session::new_session_id);

    let mut session_messages = match session::get_session(&mut redis, &session_id).await {
        Ok(msgs) => msgs,
        Err(e) => {
            tracing::error!(error = %e, "session load failed");
            return Html(
                error_fragment(
                    "無法載入對話記錄，請重新整理。/ Session load failed, please refresh.",
                )
                .into_string(),
            );
        }
    };

    let mut api_messages = session_messages.clone();
    api_messages.push(Message::new(Role::User, message.clone()));

    let reply = match state.provider().chat(api_messages).await {
        Ok(r) if r.starts_with("[SCOPE_REJECT]") => {
            let reason = r["[SCOPE_REJECT]".len()..].trim();
            return Html(error_fragment(reason).into_string());
        }
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "AI provider error");
            return Html(
                error_fragment(
                    "AI 服務暫時無法使用，請稍後再試。/ AI service unavailable, please try again.",
                )
                .into_string(),
            );
        }
    };

    session_messages.push(Message::new(Role::User, message.clone()));
    session_messages.push(Message::new(Role::Assistant, reply.clone()));

    if let Err(e) = session::save_session(
        &mut redis,
        &session_id,
        &session_messages,
        state.session_ttl_secs(),
    )
    .await
    {
        // Non-fatal: reply is still returned; session just won't persist this turn
        tracing::error!(error = %e, "session save failed");
    }

    Html(chat_fragment(&message, &reply, &session_id).into_string())
}

// ─────────────────────────────────────────────────────────────────
// Templates
// ─────────────────────────────────────────────────────────────────

fn page() -> Markup {
    html! {
        (DOCTYPE)
        html lang="zh-TW" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "台灣建築法規 AI 顧問" }
                link rel="stylesheet" href="/static/style.css";
            }
            body {
                // ── Sidebar ──────────────────────────────────────────────
                aside class="sidebar" {
                    div class="brand" {
                        div class="brand-icon" { "⚖" }
                        div {
                            div class="brand-name" { "建築法規 AI" }
                            div class="brand-sub"  { "Taiwan Construction Law" }
                        }
                    }

                    button type="button" class="new-chat-btn" {
                        (PreEscaped(r#"<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>"#))
                        " New conversation"
                    }

                    div id="sidebar-examples" {
                        div class="sidebar-section" { "Try asking" }

                        div class="example" { "建築法第51條的退縮規定是什麼？" }
                        div class="example" { "都市計畫法的使用分區有哪些類型？" }
                        div class="example" { "建蔽率與容積率有何不同？" }
                        div class="example" { "What are fire safety requirements under 消防法？" }
                        div class="example" { "公寓大廈管理條例主要規範哪些事項？" }
                    }
                    div id="sidebar-history" {}

                    div class="sidebar-footer" { "建築法規 AI · Legal Consultant" }
                }

                // ── Main ─────────────────────────────────────────────────
                main class="main" {
                    header {
                        span class="header-title" { "台灣建築法規諮詢" }
                        span class="header-badge" { "Legal AI" }
                    }

                    div id="messages" {
                        div class="welcome" {
                            div class="welcome-icon" { "🏗" }
                            h2 { "建築法規 AI 助理" }
                            p {
                                "詢問台灣建築法規相關問題，AI 將引用具體法條說明。"
                                br;
                                "Ask questions about Taiwan construction law — responses cite specific articles."
                            }
                        }
                    }

                    div class="input-area" {
                        div class="input-container" {
                            div id="thinking" {
                                div class="dots" {
                                    span class="dot" {}
                                    span class="dot" {}
                                    span class="dot" {}
                                }
                                span { "AI 分析法條中…" }
                            }
                            form id="chat-form"
                                 hx-post="/chat"
                                 hx-target="#messages"
                                 hx-swap="beforeend"
                                 hx-indicator="#thinking"
                                 hx-disabled-elt="find .send-btn" {
                                input type="hidden" id="session-id-input" name="session_id" value="";
                                div class="input-box" {
                                    textarea
                                        id="message-input"
                                        name="message"
                                        placeholder="輸入您的問題… 例：建築法第 51 條退縮規定為何？"
                                        rows="1"
                                        maxlength="2000" {}
                                    button class="send-btn" type="submit" {
                                        (PreEscaped(r#"<svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/></svg>"#))
                                    }
                                }
                                p class="input-hint" { "Enter 送出 · Shift+Enter 換行 · 最多 2000 字" }
                            }
                        }
                    }

                    script src="https://unpkg.com/htmx.org@1.9.12" {}
                    script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js" {}
                    script src="/static/app.js" defer="" {}
                }
            }
        }
    }
}

fn chat_fragment(user_msg: &str, assistant_msg: &str, session_id: &str) -> Markup {
    html! {
        // OOB: persist session_id for next turn
        input id="session-id-input" type="hidden" name="session_id"
              value=(session_id) hx-swap-oob="true";

        // OOB: clear textarea
        textarea id="message-input" name="message"
                 placeholder="輸入您的問題… 例：建築法第 51 條退縮規定為何？"
                 rows="1" maxlength="2000" hx-swap-oob="true" {}

        // User message — data-session-id read by JS to create sidebar history entry
        div class="msg-row" data-session-id=(session_id) {
            div class="msg-inner user" {
                div class="msg-bubble user-bubble" { (user_msg) }
            }
        }

        // Assistant message — data-md triggers client-side markdown rendering
        div class="msg-row" {
            div class="msg-inner assistant" {
                div class="msg-av" { "AI" }
                div class="msg-bubble ai-bubble" data-md="" { (assistant_msg) }
            }
        }
    }
}

fn error_fragment(msg: &str) -> Markup {
    html! {
        div class="msg-error" {
            span class="err-icon" { "⚠" }
            " " (msg)
        }
    }
}

// ─────────────────────────────────────────────────────────────────
// Session history replay
// ─────────────────────────────────────────────────────────────────

/// Handler for `GET /session/:session_id` (sidebar history replay).
///
/// Loads the full conversation stored under `session_id` from Redis and renders
/// every turn as HTML. Called by the client when a user clicks a past session
/// entry in the sidebar; the response replaces `#messages` with the replay.
///
/// # Parameters
/// - `state` — shared [`AppState`]; Redis connection is required.
/// - `session_id` — URL path segment identifying the session to replay.
///
/// # Returns
///
/// `Html<String>` — one of two possible fragments:
///
/// - **Success** — [`history_fragment`]: a sequence of `div.msg-row` blocks, one per
///   stored [`Message`], rendered as user or assistant bubbles in chronological order.
/// - **Error** — [`error_fragment`]: a `div.msg-error` when Redis is unconfigured,
///   the session cannot be loaded, or the session ID is not found / has expired.
pub async fn session_history(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Html<String> {
    let mut redis = match state.redis() {
        Some(r) => r,
        None => {
            return Html(
                error_fragment("Session storage not configured.").into_string(),
            );
        }
    };

    let messages = match session::get_session(&mut redis, &session_id).await {
        Ok(msgs) => msgs,
        Err(e) => {
            tracing::error!(error = %e, "session history load failed");
            return Html(error_fragment("Session load failed.").into_string());
        }
    };

    if messages.is_empty() {
        return Html(error_fragment("Session not found or expired.").into_string());
    }

    Html(history_fragment(&messages).into_string())
}

fn history_fragment(messages: &[Message]) -> Markup {
    html! {
        @for msg in messages {
            @match msg.role() {
                Role::User => {
                    div class="msg-row" {
                        div class="msg-inner user" {
                            div class="msg-bubble user-bubble" { (msg.content()) }
                        }
                    }
                }
                Role::Assistant => {
                    div class="msg-row" {
                        div class="msg-inner assistant" {
                            div class="msg-av" {
                                (PreEscaped(r#"<svg width="18" height="18" viewBox="0 0 16 16" fill="none" aria-hidden="true"><path d="M8 3 Q9.5 7 13 8 Q9.5 9 8 13 Q6.5 9 3 8 Q6.5 7 8 3 Z" fill="white" opacity="0.92"/></svg>"#))
                            }
                            div class="msg-bubble ai-bubble" data-md="" { (msg.content()) }
                        }
                    }
                }
            }
        }
    }
}
