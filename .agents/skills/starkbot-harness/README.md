# starkbot-harness

Claude Code plugin for commanding a Starkbot agent via the External Channel Gateway API.

![robot1](https://github.com/user-attachments/assets/486cf36b-a0ac-4623-8410-c4cc93388ba1)


## Prerequisites 

```
1. In your starkbot instance, click on Channels and create an External Gateway, turn it on

2. Copy the STARK_URL and STARK_TOKEN 

```


## Quick Start

### 1. Install

```
npx skills add ethereumdegen/starkbot-harness
```

### 2. Set environment variables

```bash
export STARK_URL="https://your-starkbot-instance.com"
export STARK_TOKEN="your-bearer-token"
```

### 3. Use it

```
/starkbot-harness:stark hello, what can you do?
```

Claude will also auto-invoke the Starkbot Gateway skill when interaction with your agent is contextually relevant.

## API Endpoints

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/gateway/chat` | Send message, get full response |
| POST | `/api/gateway/chat/stream` | Send message, stream SSE |
| POST | `/api/gateway/sessions/new` | Create new session |
| GET | `/api/gateway/sessions` | List sessions |
| GET | `/api/gateway/sessions/{id}/messages` | Get message history |

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `STARK_URL` | Yes | Base URL of your Starkbot instance |
| `STARK_TOKEN` | Yes | Bearer token for API authentication |
| `STARK_SESSION` | No | Session ID for conversation persistence |

## Local Development

```bash
claude --plugin-dir ~/ai/starkbot-harness
```
