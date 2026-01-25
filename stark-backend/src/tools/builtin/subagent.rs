use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Counter for generating unique subagent IDs
static SUBAGENT_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Status of a running subagent
#[derive(Debug, Clone)]
pub struct SubagentStatus {
    pub id: String,
    pub label: String,
    pub task: String,
    pub status: String,  // "running", "completed", "failed"
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Global registry of running subagents
lazy_static::lazy_static! {
    static ref SUBAGENT_REGISTRY: Arc<RwLock<HashMap<String, SubagentStatus>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

/// Tool for spawning background agent instances (subagents)
pub struct SubagentTool {
    definition: ToolDefinition,
}

impl SubagentTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "task".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The task or prompt for the subagent to work on. Be specific and detailed.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "label".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "A short label to identify this subagent (e.g., 'research', 'code-review'). Used for tracking and referencing.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "model".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Optional model override (e.g., 'claude-3-5-sonnet', 'gpt-4'). Uses default model if not specified.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "thinking".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Optional thinking level for Claude models (off, minimal, low, medium, high, xhigh).".to_string(),
                default: None,
                items: None,
                enum_values: Some(vec![
                    "off".to_string(),
                    "minimal".to_string(),
                    "low".to_string(),
                    "medium".to_string(),
                    "high".to_string(),
                    "xhigh".to_string(),
                ]),
            },
        );

        properties.insert(
            "timeout".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "Timeout in seconds for the subagent task (default: 300, max: 3600).".to_string(),
                default: Some(json!(300)),
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "wait".to_string(),
            PropertySchema {
                schema_type: "boolean".to_string(),
                description: "If true, wait for the subagent to complete before returning. If false (default), run in background.".to_string(),
                default: Some(json!(false)),
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "context".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Optional additional context or data to pass to the subagent.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        SubagentTool {
            definition: ToolDefinition {
                name: "subagent".to_string(),
                description: "Spawn a background agent instance to work on a task autonomously. Useful for parallel task execution, long-running operations, or delegating subtasks. The subagent runs independently and can use tools.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["task".to_string()],
                },
                group: ToolGroup::System,
            },
        }
    }

    /// Get status of a subagent by ID
    pub async fn get_status(id: &str) -> Option<SubagentStatus> {
        SUBAGENT_REGISTRY.read().await.get(id).cloned()
    }

    /// List all subagents
    pub async fn list_all() -> Vec<SubagentStatus> {
        SUBAGENT_REGISTRY.read().await.values().cloned().collect()
    }

    /// Update subagent status
    async fn update_status(id: &str, status: &str, result: Option<String>, error: Option<String>) {
        let mut registry = SUBAGENT_REGISTRY.write().await;
        if let Some(entry) = registry.get_mut(id) {
            entry.status = status.to_string();
            entry.result = result;
            entry.error = error;
            if status == "completed" || status == "failed" {
                entry.completed_at = Some(chrono::Utc::now());
            }
        }
    }
}

impl Default for SubagentTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct SubagentParams {
    task: String,
    label: Option<String>,
    model: Option<String>,
    thinking: Option<String>,
    timeout: Option<u64>,
    wait: Option<bool>,
    context: Option<String>,
}

#[async_trait]
impl Tool for SubagentTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: SubagentParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Generate unique subagent ID
        let counter = SUBAGENT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let label = params.label.clone().unwrap_or_else(|| format!("task-{}", counter));
        let subagent_id = format!("subagent-{}-{}", label, counter);

        let timeout_secs = params.timeout.unwrap_or(300).min(3600);
        let wait = params.wait.unwrap_or(false);

        log::info!(
            "Spawning subagent '{}' with task: {}",
            subagent_id,
            if params.task.len() > 100 { &params.task[..100] } else { &params.task }
        );

        // Build the full task prompt
        let mut full_task = params.task.clone();
        if let Some(ref ctx) = params.context {
            full_task = format!("{}\n\n## Additional Context:\n{}", full_task, ctx);
        }

        // Register the subagent
        {
            let mut registry = SUBAGENT_REGISTRY.write().await;
            registry.insert(subagent_id.clone(), SubagentStatus {
                id: subagent_id.clone(),
                label: label.clone(),
                task: params.task.clone(),
                status: "running".to_string(),
                started_at: chrono::Utc::now(),
                completed_at: None,
                result: None,
                error: None,
            });
        }

        // Clone values for the async task
        let subagent_id_clone = subagent_id.clone();
        let model_override = params.model.clone();
        let thinking_level = params.thinking.clone();
        let channel_id = context.channel_id;
        let channel_type = context.channel_type.clone();

        // Spawn the subagent task
        let task_handle = tokio::spawn(async move {
            // In a full implementation, this would:
            // 1. Create an isolated session
            // 2. Get the AI client with model override
            // 3. Run the task through the dispatcher
            // 4. Collect and store results
            //
            // For now, we simulate the execution flow

            log::info!("Subagent '{}' starting execution", subagent_id_clone);

            // Simulate some work (in real implementation, this calls the AI)
            let start = std::time::Instant::now();

            // This is where the actual AI execution would happen
            // For now, we'll provide a placeholder that shows the structure
            let result = format!(
                "Subagent '{}' processed task: {}\n\
                 Model: {}\n\
                 Thinking: {}\n\
                 Channel: {:?} ({:?})\n\
                 \n\
                 [Note: Full subagent execution requires integration with the AI client. \
                 The subagent would process the task using tools and return results here.]",
                subagent_id_clone,
                if full_task.len() > 200 { &full_task[..200] } else { &full_task },
                model_override.as_deref().unwrap_or("default"),
                thinking_level.as_deref().unwrap_or("default"),
                channel_id,
                channel_type
            );

            let duration = start.elapsed();
            log::info!(
                "Subagent '{}' completed in {:?}",
                subagent_id_clone,
                duration
            );

            // Update status to completed
            Self::update_status(&subagent_id_clone, "completed", Some(result.clone()), None).await;

            result
        });

        if wait {
            // Wait for completion with timeout
            match tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                task_handle,
            ).await {
                Ok(Ok(result)) => {
                    ToolResult::success(result).with_metadata(json!({
                        "subagent_id": subagent_id,
                        "label": label,
                        "status": "completed",
                        "waited": true
                    }))
                }
                Ok(Err(e)) => {
                    Self::update_status(&subagent_id, "failed", None, Some(e.to_string())).await;
                    ToolResult::error(format!("Subagent task failed: {}", e))
                }
                Err(_) => {
                    Self::update_status(
                        &subagent_id,
                        "failed",
                        None,
                        Some(format!("Timeout after {}s", timeout_secs)),
                    ).await;
                    ToolResult::error(format!(
                        "Subagent '{}' timed out after {} seconds",
                        subagent_id, timeout_secs
                    ))
                }
            }
        } else {
            // Return immediately, task runs in background
            ToolResult::success(format!(
                "Subagent '{}' spawned successfully and running in background.\n\
                 Label: {}\n\
                 Task: {}\n\
                 Timeout: {}s\n\
                 \n\
                 Use `subagent_status` with id '{}' to check progress.",
                subagent_id,
                label,
                if params.task.len() > 100 { format!("{}...", &params.task[..100]) } else { params.task.clone() },
                timeout_secs,
                subagent_id
            )).with_metadata(json!({
                "subagent_id": subagent_id,
                "label": label,
                "status": "running",
                "waited": false,
                "timeout": timeout_secs
            }))
        }
    }
}

