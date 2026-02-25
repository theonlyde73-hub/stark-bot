use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::db::Database;
use super::embeddings::EmbeddingGenerator;
use super::vector_search;

/// Configuration for the background association discovery loop.
pub struct AssociationLoopConfig {
    /// Interval between loop iterations in seconds (default: 300 = 5 minutes).
    pub interval_secs: u64,
    /// Minimum cosine similarity to create an association (default: 0.65).
    pub similarity_threshold: f32,
    /// Maximum number of associations to create per memory (default: 10).
    pub max_associations_per_memory: usize,
    /// Number of recent memories to process per iteration (default: 50).
    pub batch_size: usize,
}

impl Default for AssociationLoopConfig {
    fn default() -> Self {
        Self {
            interval_secs: 300,
            similarity_threshold: 0.65,
            max_associations_per_memory: 10,
            batch_size: 50,
        }
    }
}

/// Metadata about a memory used for association type classification.
#[derive(Debug, Clone)]
struct MemoryMeta {
    id: i64,
    content: String,
    memory_type: String,
    category: Option<String>,
    entity_type: Option<String>,
    entity_name: Option<String>,
    log_date: Option<String>,
    superseded_by: Option<i64>,
    agent_subtype: Option<String>,
}

// ── Common stopwords to skip during entity extraction ──

const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "shall", "can", "need", "must",
    "i", "you", "he", "she", "it", "we", "they", "me", "him", "her", "us", "them",
    "my", "your", "his", "its", "our", "their",
    "this", "that", "these", "those", "what", "which", "who", "whom",
    "and", "but", "or", "nor", "not", "no", "so", "if", "then", "than",
    "at", "by", "for", "from", "in", "into", "of", "on", "to", "with",
    "up", "out", "off", "over", "under", "about", "after", "before",
    "just", "also", "very", "too", "here", "there", "now", "how", "all",
    "each", "every", "both", "few", "more", "most", "some", "any", "new",
    "old", "good", "bad", "great", "little", "big", "long",
    "asked", "result", "session", "none", "see", "get", "got", "set",
    "yes", "try", "let", "via", "etc", "one", "two",
];

// ── Category keyword mappings ──

const CATEGORY_RULES: &[(&str, &[&str])] = &[
    ("trading", &["swap", "trade", "buy", "sell", "token", "liquidity", "dex", "pool", "price", "usdc", "eth", "weth", "uniswap"]),
    ("defi", &["stake", "staking", "yield", "farm", "lend", "borrow", "vault", "apy", "apr"]),
    ("wallet", &["wallet", "transfer", "send", "receive", "balance", "fund", "gas"]),
    ("media", &["image", "photo", "draw", "picture", "generate image", "dall-e", "art"]),
    ("audio", &["voice", "audio", "microphone", "speak", "listen", "sound", "mic"]),
    ("system", &["backup", "restore", "config", "setting", "deploy", "update", "restart", "status", "health"]),
    ("search", &["search", "look up", "find", "browse", "query"]),
    ("social", &["tweet", "post", "message", "reply", "mention", "discord", "telegram"]),
    ("nft", &["nft", "mint", "collection", "opensea", "metadata"]),
    ("onchain", &["contract", "transaction", "block", "chain", "hash", "deploy contract"]),
];

/// Extract category from memory content using keyword matching.
fn extract_category(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    let mut best: Option<(&str, usize)> = None;

    for &(category, keywords) in CATEGORY_RULES {
        let hits = keywords.iter().filter(|kw| lower.contains(*kw)).count();
        if hits > 0 {
            if best.is_none() || hits > best.unwrap().1 {
                best = Some((category, hits));
            }
        }
    }

    best.map(|(cat, _)| cat.to_string())
}

