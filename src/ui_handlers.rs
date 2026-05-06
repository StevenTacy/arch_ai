use axum::{
    extract::{Form, State},
    response::Html,
};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use serde::Deserialize;

use crate::{
    handlers::AppState,
    models::{Message, Role},
};

const MAX_MESSAGE_CHARS: usize = 2000;
const MAX_HISTORY_TURNS: usize = 20;

// ─────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────

pub async fn index() -> Html<String> {
    Html(page().into_string())
}

#[derive(Deserialize)]
pub struct UiChatForm {
    #[serde(default)]
    pub message: String,
    #[serde(default = "empty_history")]
    pub history: String,
}

fn empty_history() -> String {
    "[]".to_string()
}

pub async fn ui_chat(State(state): State<AppState>, Form(form): Form<UiChatForm>) -> Html<String> {
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

    let mut messages: Vec<Message> = serde_json::from_str(&form.history).unwrap_or_default();

    if messages.len() > MAX_HISTORY_TURNS {
        let excess = messages.len() - MAX_HISTORY_TURNS;
        messages.drain(0..excess);
    }

    messages.push(Message {
        role: Role::User,
        content: message.clone(),
    });

    let reply = match state.provider.chat(messages.clone()).await {
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

    messages.push(Message {
        role: Role::Assistant,
        content: reply.clone(),
    });

    let history_json = serde_json::to_string(&messages).unwrap_or_else(|_| "[]".to_string());

    Html(chat_fragment(&message, &reply, &history_json).into_string())
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
                                 hx-post="/v2/chat"
                                 hx-target="#messages"
                                 hx-swap="beforeend"
                                 hx-indicator="#thinking"
                                 hx-disabled-elt="find .send-btn" {
                                input type="hidden" id="history-input" name="history" value="[]";
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

fn chat_fragment(user_msg: &str, assistant_msg: &str, history_json: &str) -> Markup {
    html! {
        // OOB: update hidden history field
        input id="history-input" type="hidden" name="history"
              value=(history_json) hx-swap-oob="true";

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
