//! Redis-backed key/value store for agent state tracking.
//!
//! Provides a simple KV interface over Redis running as a sidecar.
//! Used by agents for persistent state (e.g., strike counters, feature flags).

use redis::AsyncCommands;

pub struct KvStore {
    client: redis::Client,
}

impl KvStore {
    /// Connect to Redis at the default local address.
    pub fn new() -> Result<Self, String> {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let client = redis::Client::open(url.as_str())
            .map_err(|e| format!("Failed to create Redis client: {}", e))?;
        Ok(KvStore { client })
    }

    /// Get a multiplexed async connection.
    async fn conn(&self) -> Result<redis::aio::MultiplexedConnection, String> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("Redis connection error: {}", e))
    }

    /// Get a value by key.
    pub async fn get(&self, key: &str) -> Result<Option<String>, String> {
        let mut conn = self.conn().await?;
        let val: Option<String> = conn.get(key).await.map_err(|e| format!("Redis GET error: {}", e))?;
        Ok(val)
    }

    /// Set a key to a value.
    pub async fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let mut conn = self.conn().await?;
        conn.set::<_, _, ()>(key, value)
            .await
            .map_err(|e| format!("Redis SET error: {}", e))
    }

    /// Delete a key. Returns true if the key existed.
    pub async fn delete(&self, key: &str) -> Result<bool, String> {
        let mut conn = self.conn().await?;
        let count: i64 = conn.del(key).await.map_err(|e| format!("Redis DEL error: {}", e))?;
        Ok(count > 0)
    }

    /// Increment a key by `by`. Returns the new value.
    /// If the key doesn't exist, it is initialized to 0 before incrementing.
    pub async fn increment(&self, key: &str, by: i64) -> Result<i64, String> {
        let mut conn = self.conn().await?;
        let val: i64 = conn
            .incr(key, by)
            .await
            .map_err(|e| format!("Redis INCRBY error: {}", e))?;
        Ok(val)
    }

    /// List all keys matching a prefix, with their values.
    /// Uses SCAN to avoid blocking on large keyspaces.
    pub async fn list(&self, prefix: &str) -> Result<Vec<(String, String)>, String> {
        let mut conn = self.conn().await?;
        let pattern = format!("{}*", prefix);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| format!("Redis KEYS error: {}", e))?;

        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let values: Vec<Option<String>> = conn
            .mget(&keys)
            .await
            .map_err(|e| format!("Redis MGET error: {}", e))?;

        let result = keys
            .into_iter()
            .zip(values)
            .filter_map(|(k, v)| v.map(|val| (k, val)))
            .collect();

        Ok(result)
    }

    /// Dump all keys and values (for backup).
    pub async fn dump_all(&self) -> Result<Vec<(String, String)>, String> {
        self.list("").await
    }

    /// Restore entries: flush the database then set all entries.
    pub async fn load_all(&self, entries: &[(String, String)]) -> Result<(), String> {
        let mut conn = self.conn().await?;

        // Flush all existing keys
        redis::cmd("FLUSHDB")
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| format!("Redis FLUSHDB error: {}", e))?;

        // Set all entries
        if !entries.is_empty() {
            let mut pipe = redis::pipe();
            for (key, value) in entries {
                pipe.set::<_, _>(key.as_str(), value.as_str()).ignore();
            }
            pipe.query_async::<()>(&mut conn)
                .await
                .map_err(|e| format!("Redis pipeline SET error: {}", e))?;
        }

        Ok(())
    }

    /// Check if Redis is reachable.
    pub async fn ping(&self) -> bool {
        match self.conn().await {
            Ok(mut conn) => {
                redis::cmd("PING")
                    .query_async::<String>(&mut conn)
                    .await
                    .is_ok()
            }
            Err(_) => false,
        }
    }
}