/// Extract the most prominent entity name from memory content.
///
/// Looks for:
/// 1. Token/coin names (ALL CAPS, 3-12 chars, not common words)
/// 2. Capitalized proper nouns after common patterns
fn extract_entity_name(content: &str) -> Option<String> {
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();

    // Strategy 1: ALL CAPS tokens (likely token/coin names like STARKBOT, USDC, ETH)
    let mut caps_candidates: HashMap<String, usize> = HashMap::new();
    for word in content.split(|c: char| !c.is_alphanumeric() && c != '_') {
        let trimmed = word.trim();
        if trimmed.len() >= 2
            && trimmed.len() <= 14
            && trimmed.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            && trimmed.chars().any(|c| c.is_ascii_alphabetic())
            && !stop.contains(trimmed.to_lowercase().as_str())
            // Skip formatting markers
            && trimmed != "OK" && trimmed != "ID" && trimmed != "URL" && trimmed != "API"
            && trimmed != "USD" && trimmed != "HTTP" && trimmed != "HTTPS" && trimmed != "JSON"
            && trimmed != "SQL" && trimmed != "CLI" && trimmed != "SSH" && trimmed != "DNS"
        {
            *caps_candidates.entry(trimmed.to_string()).or_insert(0) += 1;
        }
    }
    if let Some((name, _)) = caps_candidates.iter().max_by_key(|(_, count)| *count) {
        return Some(name.clone());
    }

    // Strategy 2: PascalCase / CamelCase proper nouns (e.g. "StarkBot", "UniSwap")
    let mut pascal_candidates: HashMap<String, usize> = HashMap::new();
    for word in content.split(|c: char| !c.is_alphanumeric()) {
        let trimmed = word.trim();
        // Must start uppercase, have at least one lowercase, 3+ chars
        if trimmed.len() >= 3
            && trimmed.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && trimmed.chars().any(|c| c.is_lowercase())
            && trimmed.chars().filter(|c| c.is_uppercase()).count() >= 2  // CamelCase
            && !stop.contains(trimmed.to_lowercase().as_str())
        {
            *pascal_candidates.entry(trimmed.to_string()).or_insert(0) += 1;
        }
    }
    if let Some((name, _)) = pascal_candidates.iter().max_by_key(|(_, count)| *count) {
        return Some(name.clone());
    }

    // Strategy 3: Capitalized words after entity-signal phrases
    let entity_signals = ["about ", "called ", "named ", "token ", "using "];
    let lower = content.to_lowercase();
    for signal in &entity_signals {
        if let Some(idx) = lower.find(signal) {
            let after = &content[idx + signal.len()..];
            let candidate: String = after
                .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                .next()
                .unwrap_or("")
                .to_string();
            if candidate.len() >= 2
                && candidate.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                && !stop.contains(candidate.to_lowercase().as_str())
            {
                return Some(candidate);
            }
        }
    }

    None
}

