use anyhow::Result;
use codemate_core::storage::{ChunkStore, GraphStore, LocationStore, ModuleStore, SqliteStorage};
use codemate_core::Language;
use colored::{Colorize, ColoredString};
use std::path::PathBuf;

/// Get a colored language label for display (colors based on GitHub Linguist)
fn colored_lang(lang: Language) -> ColoredString {
    match lang {
        Language::Rust => "rust".truecolor(222, 165, 132),        // #dea584 - Orange/Peach
        Language::Python => "python".truecolor(53, 114, 165),     // #3572A5 - Blue
        Language::Go => "go".truecolor(0, 173, 216),              // #00ADD8 - Cyan
        Language::TypeScript => "typescript".truecolor(49, 120, 198), // #3178c6 - Blue
        Language::JavaScript => "javascript".truecolor(241, 224, 90), // #f1e05a - Yellow
        Language::Java => "java".truecolor(176, 114, 25),         // #b07219 - Brown/Orange
        Language::Hcl => "hcl".truecolor(88, 103, 148),           // #586794 - Purple/Blue
        Language::Unknown => "unknown".white(),
    }
}

/// Common stdlib symbols that shouldn't trigger cycle detection noise
const COMMON_SYMBOLS: &[&str] = &[
    "Ok", "Err", "Some", "None", "Vec::new", "String::new", "HashSet::new",
    "HashMap::new", "Arc::new", "Box::new", "Rc::new", "Default::default",
    "Json", "Result", "Option", "print", "println", "format",
];

/// Check if a symbol is a common stdlib item
fn is_common_symbol(symbol: &str) -> bool {
    COMMON_SYMBOLS.iter().any(|&s| symbol == s || symbol.starts_with(&format!("{}(", s)))
}

/// Truncate symbol name to max length for display
fn truncate_symbol(symbol: &str, max_len: usize) -> String {
    // Remove newlines and extra whitespace first
    let cleaned: String = symbol.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.len() <= max_len {
        cleaned
    } else {
        format!("{}...", &cleaned[..max_len - 3])
    }
}

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

    // Start recursion for each target, skip empty roots
    let mut visited = std::collections::HashSet::new();
    let mut shown_count = 0;
    for target in targets {
        // Pre-check if root has any edges (skip empty roots in --all mode)
        if all {
            let chunks = storage.find_by_symbol(&target).await?;
            let mut has_edges = false;
            for chunk in &chunks {
                let edges = storage.get_outgoing_edges(&chunk.content_hash).await?;
                if !edges.is_empty() {
                    has_edges = true;
                    break;
                }
            }
            if !has_edges && chunks.is_empty() {
                continue; // Skip roots with no edges
            }
        }
        render_tree_recursive(&storage, &target, "", true, 0, depth, &mut visited).await?;
        shown_count += 1;
        if all {
            println!(); // Spacing between trees in a forest
        }
    }
    
    if shown_count == 0 && all {
        println!("{} No symbols with dependencies found", "⚠".yellow());
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

    // Get language for root-level symbols (placed at front with fixed-width padding)
    let chunks = storage.find_by_symbol(symbol).await?;
    let lang_prefix = if current_depth == 0 {
        if let Some(chunk) = chunks.first() {
            // Fixed-width language tag for alignment (6 chars + brackets = 9)
            format!("[{:<6}] ", colored_lang(chunk.language))
        } else {
            "         ".to_string() // 9 spaces for alignment when no language
        }
    } else {
        String::new()
    };

    // Truncate and display symbol name
    let display_symbol = truncate_symbol(symbol, 60);
    println!("{}{}{}{}", lang_prefix, prefix, connector, display_symbol.bold());

    // Check for cycles (skip common symbols to reduce noise)
    if visited.contains(symbol) && !is_common_symbol(symbol) {
        let padding = if current_depth == 0 { "         " } else { "" };
        println!("{}{}   {}(cycle detected)", padding, prefix, if is_last { " " } else { "│  " });
        return Ok(());
    }
    // Always add to visited for actual cycle prevention, but don't print noise for common ones
    if visited.contains(symbol) {
        return Ok(());
    }
    visited.insert(symbol.to_string());

    // Find the chunk for this symbol to get outgoing edges
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

    // Add 9-char padding for child nodes to align with root language prefix
    let base_prefix = if current_depth == 0 { "         " } else { "" };
    let new_prefix = if current_depth == 0 {
        format!("{}    ", base_prefix)
    } else if is_last {
        format!("{}{}    ", base_prefix, prefix)
    } else {
        format!("{}{}│   ", base_prefix, prefix)
    };

    let count = all_deps.len();
    for (i, dep) in all_deps.into_iter().enumerate() {
        let is_last_child = i == count - 1;
        render_tree_recursive(
            storage,
            &dep,
            &new_prefix,
            is_last_child,
            current_depth + 1,
            max_depth,
            visited,
        ).await?;
    }
    Ok(())
}

