//! Twitter posting tool using OAuth 1.0a
//!
//! Posts tweets on behalf of a user using their OAuth 1.0a credentials.

use crate::controllers::api_keys::ApiKeyId;
use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{json, Value};
use sha1::Sha1;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha1 = Hmac<Sha1>;

/// Tool for posting tweets via Twitter API v2
pub struct TwitterPostTool {
    definition: ToolDefinition,
}

impl TwitterPostTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "text".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The text content of the tweet (max 280 characters)".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "reply_to".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Optional: Tweet ID to reply to".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "quote_tweet_id".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Optional: Tweet ID to quote".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        TwitterPostTool {
            definition: ToolDefinition {
                name: "twitter_post".to_string(),
                description: "Post a tweet to Twitter/X. Requires Twitter OAuth credentials to be configured in Settings > API Keys.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["text".to_string()],
                },
                group: ToolGroup::Messaging,
            },
        }
    }

    /// Get a Twitter credential from context, with env var fallback
    fn get_credential(&self, key_id: ApiKeyId, context: &ToolContext) -> Option<String> {
        // Try context first
        if let Some(key) = context.get_api_key_by_id(key_id) {
            if !key.is_empty() {
                return Some(key);
            }
        }

        // Fallback to env vars
        if let Some(env_vars) = key_id.env_vars() {
            for var in env_vars {
                if let Ok(val) = std::env::var(var) {
                    if !val.is_empty() {
                        return Some(val);
                    }
                }
            }
        }

        None
    }

    /// Generate OAuth 1.0a Authorization header
    fn generate_oauth_header(
        &self,
        method: &str,
        url: &str,
        consumer_key: &str,
        consumer_secret: &str,
        access_token: &str,
        access_token_secret: &str,
    ) -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let nonce: String = (0..32)
            .map(|_| format!("{:x}", rand::random::<u8>()))
            .collect();

        // OAuth parameters
        let mut oauth_params: Vec<(&str, String)> = vec![
            ("oauth_consumer_key", consumer_key.to_string()),
            ("oauth_nonce", nonce.clone()),
            ("oauth_signature_method", "HMAC-SHA1".to_string()),
            ("oauth_timestamp", timestamp.clone()),
            ("oauth_token", access_token.to_string()),
            ("oauth_version", "1.0".to_string()),
        ];

        // Sort parameters
        oauth_params.sort_by(|a, b| a.0.cmp(&b.0));

        // Create parameter string
        let param_string: String = oauth_params
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        // Create signature base string
        let base_string = format!(
            "{}&{}&{}",
            method.to_uppercase(),
            percent_encode(url),
            percent_encode(&param_string)
        );

        // Create signing key
        let signing_key = format!(
            "{}&{}",
            percent_encode(consumer_secret),
            percent_encode(access_token_secret)
        );

        // Generate HMAC-SHA1 signature
        let mut mac = HmacSha1::new_from_slice(signing_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(base_string.as_bytes());
        let signature = BASE64.encode(mac.finalize().into_bytes());

        // Build Authorization header
        let auth_params = [
            ("oauth_consumer_key", consumer_key),
            ("oauth_nonce", &nonce),
            ("oauth_signature", &signature),
            ("oauth_signature_method", "HMAC-SHA1"),
            ("oauth_timestamp", &timestamp),
            ("oauth_token", access_token),
            ("oauth_version", "1.0"),
        ];

        let auth_string: String = auth_params
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, percent_encode(v)))
            .collect::<Vec<_>>()
            .join(", ");

        format!("OAuth {}", auth_string)
    }
}