/// Backfill missing entity_name and category metadata on memories.
/// Scans content of memories where these fields are NULL and extracts
/// values using keyword heuristics. Returns count of memories updated.
pub fn backfill_memory_metadata(db: &Database) -> Result<usize, String> {
    let conn = db.conn();

    let mut stmt = conn
        .prepare(
            "SELECT id, content FROM memories
             WHERE (entity_name IS NULL OR entity_name = '')
                OR (category IS NULL OR category = '')",
        )
        .map_err(|e| format!("Failed to query memories for metadata backfill: {}", e))?;

    let rows: Vec<(i64, String)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("Failed to read memories for backfill: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let mut updated = 0;

    for (id, content) in &rows {
        let entity = extract_entity_name(content);
        let category = extract_category(content);

        if entity.is_none() && category.is_none() {
            continue;
        }

        // Only update fields that are currently empty
        let result = conn.execute(
            "UPDATE memories
             SET entity_name = COALESCE(NULLIF(entity_name, ''), ?1),
                 category = COALESCE(NULLIF(category, ''), ?2)
             WHERE id = ?3
               AND ((entity_name IS NULL OR entity_name = '') OR (category IS NULL OR category = ''))",
            rusqlite::params![
                entity.as_deref(),
                category.as_deref(),
                id,
            ],
        );

        match result {
            Ok(n) if n > 0 => {
                log::debug!(
                    "Backfilled metadata for memory {}: entity={:?} category={:?}",
                    id,
                    entity,
                    category,
                );
                updated += 1;
            }
            Err(e) => log::warn!("Failed to backfill metadata for memory {}: {}", id, e),
            _ => {}
        }
    }

    if updated > 0 {
        log::info!(
            "Metadata backfill complete: updated {} of {} memories",
            updated,
            rows.len()
        );
    }

    Ok(updated)
}

/// Check if two memories show signs of contradiction.
///
/// Requires shared context (same entity or category) AND at least one
/// correction/negation signal — a memory_type indicating correction, or
/// explicit contradiction language in the content.
fn has_contradiction_signals(source: &MemoryMeta, target: &MemoryMeta) -> bool {
    // Must share entity or category to be considered contradictory
    let shares_entity = match (source.entity_name.as_deref(), target.entity_name.as_deref()) {
        (Some(s), Some(t)) if !s.is_empty() => s.eq_ignore_ascii_case(t),
        _ => false,
    };
    let shares_category = match (source.category.as_deref(), target.category.as_deref()) {
        (Some(s), Some(t)) if !s.is_empty() => s.eq_ignore_ascii_case(t),
        _ => false,
    };

    if !shares_entity && !shares_category {
        return false;
    }

    // Check for correction/retraction memory types
    let correction_types = ["correction", "retraction"];
    if correction_types.iter().any(|t| source.memory_type.eq_ignore_ascii_case(t))
        || correction_types.iter().any(|t| target.memory_type.eq_ignore_ascii_case(t))
    {
        return true;
    }

    // Check content for explicit contradiction prefixes/phrases
    let contradiction_markers = [
        "correction:", "actually,", "contrary to", "no longer ",
        "this contradicts", "this is wrong", "this is incorrect",
        "previously incorrect", "was wrong about",
    ];

    let s_lower = source.content.to_lowercase();
    let t_lower = target.content.to_lowercase();

    let s_has = contradiction_markers.iter().any(|m| s_lower.contains(m));
    let t_has = contradiction_markers.iter().any(|m| t_lower.contains(m));

    // At least one side has contradiction language (asymmetric)
    s_has != t_has
}

/// Classify the association type between two memories using heuristics.
///
/// Priority order (all 7 edge types):
/// 1. supersedes   — explicit superseded_by link or compaction replacing another
/// 2. contradicts  — shared context (entity/category) + correction/negation signals
/// 3. caused_by    — same entity, different dates (temporal causation chain)
/// 4. references   — same entity_name (both mention the same entity)
/// 5. temporal     — same log_date (daily co-occurrence)
/// 6. part_of      — same category grouping
/// 7. related      — default for embedding similarity
fn classify_association_type(source: &MemoryMeta, target: &MemoryMeta) -> &'static str {
    // 1. Explicit supersession link
    if source.superseded_by == Some(target.id) || target.superseded_by == Some(source.id) {
        return "supersedes";
    }

    // 1b. Compaction replacing another memory
    if (source.memory_type == "compaction" && target.memory_type != "compaction")
        || (target.memory_type == "compaction" && source.memory_type != "compaction")
    {
        return "supersedes";
    }

    // 2. Contradiction: shared context + correction/negation signals
    if has_contradiction_signals(source, target) {
        return "contradicts";
    }

    // 3 & 4. Same entity — split into caused_by (cross-date) vs references (same/no date)
    if let (Some(s_entity), Some(t_entity)) = (source.entity_name.as_deref(), target.entity_name.as_deref()) {
        if !s_entity.is_empty() && s_entity.eq_ignore_ascii_case(t_entity) {
            // Different dates on the same entity = temporal causation chain
            if let (Some(s_date), Some(t_date)) = (source.log_date.as_deref(), target.log_date.as_deref()) {
                if !s_date.is_empty() && !t_date.is_empty() && s_date != t_date {
                    return "caused_by";
                }
            }
            // Same date or no date = co-reference
            return "references";
        }
    }

    // 5. Same log date = temporal co-occurrence
    if let (Some(s_date), Some(t_date)) = (source.log_date.as_deref(), target.log_date.as_deref()) {
        if !s_date.is_empty() && s_date == t_date {
            return "temporal";
        }
    }

    // 6. Same category = part_of (grouped under the same topic)
    if let (Some(s_cat), Some(t_cat)) = (source.category.as_deref(), target.category.as_deref()) {
        if !s_cat.is_empty() && s_cat.eq_ignore_ascii_case(t_cat) {
            return "part_of";
        }
    }

    // 7. Same agent_subtype = agent_subtype (co-locality within the same agent)
    if let (Some(s_sub), Some(t_sub)) = (source.agent_subtype.as_deref(), target.agent_subtype.as_deref()) {
        if !s_sub.is_empty() && s_sub.eq_ignore_ascii_case(t_sub) {
            return "agent_subtype";
        }
    }

    // 8. Default: related by embedding similarity
    "related"
}