use codemate_core::service::exporter::ModuleGraphExporter;
use codemate_core::service::models::{ModuleDependency, ModuleResponse};
use std::fs;

pub async fn run_modules(
    database: PathBuf, 
    format: String, 
    output: Option<PathBuf>, 
    level: String,
    show_edges: bool,
    filter: Option<Vec<String>>,
    check_cycles: bool
) -> Result<()> {
    // Initialize storage
    let storage = SqliteStorage::new(&database)?;
    
    // Optional: Check for cycles
    if check_cycles {
        println!("{} Checking for circular dependencies...", "→".blue());
        let cycles = codemate_core::storage::utils::find_module_cycles(&storage).await?;
        if cycles.is_empty() {
            println!("{} No circular dependencies found.", "✓".green());
        } else {
            println!("{} Found {} circular dependency cycle(s):", "⚠".red(), cycles.len());
            for (i, cycle) in cycles.iter().enumerate() {
                println!("  Cycle {}: {}", i + 1, cycle.join(" \u{2192} ").red());
            }
        }
        println!();
    }

    // Get unified graph
    let unified_results = storage.get_unified_graph(&level, filter, show_edges).await?;
    
    if unified_results.is_empty() {
        println!("{} No results found.", "⚠".yellow());
        return Ok(());
    }

    // Convert to ModuleResponse format for the exporter
    let mut module_responses = Vec::new();
    for (module, deps_raw) in unified_results {
        let mut dependencies = Vec::new();
        for (target_id, count, edges_raw) in deps_raw {
            let target_name = storage.get_module(&target_id).await?
                .map(|m| m.name)
                .unwrap_or_else(|| target_id.clone());
            
            let edges = edges_raw.map(|e_list| {
                e_list.into_iter().collect()
            });

            dependencies.push(ModuleDependency {
                target_id,
                target_name,
                count,
                edges,
            });
        }
        
        module_responses.push(ModuleResponse {
            module,
            dependencies,
        });
    }

    // Process based on format
    let result = match format.to_lowercase().as_str() {
        "text" => {
            render_modules_text(&module_responses, &level, show_edges);
            None
        }
        "dot" => Some(ModuleGraphExporter::to_dot(&module_responses)),
        "mermaid" => Some(ModuleGraphExporter::to_mermaid(&module_responses)),
        "json" => Some(ModuleGraphExporter::to_json(&module_responses)),
        "html" => Some(ModuleGraphExporter::to_html(&module_responses)),
        _ => {
            anyhow::bail!("Unsupported format: {}. Supported formats: text, dot, mermaid, json, html", format);
        }
    };

    // Handle output
    if let Some(content) = result {
        if let Some(path) = output {
            fs::write(&path, content)?;
            println!("{} Exported module graph to: {}", "✓".green(), path.display());
        } else {
            println!("{}", content);
        }
    }

    Ok(())
}

fn render_modules_text(modules: &[ModuleResponse], level: &str, show_edges: bool) {
    println!("{} Indexing module-level dependencies (level: {})...", "→".blue(), level);
    println!("{} Found {} module(s)\n", "✓".green(), modules.len());

    for res in modules {
        println!("📦 {} ({})", res.module.name.bold(), res.module.project_type.as_str().dimmed());
        println!("   Path: {}", res.module.path.dimmed());
        if res.dependencies.is_empty() {
            println!("   No external module dependencies");
        } else {
            println!("   Dependencies:");
            for dep in &res.dependencies {
                println!("     → {} ({} edges)", dep.target_name.cyan(), dep.count);
                if show_edges {
                    if let Some(ref edges) = dep.edges {
                        for edge in edges.iter().take(5) {
                            let kind_label = match edge.kind {
                                codemate_core::EdgeKind::Calls => "calls".cyan(),
                                codemate_core::EdgeKind::Imports => "imports".magenta(),
                                codemate_core::EdgeKind::References => "references".yellow(),
                            };
                            print!("       • {} {} {}", edge.source_symbol.dimmed(), kind_label, edge.target_symbol.dimmed());
                            if let Some(line) = edge.line_number {
                                print!(" (line {})", line);
                            }
                            println!();
                        }
                        if edges.len() > 5 {
                            println!("       ... and {} more", edges.len() - 5);
                        }
                    }
                }
            }
        }
        println!();
    }
}
