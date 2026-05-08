# arch_ai

A chatbot that answers questions about Taiwan's construction law (台灣建築法規). Ask about specific statutes, permits, setback rules, structural requirements, and more — in Chinese or English.

---

## Demo

<video src="arc_ai_feature_demo.mp4" controls width="100%"></video>

---

## Requirements

- [Docker](https://docs.docker.com/get-docker/) and Docker Compose
- Rust >= 1.86 (optinoal, only if you wish to run project locally)
- An API key from one of: [Anthropic](https://console.anthropic.com/), [Google AI Studio](https://aistudio.google.com/), [OpenAI](https://platform.openai.com/), or [OpenRouter (free api key usage)](https://openrouter.ai/)

---

## Setup

### 1. Clone the repository

```bash
git clone <repo-url> arch_ai && cd arch_ai
```

### 2. Configure your environment

Create .env file for your API keys and settings:

```bash
touch .env
```

Pick your ai provider and copy the matching block into `.env`:

**Claude**
```env
AI_PROVIDER=claude
ANTHROPIC_API_KEY=your_key_here
AI_MODEL=your_desired_model
PORT=your_desired_port
# MAX_TOKENS=(optional)
```

**Gemini**
```env
AI_PROVIDER=gemini
GEMINI_API_KEY=your_key_here
AI_MODEL=your_desired_model
PORT=your_desired_port
# MAX_TOKENS=(optional)
```

**OpenAI**
```env
AI_PROVIDER=openai
OPENAI_API_KEY=your_key_here
AI_MODEL=your_desired_model
PORT=your_desired_port
# MAX_TOKENS=(optional)
```

**OpenRouter**
```env
AI_PROVIDER=openrouter
OPEROUTER_API_KEY=your_key_here
AI_MODEL=your_desired_model
PORT=your_desired_port
# MAX_TOKENS=(optional)
```

### 3. Bootstrap the project with docker

```bash
docker compose up -d
```

This brings up PostgreSQL (law-chunk storage) and Redis (session cache).

### 4. Bootstrap locally

```bash
echo "REDIS_URL=your_redis_url" >> .env
cargo run
```

The server starts on `http://localhost:8080` (or the port set in `PORT`).

### 5. Open the UI

Visit [http://localhost:8080](http://localhost:8080) in your browser and start asking questions about Taiwan construction law.

---

## Environment variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `AI_PROVIDER` | no | `claude` | Provider: `claude`, `gemini`, `openai`, `ollama`, `openrouter`, or `mock` |
| `ANTHROPIC_API_KEY` | if `AI_PROVIDER=claude` | — | Anthropic API key |
| `GEMINI_API_KEY` | if `AI_PROVIDER=gemini` | — | Google AI Studio key |
| `OPENAI_API_KEY` | if `AI_PROVIDER=openai` | — | OpenAI key |
| `AI_MODEL` | no | provider default¹ | Model ID sent to the provider |
| `PORT` | no | `8080` | TCP port the server binds to |
| `MAX_TOKENS` | no | `4096` | Maximum tokens per response |
| `REDIS_URL` | no | — | Redis connection string (enables stateful `/v2/chat` sessions) |

¹ Defaults: `claude-sonnet-4-6` / `gemini-2.0-flash` / `gpt-4o` 

---