/// Spawn a background tokio task that periodically discovers and creates
/// associations between memories based on embedding similarity.
///
/// The loop runs indefinitely, sleeping for `config.interval_secs` between
/// iterations. Errors are logged and do not halt the loop.
pub fn spawn_association_loop(
    db: Arc<Database>,
    embedding_generator: Arc<dyn EmbeddingGenerator + Send + Sync>,
    config: AssociationLoopConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        log::info!(
            "Association loop started (interval={}s, threshold={}, max_per_memory={}, batch={})",
            config.interval_secs,
            config.similarity_threshold,
            config.max_associations_per_memory,
            config.batch_size
        );

        let mut first_pass = true;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(config.interval_secs)).await;

            // On first pass, auto-backfill embeddings if coverage is very low.
            // This bootstraps the embedding pool so vector-based associations can work.
            if first_pass {
                first_pass = false;
                if let Err(e) = auto_backfill_embeddings_if_needed(&db, &embedding_generator).await {
                    log::warn!("Auto-backfill embeddings failed (non-fatal): {}", e);
                }
            }

            if let Err(e) = run_association_pass(&db, &embedding_generator, &config).await {
                log::error!("Association loop pass failed: {}", e);
            }
        }
    })
}

/// Auto-backfill embeddings when coverage is below 50%.
/// Generates embeddings for up to 200 memories per invocation to avoid
/// blocking the association loop for too long.
async fn auto_backfill_embeddings_if_needed(
    db: &Database,
    embedding_generator: &Arc<dyn EmbeddingGenerator + Send + Sync>,
) -> Result<(), String> {
    // All DB work in a block so conn/stmt are dropped before any .await
    let memories: Vec<(i64, String)> = {
        let conn = db.conn();

        let total_memories: i64 = conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count memories: {}", e))?;

        if total_memories == 0 {
            return Ok(());
        }

        let embedded_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_embeddings", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count embeddings: {}", e))?;

        let coverage = embedded_count as f64 / total_memories as f64;
        if coverage >= 0.5 {
            log::info!(
                "[Association] Embedding coverage {:.0}% ({}/{}), skipping auto-backfill",
                coverage * 100.0, embedded_count, total_memories
            );
            return Ok(());
        }

        log::info!(
            "[Association] Low embedding coverage {:.0}% ({}/{}), auto-backfilling...",
            coverage * 100.0, embedded_count, total_memories
        );

        let mut stmt = conn
            .prepare(
                "SELECT m.id, m.content FROM memories m
                 LEFT JOIN memory_embeddings e ON e.memory_id = m.id
                 WHERE e.memory_id IS NULL
                 ORDER BY m.created_at DESC
                 LIMIT 200",
            )
            .map_err(|e| format!("Failed to query memories for auto-backfill: {}", e))?;

        stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| format!("Failed to read memories: {}", e))?
            .filter_map(|r| r.ok())
            .collect()
    };

    let mut generated = 0;
    for chunk in memories.chunks(64) {
        let texts: Vec<String> = chunk.iter().map(|(_, content)| content.clone()).collect();
        match embedding_generator.generate_batch(&texts).await {
            Ok(embeddings) => {
                for ((memory_id, _), embedding) in chunk.iter().zip(embeddings.iter()) {
                    let dims = embedding.len() as i32;
                    if let Err(e) = db.upsert_memory_embedding(*memory_id, embedding, "auto_backfill", dims) {
                        log::warn!("Failed to store embedding for memory {}: {}", memory_id, e);
                        continue;
                    }
                    generated += 1;
                }
            }
            Err(e) => {
                log::warn!("[Association] Batch embedding generation failed, stopping auto-backfill: {}", e);
                break;
            }
        }
    }

    if generated > 0 {
        log::info!("[Association] Auto-backfill generated {} embeddings", generated);
    }

    Ok(())
}