impl Default for TwitterPostTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Percent-encode a string per OAuth spec (RFC 3986)
fn percent_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[derive(Debug, Deserialize)]
struct TwitterPostParams {
    text: String,
    reply_to: Option<String>,
    quote_tweet_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TwitterApiResponse {
    data: Option<TwitterTweetData>,
    errors: Option<Vec<TwitterApiError>>,
}

#[derive(Debug, Deserialize)]
struct TwitterTweetData {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct TwitterApiError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

#[async_trait]
impl Tool for TwitterPostTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, context: &ToolContext) -> ToolResult {
        let params: TwitterPostParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate tweet length
        if params.text.is_empty() {
            return ToolResult::error("Tweet text cannot be empty");
        }
        if params.text.chars().count() > 280 {
            return ToolResult::error(format!(
                "Tweet exceeds 280 characters (got {})",
                params.text.chars().count()
            ));
        }

        // Get all 4 OAuth credentials
        let consumer_key = match self.get_credential(ApiKeyId::TwitterConsumerKey, context) {
            Some(k) => k,
            None => {
                return ToolResult::error(
                    "TWITTER_CONSUMER_KEY not configured. Add it in Settings > API Keys.",
                )
            }
        };

        let consumer_secret = match self.get_credential(ApiKeyId::TwitterConsumerSecret, context) {
            Some(k) => k,
            None => {
                return ToolResult::error(
                    "TWITTER_CONSUMER_SECRET not configured. Add it in Settings > API Keys.",
                )
            }
        };

        let access_token = match self.get_credential(ApiKeyId::TwitterAccessToken, context) {
            Some(k) => k,
            None => {
                return ToolResult::error(
                    "TWITTER_ACCESS_TOKEN not configured. Add it in Settings > API Keys.",
                )
            }
        };

        let access_token_secret =
            match self.get_credential(ApiKeyId::TwitterAccessTokenSecret, context) {
                Some(k) => k,
                None => {
                    return ToolResult::error(
                        "TWITTER_ACCESS_TOKEN_SECRET not configured. Add it in Settings > API Keys.",
                    )
                }
            };

        // Build request body
        let mut body = json!({
            "text": params.text
        });

        if let Some(reply_to) = &params.reply_to {
            body["reply"] = json!({
                "in_reply_to_tweet_id": reply_to
            });
        }

        if let Some(quote_id) = &params.quote_tweet_id {
            body["quote_tweet_id"] = json!(quote_id);
        }

        // Twitter API v2 endpoint
        let url = "https://api.twitter.com/2/tweets";

        // Generate OAuth header
        let auth_header = self.generate_oauth_header(
            "POST",
            url,
            &consumer_key,
            &consumer_secret,
            &access_token,
            &access_token_secret,
        );

        // Make the request
        let client = reqwest::Client::new();
        let response = match client
            .post(url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("Failed to send request: {}", e)),
        };

        let status = response.status();
        let response_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            // Try to parse error response
            if let Ok(error_resp) = serde_json::from_str::<TwitterApiResponse>(&response_text) {
                if let Some(errors) = error_resp.errors {
                    let error_msg = errors
                        .iter()
                        .map(|e| e.message.clone())
                        .collect::<Vec<_>>()
                        .join("; ");
                    return ToolResult::error(format!("Twitter API error: {}", error_msg));
                }
            }
            return ToolResult::error(format!(
                "Twitter API error ({}): {}",
                status, response_text
            ));
        }

        // Parse success response
        match serde_json::from_str::<TwitterApiResponse>(&response_text) {
            Ok(resp) => {
                if let Some(data) = resp.data {
                    ToolResult::success(
                        json!({
                            "success": true,
                            "tweet_id": data.id,
                            "text": data.text,
                            "url": format!("https://twitter.com/i/web/status/{}", data.id)
                        })
                        .to_string(),
                    )
                } else {
                    ToolResult::error("Unexpected response format from Twitter API")
                }
            }
            Err(e) => ToolResult::error(format!("Failed to parse Twitter response: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_encode() {
        assert_eq!(percent_encode("hello"), "hello");
        assert_eq!(percent_encode("hello world"), "hello%20world");
        assert_eq!(percent_encode("a=b&c=d"), "a%3Db%26c%3Dd");
    }

    #[test]
    fn test_tool_definition() {
        let tool = TwitterPostTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "twitter_post");
        assert!(def.input_schema.required.contains(&"text".to_string()));
    }
}
