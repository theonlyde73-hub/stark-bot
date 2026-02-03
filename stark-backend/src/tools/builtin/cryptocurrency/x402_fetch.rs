//! x402 Fetch tool for making paid HTTP requests via x402 protocol
//!
//! Uses presets to build URLs from register values, preventing hallucination.

use crate::tools::http_retry::HttpRetryManager;
use crate::tools::presets::{get_chain_id, get_fetch_preset, get_network_name, list_fetch_presets};
use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::x402::X402Client;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// x402 Fetch tool for paid HTTP requests (preset-only)
pub struct X402FetchTool {
    definition: ToolDefinition,
}

impl X402FetchTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "preset".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Preset name. Available: 'swap_quote'. Presets read from registers and build URLs automatically.".to_string(),
                default: None,
                items: None,
                enum_values: Some(vec!["swap_quote".to_string()]),
            },
        );

        properties.insert(
            "network".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Network: 'base' or 'mainnet'".to_string(),
                default: Some(json!("base")),
                items: None,
                enum_values: Some(vec!["base".to_string(), "mainnet".to_string()]),
            },
        );

        properties.insert(
            "cache_as".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Register name to cache the result (e.g., 'swap_quote'). Required for passing data to web3_tx.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        X402FetchTool {
            definition: ToolDefinition {
                name: "x402_fetch".to_string(),
                description: "Make HTTP requests to x402-enabled endpoints using presets. Presets read from registers to build URLs. Available: swap_quote.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["preset".to_string()],
                },
                group: ToolGroup::Finance,
            },
        }
    }

    /// Get or create the x402 client
    fn get_client(&self) -> Result<X402Client, String> {
        let private_key = crate::config::burner_wallet_private_key()
            .ok_or("BURNER_WALLET_BOT_PRIVATE_KEY environment variable not set")?;

        X402Client::new(&private_key)
    }

    /// Apply a simple jq-like filter to extract fields from JSON
    fn apply_jq_filter(&self, value: &Value, filter: &str) -> Result<Value, String> {
        let filter = filter.trim();

        // Handle object construction: {key: .field, key2: .field2}
        if filter.starts_with('{') && filter.ends_with('}') {
            let inner = &filter[1..filter.len() - 1];
            let mut result = serde_json::Map::new();

            for part in Self::split_object_fields(inner) {
                let part = part.trim();
                if let Some(colon_pos) = part.find(':') {
                    let key = part[..colon_pos].trim();
                    let field_path = part[colon_pos + 1..].trim();
                    let extracted = self.extract_field(value, field_path)?;
                    result.insert(key.to_string(), extracted);
                }
            }

            return Ok(Value::Object(result));
        }

        // Handle simple field access
        self.extract_field(value, filter)
    }

    /// Split object fields handling nested braces
    fn split_object_fields(s: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for c in s.chars() {
            match c {
                '{' | '[' => {
                    depth += 1;
                    current.push(c);
                }
                '}' | ']' => {
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 => {
                    fields.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(c),
            }
        }

        if !current.trim().is_empty() {
            fields.push(current.trim().to_string());
        }

        fields
    }

    /// Extract a field from JSON using dot notation
    fn extract_field(&self, value: &Value, path: &str) -> Result<Value, String> {
        let path = path.trim();

        if path == "." {
            return Ok(value.clone());
        }

        let path = path.strip_prefix('.').unwrap_or(path);
        let mut current = value;

        for part in path.split('.') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            match current {
                Value::Object(map) => {
                    current = map
                        .get(part)
                        .ok_or_else(|| format!("Field '{}' not found", part))?;
                }
                Value::Array(arr) => {
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr
                            .get(index)
                            .ok_or_else(|| format!("Index {} out of bounds", index))?;
                    } else {
                        return Err(format!("Cannot access '{}' on array", part));
                    }
                }
                _ => return Err(format!("Cannot access '{}' on non-object", part)),
            }
        }

        Ok(current.clone())
    }
}

impl Default for X402FetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct X402FetchParams {
    preset: String,
    #[serde(default = "default_network")]
    network: String,
    cache_as: Option<String>,
}

fn default_network() -> String {
    "base".to_string()
}

