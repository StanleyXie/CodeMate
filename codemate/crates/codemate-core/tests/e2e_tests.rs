//! End-to-end tests for CodeMate.
//!
//! These tests verify the full workflow: parsing → embedding → storage → search.

use codemate_core::chunk::Language;
use codemate_core::storage::{ChunkStore, SqliteStorage, VectorStore};
use codemate_parser::ChunkExtractor;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test the full indexing workflow with Rust files.
#[tokio::test]
async fn test_index_rust_file() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let rust_file = fixtures_dir.join("sample_rust.rs");

    // Create temporary storage
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = SqliteStorage::new(&db_path).unwrap();

    // Extract chunks
    let extractor = ChunkExtractor::new();
    let (chunks, _edges) = extractor.extract_file(&rust_file).unwrap();

    // Verify we extracted functions
    assert!(!chunks.is_empty(), "Should extract at least one chunk");

    // Find authentication function
    let auth_fn = chunks
        .iter()
        .find(|c| c.symbol_name.as_deref() == Some("authenticate_user"));
    assert!(auth_fn.is_some(), "Should find authenticate_user function");

    // Store chunks
    for chunk in &chunks {
        ChunkStore::put(&storage, chunk).await.unwrap();
    }

    // Verify storage
    let count = storage.count().await.unwrap();
    assert_eq!(count, chunks.len(), "All chunks should be stored");
}

/// Test the full indexing workflow with Python files.
#[tokio::test]
async fn test_index_python_file() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let python_file = fixtures_dir.join("sample_python.py");

    // Create temporary storage
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = SqliteStorage::new(&db_path).unwrap();

    // Extract chunks
    let extractor = ChunkExtractor::new();
    let (chunks, _edges) = extractor.extract_file(&python_file).unwrap();

    // Verify we extracted functions
    assert!(!chunks.is_empty(), "Should extract at least one chunk");

    // Verify language detection
    for chunk in &chunks {
        assert_eq!(chunk.language, Language::Python);
    }

    // Store chunks
    for chunk in &chunks {
        ChunkStore::put(&storage, chunk).await.unwrap();
    }

    let count = storage.count().await.unwrap();
    assert!(count > 0, "Should have stored chunks");
}

/// Test vector similarity search.
#[tokio::test]
async fn test_similarity_search() {
    use codemate_core::content_hash::ContentHash;
    use codemate_core::storage::Embedding;

    let storage = SqliteStorage::in_memory().unwrap();

    // Create test embeddings
    let emb_auth = Embedding::new(vec![0.9, 0.1, 0.0, 0.0], "test".to_string());
    let emb_user = Embedding::new(vec![0.8, 0.2, 0.0, 0.0], "test".to_string());
    let emb_db = Embedding::new(vec![0.0, 0.0, 0.9, 0.1], "test".to_string());

    let hash_auth = ContentHash::from_content(b"auth");
    let hash_user = ContentHash::from_content(b"user");
    let hash_db = ContentHash::from_content(b"db");

    VectorStore::put(&storage, &hash_auth, &emb_auth).await.unwrap();
    VectorStore::put(&storage, &hash_user, &emb_user).await.unwrap();
    VectorStore::put(&storage, &hash_db, &emb_db).await.unwrap();

    // Search for authentication-related
    let query = Embedding::new(vec![0.85, 0.15, 0.0, 0.0], "test".to_string());
    let results = storage.search(&query, 2, 0.7).await.unwrap();

    // Should find auth and user, not db
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|r| r.content_hash == hash_auth));
    assert!(results.iter().any(|r| r.content_hash == hash_user));
}

/// Test that storage persists across reopens.
#[tokio::test]
async fn test_storage_persistence() {
    use codemate_core::chunk::{Chunk, ChunkKind};

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create and store a chunk
    let chunk = Chunk::new(
        "fn test() {}".to_string(),
        Language::Rust,
        ChunkKind::Function,
        Some("test".to_string()),
    );
    let hash = chunk.content_hash.clone();

    {
        let storage = SqliteStorage::new(&db_path).unwrap();
        ChunkStore::put(&storage, &chunk).await.unwrap();
    }

    // Reopen and verify
    {
        let storage = SqliteStorage::new(&db_path).unwrap();
        let retrieved = ChunkStore::get(&storage, &hash).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "fn test() {}");
    }
}

/// Test chunk deduplication.
#[tokio::test]
async fn test_chunk_deduplication() {
    use codemate_core::chunk::{Chunk, ChunkKind};

    let storage = SqliteStorage::in_memory().unwrap();

    let chunk = Chunk::new(
        "fn duplicate() {}".to_string(),
        Language::Rust,
        ChunkKind::Function,
        Some("duplicate".to_string()),
    );

    // Store same chunk twice
    ChunkStore::put(&storage, &chunk).await.unwrap();
    ChunkStore::put(&storage, &chunk).await.unwrap();

    // Should only have one entry
    let count = storage.count().await.unwrap();
    assert_eq!(count, 1, "Duplicate chunks should not create new entries");
}
