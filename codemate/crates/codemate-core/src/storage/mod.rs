//! Storage abstraction layer.
//!
//! This module provides trait-based storage abstractions that can be
//! implemented by different backends (SQLite, Qdrant, etc.).

mod traits;
mod sqlite;

pub use traits::{ChunkStore, VectorStore, LocationStore, Embedding, SimilarityResult};
pub use sqlite::SqliteStorage;