#[async_trait]
impl Tool for X402FetchTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: X402FetchParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Get preset configuration
        let preset = match get_fetch_preset(&params.preset) {
            Some(p) => p,
            None => {
                return ToolResult::error(format!(
                    "Unknown preset: '{}'. Available: {}",
                    params.preset,
                    list_fetch_presets().join(", ")
                ))
            }
        };

        // Store network info in registers for use by other tools
        let chain_id = get_chain_id(&params.network);
        let network_name = get_network_name(&params.network);
        context.set_register("network_name", json!(&network_name), "x402_fetch");
        context.set_register("chain_id", json!(&chain_id), "x402_fetch");
        log::info!(
            "[x402_fetch] Stored network info: name={}, chain_id={}",
            network_name, chain_id
        );

        // Build URL from registers
        let mut url_params: Vec<String> = Vec::new();

        // Add chain ID
        url_params.push(format!("chainId={}", chain_id));

        // Read register values and build URL params
        for (reg_key, param_name) in &preset.params {
            let value = match context.registers.get(reg_key) {
                Some(v) => match v.as_str() {
                    Some(s) => s.to_string(),
                    None => v.to_string().trim_matches('"').to_string(),
                },
                None => {
                    return ToolResult::error(format!(
                        "Preset '{}' requires register '{}' but it was not found. Available: {:?}",
                        params.preset,
                        reg_key,
                        context.registers.keys()
                    ));
                }
            };
            url_params.push(format!("{}={}", param_name, value));
        }

        let url = format!("{}?{}", preset.base_url, url_params.join("&"));
        log::info!("[x402_fetch] Preset '{}' built URL: {}", params.preset, url);

        // Validate URL is an x402 endpoint
        if !crate::x402::is_x402_endpoint(&url) {
            return ToolResult::error(
                "URL must be an x402-enabled endpoint. Check preset configuration.",
            );
        }

        // Get the x402 client
        let client = match self.get_client() {
            Ok(c) => c,
            Err(e) => return ToolResult::error(e),
        };

        // Extract host for retry tracking
        let retry_key = format!("x402:{}", params.preset);
        let retry_manager = HttpRetryManager::global();

        // Make the request
        let response = match client.get_with_payment(&url).await {
            Ok(r) => r,
            Err(e) => {
                let error_msg = format!("Request failed: {}", e);
                if HttpRetryManager::is_retryable_error(&error_msg) {
                    let delay = retry_manager.record_error(&retry_key);
                    return ToolResult::retryable_error(error_msg, delay);
                }
                return ToolResult::error(error_msg);
            }
        };

        // Check HTTP status
        let status = response.response.status();
        if !status.is_success() {
            let body = response.response.text().await.unwrap_or_default();

            // Add retry hint for 402 settlement errors (use exponential backoff)
            if status.as_u16() == 402 && (body.contains("Settlement") || body.contains("Facilitator")) {
                let delay = retry_manager.record_error(&retry_key);
                return ToolResult::retryable_error(
                    format!("HTTP error {} Payment Required: {}\n\n⚠️ This is a temporary settlement/payment relay error.", status, body),
                    delay
                );
            }

            // Check for other retryable errors
            if HttpRetryManager::is_retryable_status(status.as_u16()) {
                let delay = retry_manager.record_error(&retry_key);
                return ToolResult::retryable_error(format!("HTTP error {}: {}", status, body), delay);
            }

            return ToolResult::error(format!("HTTP error {}: {}", status, body));
        }

        // Success - reset backoff
        retry_manager.record_success(&retry_key);

        // Parse response body
        let body = match response.response.text().await {
            Ok(b) => b,
            Err(e) => return ToolResult::error(format!("Failed to read response: {}", e)),
        };

        // Parse as JSON and apply filter
        let json_value: Value = match serde_json::from_str(&body) {
            Ok(v) => v,
            Err(_) => {
                return ToolResult::error(format!("Response is not valid JSON: {}", body));
            }
        };

        let filtered = match self.apply_jq_filter(&json_value, &preset.jq_filter) {
            Ok(f) => f,
            Err(e) => return ToolResult::error(format!("Filter error: {}", e)),
        };

        let result_content =
            serde_json::to_string_pretty(&filtered).unwrap_or_else(|_| body.clone());

        // Cache result in register if cache_as is specified
        if let Some(ref register_name) = params.cache_as {
            context.set_register(register_name, filtered.clone(), "x402_fetch");
            log::info!(
                "[x402_fetch] Cached result in register '{}' (keys: {:?})",
                register_name,
                filtered.as_object().map(|o| o.keys().collect::<Vec<_>>())
            );
        }

        // Build metadata
        let mut metadata = json!({
            "preset": params.preset,
            "network": params.network,
            "status": status.as_u16(),
            "wallet": client.wallet_address(),
        });

        if let Some(payment) = response.payment {
            metadata["payment"] = json!({
                "amount": payment.amount_formatted,
                "asset": payment.asset,
                "pay_to": payment.pay_to,
            });
        }

        if let Some(ref register_name) = params.cache_as {
            metadata["cached_in_register"] = json!(register_name);
        }

        ToolResult::success(result_content).with_metadata(metadata)
    }
}
