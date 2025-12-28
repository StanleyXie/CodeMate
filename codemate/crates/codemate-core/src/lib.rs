//! CodeMate Core Library
//!
//! Core types, traits, and storage abstractions for the CodeMate code intelligence engine.

pub mod chunk;
pub mod content_hash;
pub mod error;
pub mod storage;

#[cfg(test)]
pub mod testutils;

pub use chunk::{Chunk, ChunkKind, ChunkLocation, Edge, EdgeKind, Language};
pub use content_hash::ContentHash;
pub use error::{Error, Result};
