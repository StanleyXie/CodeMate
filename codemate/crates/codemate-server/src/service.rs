use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;

use codemate_core::service::{
    CodeMateService, ModuleDependency, ModuleResponse, RelatedResponse, SearchOptions, SearchResult,
};
use codemate_core::storage::{
    ChunkStore, Embedder, GraphStore, LocationStore, ModuleStore, QueryStore, SqliteStorage, VectorStore,
};
use codemate_core::query::SearchQuery;
use codemate_core::chunk::Chunk;
use codemate_core::ProjectDetector;

pub struct DefaultCodeMateService {
    storage: Arc<SqliteStorage>,
    embedder: Arc<dyn Embedder>,
}

impl DefaultCodeMateService {
    pub fn new(storage: Arc<SqliteStorage>, embedder: Arc<dyn Embedder>) -> Self {
        Self { storage, embedder }
    }
}

#[async_trait]
impl CodeMateService for DefaultCodeMateService {
    async fn search(&self, query_str: &str, options: SearchOptions) -> Result<Vec<SearchResult>> {
        let query = SearchQuery::parse(query_str);
        
        let embedding = self.embedder.embed(&query.raw_query)?;
        
        let sim_results = QueryStore::query(&*self.storage, &query, &embedding).await
            .map_err(|e| anyhow::anyhow!(e))?;
        
        let mut results = Vec::new();
        for res in sim_results {
            if res.similarity >= options.threshold {
                let chunk = ChunkStore::get(&*self.storage, &res.content_hash).await
                    .map_err(|e| anyhow::anyhow!(e))?;
                results.push(SearchResult {
                    content_hash: res.content_hash.clone().to_string(),
                    similarity: res.similarity,
                    chunk,
                });
            }
            if results.len() >= options.limit {
                break;
            }
        }
        
        Ok(results)
    }
    
    async fn get_tree(&self, symbol: Option<&str>, depth: usize) -> Result<String> {
        if let Some(sym) = symbol {
            codemate_core::storage::utils::render_tree_string(&self.storage, sym, depth).await
                .map_err(|e| anyhow::anyhow!(e))
        } else {
            codemate_core::storage::utils::render_forest_string(&self.storage, depth).await
                .map_err(|e| anyhow::anyhow!(e))
        }
    }
    
    async fn index(&self, path: &Path, _git: bool) -> Result<()> {
        let storage = Arc::clone(&self.storage);
        let embedder = Arc::clone(&self.embedder);
        let path = path.to_path_buf();
        
        tokio::spawn(async move {
            let _ = Self::run_index(&storage, &embedder, path).await;
        });
        
        Ok(())
    }
    
    async fn get_context(&self, symbol: &str) -> Result<Vec<Chunk>> {
        self.storage.find_by_symbol(symbol).await
            .map_err(|e| anyhow::anyhow!(e))
    }
    
    async fn get_related(&self, symbol: &str, limit: usize) -> Result<RelatedResponse> {
        let source_chunks = self.storage.find_by_symbol(symbol).await?;
        
        let mut graph_neighbors = Vec::new();
        let mut semantic_relatives = Vec::new();

        if let Some(source_chunk) = source_chunks.first() {
            // 2. Get graph neighbors
            let edges = GraphStore::get_outgoing_edges(&*self.storage, &source_chunk.content_hash).await?;
            for edge in edges {
                graph_neighbors.push(edge.target_query);
            }

            // 3. Get semantically similar chunks
            let embedding = {
                let text_to_embed = format!(
                    "{} {}", 
                    source_chunk.symbol_name.as_deref().unwrap_or(""), 
                    source_chunk.docstring.as_deref().unwrap_or("")
                );
                self.embedder.embed(&text_to_embed)?
            };
            
            let sim_results = VectorStore::search(&*self.storage, &embedding, limit + 1, 0.5).await
                .map_err(|e| anyhow::anyhow!(e))?;
            
            for res in sim_results {
                if res.content_hash != source_chunk.content_hash {
                    if let Some(chunk) = ChunkStore::get(&*self.storage, &res.content_hash).await
                        .map_err(|e| anyhow::anyhow!(e))? 
                    {
                        if let Some(name) = chunk.symbol_name {
                            semantic_relatives.push(name);
                        }
                    }
                }
                if semantic_relatives.len() >= limit {
                    break;
                }
            }
        }

        Ok(RelatedResponse {
            graph_neighbors,
            semantic_relatives,
        })
    }

