//! Memory Search Tool
//!
//! Full-text search across DB memories using FTS5 BM25 ranking,
//! with optional hybrid mode (FTS + vector + graph via RRF).
//! In safe mode, results are sandboxed to the safemode identity only.

use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::tools::ToolSafetyLevel;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Tool for searching memories using full-text search
pub struct MemorySearchTool {
    definition: ToolDefinition,
}

impl MemorySearchTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "query".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Search query - words to search for in memories. Multiple words are matched with OR logic.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "limit".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "Maximum number of results to return (default: 10, max: 50).".to_string(),
                default: Some(json!(10)),
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "mode".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Search mode: 'fts' for full-text search only (default), 'hybrid' for combined FTS + vector + graph search using RRF ranking.".to_string(),
                default: Some(json!("fts")),
                items: None,
                enum_values: Some(vec!["fts".to_string(), "hybrid".to_string()]),
            },
        );

        Self {
            definition: ToolDefinition {
                name: "memory_search".to_string(),
                description: "Search your memory for relevant information. Use PROACTIVELY when a user asks about past conversations, their preferences, previous decisions, or anything you might have learned before. Returns ranked results with snippets. Use mode='hybrid' for semantic matching when exact keywords are unknown.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["query".to_string()],
                },
                group: ToolGroup::Memory,
                hidden: false,
            },
        }
    }
}

impl Default for MemorySearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    query: String,
    limit: Option<i32>,
    #[serde(default = "default_mode")]
    mode: String,
}

fn default_mode() -> String {
    "fts".to_string()
}