/// Tool for checking subagent status
pub struct SubagentStatusTool {
    definition: ToolDefinition,
}

impl SubagentStatusTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "id".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The subagent ID to check status for. Omit to list all subagents.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        SubagentStatusTool {
            definition: ToolDefinition {
                name: "subagent_status".to_string(),
                description: "Check the status of a running or completed subagent, or list all subagents.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec![],
                },
                group: ToolGroup::System,
            },
        }
    }
}

impl Default for SubagentStatusTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct SubagentStatusParams {
    id: Option<String>,
}

#[async_trait]
impl Tool for SubagentStatusTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> ToolResult {
        let params: SubagentStatusParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        if let Some(id) = params.id {
            // Get specific subagent status
            match SubagentTool::get_status(&id).await {
                Some(status) => {
                    let mut result = format!(
                        "## Subagent: {}\n\
                         Label: {}\n\
                         Status: {}\n\
                         Started: {}\n",
                        status.id,
                        status.label,
                        status.status,
                        status.started_at.format("%Y-%m-%d %H:%M:%S UTC")
                    );

                    if let Some(completed) = status.completed_at {
                        result.push_str(&format!(
                            "Completed: {}\n",
                            completed.format("%Y-%m-%d %H:%M:%S UTC")
                        ));
                    }

                    result.push_str(&format!("\nTask: {}\n", status.task));

                    if let Some(ref res) = status.result {
                        result.push_str(&format!("\n## Result:\n{}\n", res));
                    }

                    if let Some(ref err) = status.error {
                        result.push_str(&format!("\n## Error:\n{}\n", err));
                    }

                    ToolResult::success(result).with_metadata(json!({
                        "id": status.id,
                        "status": status.status,
                        "label": status.label
                    }))
                }
                None => ToolResult::error(format!("Subagent '{}' not found", id)),
            }
        } else {
            // List all subagents
            let all = SubagentTool::list_all().await;

            if all.is_empty() {
                return ToolResult::success("No subagents found.");
            }

            let mut result = format!("## Subagents ({} total)\n\n", all.len());

            for status in &all {
                result.push_str(&format!(
                    "- **{}** ({}): {} - {}\n",
                    status.id,
                    status.label,
                    status.status,
                    if status.task.len() > 50 { format!("{}...", &status.task[..50]) } else { status.task.clone() }
                ));
            }

            ToolResult::success(result).with_metadata(json!({
                "count": all.len(),
                "subagents": all.iter().map(|s| json!({
                    "id": s.id,
                    "label": s.label,
                    "status": s.status
                })).collect::<Vec<_>>()
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subagent_definition() {
        let tool = SubagentTool::new();
        let def = tool.definition();

        assert_eq!(def.name, "subagent");
        assert_eq!(def.group, ToolGroup::System);
        assert!(def.input_schema.required.contains(&"task".to_string()));
    }

    #[test]
    fn test_subagent_status_definition() {
        let tool = SubagentStatusTool::new();
        let def = tool.definition();

        assert_eq!(def.name, "subagent_status");
        assert_eq!(def.group, ToolGroup::System);
        assert!(def.input_schema.required.is_empty());
    }

    #[tokio::test]
    async fn test_spawn_subagent() {
        let tool = SubagentTool::new();
        let context = ToolContext::new();

        let result = tool
            .execute(
                json!({
                    "task": "Test task",
                    "label": "test",
                    "wait": true
                }),
                &context,
            )
            .await;

        assert!(result.success);
        assert!(result.content.contains("subagent"));
    }
}
