pub mod models;

use std::path::Path;
use async_trait::async_trait;
use crate::chunk::Chunk;
pub use models::*;

#[async_trait]
pub trait CodeMateService: Send + Sync {
    /// Search for code context using hybrid query
    async fn search(&self, query: &str, options: SearchOptions) -> anyhow::Result<Vec<SearchResult>>;
    
    /// Get a dependency tree for a symbol or the whole project
    async fn get_tree(&self, symbol: Option<&str>, depth: usize) -> anyhow::Result<String>;
    
    /// Trigger background indexing
    async fn index(&self, path: &Path, git: bool) -> anyhow::Result<()>;
    
    /// Get technical context for a symbol
    async fn get_context(&self, symbol: &str) -> anyhow::Result<Vec<Chunk>>;
    
    /// Find semantic and structural relatives
    async fn get_related(&self, symbol: &str, limit: usize) -> anyhow::Result<RelatedResponse>;
}
