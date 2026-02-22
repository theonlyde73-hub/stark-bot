---
name: turso
description: "Query and manage Turso/libSQL databases via the HTTP pipeline API."
version: 1.0.0
author: starkbot
homepage: https://docs.turso.tech/sdk/http/reference
metadata: {"clawdbot":{"emoji":"🗄️"}}
requires_tools: [run_skill_script]
requires_binaries: [uv]
scripts: [turso.py]
requires_api_keys:
  TURSO_DATABASE_URL:
    description: "Turso database HTTP URL (e.g. https://mydb-myorg.turso.io)"
    secret: false
  TURSO_GROUP_TOKEN:
    description: "Turso group auth token for the database"
    secret: true
tags: [turso, libsql, database, sql, sqlite]
arguments:
  action:
    description: "Action: list_tables, describe_table, query, execute"
    required: false
  sql:
    description: "SQL statement to run"
    required: false
  table:
    description: "Table name (for describe_table)"
    required: false
---

# Turso Database Skill

You can interact with Turso/libSQL databases using the `run_skill_script` tool with `turso.py`.

## Quick Reference

All calls follow this pattern:
```json
{
  "script": "turso.py",
  "action": "<action>",
  "args": { ... },
  "skill_name": "turso"
}
```

## Actions

### List all tables
```json
{ "action": "list_tables", "args": {} }
```

### Describe a table (columns, types, constraints)
```json
{ "action": "describe_table", "args": { "table": "users" } }
```

### Run a read-only query
```json
{ "action": "query", "args": { "sql": "SELECT * FROM users LIMIT 10" } }
```

### Execute a write statement
```json
{ "action": "execute", "args": { "sql": "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')" } }
```

## Workflow

1. **Explore**: Use `list_tables` to see what's in the database
2. **Understand**: Use `describe_table` to see column definitions
3. **Query**: Use `query` for SELECT statements
4. **Modify**: Use `execute` for INSERT/UPDATE/DELETE/CREATE statements

## SQL Reference

See [sql_reference.md](sql_reference.md) for complete SQL syntax guidance including:
- Data types (TEXT, INTEGER, REAL, BLOB)
- CREATE TABLE with constraints, foreign keys, defaults
- INSERT (single, bulk, upsert with ON CONFLICT)
- SELECT (filtering, joins, aggregation, subqueries)
- UPDATE and DELETE (always use WHERE!)
- ALTER TABLE, indexes, date/time functions, JSON functions
- Best practices (always LIMIT, always WHERE, escape quotes)

**Always consult the SQL reference when constructing queries** — Turso runs libSQL (SQLite fork), so standard SQLite syntax applies.

## Important Notes

- `query` is for read-only SQL (SELECT). `execute` is for write SQL (INSERT, UPDATE, DELETE, CREATE, DROP, ALTER).
- Always confirm with the user before executing destructive statements (DROP, DELETE, TRUNCATE).
- The database URL should NOT include a trailing slash.
