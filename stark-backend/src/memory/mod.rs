pub mod associations;
pub mod association_loop;
pub mod decay;
pub mod embeddings;
pub mod hybrid_search;
pub mod redaction;
pub mod vector_search;

// Re-exports for convenience
pub use embeddings::EmbeddingGenerator;
pub use hybrid_search::{ConsolidationHint, HybridSearchEngine, HybridSearchResult};
