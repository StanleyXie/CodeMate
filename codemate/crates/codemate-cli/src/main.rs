//! CodeMate CLI
//!
//! Command-line interface for the CodeMate code intelligence engine.

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
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
        Commands::Index { path, database } => {
            commands::index::run(path, database).await?;
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
    }

    Ok(())
}
