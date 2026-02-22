# SQL Reference for Turso/libSQL

Turso runs **libSQL**, a fork of SQLite. All standard SQLite SQL syntax applies. Use this reference when constructing SQL for the `query` and `execute` actions.

## Data Types

libSQL supports these column types (SQLite type affinity rules apply):

| Type | Use for | Examples |
|------|---------|---------|
| `TEXT` | Strings, UUIDs, JSON, dates as strings | `'hello'`, `'2024-01-15'` |
| `INTEGER` | Integers, booleans (0/1), timestamps (unix) | `42`, `1`, `0` |
| `REAL` | Floating point numbers | `3.14`, `99.99` |
| `BLOB` | Binary data | `X'48656C6C6F'` |
| `NULL` | Missing/unknown values | `NULL` |

There is no dedicated `BOOLEAN`, `DATE`, or `DATETIME` type. Use `INTEGER` (0/1) for booleans and `TEXT` (ISO 8601) or `INTEGER` (unix timestamp) for dates.

## CREATE TABLE

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT UNIQUE NOT NULL,
    bio TEXT DEFAULT '',
    is_active INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now'))
);
```

### Common constraints

| Constraint | Meaning |
|---|---|
| `PRIMARY KEY` | Unique row identifier |
| `AUTOINCREMENT` | Auto-assign incrementing integer (use with INTEGER PRIMARY KEY) |
| `NOT NULL` | Column cannot be NULL |
| `UNIQUE` | No duplicate values allowed |
| `DEFAULT value` | Default value if not provided on insert |
| `DEFAULT (expression)` | Computed default (wrap in parens) |
| `CHECK (expr)` | Validation constraint |
| `REFERENCES table(col)` | Foreign key (must enable with PRAGMA) |

### Foreign keys

```sql
CREATE TABLE posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);
```

Cascade options: `ON DELETE CASCADE`, `ON DELETE SET NULL`, `ON DELETE RESTRICT`.

**Note:** Foreign key enforcement is OFF by default in SQLite. To enable:
```sql
PRAGMA foreign_keys = ON;
```

## INSERT

```sql
-- Single row
INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');

-- Multiple rows
INSERT INTO users (name, email) VALUES
    ('Bob', 'bob@example.com'),
    ('Carol', 'carol@example.com');

-- Insert or ignore on conflict
INSERT OR IGNORE INTO users (name, email) VALUES ('Alice', 'alice@example.com');

-- Upsert (insert or update on conflict)
INSERT INTO users (name, email, bio)
VALUES ('Alice', 'alice@example.com', 'Updated bio')
ON CONFLICT(email) DO UPDATE SET bio = excluded.bio;
```

## SELECT

```sql
-- Basic
SELECT * FROM users;
SELECT name, email FROM users WHERE is_active = 1;

-- With LIMIT and OFFSET (always use LIMIT to avoid huge results)
SELECT * FROM users ORDER BY created_at DESC LIMIT 20 OFFSET 0;

-- Counting
SELECT COUNT(*) as total FROM users;
SELECT COUNT(*) as total FROM users WHERE is_active = 1;

-- Filtering
SELECT * FROM users WHERE name LIKE '%alice%';
SELECT * FROM users WHERE id IN (1, 2, 3);
SELECT * FROM users WHERE created_at > '2024-01-01';
SELECT * FROM users WHERE email IS NOT NULL;

-- Aggregation
SELECT is_active, COUNT(*) as count FROM users GROUP BY is_active;

-- Joins
SELECT p.title, u.name as author
FROM posts p
JOIN users u ON p.user_id = u.id
ORDER BY p.created_at DESC
LIMIT 10;

-- Left join (include rows with no match)
SELECT u.name, COUNT(p.id) as post_count
FROM users u
LEFT JOIN posts p ON p.user_id = u.id
GROUP BY u.id
ORDER BY post_count DESC;

-- Subquery
SELECT * FROM users
WHERE id IN (SELECT DISTINCT user_id FROM posts);
```

## UPDATE

```sql
-- Update specific rows (always use WHERE!)
UPDATE users SET bio = 'New bio' WHERE id = 1;

