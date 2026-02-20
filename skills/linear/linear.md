---
name: linear
description: "Query and manage Linear issues, projects, and team workflows."
version: 1.0.0
author: starkbot
homepage: https://linear.app
metadata: {"clawdbot":{"emoji":"📊","requires":{"env":["LINEAR_API_KEY"]}}}
requires_tools: [run_skill_script]
requires_binaries: [curl, jq]
scripts: [linear.sh]
requires_api_keys:
  LINEAR_API_KEY:
    description: "Linear API key — create one at https://linear.app/settings/api"
    secret: true
tags: [linear, project-management, issues, team, workflow, development]
arguments:
  action:
    description: "Action: my-issues, my-todos, urgent, teams, team, project, issue, branch, create, comment, status, assign, priority, standup, projects"
    required: false
  identifier:
    description: "Issue identifier (e.g. TEAM-123), team key, or project name"
    required: false
  text:
    description: "Title, comment text, or description"
    required: false
---

# Linear

Manage issues, check project status, and stay on top of your team's work.

## Setup

Requires `LINEAR_API_KEY` env var. Get one at https://linear.app/settings/api

Optional: set `LINEAR_DEFAULT_TEAM` to skip the team key in create commands.

Discover team keys:
```json
{"script": "linear.sh", "action": "teams", "skill_name": "linear"}
```

## Quick Commands

All calls use the `run_skill_script` tool:

### My Stuff

```json
{"script": "linear.sh", "action": "my-issues", "skill_name": "linear"}
```

```json
{"script": "linear.sh", "action": "my-todos", "skill_name": "linear"}
```

```json
{"script": "linear.sh", "action": "urgent", "skill_name": "linear"}
```

### Browse

List teams:
```json
{"script": "linear.sh", "action": "teams", "skill_name": "linear"}
```

All issues for a team:
```json
{"script": "linear.sh", "action": "team", "args": {"team": "TEAM_KEY"}, "skill_name": "linear"}
```

Issues in a project:
```json
{"script": "linear.sh", "action": "project", "args": {"name": "Project Name"}, "skill_name": "linear"}
```

Get issue details:
```json
{"script": "linear.sh", "action": "issue", "args": {"id": "TEAM-123"}, "skill_name": "linear"}
```

Get branch name for an issue (for GitHub integration):
```json
{"script": "linear.sh", "action": "branch", "args": {"id": "TEAM-123"}, "skill_name": "linear"}
```

### Actions

Create an issue:
```json
{"script": "linear.sh", "action": "create", "args": {"team": "TEAM_KEY", "title": "Fix auth timeout", "description": "Users getting logged out after 5 min"}, "skill_name": "linear"}
```

If `LINEAR_DEFAULT_TEAM` is set, the team arg can be omitted:
```json
{"script": "linear.sh", "action": "create", "args": {"title": "Fix auth timeout"}, "skill_name": "linear"}
```

Comment on an issue:
```json
{"script": "linear.sh", "action": "comment", "args": {"id": "TEAM-123", "body": "Comment text here"}, "skill_name": "linear"}
```

Change issue status:
```json
{"script": "linear.sh", "action": "status", "args": {"id": "TEAM-123", "status": "progress"}, "skill_name": "linear"}
```
Valid statuses: `todo`, `progress`, `review`, `done`, `blocked`

Assign an issue:
```json
{"script": "linear.sh", "action": "assign", "args": {"id": "TEAM-123", "user": "userName"}, "skill_name": "linear"}
```

Set priority:
```json
{"script": "linear.sh", "action": "priority", "args": {"id": "TEAM-123", "priority": "high"}, "skill_name": "linear"}
```
Valid priorities: `urgent`, `high`, `medium`, `low`, `none`

### Overview

Daily standup summary:
```json
{"script": "linear.sh", "action": "standup", "skill_name": "linear"}
```

All projects with progress:
```json
{"script": "linear.sh", "action": "projects", "skill_name": "linear"}
```

## Common Workflows

### Morning Standup
```json
{"script": "linear.sh", "action": "standup", "skill_name": "linear"}
```
Shows: your todos, blocked items across team, recently completed, what's in review.

### Quick Issue Creation (from chat)
```json
{"script": "linear.sh", "action": "create", "args": {"team": "TEAM", "title": "Fix auth timeout bug", "description": "Users getting logged out after 5 min"}, "skill_name": "linear"}
```

### Triage Mode
```json
{"script": "linear.sh", "action": "urgent", "skill_name": "linear"}
```

## Git Workflow (Linear <-> GitHub Integration)

**Always use Linear-derived branch names** to enable automatic issue status tracking.

### Getting the Branch Name
```json
{"script": "linear.sh", "action": "branch", "args": {"id": "TEAM-212"}, "skill_name": "linear"}
```
Returns something like: `dev/team-212-fix-auth-timeout-bug`

### Why This Matters
- Linear's GitHub integration tracks PRs by branch name pattern
- When you create a PR from a Linear branch, the issue **automatically moves to "In Review"**
- When the PR merges, the issue **automatically moves to "Done"**
- Manual branch names break this automation

## Priority Levels

| Level | Value | Use for |
|-------|-------|---------|
| urgent | 1 | Production issues, blockers |
| high | 2 | This week, important |
| medium | 3 | This sprint/cycle |
| low | 4 | Nice to have |
| none | 0 | Backlog, someday |

## Notes

- Uses GraphQL API (api.linear.app/graphql)
- Issue identifiers are like `TEAM-123`
- Team keys and IDs are discovered via the API
