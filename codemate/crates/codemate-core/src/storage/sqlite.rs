//! SQLite storage backend implementation.

use crate::chunk::{Chunk, ChunkKind, ChunkLocation, Edge, EdgeKind, Language, Module, ProjectType};
use crate::content_hash::ContentHash;
use crate::error::Result;
use crate::storage::traits::{
    ChunkStore, Embedding, GraphStore, LocationStore, ModuleStore, QueryStore, SimilarityResult, VectorStore,
};
use crate::query::SearchQuery;
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

    /// Set foreign key constraint check status.
    pub fn set_foreign_keys(&self, enabled: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let pragma = if enabled { "PRAGMA foreign_keys = ON;" } else { "PRAGMA foreign_keys = OFF;" };
        conn.execute_batch(pragma)?;
        Ok(())
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
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(
            r#"
            -- Modules table for project/crate/package detection
            CREATE TABLE IF NOT EXISTS modules (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                path            TEXT NOT NULL,
                language        TEXT NOT NULL,
                project_type    TEXT NOT NULL,
                parent_id       TEXT,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY(parent_id) REFERENCES modules(id)
            );

            CREATE INDEX IF NOT EXISTS idx_modules_path ON modules(path);
            CREATE INDEX IF NOT EXISTS idx_modules_parent ON modules(parent_id);

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
                line_start      INTEGER NOT NULL DEFAULT 0,
                line_end        INTEGER NOT NULL DEFAULT 0,
                line_count      INTEGER NOT NULL,
                module_id       TEXT,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY(module_id) REFERENCES modules(id)
            );
            
            CREATE INDEX IF NOT EXISTS idx_chunks_symbol ON chunks(symbol_name);
            CREATE INDEX IF NOT EXISTS idx_chunks_kind ON chunks(chunk_kind, language);
            CREATE INDEX IF NOT EXISTS idx_chunks_module ON chunks(module_id);

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

            -- Edges table for call graph and imports
            CREATE TABLE IF NOT EXISTS edges (
                source_hash     TEXT NOT NULL,
                target_query    TEXT NOT NULL,
                edge_kind       TEXT NOT NULL,
                line_number     INTEGER,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY(source_hash) REFERENCES chunks(content_hash)
            );

            CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_hash);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_query);

            -- FTS5 table for full-text search
            CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
                content_hash UNINDEXED,
                symbol_name,
                docstring,
                content,
                tokenize='unicode61'
            );

            -- Module edges view (aggregated cross-module dependencies)
            CREATE VIEW IF NOT EXISTS module_edges AS
            SELECT 
                src_chunk.module_id AS source_module,
                m2.id AS target_module,
                COUNT(*) AS edge_count
            FROM edges e
            JOIN chunks src_chunk ON e.source_hash = src_chunk.content_hash
            LEFT JOIN chunks tgt_chunk ON (e.target_query = tgt_chunk.symbol_name OR e.target_query LIKE tgt_chunk.symbol_name || '::%')
            LEFT JOIN modules m2 ON tgt_chunk.module_id = m2.id
            WHERE src_chunk.module_id IS NOT NULL 
              AND m2.id IS NOT NULL
              AND src_chunk.module_id != m2.id
            GROUP BY src_chunk.module_id, m2.id;
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
            (content_hash, content, language, chunk_kind, symbol_name, signature, docstring, byte_size, line_start, line_end, line_count, module_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                chunk.content_hash.to_hex(),
                chunk.content,
                chunk.language.as_str(),
                format!("{:?}", chunk.kind).to_lowercase(),
                chunk.symbol_name,
                chunk.signature,
                chunk.docstring,
                chunk.byte_size as i64,
                chunk.line_start as i64,
                chunk.line_end as i64,
                chunk.line_count as i64,
                chunk.module_id,
            ],
        )?;

        // Update FTS5 index
        conn.execute(
            r#"
            INSERT OR REPLACE INTO chunks_fts (content_hash, symbol_name, docstring, content)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                chunk.content_hash.to_hex(),
                chunk.symbol_name,
                chunk.docstring,
                chunk.content,
            ],
        )?;

        Ok(chunk.content_hash.clone())
    }

    async fn get(&self, hash: &ContentHash) -> Result<Option<Chunk>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT content_hash, content, language, chunk_kind, symbol_name, signature, docstring, byte_size, line_start, line_end, line_count, module_id
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
            let line_start: usize = row.get(8)?;
            let line_end: usize = row.get(9)?;
            let line_count: usize = row.get(10)?;
            let module_id: Option<String> = row.get(11)?;

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
                line_start,
                line_end,
                line_count,
                module_id,
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

    async fn find_by_symbol(&self, symbol_name: &str) -> Result<Vec<Chunk>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT content_hash, content, language, chunk_kind, symbol_name, signature, docstring, module_id FROM chunks WHERE symbol_name = ?1"
        )?;

        let chunks = stmt.query_map(params![symbol_name], |row| {
            let hash_str: String = row.get(0)?;
            let content: String = row.get(1)?;
            let lang_str: String = row.get(2)?;
            let kind_str: String = row.get(3)?;
            let symbol_name: Option<String> = row.get(4)?;
            let signature: Option<String> = row.get(5)?;
            let docstring: Option<String> = row.get(6)?;
            let module_id: Option<String> = row.get(7)?;

            let line_count = content.lines().count();

            Ok(Chunk {
                content_hash: ContentHash::from_hex(&hash_str).unwrap(),
                content,
                language: Language::from_str(&lang_str),
                kind: serde_json::from_str(&format!("\"{}\"", kind_str)).unwrap_or(ChunkKind::Block),
                symbol_name,
                signature,
                docstring,
                byte_size: 0,
                line_start: 0,
                line_end: 0,
                line_count,
                module_id,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(chunks)
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

#[async_trait]
impl GraphStore for SqliteStorage {
    async fn add_edge(&self, edge: &Edge) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO edges (source_hash, target_query, edge_kind, line_number)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                edge.source_hash.to_hex(),
                edge.target_query,
                edge.kind.as_str(),
                edge.line_number.map(|l| l as i64),
            ],
        )?;
        Ok(())
    }

    async fn add_edges(&self, edges: &[Edge]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO edges (source_hash, target_query, edge_kind, line_number) VALUES (?1, ?2, ?3, ?4)"
            )?;
            for edge in edges {
                stmt.execute(params![
                    edge.source_hash.to_hex(),
                    edge.target_query,
                    edge.kind.as_str(),
                    edge.line_number.map(|l| l as i64),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    async fn get_outgoing_edges(&self, source_hash: &ContentHash) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT source_hash, target_query, edge_kind, line_number FROM edges WHERE source_hash = ?1"
        )?;

        let edges = stmt.query_map(params![source_hash.to_hex()], |row| {
            let hash_str: String = row.get(0)?;
            let target_query: String = row.get(1)?;
            let kind_str: String = row.get(2)?;
            let line_number: Option<i64> = row.get(3)?;

            let kind = match kind_str.as_str() {
                "calls" => EdgeKind::Calls,
                "imports" => EdgeKind::Imports,
                _ => EdgeKind::References,
            };

            Ok(Edge {
                source_hash: ContentHash::from_hex(&hash_str).unwrap(),
                target_query,
                kind,
                line_number: line_number.map(|l| l as usize),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(edges)
    }

    async fn get_incoming_edges(&self, target_query: &str) -> Result<Vec<Edge>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT source_hash, target_query, edge_kind, line_number FROM edges WHERE target_query = ?1"
        )?;

        let edges = stmt.query_map(params![target_query], |row| {
            let hash_str: String = row.get(0)?;
            let target_query: String = row.get(1)?;
            let kind_str: String = row.get(2)?;
            let line_number: Option<i64> = row.get(3)?;

            let kind = match kind_str.as_str() {
                "calls" => EdgeKind::Calls,
                "imports" => EdgeKind::Imports,
                _ => EdgeKind::References,
            };

            Ok(Edge {
                source_hash: ContentHash::from_hex(&hash_str).unwrap(),
                target_query,
                kind,
                line_number: line_number.map(|l| l as usize),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(edges)
    }

    async fn get_roots(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT symbol_name FROM chunks 
             WHERE symbol_name IS NOT NULL 
             AND symbol_name NOT IN (SELECT target_query FROM edges)"
        )?;

        let roots = stmt.query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(roots)
    }
}

#[async_trait]
impl QueryStore for SqliteStorage {
    async fn query(
        &self,
        query: &SearchQuery,
        embedding: &Embedding,
    ) -> Result<Vec<SimilarityResult>> {
        let conn = self.conn.lock().unwrap();

        // 1. Get filtered set of content hashes based on metadata
        let mut filter_hashes: Option<std::collections::HashSet<String>> = None;

        if query.author.is_some() || query.lang.is_some() || query.after.is_some() || query.before.is_some() || query.file_pattern.is_some() {
            let mut sql = "SELECT DISTINCT c.content_hash FROM chunks c LEFT JOIN locations l ON c.content_hash = l.content_hash WHERE 1=1".to_string();
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(author) = &query.author {
                sql.push_str(" AND (l.author LIKE ? OR l.author = ?)");
                params_vec.push(Box::new(format!("%{}%", author)));
                params_vec.push(Box::new(author.clone()));
            }

            if let Some(lang) = &query.lang {
                sql.push_str(" AND c.language = ?");
                params_vec.push(Box::new(lang.as_str().to_string()));
            }

            if let Some(after) = &query.after {
                sql.push_str(" AND l.timestamp >= ?");
                params_vec.push(Box::new(after.to_rfc3339()));
            }

            if let Some(before) = &query.before {
                sql.push_str(" AND l.timestamp <= ?");
                params_vec.push(Box::new(before.to_rfc3339()));
            }

            if let Some(pattern) = &query.file_pattern {
                sql.push_str(" AND l.file_path LIKE ?");
                params_vec.push(Box::new(format!("%{}%", pattern)));
            }

            let mut stmt = conn.prepare(&sql)?;
            let hashes_iter = stmt.query_map(rusqlite::params_from_iter(params_vec.iter()), |row| {
                row.get::<_, String>(0)
            })?;

            let mut hashes = std::collections::HashSet::new();
            for hash in hashes_iter {
                hashes.insert(hash?);
            }
            filter_hashes = Some(hashes);
        }

        // 2. Perform Vector Search (Filter by metadata hashes if present)
        let mut vector_stmt = conn.prepare("SELECT content_hash, vector FROM embeddings")?;
        let vector_results: Vec<(String, f32)> = vector_stmt
            .query_map([], |row| {
                let hash_str: String = row.get(0)?;
                let vector_bytes: Vec<u8> = row.get(1)?;
                
                let other_vector: Vec<f32> = vector_bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();

                let similarity = embedding.cosine_similarity(&Embedding {
                    vector: other_vector,
                    model_id: String::new(),
                    dimensions: embedding.dimensions,
                });

                Ok((hash_str, similarity))
            })?
            .filter_map(|r| r.ok())
            .filter(|(hash, _)| {
                if let Some(hashes) = &filter_hashes {
                    hashes.contains(hash)
                } else {
                    true
                }
            })
            .collect();

        // 3. Perform FTS5 Search
        let mut lexical_results = Vec::new();
        if !query.raw_query.is_empty() {
            let mut fts_stmt = conn.prepare(
                "SELECT content_hash, rank FROM chunks_fts WHERE chunks_fts MATCH ? ORDER BY rank LIMIT 100"
            )?;
            let fts_iter = fts_stmt.query_map(params![query.raw_query], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })?;

            for res in fts_iter {
                if let Ok((hash, rank)) = res {
                    if filter_hashes.as_ref().map_or(true, |h| h.contains(&hash)) {
                        lexical_results.push((hash, rank));
                    }
                }
            }
        }

        // 4. Reciprocal Rank Fusion (RRF)
        let mut rrf_scores: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        let k = 60.0;

        // Rank Vector Results
        let mut vector_sorted = vector_results;
        vector_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        for (i, (hash, _)) in vector_sorted.iter().enumerate() {
            let score = 1.0 / (k + (i + 1) as f32);
            *rrf_scores.entry(hash.clone()).or_insert(0.0) += score;
        }

        // Rank Lexical Results (FTS5 rank is smaller -> better match)
        for (i, (hash, _)) in lexical_results.iter().enumerate() {
            let score = 1.0 / (k + (i + 1) as f32);
            *rrf_scores.entry(hash.clone()).or_insert(0.0) += score;
        }

        let mut final_results: Vec<SimilarityResult> = rrf_scores
            .into_iter()
            .map(|(hash, score)| {
                SimilarityResult {
                    content_hash: crate::ContentHash::from_hex(&hash).unwrap(),
                    similarity: score, // This is now an RRF score, not cosine similarity
                }
            })
            .collect();

        final_results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
        final_results.truncate(query.limit);

        Ok(final_results)
    }
}

#[async_trait]
impl ModuleStore for SqliteStorage {
    async fn put_module(&self, module: &Module) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO modules (id, name, path, language, project_type, parent_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                language = excluded.language,
                project_type = excluded.project_type,
                parent_id = excluded.parent_id
            "#,
            params![
                module.id,
                module.name,
                module.path,
                module.language.as_str(),
                module.project_type.as_str(),
                module.parent_id
            ],
        )?;
        Ok(())
    }

    async fn get_module(&self, id: &str) -> Result<Option<Module>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, project_type, parent_id FROM modules WHERE id = ?1"
        )?;

        let result = stmt.query_row(params![id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let path: String = row.get(2)?;
            let lang_str: String = row.get(3)?;
            let type_str: String = row.get(4)?;
            let parent_id: Option<String> = row.get(5)?;

            Ok(Module {
                id,
                name,
                path,
                language: Language::from_str(&lang_str),
                project_type: ProjectType::from_str(&type_str),
                parent_id,
            })
        });

        match result {
            Ok(module) => Ok(Some(module)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn get_all_modules(&self) -> Result<Vec<Module>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, project_type, parent_id FROM modules"
        )?;

        let modules = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let path: String = row.get(2)?;
            let lang_str: String = row.get(3)?;
            let type_str: String = row.get(4)?;
            let parent_id: Option<String> = row.get(5)?;

            Ok(Module {
                id,
                name,
                path,
                language: Language::from_str(&lang_str),
                project_type: ProjectType::from_str(&type_str),
                parent_id,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(modules)
    }

    async fn get_child_modules(&self, parent_id: &str) -> Result<Vec<Module>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, project_type, parent_id FROM modules WHERE parent_id = ?1"
        )?;

        let modules = stmt.query_map(params![parent_id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let path: String = row.get(2)?;
            let lang_str: String = row.get(3)?;
            let type_str: String = row.get(4)?;
            let parent_id: Option<String> = row.get(5)?;

            Ok(Module {
                id,
                name,
                path,
                language: Language::from_str(&lang_str),
                project_type: ProjectType::from_str(&type_str),
                parent_id,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(modules)
    }

    async fn get_module_dependencies(&self, module_id: &str) -> Result<Vec<(String, usize)>> {
        let deps = self.get_unified_graph("module", Some(vec![module_id.to_string()]), false).await?;
        if let Some((_, dependencies)) = deps.into_iter().next() {
            Ok(dependencies.into_iter().map(|(id, count, _)| (id, count)).collect())
        } else {
            Ok(vec![])
        }
    }

    async fn get_unified_graph(&self, level: &str, filter_ids: Option<Vec<String>>, include_edges: bool) -> anyhow::Result<Vec<(Module, Vec<(String, usize, Option<Vec<crate::service::models::ModuleEdgeDetail>>)>)>> {
        // 1. Get modules to process (this might involve awaits)
        let mut target_modules = Vec::new();
        if let Some(ids) = &filter_ids {
            for id in ids {
                if let Some(m) = self.get_module(id).await? {
                    target_modules.push(m);
                }
            }
        } else {
            target_modules = self.get_all_modules().await?;
        }

        if level == "crate" {
            target_modules.retain(|m| m.project_type.as_str() != "directory");
        }

        let mut result = Vec::new();

        for module in target_modules {
            let mut dependencies = Vec::new();

            let deps_raw: Vec<(String, usize)> = {
                let conn = self.conn.lock().unwrap();
                let dep_query = if level == "crate" {
                    r#"
                    WITH RECURSIVE crate_map(mod_id, crate_id, crate_name) AS (
                        SELECT id, id, name FROM modules WHERE project_type != 'directory'
                        UNION ALL
                        SELECT m.id, cm.crate_id, cm.crate_name
                        FROM modules m
                        JOIN crate_map cm ON m.parent_id = cm.mod_id
                        WHERE m.project_type = 'directory'
                    )
                    SELECT 
                        target_id,
                        COUNT(*) as edge_count
                    FROM (
                        -- 1. Direct symbol matching via chunks
                        SELECT cm2.crate_id as target_id
                        FROM edges e
                        JOIN chunks c1 ON e.source_hash = c1.content_hash
                        JOIN crate_map cm1 ON c1.module_id = cm1.mod_id
                        JOIN chunks c2 ON e.target_query = c2.symbol_name
                        JOIN crate_map cm2 ON c2.module_id = cm2.mod_id
                        WHERE cm1.crate_id = ?1 AND cm2.crate_id != ?1

                        UNION ALL

                        -- 2. Symbol prefix matching via module names
                        SELECT m2.id as target_id
                        FROM edges e
                        JOIN chunks c1 ON e.source_hash = c1.content_hash
                        JOIN crate_map cm1 ON c1.module_id = cm1.mod_id
                        JOIN modules m2 ON m2.project_type != 'directory'
                        WHERE cm1.crate_id = ?1 
                          AND m2.id != cm1.crate_id
                          AND (
                              e.target_query LIKE REPLACE(m2.name, '-', '_') || '::%'
                              OR e.target_query = REPLACE(m2.name, '-', '_')
                              OR e.target_query LIKE m2.name || '::%'
                              OR e.target_query = m2.name
                          )
                          -- Important: only count prefix matches if they didn't match exactly via chunks
                          AND NOT EXISTS (SELECT 1 FROM chunks c3 WHERE c3.symbol_name = e.target_query)
                    )
                    GROUP BY target_id
                    HAVING edge_count > 0
                    ORDER BY edge_count DESC
                    "#
                } else {
                    r#"
                    SELECT target_module, edge_count 
                    FROM module_edges 
                    WHERE source_module = ?1
                    ORDER BY edge_count DESC
                    "#
                };

                let mut stmt = conn.prepare(dep_query)?;
                let rows = stmt.query_map(params![module.id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
                })?;
                
                let mut results = Vec::new();
                for row in rows {
                    if let Ok(res) = row {
                        results.push(res);
                    }
                }
                results
            };

            for (target_id, count) in deps_raw {
                let edges = if include_edges {
                    let conn = self.conn.lock().unwrap();
                    let mut edge_stmt = conn.prepare(
                        r#"
                        WITH RECURSIVE crate_map(mod_id, crate_id) AS (
                            SELECT id, id FROM modules
                            UNION ALL
                            SELECT m.id, cm.crate_id
                            FROM modules m
                            JOIN crate_map cm ON m.parent_id = cm.mod_id
                        )
                        SELECT c1.symbol_name, e.target_query, e.line_number, e.edge_kind
                        FROM edges e
                        JOIN chunks c1 ON e.source_hash = c1.content_hash
                        JOIN crate_map cm1 ON c1.module_id = cm1.mod_id
                        -- Target matching: either by symbol_name or by ID prefix
                        JOIN chunks c2 ON (e.target_query = c2.symbol_name OR e.target_query LIKE c2.symbol_name || '::%')
                        JOIN crate_map cm2 ON c2.module_id = cm2.mod_id
                        WHERE cm1.crate_id = ?1 AND cm2.crate_id = ?2
                        "#
                    )?;
                    let edges_rows = edge_stmt.query_map(params![module.id, target_id], |row| {
                        let kind_str: String = row.get(3)?;
                        let kind = match kind_str.as_str() {
                            "calls" | "Calls" => EdgeKind::Calls,
                            "imports" | "Imports" => EdgeKind::Imports,
                            _ => EdgeKind::References,
                        };
                        Ok(crate::service::models::ModuleEdgeDetail {
                            source_symbol: row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "unknown".to_string()),
                            target_symbol: row.get(1)?,
                            line_number: row.get::<_, Option<i64>>(2)?.map(|l| l as usize),
                            kind,
                        })
                    })?;

                    let mut e_list = Vec::new();
                    for e_row in edges_rows {
                        if let Ok(e) = e_row {
                            e_list.push(e);
                        }
                    }
                    Some(e_list)
                } else {
                    None
                };

                dependencies.push((target_id, count, edges));
            }

            result.push((module, dependencies));
        }

        Ok(result)
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

    #[tokio::test]
    async fn test_graph_store() {
        let storage = SqliteStorage::in_memory().unwrap();
        let hash1 = ContentHash::from_content(b"test1");
        
        // Insert chunk first to satisfy foreign key constraint
        let chunk = Chunk::new("test1".to_string(), Language::Rust, ChunkKind::Function, None);
        ChunkStore::put(&storage, &chunk).await.unwrap();
        
        // Add edges
        let edge1 = Edge::new(hash1.clone(), "FuncA".to_string(), EdgeKind::Calls).with_line(10);
        let edge2 = Edge::new(hash1.clone(), "FuncB".to_string(), EdgeKind::Calls).with_line(20);
        
        storage.add_edges(&[edge1, edge2]).await.unwrap();
        
        // Verify outgoing
        let outgoing = storage.get_outgoing_edges(&hash1).await.unwrap();
        assert_eq!(outgoing.len(), 2);
        assert!(outgoing.iter().any(|e| e.target_query == "FuncA" && e.line_number == Some(10)));
        assert!(outgoing.iter().any(|e| e.target_query == "FuncB" && e.line_number == Some(20)));
        
        // Verify incoming
        let incoming = storage.get_incoming_edges("FuncA").await.unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].source_hash, hash1);
    }
}
