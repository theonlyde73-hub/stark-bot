/// Result of a vector similarity search.
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub memory_id: i64,
    pub similarity: f32,
}

/// Compute cosine similarity between two vectors.
///
/// Returns 0.0 if either vector has zero magnitude.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot_product: f32 = 0.0;
    let mut norm_a: f32 = 0.0;
    let mut norm_b: f32 = 0.0;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let magnitude = norm_a.sqrt() * norm_b.sqrt();
    if magnitude == 0.0 {
        return 0.0;
    }

    dot_product / magnitude
}

/// Brute-force vector search over a set of candidate embeddings.
///
/// Returns up to `limit` results with similarity >= `threshold`, sorted by
/// similarity in descending order.
pub fn find_similar(
    query_embedding: &[f32],
    candidates: &[(i64, Vec<f32>)],
    limit: usize,
    threshold: f32,
) -> Vec<VectorSearchResult> {
    let mut results: Vec<VectorSearchResult> = candidates
        .iter()
        .map(|(id, embedding)| VectorSearchResult {
            memory_id: *id,
            similarity: cosine_similarity(query_embedding, embedding),
        })
        .filter(|r| r.similarity >= threshold)
        .collect();

    // Sort by similarity descending
    results.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results.truncate(limit);
    results
}