/// Execute a single association discovery pass.
pub async fn run_association_pass(
    db: &Database,
    embedding_generator: &Arc<dyn EmbeddingGenerator + Send + Sync>,
    config: &AssociationLoopConfig,
) -> Result<(), String> {
    // 0. Backfill missing entity_name / category metadata from content
    if let Err(e) = backfill_memory_metadata(db) {
        log::warn!("Metadata backfill failed (non-fatal): {}", e);
    }

    // 1. Load all memory metadata for classification
    let all_metas = load_all_memory_metas(db)?;
    let meta_map: HashMap<i64, &MemoryMeta> = all_metas.iter().map(|m| (m.id, m)).collect();

    // 2. Find recent memories that have fewer than max_associations_per_memory associations
    let memories_to_process: Vec<&MemoryMeta> = {
        let conn = db.conn();

        let mut stmt = conn
            .prepare(
                "SELECT m.id
                 FROM memories m
                 LEFT JOIN (
                     SELECT memory_id, COUNT(*) AS cnt FROM (
                         SELECT source_memory_id AS memory_id FROM memory_associations
                         UNION ALL
                         SELECT target_memory_id AS memory_id FROM memory_associations
                     ) GROUP BY memory_id
                 ) a ON a.memory_id = m.id
                 WHERE COALESCE(a.cnt, 0) < ?1
                 ORDER BY m.created_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| format!("Failed to prepare association loop query: {}", e))?;

        let ids: Vec<i64> = stmt
            .query_map(
                rusqlite::params![config.max_associations_per_memory as i32, config.batch_size as i32],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to query memories for association loop: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        ids.iter().filter_map(|id| meta_map.get(id).copied()).collect()
    };

    if memories_to_process.is_empty() {
        log::info!("Association loop: no memories to process");
        return Ok(());
    }

    log::info!(
        "Association loop: processing {} memories",
        memories_to_process.len()
    );

    // 3. Load all existing embeddings
    let mut all_embeddings = load_all_embeddings(db)?;

    // 3b. Batch-generate missing embeddings upfront (chunks of 64)
    {
        let embedded_ids: HashSet<i64> = all_embeddings.iter().map(|(id, _)| *id).collect();
        let missing: Vec<&MemoryMeta> = memories_to_process
            .iter()
            .filter(|m| !embedded_ids.contains(&m.id))
            .copied()
            .collect();

        if !missing.is_empty() {
            log::info!(
                "[Association] Batch-generating {} missing embeddings",
                missing.len()
            );
            for chunk in missing.chunks(64) {
                let texts: Vec<String> = chunk.iter().map(|m| m.content.clone()).collect();
                match embedding_generator.generate_batch(&texts).await {
                    Ok(embeddings) => {
                        for (meta, embedding) in chunk.iter().zip(embeddings.iter()) {
                            if let Err(e) = store_embedding(db, meta.id, embedding) {
                                log::warn!(
                                    "Failed to store embedding for memory {}: {}",
                                    meta.id, e
                                );
                                continue;
                            }
                            all_embeddings.push((meta.id, embedding.clone()));
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "[Association] Batch embedding generation failed: {}",
                            e
                        );
                        break;
                    }
                }
            }
        }
    }

    let mut total_created: usize = 0;
    let mut type_counts: HashMap<&str, usize> = HashMap::new();

    for source_meta in &memories_to_process {
        // 4. Look up this memory's embedding (should exist after batch generation)
        let embedding = match find_embedding(&all_embeddings, source_meta.id) {
            Some(emb) => emb.clone(),
            None => {
                log::debug!(
                    "No embedding available for memory {}, skipping",
                    source_meta.id
                );
                continue;
            }
        };

        // 5. Find similar memories via vector search
        let similar = vector_search::find_similar(
            &embedding,
            &all_embeddings,
            config.max_associations_per_memory,
            config.similarity_threshold,
        );

        // 6. Count existing associations for this memory
        let existing_count = count_associations(db, source_meta.id)?;
        let slots_available = config
            .max_associations_per_memory
            .saturating_sub(existing_count);

        if slots_available == 0 {
            continue;
        }

        // 7. Create associations for the most similar memories
        let mut created_in_batch = 0;

        for result in similar.iter().take(slots_available) {
            // Skip self-associations
            if result.memory_id == source_meta.id {
                continue;
            }

            // Check if association already exists
            if association_exists(db, source_meta.id, result.memory_id)? {
                continue;
            }

            // Classify the association type using memory metadata
            let assoc_type = match meta_map.get(&result.memory_id) {
                Some(target_meta) => classify_association_type(source_meta, target_meta),
                None => "related",
            };

            // Create the association
            if let Err(e) = create_association(
                db,
                source_meta.id,
                result.memory_id,
                assoc_type,
                result.similarity,
            ) {
                log::warn!(
                    "Failed to create association {} -> {}: {}",
                    source_meta.id,
                    result.memory_id,
                    e
                );
                continue;
            }

            *type_counts.entry(assoc_type).or_insert(0) += 1;
            created_in_batch += 1;
            total_created += 1;
        }

        if created_in_batch > 0 {
            log::info!(
                "Created {} associations for memory {}",
                created_in_batch,
                source_meta.id
            );
        }
    }

    // 8. Create supersedes associations from superseded_by column
    let supersedes_created = create_supersedes_from_column(db)?;
    if supersedes_created > 0 {
        *type_counts.entry("supersedes").or_insert(0) += supersedes_created;
        total_created += supersedes_created;
    }

    // 9. Create metadata-based associations (entity_name, category, log_date)
    //    These work without embeddings and provide edges even when the embedding server is unavailable.
    match create_metadata_based_associations(db, &all_metas, config.max_associations_per_memory) {
        Ok(meta_counts) => {
            for (assoc_type, count) in &meta_counts {
                *type_counts.entry(assoc_type).or_insert(0) += count;
                total_created += count;
            }
        }
        Err(e) => log::warn!("Metadata-based association creation failed: {}", e),
    }

    // Log type breakdown
    if !type_counts.is_empty() {
        let breakdown: Vec<String> = type_counts
            .iter()
            .map(|(t, c)| format!("{}={}", t, c))
            .collect();
        log::info!(
            "Association loop pass complete: created {} new associations ({})",
            total_created,
            breakdown.join(", ")
        );
    } else {
        log::info!(
            "Association loop pass complete: created {} new associations",
            total_created
        );
    }

    Ok(())
}

/// Reclassify existing "related" associations using memory metadata heuristics.
/// Called during explicit rebuild to fix previously unclassified associations.
pub fn reclassify_existing_associations(db: &Database) -> Result<usize, String> {
    let all_metas = load_all_memory_metas(db)?;
    let meta_map: HashMap<i64, &MemoryMeta> = all_metas.iter().map(|m| (m.id, m)).collect();

    // Load all associations that are currently "related"
    let conn = db.conn();
    let mut stmt = conn
        .prepare(
            "SELECT id, source_memory_id, target_memory_id FROM memory_associations WHERE association_type = 'related'",
        )
        .map_err(|e| format!("Failed to query associations for reclassification: {}", e))?;

    let associations: Vec<(i64, i64, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| format!("Failed to read associations: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let mut reclassified: usize = 0;
    let mut type_counts: HashMap<&str, usize> = HashMap::new();

    for (assoc_id, source_id, target_id) in &associations {
        let (source_meta, target_meta) = match (meta_map.get(source_id), meta_map.get(target_id)) {
            (Some(s), Some(t)) => (s, t),
            _ => continue,
        };

        let new_type = classify_association_type(source_meta, target_meta);
        if new_type == "related" {
            continue; // No change needed
        }

        // Update the association type
        if let Err(e) = conn.execute(
            "UPDATE memory_associations SET association_type = ?1 WHERE id = ?2",
            rusqlite::params![new_type, assoc_id],
        ) {
            log::warn!("Failed to reclassify association {}: {}", assoc_id, e);
            continue;
        }

        *type_counts.entry(new_type).or_insert(0) += 1;
        reclassified += 1;
    }

    if reclassified > 0 {
        let breakdown: Vec<String> = type_counts
            .iter()
            .map(|(t, c)| format!("{}={}", t, c))
            .collect();
        log::info!(
            "Reclassified {} associations from 'related' ({})",
            reclassified,
            breakdown.join(", ")
        );
    }

    Ok(reclassified)
}

/// Create supersedes associations from the memories.superseded_by column.
fn create_supersedes_from_column(db: &Database) -> Result<usize, String> {
    let conn = db.conn();

    let mut stmt = conn
        .prepare(
            "SELECT id, superseded_by FROM memories WHERE superseded_by IS NOT NULL",
        )
        .map_err(|e| format!("Failed to query superseded_by: {}", e))?;

    let pairs: Vec<(i64, i64)> = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("Failed to read superseded_by pairs: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let mut created = 0;
    for (old_id, new_id) in &pairs {
        if association_exists(db, *old_id, *new_id)? {
            continue;
        }
        if let Err(e) = create_association(db, *old_id, *new_id, "supersedes", 0.9) {
            log::warn!(
                "Failed to create supersedes association {} -> {}: {}",
                old_id,
                new_id,
                e
            );
            continue;
        }
        created += 1;
    }

    Ok(created)
}

/// Create associations from shared metadata (entity_name, category, log_date).
/// Works without embeddings — purely heuristic-based.
fn create_metadata_based_associations(
    db: &Database,
    metas: &[MemoryMeta],
    max_per_memory: usize,
) -> Result<HashMap<&'static str, usize>, String> {
    let mut type_counts: HashMap<&str, usize> = HashMap::new();

    // Build indexes: group memory IDs by entity_name, category, log_date, agent_subtype
    let mut by_entity: HashMap<String, Vec<i64>> = HashMap::new();
    let mut by_category: HashMap<String, Vec<i64>> = HashMap::new();
    let mut by_date: HashMap<String, Vec<i64>> = HashMap::new();
    let mut by_subtype: HashMap<String, Vec<i64>> = HashMap::new();

    for m in metas {
        if let Some(ref entity) = m.entity_name {
            let key = entity.to_lowercase();
            if !key.is_empty() {
                by_entity.entry(key).or_default().push(m.id);
            }
        }
        if let Some(ref cat) = m.category {
            let key = cat.to_lowercase();
            if !key.is_empty() {
                by_category.entry(key).or_default().push(m.id);
            }
        }
        if let Some(ref date) = m.log_date {
            if !date.is_empty() {
                by_date.entry(date.clone()).or_default().push(m.id);
            }
        }
        if let Some(ref subtype) = m.agent_subtype {
            let key = subtype.to_lowercase();
            if !key.is_empty() {
                by_subtype.entry(key).or_default().push(m.id);
            }
        }
    }

    // Helper: create edges for pairs within a group, respecting max_per_memory
    let mut assoc_counts: HashMap<i64, usize> = HashMap::new();

    let mut try_create = |id_a: i64, id_b: i64, assoc_type: &'static str, strength: f32| -> Result<(), String> {
        let count_a = assoc_counts.get(&id_a).copied().unwrap_or(0);
        let count_b = assoc_counts.get(&id_b).copied().unwrap_or(0);
        if count_a >= max_per_memory || count_b >= max_per_memory {
            return Ok(());
        }
        if association_exists(db, id_a, id_b)? {
            return Ok(());
        }
        create_association(db, id_a, id_b, assoc_type, strength)?;
        *assoc_counts.entry(id_a).or_insert(0) += 1;
        *assoc_counts.entry(id_b).or_insert(0) += 1;
        *type_counts.entry(assoc_type).or_insert(0) += 1;
        Ok(())
    };

    // Same entity_name → "references" (high priority, strength 0.8)
    for ids in by_entity.values() {
        if ids.len() < 2 || ids.len() > 100 { continue; } // skip huge groups
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let _ = try_create(ids[i], ids[j], "references", 0.8);
            }
        }
    }

    // Same category → "part_of" (medium priority, strength 0.5)
    // Only connect recent pairs to avoid O(n^2) explosion
    for ids in by_category.values() {
        if ids.len() < 2 { continue; }
        let window = ids.len().min(20); // last 20 per category
        let start = ids.len().saturating_sub(window);
        for i in start..ids.len() {
            for j in (i + 1)..ids.len() {
                let _ = try_create(ids[i], ids[j], "part_of", 0.5);
            }
        }
    }

    // Same log_date → "temporal" (lower priority, strength 0.4)
    // Only connect within the same day, limit pairs
    for ids in by_date.values() {
        if ids.len() < 2 || ids.len() > 50 { continue; }
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                let _ = try_create(ids[i], ids[j], "temporal", 0.4);
            }
        }
    }

    // Same agent_subtype → "agent_subtype" (localization signal, strength 0.3)
    // Only link recent memories within the same subtype (window of 30 to avoid O(n^2))
    for ids in by_subtype.values() {
        if ids.len() < 2 { continue; }
        let window = ids.len().min(30);
        let start = ids.len().saturating_sub(window);
        for i in start..ids.len() {
            for j in (i + 1)..ids.len() {
                let _ = try_create(ids[i], ids[j], "agent_subtype", 0.3);
            }
        }
    }

    Ok(type_counts)
}

