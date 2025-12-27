//! Storage trait definitions.

use crate::{Chunk, ContentHash, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// An embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// The vector data
    pub vector: Vec<f32>,
    /// Model used to generate this embedding
    pub model_id: String,
    /// Dimensions
    pub dimensions: usize,
}

impl Embedding {
    /// Create a new embedding.
    pub fn new(vector: Vec<f32>, model_id: String) -> Self {
        let dimensions = vector.len();
        Self {
            vector,
            model_id,
            dimensions,
        }
    }

    /// Compute cosine similarity with another embedding.
    pub fn cosine_similarity(&self, other: &Embedding) -> f32 {
        if self.dimensions != other.dimensions {
            return 0.0;
        }

        let dot_product: f32 = self
            .vector
            .iter()
            .zip(other.vector.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm_a: f32 = self.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.vector.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

/// Result of a similarity search.
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    /// Content hash of the similar chunk
    pub content_hash: ContentHash,
    /// Similarity score (0.0 to 1.0)
    pub similarity: f32,
}

/// Content-addressable chunk storage trait.
#[async_trait]
pub trait ChunkStore: Send + Sync {
    /// Store a chunk, returns content hash.
    async fn put(&self, chunk: &Chunk) -> Result<ContentHash>;

    /// Retrieve chunk by hash.
    async fn get(&self, hash: &ContentHash) -> Result<Option<Chunk>>;

    /// Check if chunk exists.
    async fn exists(&self, hash: &ContentHash) -> Result<bool>;

    /// Batch retrieval.
    async fn get_many(&self, hashes: &[ContentHash]) -> Result<Vec<Chunk>>;

    /// Count total chunks.
    async fn count(&self) -> Result<usize>;
}

/// Vector storage and similarity search trait.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Store embedding for a content hash.
    async fn put(&self, hash: &ContentHash, embedding: &Embedding) -> Result<()>;

    /// Retrieve embedding.
    async fn get(&self, hash: &ContentHash) -> Result<Option<Embedding>>;

    /// Find similar vectors (k-NN).
    async fn search(
        &self,
        query: &Embedding,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<SimilarityResult>>;

    /// Batch insert.
    async fn put_many(&self, items: &[(ContentHash, Embedding)]) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let e1 = Embedding::new(vec![1.0, 0.0, 0.0], "test".to_string());
        let e2 = Embedding::new(vec![1.0, 0.0, 0.0], "test".to_string());
        let e3 = Embedding::new(vec![0.0, 1.0, 0.0], "test".to_string());

        // Same vectors should have similarity 1.0
        assert!((e1.cosine_similarity(&e2) - 1.0).abs() < 0.001);

        // Orthogonal vectors should have similarity 0.0
        assert!((e1.cosine_similarity(&e3) - 0.0).abs() < 0.001);
    }
}
