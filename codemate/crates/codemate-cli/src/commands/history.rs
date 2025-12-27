//! History command implementation.

use anyhow::Result;
use codemate_core::storage::{LocationStore, SqliteStorage};
use codemate_core::ContentHash;
use colored::Colorize;
use std::path::PathBuf;

/// Run the history command.
pub async fn run(target: String, database: PathBuf, limit: usize) -> Result<()> {
    println!("{} Searching history for: {}", "→".blue(), target);

    // Open database
    if !database.exists() {
        eprintln!("{} Database not found: {}", "✗".red(), database.display());
        eprintln!("  Run 'codemate index --git' first to create the index");
        return Ok(());
    }

    let storage = SqliteStorage::new(&database)?;

    // Determine if target is a content hash or file path
    let locations = if target.len() == 64 && target.chars().all(|c| c.is_ascii_hexdigit()) {
        // Looks like a content hash
        if let Ok(hash) = ContentHash::from_hex(&target) {
            LocationStore::get_location_history(&storage, &hash).await?
        } else {
            vec![]
        }
    } else {
        // Treat as file path
        LocationStore::get_locations_in_file(&storage, &target).await?
    };

    if locations.is_empty() {
        println!("{} No history found for: {}", "⚠".yellow(), target);
        println!("  Make sure you've run 'codemate index --git' first");
        return Ok(());
    }

    println!();
    println!("{} Found {} location(s)", "✓".green(), locations.len());
    println!();

    for (i, loc) in locations.iter().take(limit).enumerate() {
        let hash_short = &loc.content_hash.to_hex()[..8];
        let commit_short = loc.commit_hash.as_ref()
            .map(|c| &c[..7.min(c.len())])
            .unwrap_or("unknown");
        let author = loc.author.as_deref().unwrap_or("unknown");
        let timestamp = loc.timestamp.as_deref().unwrap_or("");

        println!("{}. {} {}", (i + 1).to_string().cyan(), "Chunk".bold(), hash_short.yellow());
        println!("   File: {}", loc.file_path);
        println!("   Lines: {}-{}", loc.line_start, loc.line_end);
        println!("   Commit: {}", commit_short.magenta());
        println!("   Author: {}", author);
        if !timestamp.is_empty() {
            // Try to format the timestamp nicely
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
                println!("   Date: {}", dt.format("%Y-%m-%d %H:%M"));
            } else {
                println!("   Date: {}", timestamp);
            }
        }
        println!();
    }

    if locations.len() > limit {
        println!("  ... and {} more (use --limit to see more)", locations.len() - limit);
    }

    Ok(())
}
