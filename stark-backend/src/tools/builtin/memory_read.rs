//! Memory Read Tool
//!
//! Read memories from the DB-backed memory system.
//! In safe mode, access is sandboxed to the safemode identity only.

use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::tools::ToolSafetyLevel;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Tool for reading memories from the DB
pub struct MemoryReadTool {
    definition: ToolDefinition,
}

impl MemoryReadTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "date".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Read daily log for this date (YYYY-MM-DD format). Shortcut for reading a specific day's log.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "type".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Type of memory to read: \"daily\" for today's log, \"long_term\" for persistent facts.".to_string(),
                default: None,
                items: None,
                enum_values: Some(vec!["daily".to_string(), "long_term".to_string()]),
            },
        );

        properties.insert(
            "list".to_string(),
            PropertySchema {
                schema_type: "boolean".to_string(),
                description: "If true, list available memory dates and stats instead of reading content.".to_string(),
                default: Some(json!(false)),
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "id".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "Read a specific memory by its ID.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        Self {
            definition: ToolDefinition {
                name: "memory_read".to_string(),
                description: "Read memories from the database. Use `type: \"long_term\"` to review persistent facts and preferences, `type: \"daily\"` for today's activity log, `list: true` to see available dates and stats, `date: \"YYYY-MM-DD\"` to read a past day's log, or `id: N` to read a specific memory.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec![],
                },
                group: ToolGroup::Memory,
                hidden: false,
            },
        }
    }
}

impl Default for MemoryReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct ReadParams {
    date: Option<String>,
    #[serde(rename = "type")]
    memory_type: Option<String>,
    list: Option<bool>,
    id: Option<i64>,
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
impl Tool for MemoryReadTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: ReadParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let db = match &context.database {
            Some(db) => db,
            None => {
                return ToolResult::error(
                    "Database not available. Memory read requires the database to be initialized.",
                );
            }
        };

        let safe_mode = is_safe_mode(context);

        // In safe mode, override identity to "safemode" so all reads are sandboxed
        let identity_id: Option<&str> = if safe_mode {
            Some(SAFE_MODE_IDENTITY)
        } else {
            context.identity_id.as_deref()
        };

        // Handle reading a specific memory by ID
        if let Some(memory_id) = params.id {
            return match db.get_memory(memory_id) {
                Ok(Some(mem)) => {
                    // In safe mode, block access to non-safemode memories
                    if safe_mode && mem.identity_id.as_deref() != Some(SAFE_MODE_IDENTITY) {
                        return ToolResult::error("Access denied: safe mode only allows reading safemode memories.");
                    }
                    let output = format!(
                        "## Memory #{} ({})\n**Created:** {} | **Importance:** {}\n\n{}",
                        mem.id, mem.memory_type, mem.created_at, mem.importance, mem.content
                    );
                    ToolResult::success(output).with_metadata(json!({
                        "id": mem.id,
                        "memory_type": mem.memory_type,
                        "importance": mem.importance
                    }))
                }
                Ok(None) => ToolResult::success(format!("No memory found with ID {}.", memory_id)),
                Err(e) => ToolResult::error(format!("Failed to read memory: {}", e)),
            };
        }

        // Handle list request
        if params.list.unwrap_or(false) {
            return match db.get_memory_stats() {
                Ok(stats) => {
                    let mut output = format!(
                        "## Memory Stats\n**Total memories:** {}\n**Daily logs:** {} | **Long-term:** {}\n**Identities:** {} ({})\n",
                        stats.total_memories, stats.daily_log_count, stats.long_term_count,
                        stats.identity_count,
                        stats.identities.join(", "),
                    );
                    if let (Some(earliest), Some(latest)) = (&stats.earliest_date, &stats.latest_date) {
                        output.push_str(&format!("**Date range:** {} to {}\n", earliest, latest));
                    }

                    // Show recent dates
                    if let Ok(dates) = db.list_memory_dates(identity_id) {
                        if !dates.is_empty() {
                            output.push_str("\n**Recent dates:**\n");
                            for date in dates.iter().take(20) {
                                output.push_str(&format!("- {}\n", date));
                            }
                        }
                    }

                    ToolResult::success(output).with_metadata(json!({
                        "total_memories": stats.total_memories,
                        "daily_log_count": stats.daily_log_count,
                        "long_term_count": stats.long_term_count,
                    }))
                }
                Err(e) => ToolResult::error(format!("Failed to get memory stats: {}", e)),
            };
        }

