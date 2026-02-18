use crate::db::Database;

/// Configuration for memory importance decay.
pub struct DecayConfig {
    /// Number of days for importance to halve (default: 30.0).
    pub half_life_days: f64,
    /// Bonus importance added when a memory was recently accessed (default: 1.0).
    pub access_boost: f64,
    /// Importance threshold below which a memory may be pruned (default: 2.0).
    pub prune_threshold: f64,
    /// Hard age limit in days — non-exempt memories older than this are pruned
    /// regardless of importance (default: 30.0). Set to 0.0 to disable.
    pub max_age_days: f64,
    /// Memory types that are exempt from pruning (default: ["preference", "fact"]).
    pub exempt_types: Vec<String>,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            half_life_days: 30.0,
            access_boost: 1.0,
            prune_threshold: 2.0,
            max_age_days: 30.0,
            exempt_types: vec!["preference".to_string(), "fact".to_string()],
        }
    }
}

/// Calculate the decayed importance of a memory.
///
/// Formula: `original * 2^(-days / half_life) + access_boost` (if accessed recently,
/// i.e. days_since_last_access < 1.0).
pub fn calculate_decayed_importance(
    original_importance: f64,
    days_since_last_access: f64,
    config: &DecayConfig,
) -> f64 {
    let decay_factor = 2.0_f64.powf(-days_since_last_access / config.half_life_days);
    let decayed = original_importance * decay_factor;

    // Apply access boost if the memory was accessed recently (within the last day)
    if days_since_last_access < 1.0 {
        decayed + config.access_boost
    } else {
        decayed
    }
}

/// Determine whether a memory should be pruned based on its current importance,
/// age, and type.
///
/// Returns `true` if:
/// - The importance is below the prune threshold, OR
/// - The memory is older than `max_age_days` (hard age limit)
/// AND the memory type is not in the exempt list.
pub fn should_prune(current_importance: f64, memory_type: &str, days_since_access: f64, config: &DecayConfig) -> bool {
    if config.exempt_types.iter().any(|t| t == memory_type) {
        return false;
    }
    // Hard age-based pruning: memories not accessed in max_age_days get pruned
    if config.max_age_days > 0.0 && days_since_access >= config.max_age_days {
        return true;
    }
    current_importance < config.prune_threshold
}

/// Run a full decay pass over all memories in the database.
///
/// For each memory, calculates the decayed importance based on time since last
/// access, updates the importance value, and optionally prunes memories that
/// fall below the threshold.
///
/// Returns `(updated_count, pruned_count)` on success.
pub fn run_decay_pass(db: &Database, config: &DecayConfig) -> Result<(usize, usize), String> {
    let conn = db.conn();

    // Fetch all memories with their current importance, type, and last access time
    let mut stmt = conn
        .prepare(
            "SELECT id, importance, memory_type, last_accessed
             FROM memories",
        )
        .map_err(|e| format!("Failed to prepare decay query: {}", e))?;

    let memories: Vec<(i64, f64, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("Failed to query memories for decay: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let now = chrono::Utc::now();
    let mut updated_count: usize = 0;
    let mut pruned_count: usize = 0;

    for (id, original_importance, memory_type, last_accessed) in &memories {
        // Parse the last_accessed timestamp
        let last_access_time = chrono::DateTime::parse_from_rfc3339(last_accessed)
            .or_else(|_| {
                chrono::NaiveDateTime::parse_from_str(last_accessed, "%Y-%m-%d %H:%M:%S")
                    .map(|naive| {
                        naive
                            .and_utc()
                            .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
                    })
            })
            .unwrap_or_else(|_| now.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

        let days_since_access = (now - last_access_time.with_timezone(&chrono::Utc))
            .num_seconds() as f64
            / 86400.0;

        let decayed_importance =
            calculate_decayed_importance(*original_importance, days_since_access, config);

        if should_prune(decayed_importance, memory_type, days_since_access, config) {
            // Delete the memory and its related data atomically
            conn.execute_batch("SAVEPOINT prune_memory")
                .map_err(|e| format!("Failed to start savepoint for memory {}: {}", id, e))?;

            let prune_result = (|| -> Result<(), String> {
                conn.execute(
                    "DELETE FROM memory_embeddings WHERE memory_id = ?1",
                    rusqlite::params![id],
                )
                .map_err(|e| format!("Failed to delete embedding for memory {}: {}", id, e))?;

                conn.execute(
                    "DELETE FROM memory_associations WHERE source_memory_id = ?1 OR target_memory_id = ?1",
                    rusqlite::params![id],
                )
                .map_err(|e| format!("Failed to delete associations for memory {}: {}", id, e))?;

                conn.execute(
                    "DELETE FROM memories WHERE id = ?1",
                    rusqlite::params![id],
                )
                .map_err(|e| format!("Failed to prune memory {}: {}", id, e))?;

                Ok(())
            })();

            match prune_result {
                Ok(()) => {
                    conn.execute_batch("RELEASE prune_memory")
                        .map_err(|e| format!("Failed to release savepoint: {}", e))?;
                }
                Err(e) => {
                    let _ = conn.execute_batch("ROLLBACK TO prune_memory");
                    let _ = conn.execute_batch("RELEASE prune_memory");
                    return Err(e);
                }
            }

            pruned_count += 1;
            log::info!(
                "Pruned memory {} (type={}, decayed_importance={:.2})",
                id,
                memory_type,
                decayed_importance
            );
        } else {
            // Update the importance value (stored as real for smooth decay)
            conn.execute(
                "UPDATE memories SET importance = ?1 WHERE id = ?2",
                rusqlite::params![decayed_importance, id],
            )
            .map_err(|e| format!("Failed to update importance for memory {}: {}", id, e))?;

            updated_count += 1;
        }
    }

    log::info!(
        "Decay pass complete: {} updated, {} pruned out of {} total",
        updated_count,
        pruned_count,
        memories.len()
    );

    Ok((updated_count, pruned_count))
}
