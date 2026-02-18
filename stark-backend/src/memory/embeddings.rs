use async_trait::async_trait;
use serde_json::Value;

/// Trait for generating text embeddings from various providers.
#[async_trait]
pub trait EmbeddingGenerator {
    async fn generate(&self, text: &str) -> Result<Vec<f32>, String>;
}

/// OpenAI-backed embedding generator using the embeddings API.
pub struct OpenAIEmbeddingGenerator {
    client: reqwest::Client,
    api_key: String,
    model: String,
    dimensions: usize,
}

impl OpenAIEmbeddingGenerator {
    /// Create a new generator with default model (text-embedding-3-small) and 256 dimensions.
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: "text-embedding-3-small".to_string(),
            dimensions: 256,
        }
    }

    /// Create a new generator with a specific model and dimension count.
    pub fn with_model(api_key: String, model: String, dimensions: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            dimensions,
        }
    }
}

#[async_trait]
impl EmbeddingGenerator for OpenAIEmbeddingGenerator {
    async fn generate(&self, text: &str) -> Result<Vec<f32>, String> {
        let client = &self.client;

        let body = serde_json::json!({
            "input": text,
            "model": self.model,
            "dimensions": self.dimensions,
        });

        let response = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to call OpenAI embeddings API: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(format!(
                "OpenAI embeddings API returned status {}: {}",
                status, error_body
            ));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse OpenAI embeddings response: {}", e))?;

        let embedding = json
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("embedding"))
            .and_then(|e| e.as_array())
            .ok_or_else(|| "Missing embedding data in OpenAI response".to_string())?;

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
