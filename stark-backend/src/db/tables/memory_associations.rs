//! Database operations for memory_associations table
//! Manages typed connections between memories (knowledge graph)

use crate::db::Database;
use serde::{Deserialize, Serialize};

/// Association record from database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAssociationRow {
    pub id: i64,
    pub source_memory_id: i64,
    pub target_memory_id: i64,
    pub association_type: String,
    pub strength: f64,
    pub metadata: Option<String>,
    pub created_at: String,
}

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGraphStats {
    pub total_associations: i64,
    pub unique_memories: i64,
    pub avg_strength: f64,
    pub type_counts: std::collections::HashMap<String, i64>,
}

impl Database {
    /// Create a new association between two memories
    pub fn create_memory_association(
        &self,
        source_memory_id: i64,
        target_memory_id: i64,
        association_type: &str,
        strength: f64,
        metadata: Option<&str>,
    ) -> Result<i64, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO memory_associations (source_memory_id, target_memory_id, association_type, strength, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            rusqlite::params![source_memory_id, target_memory_id, association_type, strength, metadata],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get associations for a memory (both directions)
    pub fn get_memory_associations(&self, memory_id: i64) -> Result<Vec<MemoryAssociationRow>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, source_memory_id, target_memory_id, association_type, strength, metadata, created_at
             FROM memory_associations
             WHERE source_memory_id = ?1 OR target_memory_id = ?1
             ORDER BY strength DESC"
        )?;
        let rows = stmt.query_map(rusqlite::params![memory_id], |row| {
            Ok(MemoryAssociationRow {
                id: row.get(0)?,
                source_memory_id: row.get(1)?,
                target_memory_id: row.get(2)?,
                association_type: row.get(3)?,
                strength: row.get(4)?,
                metadata: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    /// Get associations of a specific type for a memory
    pub fn get_memory_associations_by_type(
        &self,
        memory_id: i64,
        association_type: &str,
    ) -> Result<Vec<MemoryAssociationRow>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, source_memory_id, target_memory_id, association_type, strength, metadata, created_at
             FROM memory_associations
             WHERE (source_memory_id = ?1 OR target_memory_id = ?1) AND association_type = ?2
             ORDER BY strength DESC"
        )?;
        let rows = stmt.query_map(rusqlite::params![memory_id, association_type], |row| {
            Ok(MemoryAssociationRow {
                id: row.get(0)?,
                source_memory_id: row.get(1)?,
                target_memory_id: row.get(2)?,
                association_type: row.get(3)?,
                strength: row.get(4)?,
                metadata: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    /// Delete an association
    pub fn delete_memory_association(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let count = conn.execute(
            "DELETE FROM memory_associations WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(count > 0)
    }

    /// Count associations for a memory
    pub fn count_memory_associations(&self, memory_id: i64) -> Result<i64, rusqlite::Error> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM memory_associations
             WHERE source_memory_id = ?1 OR target_memory_id = ?1",
            rusqlite::params![memory_id],
            |row| row.get(0),
        )
    }

    /// Get graph statistics
    pub fn get_memory_graph_stats(&self) -> Result<MemoryGraphStats, rusqlite::Error> {
        let conn = self.conn();
        let total_associations: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_associations",
            [],
            |row| row.get(0),
        )?;

        let unique_memories: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT memory_id) FROM (
                SELECT source_memory_id AS memory_id FROM memory_associations
                UNION
                SELECT target_memory_id AS memory_id FROM memory_associations
            )",
            [],
            |row| row.get(0),
        )?;

        let avg_strength: f64 = conn.query_row(
            "SELECT COALESCE(AVG(strength), 0.0) FROM memory_associations",
            [],
            |row| row.get(0),
        )?;

        // Count associations by type
        let mut type_counts = std::collections::HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT association_type, COUNT(*) FROM memory_associations GROUP BY association_type",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            if let Ok((atype, count)) = row {
                type_counts.insert(atype, count);
            }
        }

        Ok(MemoryGraphStats {
            total_associations,
            unique_memories,
            avg_strength,
            type_counts,
        })
    }

    /// Get memories ranked by association count (for graph search component of hybrid search)
    pub fn get_memories_by_association_count(&self, limit: i32) -> Result<Vec<(i64, i64)>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT memory_id, COUNT(*) as cnt FROM (
                SELECT source_memory_id AS memory_id FROM memory_associations
                UNION ALL
                SELECT target_memory_id AS memory_id FROM memory_associations
            ) GROUP BY memory_id ORDER BY cnt DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(rusqlite::params![limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.collect()
    }

    /// Expand from seed memory IDs to their graph neighbors.
    /// Returns (neighbor_memory_id, total_strength_x100) pairs ranked by cumulative
    /// connection strength to the seed set.  Neighbors that ARE in the seed set
    /// are excluded so you only get new discoveries.
    pub fn graph_expand_from_seeds(
        &self,
        seed_ids: &[i64],
        limit: i32,
    ) -> Result<Vec<(i64, i32)>, rusqlite::Error> {
        if seed_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut unique_seeds: Vec<i64> = seed_ids.to_vec();
        unique_seeds.sort_unstable();
        unique_seeds.dedup();
        unique_seeds.truncate(50);

        let conn = self.conn();
        let n = unique_seeds.len();

        let make_placeholders = |offset: usize| -> String {
            (1..=n)
                .map(|i| format!("?{}", offset + i))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let p1 = make_placeholders(0);
        let p2 = make_placeholders(n);
        let p3 = make_placeholders(2 * n);
        let p4 = make_placeholders(3 * n);

        let query = format!(
            "SELECT neighbor_id, SUM(strength_int) as total_strength FROM (
                 SELECT target_memory_id AS neighbor_id,
                        CAST(strength * 100 AS INTEGER) AS strength_int
                 FROM memory_associations
                 WHERE source_memory_id IN ({p1})
                   AND target_memory_id NOT IN ({p2})
                 UNION ALL
                 SELECT source_memory_id AS neighbor_id,
                        CAST(strength * 100 AS INTEGER) AS strength_int
                 FROM memory_associations
                 WHERE target_memory_id IN ({p3})
                   AND source_memory_id NOT IN ({p4})
             )
             GROUP BY neighbor_id
             ORDER BY total_strength DESC
             LIMIT ?{}",
            4 * n + 1
        );

        // Bind seed_ids four times (once per IN clause) + limit
        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for _ in 0..4 {
            for id in &unique_seeds {
                all_params.push(Box::new(*id));
            }
        }
        all_params.push(Box::new(limit));

        let mut stmt = conn.prepare(&query)?;
        let results = stmt
            .query_map(rusqlite::params_from_iter(all_params.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i32>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// List associations where both source and target are in the given memory ID set.
    /// Used for export to capture only associations within the exported subset.
    pub fn list_associations_for_memories(
        &self,
        memory_ids: &[i64],
    ) -> Result<Vec<MemoryAssociationRow>, rusqlite::Error> {
        if memory_ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.conn();
        let n = memory_ids.len();

        // Build two sets of placeholders for the IN clauses
        let p1: String = (1..=n).map(|i| format!("?{}", i)).collect::<Vec<_>>().join(", ");
        let p2: String = (1..=n).map(|i| format!("?{}", n + i)).collect::<Vec<_>>().join(", ");

        let sql = format!(
            "SELECT id, source_memory_id, target_memory_id, association_type, strength, metadata, created_at
             FROM memory_associations
             WHERE source_memory_id IN ({}) AND target_memory_id IN ({})
             ORDER BY id",
            p1, p2
        );

        // Bind memory_ids twice (once for each IN clause)
        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for _ in 0..2 {
            for id in memory_ids {
                all_params.push(Box::new(*id));
            }
        }

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(all_params.iter()),
            |row| {
                Ok(MemoryAssociationRow {
                    id: row.get(0)?,
                    source_memory_id: row.get(1)?,
                    target_memory_id: row.get(2)?,
                    association_type: row.get(3)?,
                    strength: row.get(4)?,
                    metadata: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )?;
        rows.collect()
    }

    /// Check if an association already exists between two memories
    pub fn association_exists(
        &self,
        source_id: i64,
        target_id: i64,
        association_type: &str,
    ) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memory_associations
             WHERE ((source_memory_id = ?1 AND target_memory_id = ?2)
                OR (source_memory_id = ?2 AND target_memory_id = ?1))
             AND association_type = ?3",
            rusqlite::params![source_id, target_id, association_type],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}
