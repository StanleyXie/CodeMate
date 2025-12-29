use serde::{Deserialize, Serialize};
use crate::chunk::{Chunk, Module};

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
pub struct ModuleEdgeDetail {
    pub source_symbol: String,
    pub target_symbol: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleDependency {
    pub target_id: String,
    pub target_name: String,
    pub count: usize,
    pub edges: Option<Vec<ModuleEdgeDetail>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleResponse {
    pub module: Module,
    pub dependencies: Vec<ModuleDependency>,
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
