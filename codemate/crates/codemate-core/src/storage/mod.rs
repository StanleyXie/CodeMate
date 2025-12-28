//! Storage abstraction layer.
//!
//! This module provides trait-based storage abstractions that can be
//! implemented by different backends (SQLite, Qdrant, etc.).

mod traits;
mod sqlite;
pub mod utils;

pub use traits::{
    ChunkStore, Embedder, Embedding, GraphStore, LocationStore, QueryStore, SimilarityResult,
    VectorStore,
};
pub use sqlite::SqliteStorage;
