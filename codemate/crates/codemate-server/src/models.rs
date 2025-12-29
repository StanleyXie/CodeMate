use serde::{Deserialize, Serialize};
use codemate_core::service::SearchResult;

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
pub struct IndexRequest {
    pub path: String,
    pub git: Option<bool>,
    pub max_commits: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct IndexResponse {
    pub files: usize,
    pub chunks: usize,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct TreeRequest {
    pub symbol: Option<String>,
    pub all: Option<bool>,
    pub depth: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct TreeResponse {
    pub tree: String,
}

#[derive(Debug, Deserialize)]
pub struct ModuleGraphRequest {
    pub level: Option<String>,
    pub filters: Option<Vec<String>>,
    pub show_edges: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ModuleGraphResponse {
    pub modules: Vec<codemate_core::service::ModuleResponse>,
}
