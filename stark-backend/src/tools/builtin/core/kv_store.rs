use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Tool for reading and writing key/value pairs in a persistent Redis store.
/// Used by agents for tracking state across conversations (e.g., strike counters, flags).
pub struct KvStoreTool {
    definition: ToolDefinition,
}

impl KvStoreTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "action".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The action to perform: get, set, delete, increment, or list."
                    .to_string(),
                default: None,
                items: None,
                enum_values: Some(vec![
                    "get".to_string(),
                    "set".to_string(),
                    "delete".to_string(),
                    "increment".to_string(),
                    "list".to_string(),
                ]),
            },
        );

        properties.insert(
            "key".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The key name. Must be alphanumeric + underscores, max 128 chars. Auto-uppercased.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "value".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The value to set (required for 'set' action).".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "amount".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "Amount to increment by (default: 1). Can be negative for decrement."
                    .to_string(),
                default: Some(json!(1)),
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "prefix".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Key prefix to filter by when using 'list' action. Lists all keys if empty.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        KvStoreTool {
            definition: ToolDefinition {
                name: "kv_store".to_string(),
                description: "Persistent key/value store for tracking state across conversations. Supports get, set, delete, increment (atomic counter), and list (with prefix filter). Keys are auto-uppercased. Use for counters, flags, user-specific state, etc.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["action".to_string()],
                },
                group: ToolGroup::System,
                hidden: false,
            },
        }
    }
}

impl Default for KvStoreTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct KvStoreParams {
    action: String,
    key: Option<String>,
    value: Option<String>,
    amount: Option<i64>,
    prefix: Option<String>,
}

/// Validate and normalize a key name.
fn validate_key(key: &str) -> Result<String, String> {
    if key.is_empty() {
        return Err("key cannot be empty".to_string());
    }
    if key.len() > 128 {
        return Err("key must be at most 128 characters".to_string());
    }
    let valid = key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid {
        return Err(
            "key must contain only letters, digits, and underscores (A-Za-z0-9_)".to_string(),
        );
    }
    Ok(key.to_ascii_uppercase())
}

#[async_trait]
impl Tool for KvStoreTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        // Block in safe mode
        if context
            .extra
            .get("safe_mode")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return ToolResult::error("kv_store is not available in safe mode");
        }

        let params: KvStoreParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let kv = match &context.kv_store {
            Some(store) => store,
            None => {
                return ToolResult::error(
                    "KV store is not available. Redis may not be running.",
                )
            }
        };

        match params.action.as_str() {
            "get" => {
                let key = match params.key {
                    Some(k) => match validate_key(&k) {
                        Ok(k) => k,
                        Err(e) => return ToolResult::error(e),
                    },
                    None => return ToolResult::error("'key' is required for 'get' action"),
                };
                match kv.get(&key).await {
                    Ok(Some(val)) => ToolResult::success(
                        json!({"key": key, "value": val}).to_string(),
                    ),
                    Ok(None) => ToolResult::success(
                        json!({"key": key, "value": null, "message": "Key not found"}).to_string(),
                    ),
                    Err(e) => ToolResult::error(format!("Failed to get key: {}", e)),
                }
            }

            "set" => {
                let key = match params.key {
                    Some(k) => match validate_key(&k) {
                        Ok(k) => k,
                        Err(e) => return ToolResult::error(e),
                    },
                    None => return ToolResult::error("'key' is required for 'set' action"),
                };
                let value = match params.value {
                    Some(v) => v,
                    None => return ToolResult::error("'value' is required for 'set' action"),
                };
                match kv.set(&key, &value).await {
                    Ok(()) => ToolResult::success(
                        json!({"key": key, "value": value, "message": "Value set successfully"})
                            .to_string(),
                    ),
                    Err(e) => ToolResult::error(format!("Failed to set key: {}", e)),
                }
            }

            "delete" => {
                let key = match params.key {
                    Some(k) => match validate_key(&k) {
                        Ok(k) => k,
                        Err(e) => return ToolResult::error(e),
                    },
                    None => return ToolResult::error("'key' is required for 'delete' action"),
                };
                match kv.delete(&key).await {
                    Ok(existed) => ToolResult::success(
                        json!({"key": key, "deleted": existed}).to_string(),
                    ),
                    Err(e) => ToolResult::error(format!("Failed to delete key: {}", e)),
                }
            }

            "increment" => {
                let key = match params.key {
                    Some(k) => match validate_key(&k) {
                        Ok(k) => k,
                        Err(e) => return ToolResult::error(e),
                    },
                    None => {
                        return ToolResult::error("'key' is required for 'increment' action")
                    }
                };
                let amount = params.amount.unwrap_or(1);
                match kv.increment(&key, amount).await {
                    Ok(new_val) => ToolResult::success(
                        json!({"key": key, "new_value": new_val, "incremented_by": amount})
                            .to_string(),
                    ),
                    Err(e) => ToolResult::error(format!("Failed to increment key: {}", e)),
                }
            }

            "list" => {
                let prefix = params
                    .prefix
                    .or(params.key)
                    .unwrap_or_default()
                    .to_ascii_uppercase();
                match kv.list(&prefix).await {
                    Ok(entries) => {
                        let items: Vec<Value> = entries
                            .iter()
                            .map(|(k, v)| json!({"key": k, "value": v}))
                            .collect();
                        ToolResult::success(
                            json!({"prefix": prefix, "count": items.len(), "entries": items})
                                .to_string(),
                        )
                    }
                    Err(e) => ToolResult::error(format!("Failed to list keys: {}", e)),
                }
            }

            other => ToolResult::error(format!(
                "Unknown action '{}'. Use: get, set, delete, increment, list",
                other
            )),
        }
    }
}
