---
name: starkbot-gateway
description: Interact with a Starkbot agent via the External Channel Gateway API
auto-invocable: true
---

# Starkbot External Channel Gateway API

Use this skill when the user wants to interact with their Starkbot agent — sending messages, managing sessions, or retrieving conversation history. All requests use `curl` against the HTTP API.

## Authentication

All endpoints require:
- **Base URL**: Read from `STARK_URL` environment variable
- **Bearer token**: Read from `STARK_TOKEN` environment variable

Header: `Authorization: Bearer $STARK_TOKEN`

If either variable is missing, prompt the user to set them:
```bash
export STARK_URL="http://localhost:3000"
export STARK_TOKEN="your-token-here"
```

## API Endpoints

### Send Message (synchronous)

```bash
curl -s -X POST "$STARK_URL/api/gateway/chat" \
  -H "Authorization: Bearer $STARK_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "your message here",
    "session_id": "optional-session-id",
    "user_name": "optional-user-name"
  }'
```

Returns the full agent response as JSON. Use this for normal interactions.

### Send Message (streaming)

```bash
curl -s -N -X POST "$STARK_URL/api/gateway/chat/stream" \
  -H "Authorization: Bearer $STARK_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "your message here",
    "session_id": "optional-session-id",
    "user_name": "optional-user-name"
  }'
```

Returns Server-Sent Events (SSE). Use for long-running responses where incremental output is preferred.

### Create New Session

```bash
curl -s -X POST "$STARK_URL/api/gateway/sessions/new" \
  -H "Authorization: Bearer $STARK_TOKEN" \
  -H "Content-Type: application/json"
```

Returns a new `session_id`. Use this when starting a new conversation thread.

### List Sessions

```bash
curl -s -X GET "$STARK_URL/api/gateway/sessions" \
  -H "Authorization: Bearer $STARK_TOKEN"
```

Returns all available sessions.

### Get Session Messages

```bash
curl -s -X GET "$STARK_URL/api/gateway/sessions/{session_id}/messages" \
  -H "Authorization: Bearer $STARK_TOKEN"
```

Returns the message history for a given session.

## Session Management

- If `STARK_SESSION` is set in the environment, use it as the `session_id` in chat requests.
- When a response returns a `session_id`, store it and reuse it for follow-up messages in the same conversation.
- Use the "Create New Session" endpoint when the user wants to start a fresh conversation.

## Error Handling

- **401 Unauthorized**: Token is invalid or expired. Ask the user to check `STARK_TOKEN`.
- **404 Not Found**: Endpoint not available. Verify `STARK_URL` is correct.
- **Connection refused**: Starkbot instance is not running at the configured URL.
- **Empty response**: The agent may still be processing. Consider using the streaming endpoint.

## Request Body Reference

The chat endpoints accept:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `message` | string | yes | The message to send to the agent |
| `session_id` | string | no | Session ID for conversation continuity |
| `user_name` | string | no | Display name for the sender |