/// Load metadata for all memories (used for association type classification).
fn load_all_memory_metas(db: &Database) -> Result<Vec<MemoryMeta>, String> {
    let conn = db.conn();

    let mut stmt = conn
        .prepare(
            "SELECT id, content, memory_type, category, entity_type, entity_name, log_date, superseded_by, agent_subtype
             FROM memories",
        )
        .map_err(|e| format!("Failed to prepare memory metadata query: {}", e))?;

    let metas: Vec<MemoryMeta> = stmt
        .query_map([], |row| {
            Ok(MemoryMeta {
                id: row.get(0)?,
                content: row.get(1)?,
                memory_type: row.get::<_, String>(2).unwrap_or_else(|_| "unknown".to_string()),
                category: row.get::<_, Option<String>>(3).unwrap_or(None),
                entity_type: row.get::<_, Option<String>>(4).unwrap_or(None),
                entity_name: row.get::<_, Option<String>>(5).unwrap_or(None),
                log_date: row.get::<_, Option<String>>(6).unwrap_or(None),
                superseded_by: row.get::<_, Option<i64>>(7).unwrap_or(None),
                agent_subtype: row.get::<_, Option<String>>(8).unwrap_or(None),
            })
        })
        .map_err(|e| format!("Failed to query memory metadata: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(metas)
}

/// Load all memory embeddings from the database.
fn load_all_embeddings(db: &Database) -> Result<Vec<(i64, Vec<f32>)>, String> {
    db.list_memory_embeddings()
        .map_err(|e| format!("Failed to load memory embeddings: {}", e))
}

/// Find an embedding for a specific memory ID in the preloaded list.
fn find_embedding(embeddings: &[(i64, Vec<f32>)], memory_id: i64) -> Option<&Vec<f32>> {
    embeddings
        .iter()
        .find(|(id, _)| *id == memory_id)
        .map(|(_, emb)| emb)
}

/// Store an embedding in the database.
fn store_embedding(db: &Database, memory_id: i64, embedding: &[f32]) -> Result<(), String> {
    let dims = embedding.len() as i32;
    db.upsert_memory_embedding(memory_id, embedding, "association_loop", dims)
        .map_err(|e| format!("Failed to insert embedding: {}", e))
}

/// Count existing associations for a memory (both directions).
fn count_associations(db: &Database, memory_id: i64) -> Result<usize, String> {
    let conn = db.conn();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_associations
             WHERE source_memory_id = ?1 OR target_memory_id = ?1",
            rusqlite::params![memory_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count associations: {}", e))?;

    Ok(count as usize)
}

/// Check if an association already exists between two memories (in either direction).
fn association_exists(
    db: &Database,
    source_id: i64,
    target_id: i64,
) -> Result<bool, String> {
    let conn = db.conn();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_associations
             WHERE (source_memory_id = ?1 AND target_memory_id = ?2)
                OR (source_memory_id = ?2 AND target_memory_id = ?1)",
            rusqlite::params![source_id, target_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check association existence: {}", e))?;

    Ok(count > 0)
}

/// Create a new association between two memories.
fn create_association(
    db: &Database,
    source_id: i64,
    target_id: i64,
    association_type: &str,
    strength: f32,
) -> Result<(), String> {
    let conn = db.conn();

    conn.execute(
        "INSERT INTO memory_associations (source_memory_id, target_memory_id, association_type, strength, created_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        rusqlite::params![source_id, target_id, association_type, strength],
    )
    .map_err(|e| format!("Failed to insert association: {}", e))?;

    Ok(())
}
