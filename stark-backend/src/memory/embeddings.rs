use async_trait::async_trait;
use std::sync::RwLock;
use std::time::Duration;

/// Trait for generating text embeddings from various providers.
#[async_trait]
pub trait EmbeddingGenerator {
    async fn generate(&self, text: &str) -> Result<Vec<f32>, String>;

    /// Generate embeddings for multiple texts in a single request.
    /// Default implementation falls back to calling `generate()` in a loop.
    async fn generate_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.generate(text).await?);
        }
        Ok(results)
    }
}

/// Remote embedding generator that calls a self-hosted ONNX embeddings server.
/// Mirrors the whisper-server pattern (POST JSON, get JSON response).
pub struct RemoteEmbeddingGenerator {
    client: reqwest::Client,
    server_url: RwLock<String>,
}

impl RemoteEmbeddingGenerator {
    pub fn new(server_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, server_url: RwLock::new(server_url) }
    }

    /// Update the server URL at runtime (e.g. when bot settings change).
    pub fn update_server_url(&self, url: &str) {
        *self.server_url.write().unwrap() = url.to_string();
    }
}

#[async_trait]
impl EmbeddingGenerator for RemoteEmbeddingGenerator {
    async fn generate(&self, text: &str) -> Result<Vec<f32>, String> {
        let server_url = self.server_url.read().unwrap().clone();
        let url = format!("{}/embed", server_url.trim_end_matches('/'));

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

    async fn generate_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        // Single text: use the regular endpoint
        if texts.len() == 1 {
            return Ok(vec![self.generate(&texts[0]).await?]);
        }

        let server_url = self.server_url.read().unwrap().clone();
        let url = format!("{}/embed_batch", server_url.trim_end_matches('/'));

        let body = serde_json::json!({ "texts": texts });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Batch embeddings request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(format!(
                "Batch embeddings server returned status {}: {}",
                status, error_body
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse batch embeddings response: {}", e))?;

        let embeddings_arr = json
            .get("embeddings")
            .and_then(|e| e.as_array())
            .ok_or_else(|| "Missing 'embeddings' field in batch response".to_string())?;

        if embeddings_arr.len() != texts.len() {
            return Err(format!(
                "Batch response returned {} embeddings but {} texts were sent",
                embeddings_arr.len(),
                texts.len()
            ));
        }

        let mut results = Vec::with_capacity(embeddings_arr.len());
        for emb_val in embeddings_arr {
            let arr = emb_val
                .as_array()
                .ok_or_else(|| "Embedding entry is not an array".to_string())?;
            let vector: Vec<f32> = arr
                .iter()
                .map(|v| {
                    v.as_f64()
                        .map(|f| f as f32)
                        .ok_or_else(|| "Invalid float in batch embedding vector".to_string())
                })
                .collect::<Result<Vec<f32>, String>>()?;
            results.push(vector);
        }

        Ok(results)
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

    async fn generate_batch(&self, _texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        Err("No embedding provider configured".to_string())
    }
}
