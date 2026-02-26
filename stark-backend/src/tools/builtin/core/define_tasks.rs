//! Define tasks tool - allows the agent (or a skill) to define/replace the task queue
//!
//! This tool replaces the entire task queue with a new set of tasks.
//! The dispatcher intercepts the metadata and replaces the orchestrator's queue.

use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::tools::ToolSafetyLevel;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Tool for defining/replacing the task queue
pub struct DefineTasksTool {
    definition: ToolDefinition,
}

impl DefineTasksTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "tasks".to_string(),
            PropertySchema {
                schema_type: "array".to_string(),
                description: "List of task descriptions to execute in order. Each should be specific and actionable.".to_string(),
                default: None,
                items: Some(Box::new(PropertySchema {
                    schema_type: "string".to_string(),
                    description: "A specific, actionable task description".to_string(),
                    default: None,
                    items: None,
                    enum_values: None,
                })),
                enum_values: None,
            },
        );

        DefineTasksTool {
            definition: ToolDefinition {
                name: "define_tasks".to_string(),
                description: "Define the list of tasks to accomplish a goal. Replaces any existing task queue. Each task should be specific and actionable. Tasks will be executed in order.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["tasks".to_string()],
                },
                group: ToolGroup::System,
                hidden: true,
            },
        }
    }
}

impl Default for DefineTasksTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DefineTasksTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> ToolResult {
        let tasks = match params.get("tasks").and_then(|v| v.as_array()) {
            Some(arr) => arr.clone(),
            None => return ToolResult::error("Missing or invalid 'tasks' parameter. Must be an array of strings."),
        };

        let task_descriptions: Vec<String> = tasks
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if task_descriptions.is_empty() {
            return ToolResult::error("No valid tasks provided. Each task must be a non-empty string.");
        }

        let count = task_descriptions.len();

        // Return minimal response — hide task details from AI context to prevent
        // it from seeing future tasks and doing all work in one shot.
        // The full task list is still in metadata for the dispatcher to consume.
        ToolResult::success(format!(
            "Tasks planned ({}). Starting task 1 now. Focus on the CURRENT TASK shown in your instructions.",
            count
        ))
        .with_metadata(json!({
            "define_tasks": true,
            "tasks": task_descriptions
        }))
    }

    fn safety_level(&self) -> ToolSafetyLevel {
        ToolSafetyLevel::SafeMode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_tasks_definition() {
        let tool = DefineTasksTool::new();
        let def = tool.definition();

        assert_eq!(def.name, "define_tasks");
        assert_eq!(def.group, ToolGroup::System);
        assert!(def.input_schema.required.contains(&"tasks".to_string()));
    }

    #[tokio::test]
    async fn test_define_tasks_success() {
        let tool = DefineTasksTool::new();
        let context = ToolContext::default();
        let result = tool
            .execute(
                json!({"tasks": ["Look up tokens", "Approve", "Get quote", "Swap"]}),
                &context,
            )
            .await;

        assert!(result.success);
        // Response should be minimal — no task details visible to AI
        assert!(result.content.contains("Tasks planned (4)"));
        assert!(!result.content.contains("Look up tokens"), "Task details should NOT be in response");
        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["define_tasks"], true);
        let tasks = metadata["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[0], "Look up tokens");
    }

    #[tokio::test]
    async fn test_define_tasks_empty() {
        let tool = DefineTasksTool::new();
        let context = ToolContext::default();
        let result = tool.execute(json!({"tasks": []}), &context).await;

        assert!(!result.success);
        assert!(result.content.contains("No valid tasks"));
    }

    #[tokio::test]
    async fn test_define_tasks_missing_param() {
        let tool = DefineTasksTool::new();
        let context = ToolContext::default();
        let result = tool.execute(json!({}), &context).await;

        assert!(!result.success);
        assert!(result.content.contains("Missing"));
    }
}
