use axum::{
    extract::{Form, State},
    response::Html,
};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use serde::Deserialize;

use crate::{
    db,
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

    // RAG: inject relevant law chunks as ephemeral context (not persisted to session)
    let rag_context = if let Some(pool) = state.db() {
        match db::search_law(pool, &message, state.rag_top_k()).await {
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

    let mut api_messages: Vec<Message> = Vec::new();
    if let Some(ctx) = rag_context {
        api_messages.push(Message::new(Role::User, format!("[法條參考資料]\n{ctx}")));
        api_messages.push(Message::new(Role::Assistant, "已閱讀法條參考資料，請提問。"));
    }
    api_messages.extend(session_messages.clone());
    api_messages.push(Message::new(Role::User, message.clone()));

    let reply = match state.provider().chat(api_messages).await {
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

                    a href="/" class="new-chat-btn" {
                        (PreEscaped(r#"<svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>"#))
                        " New conversation"
                    }

                    div class="sidebar-section" { "Try asking" }

                    div class="example" onclick="fillExample(this)" { "建築法第51條的退縮規定是什麼？" }
                    div class="example" onclick="fillExample(this)" { "都市計畫法的使用分區有哪些類型？" }
                    div class="example" onclick="fillExample(this)" { "建蔽率與容積率有何不同？" }
                    div class="example" onclick="fillExample(this)" { "What are fire safety requirements under 消防法？" }
                    div class="example" onclick="fillExample(this)" { "公寓大廈管理條例主要規範哪些事項？" }

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

        // User message
        div class="msg-row" {
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