/// Check if tool context indicates safe mode
fn is_safe_mode(context: &ToolContext) -> bool {
    context.extra.get("safe_mode")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Safe mode identity
const SAFE_MODE_IDENTITY: &str = "safemode";

#[async_trait]
impl Tool for MemorySearchTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: SearchParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate query
        if params.query.trim().is_empty() {
            return ToolResult::error("Query cannot be empty");
        }

        let db = match &context.database {
            Some(db) => db,
            None => {
                return ToolResult::error(
                    "Database not available. Memory search requires the database to be initialized.",
                );
            }
        };

        let safe_mode = is_safe_mode(context);
        let result_limit = params.limit.unwrap_or(10).min(50).max(1);

        // Identity filter: safe mode restricts to safemode identity only;
        // standard mode searches ALL memories (no identity filter).
        let identity_id: Option<&str> = if safe_mode {
            Some(SAFE_MODE_IDENTITY)
        } else {
            None
        };

        // Extract agent_subtype from tool context for search boost
        let agent_subtype: Option<String> = context.extra.get("agent_subtype")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Hybrid mode: use combined FTS + vector + graph search
        if params.mode == "hybrid" {
            if let Some(ref hybrid_engine) = context.hybrid_search {
                match hybrid_engine.search(&params.query, result_limit as usize, agent_subtype.as_deref()).await {
                    Ok(results) => {
                        // In safe mode, filter to safemode identity
                        // (hybrid engine doesn't have identity filtering, so we filter here)
                        let results: Vec<_> = if safe_mode {
                            results.into_iter()
                                .filter(|r| {
                                    // Check if the memory belongs to safemode identity
                                    if let Ok(Some(mem)) = db.get_memory(r.memory_id) {
                                        mem.identity_id.as_deref() == Some(SAFE_MODE_IDENTITY)
                                    } else {
                                        false
                                    }
                                })
                                .take(result_limit as usize)
                                .collect()
                        } else {
                            results
                        };

                        if results.is_empty() {
                            return ToolResult::success(format!(
                                "No memories found matching: \"{}\" (hybrid mode)",
                                params.query
                            ));
                        }

                        let mut output = format!(
                            "## Hybrid Memory Search Results\n**Query:** \"{}\"\n**Found:** {} result(s)\n**Mode:** hybrid (FTS5 + vector + graph RRF)\n\n",
                            params.query, results.len()
                        );

                        for (i, result) in results.iter().enumerate() {
                            output.push_str(&format!(
                                "### {}. Memory #{} ({})\n**RRF Score:** {:.4} | **Importance:** {} | **Type:** {}\n{}\n\n",
                                i + 1,
                                result.memory_id,
                                result.memory_type,
                                result.rrf_score,
                                result.importance,
                                result.memory_type,
                                if result.content.chars().count() > 300 {
                                    let truncated: String = result.content.chars().take(300).collect();
                                    format!("{}...", truncated)
                                } else {
                                    result.content.clone()
                                }
                            ));
                        }

                        return ToolResult::success(output).with_metadata(json!({
                            "query": params.query,
                            "mode": "hybrid",
                            "result_count": results.len(),
                            "memory_ids": results.iter().map(|r| r.memory_id).collect::<Vec<_>>()
                        }));
                    }
                    Err(e) => {
                        return ToolResult::error(format!("Hybrid search failed: {}. Try mode='fts' as fallback.", e));
                    }
                }
            } else {
                return ToolResult::error("Hybrid search engine not available. Use mode='fts' for full-text search.");
            }
        }

        // FTS mode (default): use DB full-text search + graph expansion
        match db.search_memories_fts(&params.query, identity_id, result_limit) {
            Ok(results) => {
                if results.is_empty() {
                    return ToolResult::success(format!(
                        "No memories found matching: \"{}\"",
                        params.query
                    ));
                }

                let mut all_memory_ids: Vec<i64> = results.iter().map(|(m, _)| m.id).collect();

                let mut output = format!(
                    "## Memory Search Results\n**Query:** \"{}\"\n**Found:** {} result(s)\n\n",
                    params.query,
                    results.len()
                );

                for (i, (mem, rank)) in results.iter().enumerate() {
                    let snippet: String = if mem.content.chars().count() > 300 {
                        let truncated: String = mem.content.chars().take(300).collect();
                        format!("{}...", truncated)
                    } else {
                        mem.content.clone()
                    };
                    output.push_str(&format!(
                        "### {}. Memory #{} ({})\n**Score:** {:.2} | **Importance:** {} | **Type:** {}\n{}\n\n",
                        i + 1,
                        mem.id,
                        mem.memory_type,
                        -rank, // Negate because BM25 returns negative scores
                        mem.importance,
                        mem.memory_type,
                        snippet,
                    ));
                }

                // Graph expansion: surface memories connected to FTS hits via edges
                let seed_ids: Vec<i64> = results.iter().map(|(m, _)| m.id).collect();
                let graph_limit = (result_limit / 2).max(3).min(10);
                if let Ok(neighbors) = db.graph_expand_from_seeds(&seed_ids, graph_limit) {
                    if !neighbors.is_empty() {
                        // Fetch neighbor memory details
                        let mut graph_entries = Vec::new();
                        for (neighbor_id, strength) in &neighbors {
                            if let Ok(Some(mem)) = db.get_memory(*neighbor_id) {
                                // In safe mode, only show safemode-identity memories
                                if safe_mode && mem.identity_id.as_deref() != Some(SAFE_MODE_IDENTITY) {
                                    continue;
                                }
                                graph_entries.push((mem, *strength));
                            }
                        }

                        if !graph_entries.is_empty() {
                            output.push_str(&format!(
                                "---\n### Related via Graph ({} connected)\n\n",
                                graph_entries.len()
                            ));
                            for (mem, strength) in &graph_entries {
                                let snippet: String = if mem.content.chars().count() > 200 {
                                    let truncated: String = mem.content.chars().take(200).collect();
                                    format!("{}...", truncated)
                                } else {
                                    mem.content.clone()
                                };
                                output.push_str(&format!(
                                    "- **Memory #{}** ({}) | strength: {} | importance: {}\n  {}\n\n",
                                    mem.id,
                                    mem.memory_type,
                                    strength,
                                    mem.importance,
                                    snippet,
                                ));
                                all_memory_ids.push(mem.id);
                            }
                        }
                    }
                }

                ToolResult::success(output).with_metadata(json!({
                    "query": params.query,
                    "mode": "fts",
                    "result_count": all_memory_ids.len(),
                    "memory_ids": all_memory_ids
                }))
            }
            Err(e) => ToolResult::error(format!("Search failed: {}", e)),
        }
    }

    fn safety_level(&self) -> ToolSafetyLevel {
        ToolSafetyLevel::SafeMode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_search_definition() {
        let tool = MemorySearchTool::new();
        let def = tool.definition();

        assert_eq!(def.name, "memory_search");
        assert_eq!(def.group, ToolGroup::Memory);
        assert!(def.input_schema.required.contains(&"query".to_string()));
    }
}
