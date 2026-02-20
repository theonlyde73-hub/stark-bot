//! Memory Merge Tool
//!
//! Merge two memories into one, combining their content and associations.
//! The originals are marked as superseded by the new merged memory.

use crate::db::tables::memories::MergeStrategy;
use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::tools::ToolSafetyLevel;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Tool for merging duplicate or related memories.
pub struct MemoryMergeTool {
    definition: ToolDefinition,
}

impl MemoryMergeTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "memory_id_a".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "ID of the first memory to merge.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "memory_id_b".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "ID of the second memory to merge.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "strategy".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Merge strategy: \"append\" joins both contents, \"replace_with_newer\" keeps only the newer content, \"custom\" uses the provided custom_content.".to_string(),
                default: Some(json!("append")),
                items: None,
                enum_values: Some(vec![
                    "append".to_string(),
                    "replace_with_newer".to_string(),
                    "custom".to_string(),
                ]),
            },
        );

        properties.insert(
            "custom_content".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Custom merged content (required when strategy is \"custom\").".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        Self {
            definition: ToolDefinition {
                name: "memory_merge".to_string(),
                description: "Merge two related or duplicate memories into one. The original memories are preserved but marked as superseded. Associations from both are transferred to the new merged memory. Use after finding duplicate or overlapping memories.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec![
                        "memory_id_a".to_string(),
                        "memory_id_b".to_string(),
                    ],
                },
                group: ToolGroup::Memory,
                hidden: false,
            },
        }
    }
}

impl Default for MemoryMergeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct MergeParams {
    memory_id_a: i64,
    memory_id_b: i64,
    strategy: Option<String>,
    custom_content: Option<String>,
}

#[async_trait]
impl Tool for MemoryMergeTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: MergeParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let db = match &context.database {
            Some(db) => db,
            None => {
                return ToolResult::error(
                    "Database not available. Memory merge requires the database to be initialized.",
                );
            }
        };

        if params.memory_id_a == params.memory_id_b {
            return ToolResult::error("Cannot merge a memory with itself.");
        }

        let strategy_str = params.strategy.as_deref().unwrap_or("append");
        let strategy = match strategy_str {
            "append" => MergeStrategy::Append,
            "replace_with_newer" => MergeStrategy::ReplaceWithNewer,
            "custom" => {
                let content = match params.custom_content {
                    Some(c) if !c.is_empty() => c,
                    _ => return ToolResult::error("custom_content is required when strategy is \"custom\"."),
                };
                MergeStrategy::Custom(content)
            }
            _ => return ToolResult::error(format!(
                "Unknown strategy: \"{}\". Use \"append\", \"replace_with_newer\", or \"custom\".",
                strategy_str
            )),
        };

        match db.merge_memories(params.memory_id_a, params.memory_id_b, &strategy) {
            Ok(new_id) => {
                let output = format!(
                    "## Memories Merged\n\n\
                    **New Memory ID:** {}\n\
                    **Superseded:** {} and {}\n\
                    **Strategy:** {}",
                    new_id, params.memory_id_a, params.memory_id_b, strategy_str
                );
                ToolResult::success(output).with_metadata(json!({
                    "new_memory_id": new_id,
                    "superseded_ids": [params.memory_id_a, params.memory_id_b],
                    "strategy": strategy_str
                }))
            }
            Err(e) => ToolResult::error(format!("Failed to merge memories: {}", e)),
        }
    }

    fn safety_level(&self) -> ToolSafetyLevel {
        ToolSafetyLevel::Standard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_merge_definition() {
        let tool = MemoryMergeTool::new();
        let def = tool.definition();

        assert_eq!(def.name, "memory_merge");
        assert_eq!(def.group, ToolGroup::Memory);
        assert!(def.input_schema.required.contains(&"memory_id_a".to_string()));
        assert!(def.input_schema.required.contains(&"memory_id_b".to_string()));
    }
}
