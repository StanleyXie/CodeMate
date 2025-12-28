use anyhow::Result;
use codemate_core::storage::{ChunkStore, GraphStore, LocationStore, SqliteStorage};
use colored::Colorize;
use std::path::PathBuf;

/// Run the graph command.
pub async fn run_callers(symbol: String, database: PathBuf) -> Result<()> {
    println!("{} Searching callers for: {}", "→".blue(), symbol.bold());

    // Initialize storage
    let storage = SqliteStorage::new(&database)?;

    // Get incoming edges (callers)
    let callers = storage.get_incoming_edges(&symbol).await?;

    if callers.is_empty() {
        println!("{} No callers found for {}", "⚠".yellow(), symbol.bold());
        return Ok(());
    }

    println!("{} Found {} caller(s)\n", "✓".green(), callers.len());

    for (i, edge) in callers.iter().enumerate() {
        // Find the source chunk to get its symbol name
        let source_chunk = ChunkStore::get(&storage, &edge.source_hash).await?;
        let source_name = source_chunk
            .as_ref()
            .and_then(|c| c.symbol_name.clone())
            .unwrap_or_else(|| "unknown".to_string());
            
        println!("{}. {}", i + 1, source_name.bold());
        if let Some(line) = edge.line_number {
            println!("   Line: {}", line);
        }
        println!();
    }

    Ok(())
}

pub async fn run_deps(file_path: String, database: PathBuf) -> Result<()> {
    println!("{} Searching dependencies for: {}", "→".blue(), file_path.bold());

    // Initialize storage
    let storage = SqliteStorage::new(&database)?;
    
    // 1. Find all locations in the file to get content hashes
    let locations = storage.get_locations_in_file(&file_path).await?;
    
    if locations.is_empty() {
        println!("{} No chunks found for file: {}", "⚠".yellow(), file_path.bold());
        return Ok(());
    }

    let mut total_deps = 0;
    println!("{} Found {} code chunk(s) in file\n", "→".blue(), locations.len());

    for location in locations {
        // 2. Get outgoing edges for each chunk
        let edges = storage.get_outgoing_edges(&location.content_hash).await?;
        
        if edges.is_empty() {
            continue;
        }

        // Get chunk info for display
        let chunk = ChunkStore::get(&storage, &location.content_hash).await?;
        let symbol = chunk.as_ref()
            .and_then(|c| c.symbol_name.clone())
            .unwrap_or_else(|| format!("Lines {}-{}", location.line_start, location.line_end));

        println!("{} {}:", "•".blue(), symbol.bold());
        for edge in edges {
            let kind_label = match edge.kind {
                codemate_core::EdgeKind::Calls => "calls".cyan(),
                codemate_core::EdgeKind::Imports => "imports".magenta(),
                codemate_core::EdgeKind::References => "references".yellow(),
            };
            
            print!("   {} {}", kind_label, edge.target_query.bold());
            if let Some(line) = edge.line_number {
                print!(" (line {})", line);
            }
            println!();
            total_deps += 1;
        }
        println!();
    }

    if total_deps == 0 {
        println!("{} No outgoing dependencies found", "⚠".yellow());
    }

    Ok(())
}