        // Handle date request
        if let Some(date_str) = params.date {
            if chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").is_err() {
                return ToolResult::error(format!("Invalid date format: \"{}\". Use YYYY-MM-DD.", date_str));
            }

            return match db.get_daily_log_memories(&date_str, identity_id, 50) {
                Ok(entries) => {
                    if entries.is_empty() {
                        return ToolResult::success(format!("No daily log found for {}.", date_str));
                    }

                    let mut output = format!("## Daily Log: {} ({} entries)\n\n", date_str, entries.len());
                    for entry in &entries {
                        output.push_str(&format!("### Memory #{} ({})\n{}\n\n", entry.id, entry.created_at, entry.content));
                    }

                    ToolResult::success(output).with_metadata(json!({
                        "date": date_str,
                        "entry_count": entries.len()
                    }))
                }
                Err(e) => ToolResult::error(format!("Failed to read daily log for {}: {}", date_str, e)),
            };
        }

        // Handle type request
        if let Some(memory_type) = params.memory_type {
            match memory_type.as_str() {
                "daily" => {
                    return match db.get_today_daily_log(identity_id, 50) {
                        Ok(entries) => {
                            if entries.is_empty() {
                                return ToolResult::success("No entries in today's daily log yet.");
                            }

                            let mut output = format!("## Today's Daily Log ({} entries)\n\n", entries.len());
                            for entry in &entries {
                                output.push_str(&format!("### Memory #{} ({})\n{}\n\n", entry.id, entry.created_at, entry.content));
                            }

                            ToolResult::success(output).with_metadata(json!({
                                "type": "daily",
                                "entry_count": entries.len()
                            }))
                        }
                        Err(e) => ToolResult::error(format!("Failed to read daily log: {}", e)),
                    };
                }
                "long_term" => {
                    return match db.get_long_term_memories(identity_id, 50) {
                        Ok(entries) => {
                            if entries.is_empty() {
                                return ToolResult::success("No long-term memories stored yet.");
                            }

                            let mut output = format!("## Long-Term Memory ({} entries)\n\n", entries.len());
                            for entry in &entries {
                                output.push_str(&format!(
                                    "### Memory #{} (importance: {}, {})\n{}\n\n",
                                    entry.id, entry.importance, entry.created_at, entry.content
                                ));
                            }

                            ToolResult::success(output).with_metadata(json!({
                                "type": "long_term",
                                "entry_count": entries.len()
                            }))
                        }
                        Err(e) => ToolResult::error(format!("Failed to read long-term memory: {}", e)),
                    };
                }
                _ => {
                    return ToolResult::error(format!("Unknown memory type: \"{}\". Use \"daily\" or \"long_term\".", memory_type));
                }
            }
        }

        // No specific request - show help
        ToolResult::success(
            "## Memory Read Usage\n\n\
            Provide one of the following parameters:\n\
            - `id`: Read a specific memory by ID\n\
            - `date`: YYYY-MM-DD to read that day's log\n\
            - `type`: \"daily\" or \"long_term\"\n\
            - `list`: true to see stats and available dates\n\n\
            Example: `{\"type\": \"long_term\"}` to read persistent memories."
        )
    }

    fn safety_level(&self) -> ToolSafetyLevel {
        ToolSafetyLevel::SafeMode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_read_definition() {
        let tool = MemoryReadTool::new();
        let def = tool.definition();

        assert_eq!(def.name, "memory_read");
        assert_eq!(def.group, ToolGroup::Memory);
        // No required params - all optional
        assert!(def.input_schema.required.is_empty());
    }
}
