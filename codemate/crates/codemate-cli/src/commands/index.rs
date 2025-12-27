//! Index command implementation.

use anyhow::Result;
use codemate_core::storage::{ChunkStore, SqliteStorage, VectorStore};
use codemate_embeddings::EmbeddingGenerator;
use codemate_parser::ChunkExtractor;
use colored::Colorize;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Run the index command.
pub async fn run(path: PathBuf, database: PathBuf) -> Result<()> {
    println!("{} Indexing {}", "→".blue(), path.display());

    // Create database directory if needed
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Initialize storage
    let storage = SqliteStorage::new(&database)?;
    
    // Initialize parser
    let extractor = ChunkExtractor::new();
    
    // Initialize embeddings (lazy - created on first use)
    println!("{} Loading embedding model...", "→".blue());
    let mut embedder = EmbeddingGenerator::new()?;

    // Collect files to index
    let mut total_files = 0;
    let mut total_chunks = 0;
    let mut errors = 0;

    // Walk directory
    for entry in WalkDir::new(&path)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_ignored(e))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Error walking directory: {}", e);
                errors += 1;
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path();
        
        // Skip non-code files
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !is_code_file(ext) {
            continue;
        }

        total_files += 1;
        
        // Extract chunks
        let chunks = match extractor.extract_file(file_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Error parsing {}: {}", file_path.display(), e);
                errors += 1;
                continue;
            }
        };

        // Store chunks and embeddings
        for chunk in &chunks {
            // Store chunk
            ChunkStore::put(&storage, chunk).await?;
            
            // Generate and store embedding
            let embedding_text = format!(
                "{} {}\n{}",
                chunk.symbol_name.as_deref().unwrap_or(""),
                chunk.docstring.as_deref().unwrap_or(""),
                &chunk.content
            );
            
            match embedder.embed(&embedding_text) {
                Ok(embedding) => {
                    VectorStore::put(&storage, &chunk.content_hash, &embedding).await?;
                }
                Err(e) => {
                    tracing::warn!("Error generating embedding: {}", e);
                }
            }
            
            total_chunks += 1;
        }

        if total_files % 10 == 0 {
            print!("\r{} Indexed {} files, {} chunks...", "→".blue(), total_files, total_chunks);
        }
    }

    println!();
    println!();
    println!("{} Indexing complete!", "✓".green());
    println!("  Files: {}", total_files);
    println!("  Chunks: {}", total_chunks);
    println!("  Errors: {}", errors);
    println!("  Database: {}", database.display());

    Ok(())
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
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
