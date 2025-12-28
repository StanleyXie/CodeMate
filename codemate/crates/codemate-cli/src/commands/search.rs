//! Search command implementation.

use anyhow::Result;
use codemate_core::storage::{ChunkStore, QueryStore, SqliteStorage};
use codemate_core::SearchQuery;
use codemate_embeddings::EmbeddingGenerator;
use colored::Colorize;
use std::path::PathBuf;

/// Run the search command.
pub async fn run(query_str: String, database: PathBuf, limit: usize, _threshold: f32) -> Result<()> {
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

    // Parse Query DSL
    let mut query = SearchQuery::parse(&query_str);
    if limit > 0 {
        query.limit = limit;
    }

    println!("{} Searching for: {}", "→".blue(), query.raw_query.yellow());
    if let Some(ref author) = query.author {
        println!("  {} author: {}", "•".dimmed(), author.cyan());
    }
    if let Some(ref lang) = query.lang {
        println!("  {} lang: {}", "•".dimmed(), lang.as_str().cyan());
    }
    println!();

    // Initialize storage
    let storage = SqliteStorage::new(&database)?;
    
    // Initialize embeddings
    let mut embedder = EmbeddingGenerator::new()?;
    
    // Generate query embedding (using the semantic part of the query)
    let query_embedding = embedder.embed(&query.raw_query)?;
    
    // Search using Unified Query Store
    let results = storage.query(&query, &query_embedding).await?;
    
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
            // Header
            println!(
                "{} {}",
                format!("[{}]", i + 1).blue(),
                format!("score: {:.4}", result.similarity).green()
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
