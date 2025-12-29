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
    
    // Look up language for root symbols
    let lang_suffix = if current_depth == 0 {
        let chunks = storage.find_by_symbol(symbol).await?;
        if let Some(chunk) = chunks.first() {
            format!(" [{}]", chunk.language.as_str())
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    
    output.push_str(&format!("{}{}{}{}\n", prefix, connector, symbol, lang_suffix));

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

use std::collections::HashMap;
use crate::storage::ModuleStore;

/// Finds circular dependencies between modules.
pub async fn find_module_cycles(storage: &SqliteStorage) -> Result<Vec<Vec<String>>> {
    let modules = storage.get_all_modules().await?;
    let mut adj = HashMap::new();
    for module in modules {
        let deps = storage.get_module_dependencies(&module.id).await?;
        adj.insert(module.id, deps.into_iter().map(|(id, _)| id).collect::<Vec<_>>());
    }

    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut on_stack = HashMap::new();
    let mut path = Vec::new();

    for module_id in adj.keys() {
        if !visited.contains(module_id) {
            dfs_find_module_cycles(module_id, &adj, &mut visited, &mut on_stack, &mut path, &mut cycles);
        }
    }

    Ok(cycles)
}

fn dfs_find_module_cycles(
    u: &str,
    adj: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    on_stack: &mut HashMap<String, usize>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(u.to_string());
    on_stack.insert(u.to_string(), path.len());
    path.push(u.to_string());

    if let Some(neighbors) = adj.get(u) {
        for v in neighbors {
            if let Some(&start_idx) = on_stack.get(v) {
                // Cycle detected
                let mut cycle = path[start_idx..].to_vec();
                cycle.push(v.clone()); // Close the cycle for display
                cycles.push(cycle);
            } else if !visited.contains(v) {
                dfs_find_module_cycles(v, adj, visited, on_stack, path, cycles);
            }
        }
    }

    on_stack.remove(u);
    path.pop();
}
