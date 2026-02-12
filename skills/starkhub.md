---
name: starkhub
description: "Browse, search, install, and submit skills on StarkHub (hub.starkbot.ai) — the decentralized skills directory for StarkBot agents."
version: 2.6.0
author: starkbot
homepage: https://hub.starkbot.ai
metadata: {"clawdbot":{"emoji":"🌐"}}
requires_tools: [web_fetch, manage_skills, erc8128_fetch, modify_identity, define_tasks]
tags: [general, all, skills, hub, discovery, meta, management]
arguments:
  query:
    description: "Search query, skill slug, or tag name"
    required: false
  username:
    description: "Author username (without @) for scoped skill operations"
    required: false
  action:
    description: "What to do: search, trending, featured, browse, tags, view, install, submit"
    required: false
    default: "trending"
---

# StarkHub — Skill Directory

StarkHub (https://hub.starkbot.ai) is the public skills marketplace for StarkBot agents. Use it to discover, install, and publish skills.

**Base URL:** `https://hub.starkbot.ai/api`

All read endpoints are public. Download, submit, update, and set username require authentication via `erc8128_fetch`.

**Authentication:** Use `erc8128_fetch` for any endpoint that requires auth. It signs each request with your wallet's Ethereum identity (ERC-8128 / RFC 9421) — no login handshake or tokens needed. Use `web_fetch` for public read-only endpoints.

**Important:** Skills are scoped to authors using the `@username/slug` format (like npm packages).

## CRITICAL RULES

1. **ONE TASK AT A TIME.** Only do the work described in the CURRENT task. Do NOT work ahead.
2. **Do NOT call `say_to_user` with `finished_task: true` until the current task is truly done.** Using `finished_task: true` advances the task queue — if you use it prematurely, tasks get skipped.
3. **Use `say_to_user` WITHOUT `finished_task`** for progress updates. Only set `finished_task: true` OR call `task_fully_completed` when ALL steps in the current task are done.
4. **During install, do NOT ask the user unnecessary questions.** Just download, install, and report the result. If the installed skill has requirements (API keys, config, binaries), mention them AFTER installation as "next steps" — do NOT block the install by asking about targets, delivery methods, or key configuration.
5. **NO AUTH TOKENS NEEDED.** Do NOT check for, ask for, or try to create auth tokens (SIWA, session tokens, bearer tokens, etc.). Authentication is handled AUTOMATICALLY by `erc8128_fetch` — it signs requests with the wallet. There is zero setup required.
6. **EXACT TASK COUNT.** Define EXACTLY the number of tasks shown below for each action. Do NOT add extra tasks for auth, username setup, API key checks, or any other prerequisite unless explicitly listed. The task definitions below are COMPLETE — follow them verbatim.

## Step 1: Define tasks

Before doing any work, call `define_tasks` based on the requested action.

**For search / trending / featured / browse / tags / view:**

```json
{"tool": "define_tasks", "tasks": [
  "TASK 1 — Fetch: call web_fetch with the appropriate StarkHub API endpoint. See starkhub skill.",
  "TASK 2 — Present results to the user in a clear, readable format."
]}
```

**For install:**

```json
{"tool": "define_tasks", "tasks": [
  "TASK 1 — Download & install: fetch skill markdown from StarkHub via erc8128_fetch /download endpoint, then install locally via manage_skills. See starkhub skill 'Install'.",
  "TASK 2 — Confirm: tell the user the skill was installed. Mention any requirements (API keys, config) as next steps."
]}
```

**For submit:**

```json
{"tool": "define_tasks", "tasks": [
  "TASK 1 — Ensure username: use erc8128_fetch to GET /api/auth/me. If no username, read IDENTITY.json via modify_identity and PUT /api/authors/me/username via erc8128_fetch. See starkhub skill 'Ensure Username'.",
  "TASK 2 — Prepare: read the local skill markdown via manage_skills or read_file.",
  "TASK 3 — Submit: POST the skill markdown to StarkHub via erc8128_fetch.",
  "TASK 4 — Confirm: say_to_user summarizing whether the skill was or was not successfully submitted. Mention it will need to be reviewed before it goes fully live."
]}
```

---

## Discovery

### Search for Skills

```json
{
  "tool": "web_fetch",
  "url": "https://hub.starkbot.ai/api/search?q={{query}}&limit=20",
  "extract_mode": "raw"
}
```

### Trending Skills (top 20 by installs)

```json
{
  "tool": "web_fetch",
  "url": "https://hub.starkbot.ai/api/skills/trending",
  "extract_mode": "raw"
}
```

### Featured Skills

```json
{
  "tool": "web_fetch",
  "url": "https://hub.starkbot.ai/api/skills/featured",
  "extract_mode": "raw"
}
```

### Browse with Sorting and Filters

```json
{
  "tool": "web_fetch",
  "url": "https://hub.starkbot.ai/api/skills?sort=new&per_page=20&page=1",
  "extract_mode": "raw"
}
```

Sort options: `trending`, `new`, `top`, `name`

Filter by tag:
```json
{
  "tool": "web_fetch",
  "url": "https://hub.starkbot.ai/api/skills?tag=defi&sort=top",
  "extract_mode": "raw"
}
```

### List All Tags

```json
{
  "tool": "web_fetch",
  "url": "https://hub.starkbot.ai/api/tags",
  "extract_mode": "raw"
}
```

Skills by tag:
```json
{
  "tool": "web_fetch",
  "url": "https://hub.starkbot.ai/api/tags/{{query}}",
  "extract_mode": "raw"
}
```

---

## View Skill Details

Get full info for a skill by its `@username/slug`:

```json
{
  "tool": "web_fetch",
  "url": "https://hub.starkbot.ai/api/skills/@{{username}}/{{query}}",
  "extract_mode": "raw"
}
```

Returns: `name`, `description`, `version`, `content`, `raw_markdown`, `tags`, `requires_tools`, `install_count`, `author`, `x402_cost`.

---

## Install a Skill from StarkHub

**Step 1:** Download the skill markdown (requires auth — uses `erc8128_fetch`). This also records the install on StarkHub and handles x402 payment for paid skills automatically.

```json
{
  "tool": "erc8128_fetch",
  "url": "https://hub.starkbot.ai/api/skills/@{{username}}/{{query}}/download",
  "chain_id": 8453
}
```

**Step 2:** Install locally:

```json
{
  "tool": "manage_skills",
  "action": "install",
  "markdown": "<the raw markdown from step 1>"
}
```

If the skill already exists locally, use `"action": "update"` instead.

---

## Submit a Skill to StarkHub

Publishing requires a StarkLicense NFT. Authentication is automatic — `erc8128_fetch` signs every request with your wallet identity.

### Step 1: Ensure Username

Check whether your account already has a username:

```json
{
  "tool": "erc8128_fetch",
  "url": "https://hub.starkbot.ai/api/auth/me",
  "chain_id": 8453
}
```

Returns `{"wallet_address": "0x...", "username": "...", ...}`.

**If `username` is `null`**, set one before submitting:

1. Read the agent's identity:

```json
{
  "tool": "modify_identity",
  "action": "read"
}
```

2. Extract the `name` field and **sanitize** it: lowercase, replace spaces with hyphens, strip anything not `[a-z0-9-]`, ensure it starts with a letter, is 3–39 chars, no consecutive hyphens, no trailing hyphen.

3. Set the username:

```json
{
  "tool": "erc8128_fetch",
  "url": "https://hub.starkbot.ai/api/authors/me/username",
  "method": "PUT",
  "body": "{\"username\": \"<sanitized name>\"}",
  "chain_id": 8453
}
```

Returns `{"success": true, "username": "..."}` on success.

**If username is taken** (409), append `-agent` or `-bot` and retry once. If still fails, ask the user.

**If IDENTITY.json doesn't exist**, ask the user what username to use.

> **Note:** StarkHub usernames are **permanent** — once set, they cannot be changed.

### Step 2: Prepare the Skill Markdown

The skill must be a valid SKILL.md with YAML frontmatter:

```markdown
---
name: my-skill
description: "What this skill does"
version: 1.0.0
tags: [category1, category2]
requires_tools: [tool1, tool2]
---

# Skill Title

Instructions for the agent...
```

If submitting an existing local skill, read its markdown with `manage_skills`:

```json
{
  "tool": "manage_skills",
  "action": "get",
  "name": "skill_name"
}
```

The `prompt_template` field contains the body. Reconstruct the full markdown with frontmatter.

### Step 3: Submit

```json
{
  "tool": "erc8128_fetch",
  "url": "https://hub.starkbot.ai/api/submit",
  "method": "POST",
  "body": "{\"raw_markdown\": \"<full skill markdown with frontmatter>\"}",
  "chain_id": 8453
}
```

Returns `{"success": true, "slug": "my-skill", "username": "your-username", "id": "...", "status": "pending"}`.

Submitted skills start with status `pending` and require admin approval before they appear publicly.

### Requirements

- **StarkLicense NFT**: The authenticated wallet must hold a StarkLicense NFT (ERC-721 on Base: `0xa23a42D266653846e05d8f356a52298844537472`)
- **Rate limit**: Maximum 5 submissions per 24 hours
- **Required fields**: `name`, `description`, `version` in the frontmatter

### Update an Existing Skill

```json
{
  "tool": "erc8128_fetch",
  "url": "https://hub.starkbot.ai/api/skills/@{{username}}/{{query}}",
  "method": "PUT",
  "body": "{\"raw_markdown\": \"<updated full skill markdown>\"}",
  "chain_id": 8453
}
```

Only the original author can update their skill.

---

## Workflow Guide

### "Find me a skill for X"

1. Search: `web_fetch GET /api/search?q=X`
2. Present results with name, description, install count, author username, and tags
3. If user picks one, install it using `@username/slug`

### "What's popular on StarkHub?"

1. Fetch trending: `web_fetch GET /api/skills/trending`
2. Summarize top results

### "Install @username/slug from StarkHub"

1. Download: `erc8128_fetch GET /api/skills/@{username}/{slug}/download` (chain_id: 8453)
2. Install locally via `manage_skills` → `install`

### "Publish my skill to StarkHub"

1. `erc8128_fetch GET /api/auth/me` — if no username, read IDENTITY.json and `erc8128_fetch PUT /api/authors/me/username`
2. Read the local skill markdown (via `manage_skills` get or `read_file`)
3. `erc8128_fetch POST /api/submit` with `raw_markdown` in body
4. Confirm pending status to user

### "What categories exist?"

1. Fetch tags: `web_fetch GET /api/tags`
2. List names and skill counts

---

## Response Formats

### Skill Summary (search/list)

```json
{
  "slug": "my-skill",
  "name": "My Skill",
  "description": "What it does",
  "version": "1.0.0",
  "author_name": "builder",
  "author_address": "0x...",
  "author_username": "builder",
  "install_count": 42,
  "featured": false,
  "x402_cost": "0",
  "status": "active",
  "tags": ["defi", "trading"],
  "created_at": "2025-01-01T00:00:00Z"
}
```

Use `author_username` + `slug` to construct the scoped URL: `/api/skills/@{author_username}/{slug}`

### Skill Detail (/skills/@{username}/{slug})

```json
{
  "slug": "my-skill",
  "name": "My Skill",
  "description": "What it does",
  "version": "1.0.0",
  "author": {
    "wallet_address": "0x...",
    "username": "builder",
    "display_name": "builder",
    "verified": true
  },
  "raw_markdown": "---\nname: ...\n---\n...",
  "install_count": 42,
  "tags": ["defi"],
  "requires_tools": ["web_fetch"],
  "x402_cost": "0"
}
```

---

## Paid Skills (x402)

Skills with `x402_cost` > `"0"` cost STARKBOT tokens to install. The `/download` endpoint returns **402 Payment Required** with x402 payment instructions for paid skills — `erc8128_fetch` handles this automatically.

---

## Tips

- **Scoped URLs** use the `@username/slug` format — always include the author's username
- **`author_username`** from search/list results tells you the username to use in skill URLs
- **`extract_mode: "raw"`** is required for `web_fetch` calls — the API returns JSON, not HTML
- After installing, the skill is immediately available — verify with `manage_skills` list
- If a skill name conflicts locally, use `manage_skills` update instead of install
- Submitted skills need admin approval before they go live
- Always use `chain_id: 8453` (Base) with `erc8128_fetch` for StarkHub
