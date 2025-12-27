//! Error types for CodeMate.

use thiserror::Error;

/// Result type alias.
pub type Result<T> = std::result::Result<T, Error>;

/// CodeMate error types.
#[derive(Debug, Error)]
pub enum Error {
    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Embedding error
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// Not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Generic error
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}
