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

pub async fn ui_chat(
    State(provider): State<AppState>,
    Form(form): Form<UiChatForm>,
) -> Html<String> {
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

    // Keep context window bounded
    if messages.len() > MAX_HISTORY_TURNS {
        let excess = messages.len() - MAX_HISTORY_TURNS;
        messages.drain(0..excess);
    }

    messages.push(Message {
        role: Role::User,
        content: message.clone(),
    });

    let reply = match provider.chat(messages.clone()).await {
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
                style { (PreEscaped(CSS)) }
            }
            body {
                header {
                    div class="logo" { "🏗" }
                    div class="header-text" {
                        h1 { "台灣建築法規 AI 顧問" }
                        p { "Taiwan Construction Law AI Assistant" }
                    }
                    span class="badge" { "Legal AI" }
                }

                div id="messages" {
                    div class="welcome" {
                        div class="welcome-icon" { "⚖" }
                        h2 { "歡迎使用建築法規 AI 顧問" }
                        p {
                            "詢問有關建築法、都市計畫法、建築技術規則等台灣建築法規問題，"
                            "AI 將引用具體法條說明。"
                            br;
                            "Ask questions about Taiwan's construction laws and regulations."
                        }
                        div class="chips" {
                            span class="chip" { "建築法" }
                            span class="chip" { "都市計畫法" }
                            span class="chip" { "建築技術規則" }
                            span class="chip" { "消防法" }
                            span class="chip" { "公寓大廈管理條例" }
                        }
                    }
                }

                div class="input-area" {
                    div id="thinking" {
                        div class="dots" {
                            span class="dot" {}
                            span class="dot" {}
                            span class="dot" {}
                        }
                        span { "AI 分析法條中…" }
                    }
                    form id="chat-form"
                         hx-post="/ui/chat"
                         hx-target="#messages"
                         hx-swap="beforeend scroll:bottom"
                         hx-indicator="#thinking" {
                        input
                            type="hidden"
                            id="history-input"
                            name="history"
                            value="[]";
                        div class="input-row" {
                            textarea
                                id="message-input"
                                name="message"
                                placeholder="輸入您的問題… 例：建築法第 51 條退縮規定為何？"
                                rows="1"
                                maxlength="2000" {}
                            button class="send-btn" type="submit" {
                                (PreEscaped("&#8593;"))
                            }
                        }
                        p class="input-hint" {
                            "Enter 送出  ·  Shift + Enter 換行  ·  最多 2000 字"
                        }
                    }
                }

                // Minimal keyboard shortcut — Enter submits, Shift+Enter inserts newline.
                // Event delegation survives htmx OOB textarea replacement.
                script {
                    (PreEscaped(r#"
document.addEventListener('keydown', function(e) {
  if (e.target.id === 'message-input' && e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    document.getElementById('chat-form').requestSubmit();
  }
});
"#))
                }
                script src="https://unpkg.com/htmx.org@1.9.12" {}
            }
        }
    }
}

