# arch_ai

A stateless Rust HTTP API that answers questions about Taiwan's construction law (台灣建築法規). Clients send a conversation history; the server enriches it with a domain-expert system prompt and proxies it to a configurable AI provider backend.

Supported providers: **Claude** (Anthropic), **Gemini** (Google), **OpenAI / Codex**.

---

## Requirements

### Runtime

| Requirement | Version |
|---|---|
| Rust | ≥ 1.85 (Rust 2024 edition) |
| Docker + Docker Compose | Any recent stable release |
| API key | One of: Anthropic / Google AI Studio / OpenAI |

### Environment variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `AI_PROVIDER` | no | `claude` | Provider to use: `claude`, `gemini`, or `openai` |
| `ANTHROPIC_API_KEY` | if `AI_PROVIDER=claude` | — | Anthropic API key |
| `GEMINI_API_KEY` | if `AI_PROVIDER=gemini` | — | Google AI Studio API key |
| `OPENAI_API_KEY` | if `AI_PROVIDER=openai` | — | OpenAI API key |
| `AI_MODEL` | no | provider default¹ | Model ID forwarded to the provider |
| `PORT` | no | `8080` | TCP port the server binds to |
| `MAX_TOKENS` | no | `4096` | Maximum tokens in each response |
| `RUST_LOG` | no | `arch_ai=debug,tower_http=debug` | `tracing` log filter |

¹ Provider defaults: `claude-sonnet-4-6` / `gemini-2.0-flash` / `gpt-4o`

---

## Bootstrap

### Local (bare metal)

```bash
# 1. Clone and enter the project
git clone <repo-url> arch_ai && cd arch_ai

# 2. Create .env from the template and set your provider + key
cp .env.example .env
# edit .env — set AI_PROVIDER and the matching *_API_KEY

# 3. Build and run
cargo run
# Server starts on http://0.0.0.0:8080
```

### Docker

```bash
# Build image and start container
docker compose up --build

# Detached
docker compose up -d --build

# Tail logs
docker compose logs -f arch_ai

# Stop
docker compose down
```

> `docker-compose.yml` reads every variable from your shell or a `.env` file — API keys are never baked into the image.

---

## API

### `GET /health`

Liveness probe. Always returns HTTP 200.

```json
{ "status": "ok" }
```

---

### `POST /chat`

Send a conversation turn. The server is **stateless** — callers own the history and must send the full `messages` array on every request.

**Request body**
```json
{
  "messages": [
    { "role": "user",      "content": "建築法第 25 條規定了什麼？" },
    { "role": "assistant", "content": "..." },
    { "role": "user",      "content": "那第 26 條呢？" }
  ]
}
```

| Field | Type | Description |
|---|---|---|
| `messages` | `Message[]` | Ordered conversation history, alternating `user` / `assistant` |

**Message object**

| Field | Type | Values |
|---|---|---|
| `role` | string | `"user"` \| `"assistant"` |
| `content` | string | Message text (Chinese or English) |

**Success — 200**
```json
{ "message": "建築法第 25 條規定，建築物非經申請主管建築機關之審查許可…" }
```

**Error**
```json
{ "error": "<description>" }
```

| HTTP status | Cause |
|---|---|
| `400` | Malformed JSON request body |
| `502` | Upstream provider call failed or returned no text |
| `500` | Internal serialisation or configuration error |

**curl example**
```bash
curl -s -X POST http://localhost:8080/chat \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"What permits are required before starting construction in Taiwan?"}]}' \
  | jq .
```

---

## Architecture

```
arch_ai/
├── src/
│   ├── main.rs              Entry point — tokio runtime, axum Router, provider dispatch, CORS + trace middleware
│   ├── config.rs            ProviderKind enum + Config; reads AI_PROVIDER and matching *_API_KEY at startup
│   ├── error.rs             AppError (thiserror) — implements axum IntoResponse → JSON error body
│   ├── models.rs            Shared API types: Message / Role / ChatRequest / ChatResponse
│   ├── handlers.rs          Axum handlers; State holds Arc<dyn AiProvider + Send + Sync>
│   └── provider/
│       ├── mod.rs           AiProvider trait + shared SYSTEM_PROMPT constant
│       ├── claude.rs        Anthropic Claude backend (Messages API)
│       ├── gemini.rs        Google Gemini backend (generateContent API)
│       └── openai.rs        OpenAI backend (Chat Completions API)
├── Dockerfile               Multi-stage build: rust:1.85-slim → debian:bookworm-slim
├── docker-compose.yml
└── .env.example
```

### Request flow

```
POST /chat
  └─ handlers::chat
       ├─ extracts Arc<dyn AiProvider> from axum State
       ├─ deserialises ChatRequest { messages: Vec<Message> }
       ├─ calls provider.chat(messages)           ← dispatches to Claude / Gemini / OpenAI
       │    ├─ injects SYSTEM_PROMPT (Taiwan construction law expert)
       │    ├─ translates Message roles to provider-native format
       │    ├─ POST to provider API
       │    └─ extracts first text reply
       └─ returns ChatResponse { message: String }
```

### Adding a new provider

1. Create `src/provider/<name>.rs` — implement `AiProvider` trait.
2. Declare `pub mod <name>;` in `src/provider/mod.rs`.
3. Add variant to `ProviderKind` in `src/config.rs` and wire the `FromStr` match.
4. Add the `Arc::new(<Name>Provider::new(config))` arm in `main.rs`.

### Key design choices

- **Stateless server** — conversation history lives entirely on the client; every request is independent.
- **Provider abstraction** — `AiProvider` trait (`async_trait`) lets any backend plug in behind a uniform `chat(Vec<Message>) -> Result<String, AppError>` interface. Role mapping (e.g., `"assistant"` → `"model"` for Gemini) is encapsulated per provider.
- **Error propagation** — all fallible paths return `Result<_, AppError>`; no `unwrap` on `Result`. `AppError` implements `axum::IntoResponse` so error conversion is centralised.
- **CORS** — `allow_origin(Any)` permits all origins. To restrict for production, replace `Any` with a parsed `HeaderValue` of your allowed origin.
- **System prompt** — domain scope is isolated in `src/provider/mod.rs::SYSTEM_PROMPT`. Change coverage there only; all providers share it automatically.

---

## Development

```bash
cargo check                        # type-check, no artefacts
cargo clippy -- -D warnings        # lint
cargo fmt                          # format
cargo test                         # all tests
cargo test <test_name>             # single test
```

---

## Domain coverage

The system prompt scopes the agent to the following Taiwan statutes and regulations:

- 建築法 (Building Act)
- 都市計畫法 (Urban Planning Act)
- 建築技術規則 — 建築設計施工編 / 建築構造編 / 建築設備編
- 消防法 (Fire Services Act) and fire-safety building codes
- 建築師法 (Architects Act)
- 營造業法 (Construction Industry Act)
- 公寓大廈管理條例 (Condominium Act)
- Seismic, wind, and structural design standards (耐震、風力、載重設計規範)
- MOI interpretation letters (內政部函釋)
- Municipal implementation rules (Taipei, New Taipei, Taichung, etc.)

Responses cite specific articles (法條), explain practical application, and flag regional variations. Language (Chinese / English) auto-detected from user input.
