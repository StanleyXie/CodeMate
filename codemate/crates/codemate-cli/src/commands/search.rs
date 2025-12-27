//! Search command implementation.

use anyhow::Result;
use codemate_core::storage::{ChunkStore, SqliteStorage, VectorStore};
use codemate_embeddings::EmbeddingGenerator;
use colored::Colorize;
use std::path::PathBuf;

/// Run the search command.
pub async fn run(query: String, database: PathBuf, limit: usize, threshold: f32) -> Result<()> {
    // Check if database exists
    if !database.exists() {
        eprintln!(
            "{} Database not found: {}",
            "✗".red(),
            database.display()
        );
        eprintln!("Run 'codemate index' first to create an index.");
        return Ok(());
    }

    println!("{} Searching for: {}", "→".blue(), query.yellow());
    println!();

    // Initialize storage
    let storage = SqliteStorage::new(&database)?;
    
    // Initialize embeddings
    let mut embedder = EmbeddingGenerator::new()?;
    
    // Generate query embedding
    let query_embedding = embedder.embed(&query)?;
    
    // Search
    let results = storage.search(&query_embedding, limit, threshold).await?;
    
    if results.is_empty() {
        println!("{} No results found.", "→".yellow());
        return Ok(());
    }

    println!("{} Found {} results:", "✓".green(), results.len());
    println!();

    for (i, result) in results.iter().enumerate() {
        // Get the chunk
        let chunk = ChunkStore::get(&storage, &result.content_hash).await?;
        
        if let Some(chunk) = chunk {
            let similarity_pct = (result.similarity * 100.0) as u32;
            
            // Header
            println!(
                "{} {}",
                format!("[{}]", i + 1).blue(),
                format!("{}% match", similarity_pct).green()
            );
            
            // Symbol name if available
            if let Some(ref name) = chunk.symbol_name {
                println!(
                    "    {} {} ({})",
                    "→".dimmed(),
                    name.as_str().yellow(),
                    format!("{:?}", chunk.kind).to_lowercase().dimmed()
                );
            }
            
            // Language
            println!(
                "    {} lang: {}",
                "→".dimmed(),
                chunk.language.as_str().cyan()
            );
            
            // Code preview (first 5 lines)
            println!();
            for (j, line) in chunk.content.lines().take(5).enumerate() {
                if j == 0 {
                    println!("    {}", line.dimmed());
                } else {
                    println!("    {}", line.dimmed());
                }
            }
            if chunk.line_count > 5 {
                println!("    {} ({} more lines)", "...".dimmed(), chunk.line_count - 5);
            }
            println!();
        }
    }

    Ok(())
}
