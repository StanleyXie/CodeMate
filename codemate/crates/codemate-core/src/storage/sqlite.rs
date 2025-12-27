//! SQLite storage backend implementation.

use crate::chunk::{Chunk, ChunkKind, ChunkLocation, Language};
use crate::content_hash::ContentHash;
use crate::error::Result;
use crate::storage::traits::{ChunkStore, Embedding, LocationStore, SimilarityResult, VectorStore};
use async_trait::async_trait;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

/// SQLite-based storage implementation.
pub struct SqliteStorage {
    conn: Mutex<Connection>,
}

impl SqliteStorage {
    /// Create a new SQLite storage at the given path.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Create an in-memory SQLite storage (for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let storage = Self {
            conn: Mutex::new(conn),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    /// Initialize the database schema.
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            -- Chunks table
            CREATE TABLE IF NOT EXISTS chunks (
                content_hash    TEXT PRIMARY KEY,
                content         TEXT NOT NULL,
                language        TEXT NOT NULL,
                chunk_kind      TEXT NOT NULL,
                symbol_name     TEXT,
                signature       TEXT,
                docstring       TEXT,
                byte_size       INTEGER NOT NULL,
                line_count      INTEGER NOT NULL,
                created_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );
            
            CREATE INDEX IF NOT EXISTS idx_chunks_symbol ON chunks(symbol_name);
            CREATE INDEX IF NOT EXISTS idx_chunks_kind ON chunks(chunk_kind, language);

            -- Embeddings table
            CREATE TABLE IF NOT EXISTS embeddings (
                content_hash    TEXT PRIMARY KEY,
                model_id        TEXT NOT NULL,
                vector          BLOB NOT NULL,
                dimensions      INTEGER NOT NULL,
                created_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Locations table (for git-aware tracking)
            CREATE TABLE IF NOT EXISTS locations (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                content_hash    TEXT NOT NULL,
                file_path       TEXT NOT NULL,
                byte_start      INTEGER NOT NULL,
                byte_end        INTEGER NOT NULL,
                line_start      INTEGER NOT NULL,
                line_end        INTEGER NOT NULL,
                commit_hash     TEXT,
                author          TEXT,
                timestamp       TEXT,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(content_hash, file_path, commit_hash)
            );
            
            CREATE INDEX IF NOT EXISTS idx_locations_hash ON locations(content_hash);
            CREATE INDEX IF NOT EXISTS idx_locations_commit ON locations(commit_hash);
            CREATE INDEX IF NOT EXISTS idx_locations_file ON locations(file_path);
            "#,
        )?;
        Ok(())
    }
}

