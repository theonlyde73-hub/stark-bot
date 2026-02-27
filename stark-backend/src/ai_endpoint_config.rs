use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

const DEFAULT_INFERENCE_ROUTER_URL: &str = "https://inference.defirelay.com";

static AI_ENDPOINTS: OnceLock<HashMap<String, AiEndpointPreset>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEndpointPreset {
    pub display_name: String,
    pub endpoint: String,
    pub model_archetype: String,
    /// Model name to send in request body (for unified router dispatch)
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub x402_cost: Option<u64>,
}

/// Response shape from inference-super-router GET /endpoints
#[derive(Debug, Deserialize)]
struct RouterEndpoint {
    id: String,
    display_name: String,
    endpoint: String,
    model_archetype: String,
    model: Option<String>,
    x402_cost: Option<u64>,
}

/// Fetch endpoint catalog from inference-super-router, fall back to hardcoded default.
pub async fn load_ai_endpoints() {
    let router_url = std::env::var("INFERENCE_ROUTER_URL")
        .unwrap_or_else(|_| DEFAULT_INFERENCE_ROUTER_URL.to_string());
    let url = format!("{}/endpoints", router_url.trim_end_matches('/'));

    let endpoints = match fetch_from_router(&url).await {
        Ok(eps) => {
            log::info!(
                "Loaded {} AI endpoint presets from inference router ({}): {:?}",
                eps.len(),
                url,
                eps.keys().collect::<Vec<_>>()
            );
            eps
        }
        Err(e) => {
            log::warn!(
                "Failed to fetch endpoints from inference router ({}): {}. Using defaults.",
                url, e
            );
            default_endpoints()
        }
    };

    if AI_ENDPOINTS.set(endpoints).is_err() {
        log::warn!("AI endpoints already initialized");
    }
}

async fn fetch_from_router(url: &str) -> Result<HashMap<String, AiEndpointPreset>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let items: Vec<RouterEndpoint> = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let mut map = HashMap::new();
    for item in items {
        let id = item.id.clone();
        map.insert(
            id,
            AiEndpointPreset {
                display_name: item.display_name,
                endpoint: item.endpoint,
                model_archetype: item.model_archetype,
                model: item.model,
                x402_cost: item.x402_cost,
            },
        );
    }
    Ok(map)
}

fn default_endpoints() -> HashMap<String, AiEndpointPreset> {
    let mut endpoints = HashMap::new();
    endpoints.insert(
        "minimax".to_string(),
        AiEndpointPreset {
            display_name: "MiniMax M2.5".to_string(),
            endpoint: "https://inference.defirelay.com/minimax/api/v1/chat/completions".to_string(),
            model_archetype: "minimax".to_string(),
            model: Some("MiniMax-M2.5".to_string()),
            x402_cost: Some(1000),
        },
    );
    endpoints
}

pub fn get_ai_endpoint(key: &str) -> Option<AiEndpointPreset> {
    AI_ENDPOINTS.get().and_then(|endpoints| endpoints.get(key).cloned())
}

pub fn list_ai_endpoints() -> Vec<(String, AiEndpointPreset)> {
    AI_ENDPOINTS
        .get()
        .map(|endpoints| {
            let mut list: Vec<_> = endpoints
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            list.sort_by(|a, b| a.0.cmp(&b.0));
            list
        })
        .unwrap_or_default()
}
