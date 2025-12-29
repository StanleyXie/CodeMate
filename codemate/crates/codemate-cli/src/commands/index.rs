//! Index command implementation.

use anyhow::Result;
use codemate_core::storage::{ChunkStore, Embedder, GraphStore, LocationStore, ModuleStore, SqliteStorage, VectorStore};
use codemate_core::{ChunkLocation, ProjectDetector};
use codemate_embeddings::EmbeddingGenerator;
use codemate_parser::ChunkExtractor;
use colored::Colorize;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Run the index command.
pub async fn run(path: PathBuf, database: PathBuf, git_mode: bool, _max_commits: usize) -> Result<()> {
    if git_mode {
        run_git_aware(&path, &database).await
    } else {
        run_simple(&path, &database).await
    }
}

/// Simple indexing (current files only)
async fn run_simple(path: &PathBuf, database: &PathBuf) -> Result<()> {
    println!("{} Indexing {}", "→".blue(), path.display());

    // Create database directory if needed
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Initialize storage
    let storage = SqliteStorage::new(database)?;
    
    // Initialize parser
    let extractor = ChunkExtractor::new();
    
    // Detect modules
    println!("{} Detecting modules...", "→".blue());
    let mut detector = ProjectDetector::new(path.as_path());
    let modules = detector.detect_modules();
    for module in &modules {
        ModuleStore::put_module(&storage, module).await?;
    }
    println!("  Found {} modules", modules.len());

    // Initialize embeddings (lazy - created on first use)
    println!("{} Loading embedding model...", "→".blue());
    let mut embedder = EmbeddingGenerator::new()?;

    // Collect files to index
    let mut total_files = 0;
    let mut total_chunks = 0;
    let mut errors = 0;

    // Walk directory
    for entry in WalkDir::new(path)
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
        
        // Extract chunks and edges
        let (chunks, edges) = match extractor.extract_file(file_path) {
            Ok(res) => res,
            Err(e) => {
                tracing::warn!("Error parsing {}: {}", file_path.display(), e);
                errors += 1;
                continue;
            }
        };

        // Find containing module
        let module_id = detector.get_module_id_for_file(file_path);

        // Get relative path for location tracking
        let relative_path = file_path.strip_prefix(path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        // Store chunks and embeddings
        for chunk in &chunks {
            // Link to module
            let chunk = if let Some(ref mid) = module_id {
                chunk.clone().with_module_id(mid.clone())
            } else {
                chunk.clone()
            };

            // Store chunk
            ChunkStore::put(&storage, &chunk).await?;
            
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

            // Store location
            let location = ChunkLocation::new(
                chunk.content_hash.clone(),
                relative_path.clone(),
                0,
                chunk.byte_size,
                chunk.line_start,
                chunk.line_end,
            );
            LocationStore::put_location(&storage, &location).await?;
            
            total_chunks += 1;
        }

        // Store edges
        if !edges.is_empty() {
            GraphStore::add_edges(&storage, &edges).await?;
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

/// Git-aware indexing with location tracking
async fn run_git_aware(path: &PathBuf, database: &PathBuf) -> Result<()> {
    use codemate_git::GitRepository;

    println!("{} Git-aware indexing {}", "→".blue(), path.display());

    // Open git repository
    let repo = match GitRepository::open(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} Failed to open git repository: {}", "✗".red(), e);
            eprintln!("  Use without --git flag for non-git directories");
            return Err(e.into());
        }
    };

    let head = repo.head_commit()?;
    println!("{} HEAD: {} - {}", "→".blue(), head.short_hash, head.summary);

    let repo_root = repo.root().canonicalize()?;
    let path = path.canonicalize()?;
    println!("{} Repo root: {}", "→".blue(), repo_root.display());
    println!("{} Indexing path: {}", "→".blue(), path.display());

    // Create database directory if needed
    if let Some(parent) = database.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Initialize storage
    let storage = SqliteStorage::new(database)?;
    
    // Initialize parser
    let extractor = ChunkExtractor::new();
    
    // Detect modules
    println!("{} Detecting modules...", "→".blue());
    let mut detector = ProjectDetector::new(path.as_path());
    let modules = detector.detect_modules();
    for module in &modules {
        ModuleStore::put_module(&storage, module).await?;
    }
    println!("  Found {} modules", modules.len());

    // Initialize embeddings
    println!("{} Loading embedding model...", "→".blue());
    let mut embedder = EmbeddingGenerator::new()?;

    let mut total_files = 0;
    let mut total_chunks = 0;
    let mut total_locations = 0;
    let mut errors = 0;

    // Walk directory
    for entry in WalkDir::new(path)
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

        let file_path = entry.path().canonicalize()?;
        
        // Skip non-code files
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !is_code_file(ext) {
            continue;
        }

        // Get path relative to git root for git operations and storage
        let git_relative_path = file_path.strip_prefix(&repo_root)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .to_string();

        total_files += 1;
        
        // Extract chunks and edges
        let (chunks, edges) = match extractor.extract_file(&file_path) {
            Ok(res) => res,
            Err(e) => {
                tracing::warn!("Error parsing {}: {}", file_path.display(), e);
                errors += 1;
                continue;
            }
        };

        // Find containing module
        let module_id = detector.get_module_id_for_file(&file_path);

        // Store chunks with location info
        for chunk in &chunks {
            // Link to module
            let chunk = if let Some(ref mid) = module_id {
                chunk.clone().with_module_id(mid.clone())
            } else {
                chunk.clone()
            };

            // Store chunk
            ChunkStore::put(&storage, &chunk).await?;
            
            // Generate and store embedding
            let embedding_text = format!(
                "{} {}\n{}",
                chunk.symbol_name.as_deref().unwrap_or(""),
                chunk.docstring.as_deref().unwrap_or(""),
                &chunk.content
            );
            
            if let Ok(embedding) = embedder.embed(&embedding_text) {
                VectorStore::put(&storage, &chunk.content_hash, &embedding).await?;
            }

            // Create location with git info
            let mut location = ChunkLocation::new(
                chunk.content_hash.clone(),
                git_relative_path.clone(),
                0, // TODO: track actual byte offsets
                chunk.byte_size,
                chunk.line_start,
                chunk.line_end,
            ).with_commit(head.hash.clone());

            // Add accurate blame info if available
            if let Ok(Some(info)) = repo.primary_author(&git_relative_path, chunk.line_start, chunk.line_end) {
                location = location
                    .with_author(info.author())
                    .with_timestamp(info.timestamp.to_rfc3339());
            }

            LocationStore::put_location(&storage, &location).await?;
            total_locations += 1;
            total_chunks += 1;
        }

        // Store edges
        if !edges.is_empty() {
            GraphStore::add_edges(&storage, &edges).await?;
        }

        if total_files % 10 == 0 {
            print!("\r{} Indexed {} files, {} chunks...", "→".blue(), total_files, total_chunks);
        }
    }

    println!();
    println!();
    println!("{} Git-aware indexing complete!", "✓".green());
    println!("  Commit: {} ({})", head.short_hash, head.summary);
    println!("  Files: {}", total_files);
    println!("  Chunks: {}", total_chunks);
    println!("  Locations: {}", total_locations);
    println!("  Errors: {}", errors);
    println!("  Database: {}", database.display());

    Ok(())
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_str().unwrap_or("");
    if name == "." || name == ".." {
        return false;
    }
    name.starts_with('.')
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