#[async_trait]
impl ChunkStore for SqliteStorage {
    async fn put(&self, chunk: &Chunk) -> Result<ContentHash> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO chunks 
            (content_hash, content, language, chunk_kind, symbol_name, signature, docstring, byte_size, line_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                chunk.content_hash.to_hex(),
                chunk.content,
                chunk.language.as_str(),
                format!("{:?}", chunk.kind).to_lowercase(),
                chunk.symbol_name,
                chunk.signature,
                chunk.docstring,
                chunk.byte_size,
                chunk.line_count,
            ],
        )?;
        Ok(chunk.content_hash.clone())
    }

    async fn get(&self, hash: &ContentHash) -> Result<Option<Chunk>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT content_hash, content, language, chunk_kind, symbol_name, signature, docstring, byte_size, line_count
            FROM chunks WHERE content_hash = ?1
            "#,
        )?;

        let result = stmt.query_row(params![hash.to_hex()], |row| {
            let hash_str: String = row.get(0)?;
            let content: String = row.get(1)?;
            let lang_str: String = row.get(2)?;
            let kind_str: String = row.get(3)?;
            let symbol_name: Option<String> = row.get(4)?;
            let signature: Option<String> = row.get(5)?;
            let docstring: Option<String> = row.get(6)?;
            let byte_size: usize = row.get(7)?;
            let line_count: usize = row.get(8)?;

            let language = Language::from_extension(&lang_str);
            let kind = match kind_str.as_str() {
                "function" => ChunkKind::Function,
                "class" => ChunkKind::Class,
                "struct" => ChunkKind::Struct,
                "trait" => ChunkKind::Trait,
                "enum" => ChunkKind::Enum,
                "module" => ChunkKind::Module,
                "impl" => ChunkKind::Impl,
                _ => ChunkKind::Block,
            };

            Ok(Chunk {
                content_hash: ContentHash::from_hex(&hash_str).unwrap(),
                content,
                language,
                kind,
                symbol_name,
                signature,
                docstring,
                byte_size,
                line_count,
            })
        });

        match result {
            Ok(chunk) => Ok(Some(chunk)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn exists(&self, hash: &ContentHash) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks WHERE content_hash = ?1",
            params![hash.to_hex()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    async fn get_many(&self, hashes: &[ContentHash]) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        for hash in hashes {
            if let Some(chunk) = ChunkStore::get(self, hash).await? {
                chunks.push(chunk);
            }
        }
        Ok(chunks)
    }

    async fn count(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

#[async_trait]
impl VectorStore for SqliteStorage {
    async fn put(&self, hash: &ContentHash, embedding: &Embedding) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        // Serialize vector to bytes (f32 little-endian)
        let vector_bytes: Vec<u8> = embedding
            .vector
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();

        conn.execute(
            r#"
            INSERT OR REPLACE INTO embeddings 
            (content_hash, model_id, vector, dimensions)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                hash.to_hex(),
                embedding.model_id,
                vector_bytes,
                embedding.dimensions,
            ],
        )?;
        Ok(())
    }

    async fn get(&self, hash: &ContentHash) -> Result<Option<Embedding>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT model_id, vector, dimensions FROM embeddings WHERE content_hash = ?1",
        )?;

        let result = stmt.query_row(params![hash.to_hex()], |row| {
            let model_id: String = row.get(0)?;
            let vector_bytes: Vec<u8> = row.get(1)?;
            let dimensions: usize = row.get(2)?;

            // Deserialize vector from bytes
            let vector: Vec<f32> = vector_bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();

            Ok(Embedding {
                vector,
                model_id,
                dimensions,
            })
        });

        match result {
            Ok(embedding) => Ok(Some(embedding)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn search(
        &self,
        query: &Embedding,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<SimilarityResult>> {
        // For MVP, we do a brute-force search
        // In production, this would use sqlite-vec or Qdrant
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT content_hash, vector, dimensions FROM embeddings")?;

        let mut results: Vec<SimilarityResult> = stmt
            .query_map([], |row| {
                let hash_str: String = row.get(0)?;
                let vector_bytes: Vec<u8> = row.get(1)?;
                let dimensions: usize = row.get(2)?;

                let vector: Vec<f32> = vector_bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();

                Ok((hash_str, Embedding {
                    vector,
                    model_id: String::new(),
                    dimensions,
                }))
            })?
            .filter_map(|r| r.ok())
            .map(|(hash_str, embedding)| {
                let similarity = query.cosine_similarity(&embedding);
                SimilarityResult {
                    content_hash: ContentHash::from_hex(&hash_str).unwrap(),
                    similarity,
                }
            })
            .filter(|r| r.similarity >= threshold)
            .collect();

        // Sort by similarity descending
        results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        results.truncate(limit);

        Ok(results)
    }

    async fn put_many(&self, items: &[(ContentHash, Embedding)]) -> Result<()> {
        for (hash, embedding) in items {
            VectorStore::put(self, hash, embedding).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl LocationStore for SqliteStorage {
    async fn put_location(&self, location: &ChunkLocation) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT OR REPLACE INTO locations 
            (content_hash, file_path, byte_start, byte_end, line_start, line_end, commit_hash, author, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                location.content_hash.to_hex(),
                location.file_path,
                location.byte_start as i64,
                location.byte_end as i64,
                location.line_start as i64,
                location.line_end as i64,
                location.commit_hash,
                location.author,
                location.timestamp,
            ],
        )?;
        Ok(())
    }

    async fn get_locations(&self, content_hash: &ContentHash) -> Result<Vec<ChunkLocation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT content_hash, file_path, byte_start, byte_end, line_start, line_end, commit_hash, author, timestamp FROM locations WHERE content_hash = ?1 ORDER BY created_at DESC",
        )?;

        let locations = stmt
            .query_map(params![content_hash.to_hex()], |row| {
                Ok(ChunkLocation {
                    content_hash: ContentHash::from_hex(&row.get::<_, String>(0)?).unwrap(),
                    file_path: row.get(1)?,
                    byte_start: row.get::<_, i64>(2)? as usize,
                    byte_end: row.get::<_, i64>(3)? as usize,
                    line_start: row.get::<_, i64>(4)? as usize,
                    line_end: row.get::<_, i64>(5)? as usize,
                    commit_hash: row.get(6)?,
                    author: row.get(7)?,
                    timestamp: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(locations)
    }

    async fn get_locations_at_commit(&self, commit_hash: &str) -> Result<Vec<ChunkLocation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT content_hash, file_path, byte_start, byte_end, line_start, line_end, commit_hash, author, timestamp FROM locations WHERE commit_hash = ?1 ORDER BY file_path",
        )?;

        let locations = stmt
            .query_map(params![commit_hash], |row| {
                Ok(ChunkLocation {
                    content_hash: ContentHash::from_hex(&row.get::<_, String>(0)?).unwrap(),
                    file_path: row.get(1)?,
                    byte_start: row.get::<_, i64>(2)? as usize,
                    byte_end: row.get::<_, i64>(3)? as usize,
                    line_start: row.get::<_, i64>(4)? as usize,
                    line_end: row.get::<_, i64>(5)? as usize,
                    commit_hash: row.get(6)?,
                    author: row.get(7)?,
                    timestamp: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(locations)
    }

    async fn get_locations_in_file(&self, file_path: &str) -> Result<Vec<ChunkLocation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT content_hash, file_path, byte_start, byte_end, line_start, line_end, commit_hash, author, timestamp FROM locations WHERE file_path = ?1 ORDER BY line_start",
        )?;

        let locations = stmt
            .query_map(params![file_path], |row| {
                Ok(ChunkLocation {
                    content_hash: ContentHash::from_hex(&row.get::<_, String>(0)?).unwrap(),
                    file_path: row.get(1)?,
                    byte_start: row.get::<_, i64>(2)? as usize,
                    byte_end: row.get::<_, i64>(3)? as usize,
                    line_start: row.get::<_, i64>(4)? as usize,
                    line_end: row.get::<_, i64>(5)? as usize,
                    commit_hash: row.get(6)?,
                    author: row.get(7)?,
                    timestamp: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(locations)
    }

    async fn get_location_history(&self, content_hash: &ContentHash) -> Result<Vec<ChunkLocation>> {
        // Same as get_locations but ordered by timestamp
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT content_hash, file_path, byte_start, byte_end, line_start, line_end, commit_hash, author, timestamp FROM locations WHERE content_hash = ?1 ORDER BY timestamp DESC",
        )?;

        let locations = stmt
            .query_map(params![content_hash.to_hex()], |row| {
                Ok(ChunkLocation {
                    content_hash: ContentHash::from_hex(&row.get::<_, String>(0)?).unwrap(),
                    file_path: row.get(1)?,
                    byte_start: row.get::<_, i64>(2)? as usize,
                    byte_end: row.get::<_, i64>(3)? as usize,
                    line_start: row.get::<_, i64>(4)? as usize,
                    line_end: row.get::<_, i64>(5)? as usize,
                    commit_hash: row.get(6)?,
                    author: row.get(7)?,
                    timestamp: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(locations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chunk_store() {
        let storage = SqliteStorage::in_memory().unwrap();

        let chunk = Chunk::new(
            "fn main() {}".to_string(),
            Language::Rust,
            ChunkKind::Function,
            Some("main".to_string()),
        );

        // Store chunk
        let hash = ChunkStore::put(&storage, &chunk).await.unwrap();
        assert_eq!(hash, chunk.content_hash);

        // Retrieve chunk
        let retrieved = ChunkStore::get(&storage, &hash).await.unwrap().unwrap();
        assert_eq!(retrieved.content, chunk.content);
        assert_eq!(retrieved.symbol_name, chunk.symbol_name);
    }

    #[tokio::test]
    async fn test_vector_store() {
        let storage = SqliteStorage::in_memory().unwrap();

        let hash = ContentHash::from_content(b"test");
        let embedding = Embedding::new(vec![1.0, 0.5, 0.0], "test-model".to_string());

        // Store embedding
        VectorStore::put(&storage, &hash, &embedding).await.unwrap();

        // Retrieve embedding
        let retrieved = VectorStore::get(&storage, &hash).await.unwrap().unwrap();
        assert_eq!(retrieved.vector, embedding.vector);
        assert_eq!(retrieved.model_id, embedding.model_id);
    }


    #[tokio::test]
    async fn test_vector_search() {
        let storage = SqliteStorage::in_memory().unwrap();

        // Store some embeddings
        let hash1 = ContentHash::from_content(b"test1");
        let hash2 = ContentHash::from_content(b"test2");
        let hash3 = ContentHash::from_content(b"test3");

        VectorStore::put(&storage, &hash1, &Embedding::new(vec![1.0, 0.0, 0.0], "test".to_string()))
            .await
            .unwrap();
        VectorStore::put(&storage, &hash2, &Embedding::new(vec![0.9, 0.1, 0.0], "test".to_string()))
            .await
            .unwrap();
        VectorStore::put(&storage, &hash3, &Embedding::new(vec![0.0, 1.0, 0.0], "test".to_string()))
            .await
            .unwrap();

        // Search for similar vectors
        let query = Embedding::new(vec![1.0, 0.0, 0.0], "test".to_string());
        let results = storage.search(&query, 2, 0.8).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].content_hash, hash1);
    }
}
