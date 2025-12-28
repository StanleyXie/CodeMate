use serde::{Deserialize, Serialize};
use crate::chunk::Chunk;
use crate::storage::SimilarityResult;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub content_hash: String,
    pub similarity: f32,
    pub chunk: Option<Chunk>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RelatedResponse {
    pub graph_neighbors: Vec<String>,
    pub semantic_relatives: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchOptions {
    pub limit: usize,
    pub threshold: f32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 5,
            threshold: 0.3,
        }
    }
}