    async fn get_module_graph(&self, level: Option<String>, filter_ids: Option<Vec<String>>, show_edges: bool) -> Result<Vec<ModuleResponse>> {
        let level = level.unwrap_or_else(|| "crate".to_string());
        
        let unified_results = self.storage.get_unified_graph(&level, filter_ids, show_edges).await
            .map_err(|e| anyhow::anyhow!(e))?;
        
        let mut response = Vec::new();
        for (module, deps_raw) in unified_results {
            let mut dependencies = Vec::new();
            for (target_id, count, edges_raw) in deps_raw {
                let target_name = self.storage.get_module(&target_id).await?
                    .map(|m| m.name)
                    .unwrap_or_else(|| target_id.clone());
                
                let edges = edges_raw.map(|e_list| {
                    e_list.into_iter().map(|(src, tgt)| {
                        codemate_core::service::models::ModuleEdgeDetail {
                            source_symbol: src,
                            target_symbol: tgt,
                        }
                    }).collect()
                });

                dependencies.push(ModuleDependency {
                    target_id,
                    target_name,
                    count,
                    edges,
                });
            }
            
            response.push(ModuleResponse {
                module,
                dependencies,
            });
        }
        
        Ok(response)
    }

    async fn find_module_cycles(&self) -> Result<Vec<Vec<String>>> {
        codemate_core::storage::utils::find_module_cycles(&self.storage).await
            .map_err(|e| anyhow::anyhow!(e))
    }
}

impl DefaultCodeMateService {
    async fn run_index(storage: &SqliteStorage, embedder: &Arc<dyn Embedder>, path: PathBuf) -> Result<()> {
        use walkdir::WalkDir;
        use codemate_parser::ChunkExtractor;
        use codemate_core::ChunkLocation;
        
        let extractor = ChunkExtractor::new();
        
        // Detect and store modules
        let mut detector = ProjectDetector::new(&path);
        let modules = detector.detect_modules();
        for module in &modules {
            let _ = storage.put_module(module).await;
        }

        let mut total_files = 0;
        let mut total_chunks = 0;

        for entry in WalkDir::new(&path)
            .into_iter()
            .filter_entry(|e| !Self::is_hidden(e) && !Self::is_ignored(e))
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            if !entry.file_type().is_file() {
                continue;
            }

            let file_path = entry.path();
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !Self::is_code_file(ext) {
                continue;
            }

            let (chunks, edges) = match extractor.extract_file(file_path) {
                Ok(res) => res,
                Err(_) => continue,
            };

            // Find containing module
            let module_id = detector.get_module_id_for_file(file_path);

            let relative_path = file_path.strip_prefix(&path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            for chunk in &chunks {
                // Link to module
                let chunk = if let Some(ref mid) = module_id {
                    chunk.clone().with_module_id(mid.clone())
                } else {
                    chunk.clone()
                };

                ChunkStore::put(storage, &chunk).await
                    .map_err(|e| anyhow::anyhow!(e))?;
                
                let embedding_text = format!(
                    "{} {}\n{}",
                    chunk.symbol_name.as_deref().unwrap_or(""),
                    chunk.docstring.as_deref().unwrap_or(""),
                    &chunk.content
                );
                
                if let Ok(embedding) = embedder.embed(&embedding_text) {
                    VectorStore::put(storage, &chunk.content_hash, &embedding).await
                        .map_err(|e| anyhow::anyhow!(e))?;
                }

                let location = ChunkLocation::new(
                    chunk.content_hash.clone(),
                    relative_path.clone(),
                    0,
                    chunk.byte_size,
                    chunk.line_start,
                    chunk.line_end,
                );
                LocationStore::put_location(storage, &location).await
                    .map_err(|e| anyhow::anyhow!(e))?;
                total_chunks += 1;
            }

            if !edges.is_empty() {
                GraphStore::add_edges(storage, &edges).await
                    .map_err(|e| anyhow::anyhow!(e))?;
            }
            total_files += 1;
        }

        tracing::info!("Background indexing complete: {} files, {} chunks", total_files, total_chunks);
        Ok(())
    }

    fn is_hidden(entry: &walkdir::DirEntry) -> bool {
        let name = entry.file_name().to_str().unwrap_or("");
        if name == "." || name == ".." {
            return false;
        }
        name.starts_with('.')
    }

    fn is_ignored(entry: &walkdir::DirEntry) -> bool {
        let name = entry.file_name().to_str().unwrap_or("");
        matches!(
            name,
            "node_modules" | "target" | "dist" | "build" | "__pycache__" | ".git" | "vendor"
        )
    }

    fn is_code_file(ext: &str) -> bool {
        matches!(
            ext,
            "rs" | "py" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp" | "tf" | "tfvars" | "hcl"
        )
    }
}
