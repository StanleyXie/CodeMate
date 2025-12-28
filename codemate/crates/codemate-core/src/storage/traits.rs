//! Storage trait definitions.

use crate::{Chunk, ChunkLocation, ContentHash, Edge, Result, SearchQuery};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Find chunks by symbol name.
    async fn find_by_symbol(&self, symbol_name: &str) -> Result<Vec<Chunk>>;
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

/// Graph storage trait for tracking relationships between code elements.
#[async_trait]
pub trait GraphStore: Send + Sync {
    /// Add a relationship edge.
    async fn add_edge(&self, edge: &Edge) -> Result<()>;

    /// Batch add edges.
    async fn add_edges(&self, edges: &[Edge]) -> Result<()>;

    /// Get outgoing edges for a chunk.
    async fn get_outgoing_edges(&self, source_hash: &ContentHash) -> Result<Vec<Edge>>;

    /// Get incoming edges for a target query.
    async fn get_incoming_edges(&self, target_query: &str) -> Result<Vec<Edge>>;

    /// Get all root symbols (those with no incoming edges).
    async fn get_roots(&self) -> Result<Vec<String>>;
}

/// Location storage trait for tracking chunk locations across commits.
#[async_trait]
pub trait LocationStore: Send + Sync {
    /// Store a chunk location.
    async fn put_location(&self, location: &ChunkLocation) -> Result<()>;

    /// Get all locations for a chunk (across all commits).
    async fn get_locations(&self, content_hash: &ContentHash) -> Result<Vec<ChunkLocation>>;

    /// Get locations at a specific commit.
    async fn get_locations_at_commit(&self, commit_hash: &str) -> Result<Vec<ChunkLocation>>;

    /// Get locations in a file.
    async fn get_locations_in_file(&self, file_path: &str) -> Result<Vec<ChunkLocation>>;

    /// Get location history for a chunk (all commits where it appeared).
    async fn get_location_history(&self, content_hash: &ContentHash) -> Result<Vec<ChunkLocation>>;
}

/// Unified query storage trait for hybrid and filtered search.
#[async_trait]
pub trait QueryStore: Send + Sync {
    /// Perform a filtered, hybrid search.
    async fn query(
        &self,
        query: &SearchQuery,
        embedding: &Embedding,
    ) -> Result<Vec<SimilarityResult>>;
}

/// Trait for generating text embeddings.
pub trait Embedder: Send + Sync {
    /// Generate embedding for a single text.
    fn embed(&self, text: &str) -> Result<Embedding>;
    
    /// Generate embeddings for multiple texts.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>>;
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
