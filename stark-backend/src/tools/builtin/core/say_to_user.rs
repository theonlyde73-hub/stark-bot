//! Say to user tool for agent communication
//!
//! This tool allows the agent to send messages to the user.
//! Required when tool_choice is set to "any" so the agent always has
//! a way to communicate without performing other actions.

use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Say to user tool for agent communication
pub struct SayToUserTool {
    definition: ToolDefinition,
}

impl SayToUserTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "message".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The message to send to the user.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        SayToUserTool {
            definition: ToolDefinition {
                name: "say_to_user".to_string(),
                description: "Send a message to the user. Use this tool when you want to communicate with the user without performing any other action.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["message".to_string()],
                },
                group: ToolGroup::System,
            },
        }
    }
}

impl Default for SayToUserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct SayToUserParams {
    message: String,
}

#[async_trait]
impl Tool for SayToUserTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> ToolResult {
        let params: SayToUserParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Simply return the message - the orchestrator will display it to the user
        ToolResult::success(params.message)
    }
}
