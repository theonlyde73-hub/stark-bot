//! From Raw Amount tool for converting raw blockchain units to human-readable token amounts
//!
//! Converts raw amounts like "871043093" to human-readable "871.043093" based on token decimals.
//! This is the reverse of `to_raw_amount`.

use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// From Raw Amount tool
pub struct FromRawAmountTool {
    definition: ToolDefinition,
}

impl FromRawAmountTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "raw_amount".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Raw blockchain value (e.g., '871043093', '1000000000000000000'). This is the value returned by balanceOf and similar view functions.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "decimals".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "Token decimals (e.g., 18 for ETH/most tokens, 6 for USDC, 8 for cbBTC). If not provided, reads from 'token_address_decimals' register.".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "decimals_register".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Register name containing the decimals value. Defaults to 'token_address_decimals'. Use 'sell_token_decimals' or 'buy_token_decimals' for swaps.".to_string(),
                default: Some(json!("token_address_decimals")),
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "cache_as".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Register name to cache the human-readable amount. Defaults to 'human_amount'.".to_string(),
                default: Some(json!("human_amount")),
                items: None,
                enum_values: None,
            },
        );

        FromRawAmountTool {
            definition: ToolDefinition {
                name: "from_raw_amount".to_string(),
                description: "Convert raw blockchain units to human-readable token amount. Use AFTER reading a balance or any raw uint256 value. Divides by 10^decimals. Example: raw_amount='871043093', decimals=6 → '871.043093'.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["raw_amount".to_string()],
                },
                group: ToolGroup::Finance,
                hidden: false,
            },
        }
    }

    /// Convert raw blockchain units to human-readable amount
    /// Handles large integers properly by inserting decimal point at the right position
    pub fn convert_from_raw(raw: &str, decimals: u8) -> Result<String, String> {
        let raw = raw.trim().trim_matches('"');

        // Validate input is numeric
        if raw.is_empty() || !raw.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("Invalid raw amount: '{}'. Must be a non-negative integer.", raw));
        }

        // Remove leading zeros but keep at least one digit
        let raw = raw.trim_start_matches('0');
        let raw = if raw.is_empty() { "0" } else { raw };

        let decimals = decimals as usize;

        if decimals == 0 {
            return Ok(raw.to_string());
        }

        let raw_len = raw.len();

        if raw_len <= decimals {
            // Raw value is smaller than 1.0 — need leading zeros after decimal
            let leading_zeros = decimals - raw_len;
            let decimal_part = format!("{}{}", "0".repeat(leading_zeros), raw);
            // Trim trailing zeros
            let trimmed = decimal_part.trim_end_matches('0');
            if trimmed.is_empty() {
                Ok("0".to_string())
            } else {
                Ok(format!("0.{}", trimmed))
            }
        } else {
            // Split into integer and decimal parts
            let split_pos = raw_len - decimals;
            let integer_part = &raw[..split_pos];
            let decimal_part = raw[split_pos..].trim_end_matches('0');
            if decimal_part.is_empty() {
                Ok(integer_part.to_string())
            } else {
                Ok(format!("{}.{}", integer_part, decimal_part))
            }
        }
    }
}

impl Default for FromRawAmountTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct FromRawAmountParams {
    raw_amount: String,
    decimals: Option<u8>,
    #[serde(default = "default_decimals_register")]
    decimals_register: String,
    #[serde(default = "default_cache_as")]
    cache_as: String,
}

fn default_decimals_register() -> String {
    "token_address_decimals".to_string()
}

fn default_cache_as() -> String {
    "human_amount".to_string()
}

