use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

use crate::models::{ApiKey, Channel, Session};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(database_url: &str) -> SqliteResult<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(database_url).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }

        let conn = Connection::open(database_url)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        // Sessions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                token TEXT UNIQUE NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )",
            [],
        )?;

        // External API keys table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS external_api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service_name TEXT UNIQUE NOT NULL,
                api_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // External channels table (Telegram, Slack, etc.)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS external_channels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_type TEXT NOT NULL,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 0,
                bot_token TEXT NOT NULL,
                app_token TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(channel_type, name)
            )",
            [],
        )?;

        Ok(())
    }

    // Session methods
    pub fn create_session(&self) -> SqliteResult<Session> {
        let conn = self.conn.lock().unwrap();
        let token = Uuid::new_v4().to_string();
        let created_at = Utc::now();
        let expires_at = created_at + Duration::hours(24);

        conn.execute(
            "INSERT INTO sessions (token, created_at, expires_at) VALUES (?1, ?2, ?3)",
            [
                &token,
                &created_at.to_rfc3339(),
                &expires_at.to_rfc3339(),
            ],
        )?;

        let id = conn.last_insert_rowid();

        Ok(Session {
            id,
            token,
            created_at,
            expires_at,
        })
    }

    pub fn validate_session(&self, token: &str) -> SqliteResult<Option<Session>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let mut stmt = conn.prepare(
            "SELECT id, token, created_at, expires_at FROM sessions WHERE token = ?1 AND expires_at > ?2",
        )?;

        let session = stmt
            .query_row([token, &now], |row| {
                let created_at_str: String = row.get(2)?;
                let expires_at_str: String = row.get(3)?;

                Ok(Session {
                    id: row.get(0)?,
                    token: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    expires_at: DateTime::parse_from_rfc3339(&expires_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })
            .ok();

        Ok(session)
    }

    pub fn delete_session(&self, token: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute("DELETE FROM sessions WHERE token = ?1", [token])?;
        Ok(rows_affected > 0)
    }

    // API Key methods
    pub fn get_api_key(&self, service_name: &str) -> SqliteResult<Option<ApiKey>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, service_name, api_key, created_at, updated_at FROM external_api_keys WHERE service_name = ?1",
        )?;

        let api_key = stmt
            .query_row([service_name], |row| {
                let created_at_str: String = row.get(3)?;
                let updated_at_str: String = row.get(4)?;

                Ok(ApiKey {
                    id: row.get(0)?,
                    service_name: row.get(1)?,
                    api_key: row.get(2)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })
            .ok();

        Ok(api_key)
    }

    pub fn list_api_keys(&self) -> SqliteResult<Vec<ApiKey>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, service_name, api_key, created_at, updated_at FROM external_api_keys ORDER BY service_name",
        )?;

        let api_keys = stmt
            .query_map([], |row| {
                let created_at_str: String = row.get(3)?;
                let updated_at_str: String = row.get(4)?;

                Ok(ApiKey {
                    id: row.get(0)?,
                    service_name: row.get(1)?,
                    api_key: row.get(2)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(api_keys)
    }

    pub fn upsert_api_key(&self, service_name: &str, api_key: &str) -> SqliteResult<ApiKey> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // Try to update first
        let rows_affected = conn.execute(
            "UPDATE external_api_keys SET api_key = ?1, updated_at = ?2 WHERE service_name = ?3",
            [api_key, &now, service_name],
        )?;

        if rows_affected == 0 {
            // Insert new
            conn.execute(
                "INSERT INTO external_api_keys (service_name, api_key, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                [service_name, api_key, &now, &now],
            )?;
        }

        drop(conn);

        // Return the upserted key
        self.get_api_key(service_name).map(|opt| opt.unwrap())
    }

    pub fn delete_api_key(&self, service_name: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "DELETE FROM external_api_keys WHERE service_name = ?1",
            [service_name],
        )?;
        Ok(rows_affected > 0)
    }

    // Channel methods
    pub fn create_channel(
        &self,
        channel_type: &str,
        name: &str,
        bot_token: &str,
        app_token: Option<&str>,
    ) -> SqliteResult<Channel> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO external_channels (channel_type, name, enabled, bot_token, app_token, created_at, updated_at)
             VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6)",
            rusqlite::params![channel_type, name, bot_token, app_token, &now, &now],
        )?;

        let id = conn.last_insert_rowid();

        Ok(Channel {
            id,
            channel_type: channel_type.to_string(),
            name: name.to_string(),
            enabled: false,
            bot_token: bot_token.to_string(),
            app_token: app_token.map(|s| s.to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn get_channel(&self, id: i64) -> SqliteResult<Option<Channel>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, channel_type, name, enabled, bot_token, app_token, created_at, updated_at
             FROM external_channels WHERE id = ?1",
        )?;

        let channel = stmt
            .query_row([id], |row| {
                let created_at_str: String = row.get(6)?;
                let updated_at_str: String = row.get(7)?;

                Ok(Channel {
                    id: row.get(0)?,
                    channel_type: row.get(1)?,
                    name: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    bot_token: row.get(4)?,
                    app_token: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })
            .ok();

        Ok(channel)
    }

    pub fn list_channels(&self) -> SqliteResult<Vec<Channel>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, channel_type, name, enabled, bot_token, app_token, created_at, updated_at
             FROM external_channels ORDER BY channel_type, name",
        )?;

        let channels = stmt
            .query_map([], |row| {
                let created_at_str: String = row.get(6)?;
                let updated_at_str: String = row.get(7)?;

                Ok(Channel {
                    id: row.get(0)?,
                    channel_type: row.get(1)?,
                    name: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    bot_token: row.get(4)?,
                    app_token: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(channels)
    }

    pub fn list_enabled_channels(&self) -> SqliteResult<Vec<Channel>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, channel_type, name, enabled, bot_token, app_token, created_at, updated_at
             FROM external_channels WHERE enabled = 1 ORDER BY channel_type, name",
        )?;

        let channels = stmt
            .query_map([], |row| {
                let created_at_str: String = row.get(6)?;
                let updated_at_str: String = row.get(7)?;

                Ok(Channel {
                    id: row.get(0)?,
                    channel_type: row.get(1)?,
                    name: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    bot_token: row.get(4)?,
                    app_token: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(channels)
    }

    pub fn update_channel(
        &self,
        id: i64,
        name: Option<&str>,
        enabled: Option<bool>,
        bot_token: Option<&str>,
        app_token: Option<Option<&str>>,
    ) -> SqliteResult<Option<Channel>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // Build dynamic update query
        let mut updates = vec!["updated_at = ?1".to_string()];
        let mut param_idx = 2;

        if name.is_some() {
            updates.push(format!("name = ?{}", param_idx));
            param_idx += 1;
        }
        if enabled.is_some() {
            updates.push(format!("enabled = ?{}", param_idx));
            param_idx += 1;
        }
        if bot_token.is_some() {
            updates.push(format!("bot_token = ?{}", param_idx));
            param_idx += 1;
        }
        if app_token.is_some() {
            updates.push(format!("app_token = ?{}", param_idx));
            param_idx += 1;
        }

        let sql = format!(
            "UPDATE external_channels SET {} WHERE id = ?{}",
            updates.join(", "),
            param_idx
        );

        // Build params dynamically
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(now)];

        if let Some(n) = name {
            params.push(Box::new(n.to_string()));
        }
        if let Some(e) = enabled {
            params.push(Box::new(if e { 1 } else { 0 }));
        }
        if let Some(t) = bot_token {
            params.push(Box::new(t.to_string()));
        }
        if let Some(at) = app_token {
            params.push(Box::new(at.map(|s| s.to_string())));
        }
        params.push(Box::new(id));

        let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_ref.as_slice())?;

        drop(conn);
        self.get_channel(id)
    }

    pub fn set_channel_enabled(&self, id: i64, enabled: bool) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let rows_affected = conn.execute(
            "UPDATE external_channels SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![if enabled { 1 } else { 0 }, &now, id],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn delete_channel(&self, id: i64) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute(
            "DELETE FROM external_channels WHERE id = ?1",
            [id],
        )?;
        Ok(rows_affected > 0)
    }
}
