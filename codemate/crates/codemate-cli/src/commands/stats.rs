//! Stats command implementation.

use anyhow::Result;
use codemate_core::storage::{ChunkStore, SqliteStorage};
use colored::Colorize;
use std::path::PathBuf;

/// Run the stats command.
pub async fn run(database: PathBuf) -> Result<()> {
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

    println!("{} Index Statistics", "→".blue());
    println!();

    // Initialize storage
    let storage = SqliteStorage::new(&database)?;
    
    // Get stats
    let chunk_count = storage.count().await?;
    
    // Get file size
    let file_size = std::fs::metadata(&database)?.len();
    let size_mb = file_size as f64 / (1024.0 * 1024.0);

    println!("  Database: {}", database.display());
    println!("  Chunks indexed: {}", chunk_count.to_string().green());
    println!("  Database size: {:.2} MB", size_mb);

    Ok(())
}
