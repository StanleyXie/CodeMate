use std::collections::HashSet;
use crate::storage::{SqliteStorage, ChunkStore, GraphStore};
use crate::Result;
use async_recursion::async_recursion;

/// Renders a dependency tree for a symbol as a string.
pub async fn render_tree_string(
    storage: &SqliteStorage,
    symbol: &str,
    depth: usize,
) -> Result<String> {
    let mut output = String::new();
    let mut visited = HashSet::new();
    
    render_recursive(
        storage,
        symbol,
        "",
        true,
        0,
        depth,
        &mut visited,
        &mut output,
    ).await?;
    
    Ok(output)
}

/// Renders a dependency forest (all root symbols) as a string.
pub async fn render_forest_string(
    storage: &SqliteStorage,
    depth: usize,
) -> Result<String> {
    let mut output = String::new();
    let mut visited = HashSet::new();
    let roots = storage.get_roots().await?;
    
    for (i, root) in roots.iter().enumerate() {
        render_recursive(
            storage,
            root,
            "",
            true,
            0,
            depth,
            &mut visited,
            &mut output,
        ).await?;
        
        if i < roots.len() - 1 {
            output.push('\n');
        }
    }
    
    Ok(output)
}

#[async_recursion]
async fn render_recursive(
    storage: &SqliteStorage,
    symbol: &str,
    prefix: &str,
    is_last: bool,
    current_depth: usize,
    max_depth: usize,
    visited: &mut HashSet<String>,
    output: &mut String,
) -> Result<()> {
    if current_depth > max_depth {
        return Ok(());
    }

    let connector = if current_depth > 0 {
        if is_last { "└── " } else { "├── " }
    } else {
        ""
    };
    output.push_str(&format!("{}{}{}\n", prefix, connector, symbol));

    // Cycle detection
    if visited.contains(symbol) {
        output.push_str(&format!("{}    (cycle detected)\n", prefix));
        return Ok(());
    }
    visited.insert(symbol.to_string());

    // Find outgoing edges for this symbol
    let chunks = storage.find_by_symbol(symbol).await?;
    let mut all_deps = Vec::new();
    for chunk in chunks {
        let edges = storage.get_outgoing_edges(&chunk.content_hash).await?;
        for edge in edges {
            all_deps.push(edge.target_query);
        }
    }
    all_deps.sort();
    all_deps.dedup();
    
    let child_prefix = format!("{}{}", prefix, if current_depth == 0 { "" } else if is_last { "    " } else { "│   " });
    
    let len = all_deps.len();
    for (i, dep) in all_deps.into_iter().enumerate() {
        let is_last_child = i == len - 1;
        render_recursive(
            storage,
            &dep,
            &child_prefix,
            is_last_child,
            current_depth + 1,
            max_depth,
            visited,
            output,
        ).await?;
    }

    Ok(())
}
