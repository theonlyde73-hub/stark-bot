---
name: starkbot-harness
description: Command a Starkbot AI agent via the External Channel Gateway API — send messages, manage sessions, and retrieve conversation history
---

# Starkbot Harness

A skill for interacting with a Starkbot AI agent through its External Channel Gateway API. Send messages, stream responses, manage sessions, and retrieve conversation history — all from your coding agent.

## When to use

Use this skill when you need to communicate with a running Starkbot agent instance — for example, to send it instructions, ask it questions, or manage conversation sessions.

## Setup

Set the required environment variables:

```bash
export STARK_URL="https://your-starkbot-instance.com"
export STARK_TOKEN="your-bearer-token"
```

## Capabilities

- **Send messages** to a Starkbot agent (synchronous or streaming)
- **Create and manage sessions** for persistent conversations
- **Retrieve message history** from previous sessions

## API Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/gateway/chat` | Send message, get full response |
| POST | `/api/gateway/chat/stream` | Send message, stream SSE |
| POST | `/api/gateway/sessions/new` | Create new session |
| GET | `/api/gateway/sessions` | List sessions |
| GET | `/api/gateway/sessions/{id}/messages` | Get message history |
