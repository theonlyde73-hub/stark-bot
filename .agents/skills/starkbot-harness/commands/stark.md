---
name: stark
description: Send a message to a Starkbot agent via the External Channel Gateway API
user-invocable: true
argument-description: The message to send to the Starkbot agent
---

# Starkbot Gateway: Send Message

The user wants to send a message to their Starkbot agent. Use the External Channel Gateway API.

## Configuration

Read these environment variables:
- `STARK_URL` — Base URL of the Starkbot instance (e.g. `http://localhost:3000`)
- `STARK_TOKEN` — Bearer token for authentication
- `STARK_SESSION` (optional) — Session ID for conversation persistence

If `STARK_URL` or `STARK_TOKEN` are not set, tell the user they need to export them first.

## Send the message

The user's message is: `$ARGUMENTS`

Run:

```bash
curl -s -X POST "$STARK_URL/api/gateway/chat" \
  -H "Authorization: Bearer $STARK_TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(jq -n --arg msg "$ARGUMENTS" --arg sid "${STARK_SESSION:-}" '{message: $msg} + (if $sid != "" then {session_id: $sid} else {} end)')"
```

## Handle the response

- Parse the JSON response and present the Starkbot agent's reply to the user.
- If the response contains an error, report it clearly.
- If the response includes a `session_id`, note it so the user can continue the conversation.
