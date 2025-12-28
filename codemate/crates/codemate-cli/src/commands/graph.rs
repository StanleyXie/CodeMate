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
pub async fn run_tree(symbol: Option<String>, all: bool, database: PathBuf, depth: usize) -> Result<()> {
    // Initialize storage
    let storage = SqliteStorage::new(&database)?;

    let targets = if all || symbol.is_none() {
        println!("{} Building full dependency forest...", "→".blue());
        let roots = storage.get_roots().await?;
        if roots.is_empty() {
            println!("{} No entry points (roots) found in index.", "⚠".yellow());
            return Ok(());
        }
        roots
    } else if let Some(sym) = symbol {
        println!("{} Building dependency tree for: {}", "→".blue(), sym.bold());
        
        // Check if the starting symbol exists
        let chunks = storage.find_by_symbol(&sym).await?;
        if chunks.is_empty() {
            println!("{} Symbol not found in index: {}", "⚠".yellow(), sym.bold());
            println!("   Make sure you have indexed the files and are using the correct database.");
            return Ok(());
        }
        vec![sym]
    } else {
        return Ok(());
    };

    // Start recursion for each target
    let mut visited = std::collections::HashSet::new();
    for target in targets {
        render_tree_recursive(&storage, &target, "", true, 0, depth, &mut visited).await?;
        if all {
            println!(); // Spacing between trees in a forest
        }
    }

    Ok(())
}

#[async_recursion::async_recursion]
async fn render_tree_recursive(
    storage: &SqliteStorage,
    symbol: &str,
    prefix: &str,
    is_last: bool,
    current_depth: usize,
    max_depth: usize,
    visited: &mut std::collections::HashSet<String>,
) -> Result<()> {
    if current_depth > max_depth {
        return Ok(());
    }

    // Print current node
    let connector = if current_depth == 0 {
        ""
    } else if is_last {
        "└── "
    } else {
        "├── "
    };

    println!("{}{}{}", prefix, connector, symbol.bold());

    // Check for cycles
    if visited.contains(symbol) {
        println!("{}   {}(cycle detected)", prefix, if is_last { " " } else { "│  " });
        return Ok(());
    }
    visited.insert(symbol.to_string());

    // Find the chunk for this symbol to get outgoing edges
    let chunks = storage.find_by_symbol(symbol).await?;
    if chunks.is_empty() {
        return Ok(());
    }

    // Collect all outgoing edges from all chunks with this symbol name
    let mut all_deps = Vec::new();
    for chunk in chunks {
        let edges = storage.get_outgoing_edges(&chunk.content_hash).await?;
        for edge in edges {
            all_deps.push(edge.target_query);
        }
    }

    // Sort and dedup dependencies
    all_deps.sort();
    all_deps.dedup();

    let new_prefix = if current_depth == 0 {
        ""
    } else if is_last {
        &format!("{}    ", prefix)
    } else {
        &format!("{}│   ", prefix)
    };

    let count = all_deps.len();
    for (i, dep) in all_deps.into_iter().enumerate() {
        let is_last_child = i == count - 1;
        render_tree_recursive(
            storage,
            &dep,
            new_prefix,
            is_last_child,
            current_depth + 1,
            max_depth,
            visited,
        ).await?;
    }

    Ok(())
}
