//! Test utilities for CodeMate.
//!
//! Provides reusable test helpers, fixtures, and mocks.

use crate::chunk::{Chunk, ChunkKind, Language};
use crate::content_hash::ContentHash;
use crate::storage::{Embedding, SqliteStorage};

/// Test fixture for creating sample chunks.
pub struct TestFixtures;

impl TestFixtures {
    /// Create a sample Rust function chunk.
    pub fn rust_function(name: &str, body: &str) -> Chunk {
        let content = format!(
            r#"fn {}() {{
    {}
}}"#,
            name, body
        );
        Chunk::new(content, Language::Rust, ChunkKind::Function, Some(name.to_string()))
    }

    /// Create a sample Rust struct chunk.
    pub fn rust_struct(name: &str, fields: &[(&str, &str)]) -> Chunk {
        let field_strs: Vec<String> = fields
            .iter()
            .map(|(n, t)| format!("    {}: {},", n, t))
            .collect();
        let content = format!(
            r#"pub struct {} {{
{}
}}"#,
            name,
            field_strs.join("\n")
        );
        Chunk::new(content, Language::Rust, ChunkKind::Struct, Some(name.to_string()))
    }

    /// Create a sample Python function chunk.
    pub fn python_function(name: &str, body: &str) -> Chunk {
        let content = format!(
            r#"def {}():
    {}"#,
            name, body
        );
        Chunk::new(content, Language::Python, ChunkKind::Function, Some(name.to_string()))
    }

    /// Create a sample TypeScript function chunk.
    pub fn typescript_function(name: &str, body: &str) -> Chunk {
        let content = format!(
            r#"function {}() {{
    {}
}}"#,
            name, body
        );
        Chunk::new(content, Language::TypeScript, ChunkKind::Function, Some(name.to_string()))
    }

    /// Create a sample embedding with random-ish values.
    pub fn embedding(dimensions: usize, model_id: &str) -> Embedding {
        let vector: Vec<f32> = (0..dimensions)
            .map(|i| i as f32 / dimensions as f32)
            .collect();
        Embedding::new(vector, model_id.to_string())
    }

    /// Create similar embeddings (for testing similarity search).
    pub fn similar_embeddings(base: &[f32], variance: f32, model_id: &str) -> Embedding {
        let vector: Vec<f32> = base
            .iter()
            .enumerate()
            .map(|(i, v)| v + (i as f32 % 2.0) * variance - variance / 2.0)
            .collect();
        Embedding::new(vector, model_id.to_string())
    }
}

/// Create a temporary in-memory storage for testing.
pub fn test_storage() -> SqliteStorage {
    SqliteStorage::in_memory().expect("Failed to create in-memory storage")
}

/// Test helper for creating content hashes from strings.
pub fn hash(s: &str) -> ContentHash {
    ContentHash::from_content(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_function_fixture() {
        let chunk = TestFixtures::rust_function("hello", "println!(\"Hello!\");");
        assert_eq!(chunk.symbol_name, Some("hello".to_string()));
        assert_eq!(chunk.language, Language::Rust);
        assert_eq!(chunk.kind, ChunkKind::Function);
        assert!(chunk.content.contains("fn hello()"));
    }

    #[test]
    fn test_rust_struct_fixture() {
        let chunk = TestFixtures::rust_struct("User", &[("name", "String"), ("age", "u32")]);
        assert_eq!(chunk.symbol_name, Some("User".to_string()));
        assert!(chunk.content.contains("name: String"));
        assert!(chunk.content.contains("age: u32"));
    }

    #[test]
    fn test_embedding_fixture() {
        let emb = TestFixtures::embedding(384, "test-model");
        assert_eq!(emb.dimensions, 384);
        assert_eq!(emb.model_id, "test-model");
    }
}
