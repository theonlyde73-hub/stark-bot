//! Database operations for the `memories` table
//! Core CRUD for the structured memory system (SQL-backed).

use crate::db::Database;

/// A row from the `memories` table
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryRow {
    pub id: i64,
    pub memory_type: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub importance: i64,
    pub identity_id: Option<String>,
    pub session_id: Option<i64>,
    pub entity_type: Option<String>,
    pub entity_name: Option<String>,
    pub source_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_accessed: Option<String>,
}

impl Database {
    /// Insert a new memory and return its ID.
    /// FTS index is updated automatically via database triggers.
    pub fn insert_memory(
        &self,
        memory_type: &str,
        content: &str,
        category: Option<&str>,
        tags: Option<&str>,
        importance: i64,
        identity_id: Option<&str>,
        session_id: Option<i64>,
        entity_type: Option<&str>,
        entity_name: Option<&str>,
        source_type: Option<&str>,
    ) -> Result<i64, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO memories (
                memory_type, content, category, tags, importance,
                identity_id, session_id, entity_type, entity_name,
                source_type, created_at, updated_at, last_accessed
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9,
                ?10, datetime('now'), datetime('now'), datetime('now')
            )",
            rusqlite::params![
                memory_type, content, category, tags, importance,
                identity_id, session_id, entity_type, entity_name,
                source_type,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// List all memories (for backup export).
    /// Returns lightweight rows without embeddings or associations.
    pub fn list_all_memories(&self) -> Result<Vec<MemoryRow>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, memory_type, content, category, tags, importance,
                    identity_id, session_id, entity_type, entity_name,
                    source_type, created_at, updated_at, last_accessed
             FROM memories
             ORDER BY id"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MemoryRow {
                id: row.get(0)?,
                memory_type: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                tags: row.get(4)?,
                importance: row.get::<_, Option<f64>>(5)?.map(|v| v.round() as i64).unwrap_or(5),
                identity_id: row.get(6)?,
                session_id: row.get(7)?,
                entity_type: row.get(8)?,
                entity_name: row.get(9)?,
                source_type: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                last_accessed: row.get(13)?,
            })
        })?;
        rows.collect()
    }

    /// Count total memories in the table.
    pub fn count_memories(&self) -> Result<i64, rusqlite::Error> {
        let conn = self.conn();
        conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
    }

    /// Touch a memory's last_accessed timestamp (for decay tracking).
    pub fn touch_memory(&self, memory_id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE memories SET last_accessed = datetime('now') WHERE id = ?1",
            rusqlite::params![memory_id],
        )?;
        Ok(())
    }

    /// Get a single memory by ID.
    pub fn get_memory(&self, memory_id: i64) -> Result<Option<MemoryRow>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, memory_type, content, category, tags, importance,
                    identity_id, session_id, entity_type, entity_name,
                    source_type, created_at, updated_at, last_accessed
             FROM memories WHERE id = ?1"
        )?;
        let result = stmt.query_row(rusqlite::params![memory_id], |row| {
            Ok(MemoryRow {
                id: row.get(0)?,
                memory_type: row.get(1)?,
                content: row.get(2)?,
                category: row.get(3)?,
                tags: row.get(4)?,
                importance: row.get::<_, Option<f64>>(5)?.map(|v| v.round() as i64).unwrap_or(5),
                identity_id: row.get(6)?,
                session_id: row.get(7)?,
                entity_type: row.get(8)?,
                entity_name: row.get(9)?,
                source_type: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                last_accessed: row.get(13)?,
            })
        });
        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
