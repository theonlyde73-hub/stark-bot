use async_trait::async_trait;
use std::time::Duration;

/// Trait for generating text embeddings from various providers.
#[async_trait]
pub trait EmbeddingGenerator {
    async fn generate(&self, text: &str) -> Result<Vec<f32>, String>;
}

/// Remote embedding generator that calls a self-hosted ONNX embeddings server.
/// Mirrors the whisper-server pattern (POST JSON, get JSON response).
pub struct RemoteEmbeddingGenerator {
    client: reqwest::Client,
    server_url: String,
}

impl RemoteEmbeddingGenerator {
    pub fn new(server_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, server_url }
    }
}

#[async_trait]
impl EmbeddingGenerator for RemoteEmbeddingGenerator {
    async fn generate(&self, text: &str) -> Result<Vec<f32>, String> {
        let url = format!("{}/embed", self.server_url.trim_end_matches('/'));

        let body = serde_json::json!({ "text": text });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Embeddings server request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(format!(
                "Embeddings server returned status {}: {}",
                status, error_body
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse embeddings response: {}", e))?;

        let embedding = json
            .get("embedding")
            .and_then(|e| e.as_array())
            .ok_or_else(|| "Missing 'embedding' field in response".to_string())?;

        let vector: Vec<f32> = embedding
            .iter()
            .map(|v| {
                v.as_f64()
                    .map(|f| f as f32)
                    .ok_or_else(|| "Invalid float in embedding vector".to_string())
            })
            .collect::<Result<Vec<f32>, String>>()?;

        Ok(vector)
    }
}

/// Null embedding generator that always returns an error.
/// Used when no embedding provider is configured.
pub struct NullEmbeddingGenerator;

#[async_trait]
impl EmbeddingGenerator for NullEmbeddingGenerator {
    async fn generate(&self, _text: &str) -> Result<Vec<f32>, String> {
        Err("No embedding provider configured".to_string())
    }
}