/// Fragment returned to htmx on each chat turn.
/// OOB elements update the form state; remaining divs are appended to #messages.
fn chat_fragment(user_msg: &str, assistant_msg: &str, history_json: &str) -> Markup {
    html! {
        // OOB: persist updated conversation history in the hidden field
        input
            id="history-input"
            type="hidden"
            name="history"
            value=(history_json)
            hx-swap-oob="true";

        // OOB: clear the textarea for the next message
        textarea
            id="message-input"
            name="message"
            placeholder="輸入您的問題… 例：建築法第 51 條退縮規定為何？"
            rows="1"
            maxlength="2000"
            hx-swap-oob="true" {}

        // Main content — appended to #messages
        div class="msg user" {
            div class="msg-av user-av" { "你" }
            div class="msg-bubble" { (user_msg) }
        }
        div class="msg assistant" {
            div class="msg-av ai-av" { "AI" }
            div class="msg-bubble ai-bubble" { (assistant_msg) }
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
// Embedded styles
// ─────────────────────────────────────────────────────────────────

const CSS: &str = r#"
/* ── Reset ── */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
    --font: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang TC",
            "Microsoft JhengHei", "Noto Sans TC", system-ui, sans-serif;
    --bg:         #080812;
    --surface:    #10101e;
    --surface2:   #18182c;
    --surface3:   #1f1f36;
    --border:     #272740;
    --border2:    #333355;
    --text:       #dde4f0;
    --muted:      #6b7280;
    --dim:        #3d3d5c;
    --accent-a:   #2563eb;
    --accent-b:   #7c3aed;
    --user-grad:  linear-gradient(135deg, #1d4ed8 0%, #6d28d9 100%);
    --user-glow:  0 4px 20px rgba(109,40,217,.4);
    --err-bg:     #1c0a0a;
    --err-bd:     #991b1b;
    --err-txt:    #fca5a5;
    --r:          1rem;
}

html, body { height: 100%; font-family: var(--font); background: var(--bg); color: var(--text); -webkit-font-smoothing: antialiased; }

body { display: flex; flex-direction: column; height: 100dvh; }

/* ── Ambient glow ── */
body::before, body::after {
    content: ''; position: fixed; pointer-events: none; border-radius: 50%; z-index: 0;
}
body::before {
    top: -25%; right: -15%; width: 42rem; height: 42rem;
    background: radial-gradient(circle, rgba(124,58,237,.07) 0%, transparent 65%);
}
body::after {
    bottom: -20%; left: -10%; width: 32rem; height: 32rem;
    background: radial-gradient(circle, rgba(37,99,235,.05) 0%, transparent 65%);
}

header, #messages, .input-area { position: relative; z-index: 1; }

/* ── Header ── */
header {
    display: flex; align-items: center; gap: .75rem;
    padding: .875rem 1.5rem;
    background: var(--surface); border-bottom: 1px solid var(--border);
    flex-shrink: 0;
}

.logo {
    width: 2.125rem; height: 2.125rem; flex-shrink: 0;
    background: var(--user-grad); border-radius: .6rem;
    display: flex; align-items: center; justify-content: center;
    font-size: 1rem;
    box-shadow: 0 2px 10px rgba(109,40,217,.45);
}

.header-text { flex: 1; }
.header-text h1 { font-size: .9375rem; font-weight: 700; letter-spacing: -.01em; }
.header-text p  { font-size: .6875rem; color: var(--muted); margin-top: .1rem; }

.badge {
    font-size: .6875rem; font-weight: 600;
    padding: .25rem .625rem;
    background: rgba(124,58,237,.14); border: 1px solid rgba(124,58,237,.3);
    border-radius: 99px; color: #a78bfa; letter-spacing: .03em;
}

/* ── Messages ── */
#messages {
    flex: 1; overflow-y: auto;
    padding: 1.5rem; display: flex; flex-direction: column;
    gap: 1.25rem; scroll-behavior: smooth; overscroll-behavior: contain;
}

/* ── Welcome ── */
.welcome {
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; gap: .875rem; flex: 1;
    text-align: center; color: var(--muted); padding: 2.5rem 1rem;
}

.welcome-icon {
    font-size: 3rem;
    filter: drop-shadow(0 0 24px rgba(124,58,237,.65));
    margin-bottom: .25rem;
}

.welcome h2 {
    font-size: 1.375rem; font-weight: 700;
    background: linear-gradient(135deg, #e2e8f0, #a78bfa);
    -webkit-background-clip: text; -webkit-text-fill-color: transparent;
    background-clip: text;
}

.welcome p { font-size: .875rem; max-width: 28rem; line-height: 1.75; }

.chips { display: flex; flex-wrap: wrap; gap: .4rem; justify-content: center; margin-top: .25rem; }

.chip {
    font-size: .75rem; padding: .25rem .6875rem;
    background: var(--surface2); border: 1px solid var(--border2);
    border-radius: 99px; color: var(--muted);
}

/* ── Message rows ── */
.msg {
    display: flex; gap: .625rem; align-items: flex-start;
    max-width: 84%;
    animation: fadeUp .22s ease-out both;
}

@keyframes fadeUp {
    from { opacity: 0; transform: translateY(9px); }
    to   { opacity: 1; transform: translateY(0); }
}

.msg.user { align-self: flex-end; flex-direction: row-reverse; }

.msg-av {
    width: 1.875rem; height: 1.875rem; border-radius: 50%; flex-shrink: 0;
    display: flex; align-items: center; justify-content: center;
    font-size: .625rem; font-weight: 700; margin-top: .125rem;
}

.user-av { background: var(--user-grad); color: #fff; box-shadow: 0 2px 8px rgba(109,40,217,.45); }
.ai-av   { background: var(--surface3); border: 1px solid var(--border2); color: #a78bfa; }

.msg-bubble {
    padding: .75rem 1rem; border-radius: var(--r);
    font-size: .9375rem; line-height: 1.7; word-break: break-word;
}

.msg.user .msg-bubble {
    background: var(--user-grad);
    border-radius: var(--r) .25rem var(--r) var(--r);
    box-shadow: var(--user-glow);
}

.ai-bubble {
    background: var(--surface2); border: 1px solid var(--border);
    border-radius: .25rem var(--r) var(--r) var(--r);
    white-space: pre-wrap;
}

/* ── Error ── */
.msg-error {
    align-self: center; display: flex; align-items: center; gap: .5rem;
    background: var(--err-bg); border: 1px solid var(--err-bd);
    border-radius: .5rem; color: var(--err-txt);
    padding: .5625rem .9375rem; font-size: .875rem;
    animation: fadeUp .22s ease-out both;
}

/* ── Thinking indicator ── */
#thinking {
    display: none; align-items: center; gap: .5rem;
    color: var(--muted); font-size: .8125rem; padding: .375rem 0;
}
#thinking.htmx-request { display: flex; }

.dots { display: flex; gap: .25rem; align-items: center; }

.dot {
    width: .375rem; height: .375rem; border-radius: 50%;
    background: var(--accent-b);
    animation: pulse 1.4s ease-in-out infinite;
}
.dot:nth-child(2) { animation-delay: .2s; }
.dot:nth-child(3) { animation-delay: .4s; }

@keyframes pulse {
    0%, 60%, 100% { transform: scale(1);   opacity: .4; }
    30%            { transform: scale(1.4); opacity: 1; }
}

/* ── Input area ── */
.input-area {
    padding: .875rem 1.5rem 1rem;
    background: var(--surface); border-top: 1px solid var(--border);
    flex-shrink: 0;
}

.input-row { display: flex; gap: .625rem; align-items: flex-end; }

#message-input {
    flex: 1;
    background: var(--surface2); border: 1.5px solid var(--border2);
    border-radius: .875rem; color: var(--text);
    padding: .6875rem 1rem; font: inherit; font-size: .9375rem;
    line-height: 1.5; resize: none;
    min-height: 2.75rem; max-height: 10rem; overflow-y: auto;
    outline: none; transition: border-color .2s, box-shadow .2s;
}
#message-input:focus {
    border-color: var(--accent-b);
    box-shadow: 0 0 0 3px rgba(124,58,237,.15);
}
#message-input::placeholder { color: var(--dim); }

.send-btn {
    width: 2.75rem; height: 2.75rem; flex-shrink: 0;
    background: var(--user-grad); border: none; border-radius: .875rem;
    color: #fff; font-size: 1.25rem; font-weight: 700;
    cursor: pointer; display: flex; align-items: center; justify-content: center;
    box-shadow: 0 4px 14px rgba(109,40,217,.5);
    transition: opacity .2s, transform .15s, box-shadow .2s;
}
.send-btn:hover  { opacity: .9; transform: translateY(-1px); box-shadow: 0 6px 18px rgba(109,40,217,.55); }
.send-btn:active { transform: scale(.95); }

.input-hint { font-size: .6875rem; color: var(--dim); text-align: center; margin-top: .5rem; }

/* ── Scrollbar ── */
::-webkit-scrollbar { width: 4px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border2); border-radius: 99px; }
::-webkit-scrollbar-thumb:hover { background: var(--muted); }

/* ── Mobile ── */
@media (max-width: 640px) {
    header        { padding: .75rem 1rem; }
    #messages     { padding: 1rem; gap: 1rem; }
    .msg          { max-width: 93%; }
    .input-area   { padding: .75rem 1rem .875rem; }
    .welcome h2   { font-size: 1.125rem; }
}
"#;
