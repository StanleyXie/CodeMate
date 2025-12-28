//! CodeMate CLI
//!
//! Command-line interface for the CodeMate code intelligence engine.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(name = "codemate")]
#[command(author, version, about = "Your intelligent code companion", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a directory
    Index {
        /// Path to index (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Database path
        #[arg(short, long, default_value = ".codemate/index.db")]
        database: PathBuf,

        /// Enable git-aware indexing with commit tracking
        #[arg(long)]
        git: bool,

        /// Maximum commits to index (only with --git)
        #[arg(long, default_value = "100")]
        max_commits: usize,
    },

    /// Search for code
    Search {
        /// Search query
        query: String,

        /// Database path
        #[arg(short, long, default_value = ".codemate/index.db")]
        database: PathBuf,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Minimum similarity threshold
        #[arg(short, long, default_value = "0.5")]
        threshold: f32,
    },

    /// Show index statistics
    Stats {
        /// Database path
        #[arg(short, long, default_value = ".codemate/index.db")]
        database: PathBuf,
    },

    /// Show history of a chunk or file
    History {
        /// File path or content hash to show history for
        target: String,

        /// Database path
        #[arg(short, long, default_value = ".codemate/index.db")]
        database: PathBuf,

        /// Maximum history entries to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Explore code graph relationships
    Graph {
        #[command(subcommand)]
        subcommand: GraphSubcommand,

        /// Database path
        #[arg(short, long, default_value = ".codemate/index.db")]
        database: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum GraphSubcommand {
    /// Find callers of a function or method
    Callers {
        /// Symbol name to find callers for
        symbol: String,
    },
    /// Find dependencies of a file
    Deps {
        /// File path to find dependencies for
        file_path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("codemate=debug")
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("codemate=info")
            .init();
    }

    match cli.command {
        Commands::Index { path, database, git, max_commits } => {
            commands::index::run(path, database, git, max_commits).await?;
        }
        Commands::Search {
            query,
            database,
            limit,
            threshold,
        } => {
            commands::search::run(query, database, limit, threshold).await?;
        }
        Commands::Stats { database } => {
            commands::stats::run(database).await?;
        }
        Commands::History { target, database, limit } => {
            commands::history::run(target, database, limit).await?;
        }
        Commands::Graph { subcommand, database } => {
            match subcommand {
                GraphSubcommand::Callers { symbol } => {
                    commands::graph::run_callers(symbol, database).await?;
                }
                GraphSubcommand::Deps { file_path } => {
                    commands::graph::run_deps(file_path, database).await?;
                }
            }
        }
    }

    Ok(())
}