-- Update multiple columns
UPDATE users SET name = 'Alice B.', bio = 'Updated' WHERE email = 'alice@example.com';

-- Conditional update
UPDATE users SET is_active = 0 WHERE created_at < '2023-01-01';
```

## DELETE

```sql
-- Delete specific rows (always use WHERE!)
DELETE FROM users WHERE id = 1;

-- Delete with condition
DELETE FROM posts WHERE created_at < '2023-01-01';

-- Delete all rows (use with caution)
DELETE FROM users;
```

## ALTER TABLE

```sql
-- Add a column
ALTER TABLE users ADD COLUMN avatar_url TEXT DEFAULT '';

-- Rename a column
ALTER TABLE users RENAME COLUMN bio TO biography;

-- Rename a table
ALTER TABLE users RENAME TO app_users;

-- Drop a column (SQLite 3.35+)
ALTER TABLE users DROP COLUMN bio;
```

**Note:** SQLite does not support `ALTER TABLE ... MODIFY COLUMN` or `ALTER TABLE ... ALTER COLUMN`. To change a column type or constraint, you must recreate the table.

## Indexes

```sql
-- Create an index
CREATE INDEX idx_users_email ON users(email);

-- Unique index
CREATE UNIQUE INDEX idx_users_email ON users(email);

-- Composite index
CREATE INDEX idx_posts_user_date ON posts(user_id, created_at);

-- Drop an index
DROP INDEX idx_users_email;

-- List all indexes
SELECT * FROM sqlite_master WHERE type = 'index';
```

## Date & Time

SQLite has no dedicated date type. Store as TEXT (ISO 8601) or INTEGER (unix timestamp).

```sql
-- Current timestamp as text
SELECT datetime('now');                    -- '2024-01-15 12:30:00'
SELECT datetime('now', 'localtime');       -- local time

-- Date arithmetic
SELECT datetime('now', '-7 days');         -- 7 days ago
SELECT datetime('now', '+1 month');        -- 1 month from now
SELECT date('now', 'start of month');      -- first of current month

-- Filtering by date (text format)
SELECT * FROM posts WHERE created_at > datetime('now', '-24 hours');
SELECT * FROM posts WHERE date(created_at) = date('now');

-- Unix timestamp
SELECT unixepoch('now');                   -- current unix timestamp
SELECT datetime(1705312200, 'unixepoch');  -- unix to text
```

## JSON

libSQL/SQLite has built-in JSON functions:

```sql
-- Store JSON in a TEXT column
INSERT INTO settings (user_id, prefs) VALUES (1, '{"theme":"dark","lang":"en"}');

-- Extract a value
SELECT json_extract(prefs, '$.theme') as theme FROM settings;

-- Filter on JSON field
SELECT * FROM settings WHERE json_extract(prefs, '$.theme') = 'dark';

-- Update a JSON field
UPDATE settings SET prefs = json_set(prefs, '$.theme', 'light') WHERE user_id = 1;

-- Build JSON
SELECT json_object('id', id, 'name', name) FROM users;
```

## Useful PRAGMAs

```sql
-- Table info (same as describe_table action)
PRAGMA table_info(users);

-- List all tables
SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';

-- Database size info
PRAGMA page_count;
PRAGMA page_size;

-- Enable WAL mode (better concurrent reads)
PRAGMA journal_mode = WAL;

-- Check integrity
PRAGMA integrity_check;
```

## Best Practices

1. **Always use LIMIT** on SELECT queries to avoid returning thousands of rows. Start with `LIMIT 20`.
2. **Always use WHERE** on UPDATE and DELETE to avoid modifying all rows by accident.
3. **Use single quotes** for string literals (`'hello'`), not double quotes (those are for identifiers).
4. **Escape single quotes** by doubling them: `'it''s'`.
5. **Use parameterized values** when possible — but since this skill passes raw SQL, be careful with user-provided values.
6. **Check before destructive ops** — always `SELECT` first to preview what will be affected, then `DELETE`/`DROP`.
7. **Use transactions** for multi-step operations — but note each action is a single pipeline call, so multi-statement transactions aren't directly supported. Use multiple `execute` calls for simple cases.