#[async_trait]
impl Tool for FromRawAmountTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: FromRawAmountParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Get decimals: explicit param > register > error
        let decimals: u8 = if let Some(d) = params.decimals {
            d
        } else {
            // Try to read from register
            match context.registers.get(&params.decimals_register) {
                Some(val) => {
                    // Handle both number and string representations
                    if let Some(d) = val.as_u64() {
                        d as u8
                    } else if let Some(s) = val.as_str() {
                        match s.parse::<u8>() {
                            Ok(d) => d,
                            Err(_) => {
                                return ToolResult::error(format!(
                                    "Register '{}' contains invalid decimals value: '{}'",
                                    params.decimals_register, s
                                ));
                            }
                        }
                    } else {
                        return ToolResult::error(format!(
                            "Register '{}' contains invalid decimals value: {:?}. Expected a number.",
                            params.decimals_register, val
                        ));
                    }
                }
                None => {
                    return ToolResult::error(format!(
                        "No decimals provided and register '{}' not found. Use token_lookup first to set decimals, or provide decimals parameter explicitly.",
                        params.decimals_register
                    ));
                }
            }
        };

        // Convert amount
        let human_amount = match Self::convert_from_raw(&params.raw_amount, decimals) {
            Ok(h) => h,
            Err(e) => return ToolResult::error(e),
        };

        // Store in register
        context.set_register(&params.cache_as, json!(&human_amount), "from_raw_amount");

        log::info!(
            "[from_raw_amount] Converted {} (decimals={}) → {} (cached in '{}')",
            params.raw_amount,
            decimals,
            human_amount,
            params.cache_as
        );

        ToolResult::success(format!(
            "{} ÷ 10^{} = {}\nCached in register: '{}'",
            params.raw_amount,
            decimals,
            human_amount,
            params.cache_as
        )).with_metadata(json!({
            "raw_amount": params.raw_amount,
            "decimals": decimals,
            "human_amount": human_amount,
            "cached_in_register": params.cache_as
        }))
    }

    // Standard — writes human amount to context registers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usdc_balance() {
        assert_eq!(FromRawAmountTool::convert_from_raw("871043093", 6).unwrap(), "871.043093");
    }

    #[test]
    fn test_eth_one_token() {
        assert_eq!(FromRawAmountTool::convert_from_raw("1000000000000000000", 18).unwrap(), "1");
    }

    #[test]
    fn test_eth_fractional() {
        assert_eq!(FromRawAmountTool::convert_from_raw("1500000000000000000", 18).unwrap(), "1.5");
        assert_eq!(FromRawAmountTool::convert_from_raw("500000000000000000", 18).unwrap(), "0.5");
        assert_eq!(FromRawAmountTool::convert_from_raw("1000000000000000", 18).unwrap(), "0.001");
    }

    #[test]
    fn test_zero() {
        assert_eq!(FromRawAmountTool::convert_from_raw("0", 6).unwrap(), "0");
        assert_eq!(FromRawAmountTool::convert_from_raw("0", 18).unwrap(), "0");
    }

    #[test]
    fn test_whole_numbers() {
        assert_eq!(FromRawAmountTool::convert_from_raw("1000000", 6).unwrap(), "1");
        assert_eq!(FromRawAmountTool::convert_from_raw("100000000", 6).unwrap(), "100");
        assert_eq!(FromRawAmountTool::convert_from_raw("100000000", 8).unwrap(), "1");
    }

    #[test]
    fn test_small_values() {
        assert_eq!(FromRawAmountTool::convert_from_raw("1", 6).unwrap(), "0.000001");
        assert_eq!(FromRawAmountTool::convert_from_raw("1", 18).unwrap(), "0.000000000000000001");
        assert_eq!(FromRawAmountTool::convert_from_raw("10", 6).unwrap(), "0.00001");
    }

    #[test]
    fn test_no_decimals() {
        assert_eq!(FromRawAmountTool::convert_from_raw("12345", 0).unwrap(), "12345");
    }

    #[test]
    fn test_large_values() {
        assert_eq!(
            FromRawAmountTool::convert_from_raw("100000000000000000000", 18).unwrap(),
            "100"
        );
    }

    #[test]
    fn test_leading_zeros_in_input() {
        assert_eq!(FromRawAmountTool::convert_from_raw("000871043093", 6).unwrap(), "871.043093");
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(FromRawAmountTool::convert_from_raw(" 871043093 ", 6).unwrap(), "871.043093");
    }

    #[test]
    fn test_quoted_input() {
        assert_eq!(FromRawAmountTool::convert_from_raw("\"871043093\"", 6).unwrap(), "871.043093");
    }

    #[test]
    fn test_invalid_input() {
        assert!(FromRawAmountTool::convert_from_raw("abc", 18).is_err());
        assert!(FromRawAmountTool::convert_from_raw("1.5", 18).is_err());
        assert!(FromRawAmountTool::convert_from_raw("-1", 18).is_err());
        assert!(FromRawAmountTool::convert_from_raw("", 18).is_err());
    }
}
