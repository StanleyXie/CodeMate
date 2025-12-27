# ADR: Pluggable Storage Layer

**Topic**: Storage abstraction for swappable database backends  
**Date**: 2025-12-27  
**Status**: Proposed  
**Deciders**: Architecture Team

---

## Context

CodeMate requires multiple storage capabilities:
1. **Chunk Store**: Content-addressable storage for code chunks
2. **Vector Store**: Embeddings for semantic search
3. **Graph Store**: Relationships between code elements
4. **Metadata Store**: Flexible key-value attributes

The initial design uses SQLite + sqlite-vec for simplicity and portability. However, for production scale or specific deployment scenarios, alternative backends may be preferred:
- **Qdrant**: Purpose-built vector database with better ANN performance at scale
- **PostgreSQL + pgvector**: Enterprise deployments with existing Postgres infrastructure
- **Milvus**: Distributed vector search for multi-node deployments

---

## Decision

**Implement a trait-based storage abstraction layer** that allows swapping backends via configuration.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       APPLICATION LAYER                          │
│  (Indexer, Query Processor, CLI)                                 │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                    STORAGE ABSTRACTION LAYER                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ ChunkStore  │  │ VectorStore │  │ GraphStore  │              │
│  │   trait     │  │   trait     │  │   trait     │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
│         │                │                │                      │
└─────────┼────────────────┼────────────────┼─────────────────────┘
          │                │                │
    ┌─────┴─────┐    ┌─────┴─────┐    ┌─────┴─────┐
    │           │    │           │    │           │
    ▼           ▼    ▼           ▼    ▼           ▼
┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐
│SQLite  │ │Qdrant  │ │sqlite- │ │Qdrant  │ │SQLite  │
│Chunks  │ │Chunks  │ │vec     │ │Vectors │ │Graph   │
└────────┘ └────────┘ └────────┘ └────────┘ └────────┘
```

---

## Specification

### Core Traits

```rust
/// Content-addressable chunk storage
#[async_trait]
pub trait ChunkStore: Send + Sync {
    /// Store a chunk, returns content hash
    async fn put(&self, chunk: &Chunk) -> Result<ContentHash>;
    
    /// Retrieve chunk by hash
    async fn get(&self, hash: &ContentHash) -> Result<Option<Chunk>>;
    
    /// Check if chunk exists
    async fn exists(&self, hash: &ContentHash) -> Result<bool>;
    
    /// Batch retrieval
    async fn get_many(&self, hashes: &[ContentHash]) -> Result<Vec<Chunk>>;
}

/// Vector storage and similarity search
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Store embedding for a content hash
    async fn put(&self, hash: &ContentHash, embedding: &Embedding) -> Result<()>;
    
    /// Retrieve embedding
    async fn get(&self, hash: &ContentHash) -> Result<Option<Embedding>>;
    
    /// Find similar vectors (k-NN)
    async fn search(
        &self, 
        query: &Embedding, 
        limit: usize, 
        threshold: f32
    ) -> Result<Vec<SimilarityResult>>;
    
    /// Batch insert
    async fn put_many(&self, items: &[(ContentHash, Embedding)]) -> Result<()>;
}

/// Graph storage for code relationships
#[async_trait]
pub trait GraphStore: Send + Sync {
    /// Add a node
    async fn put_node(&self, node: &GraphNode) -> Result<()>;
    
    /// Add an edge
    async fn put_edge(&self, edge: &GraphEdge) -> Result<()>;
    
    /// Get outgoing edges from a node
    async fn get_outgoing(&self, node_id: &NodeId, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>>;
    
    /// Get incoming edges to a node
    async fn get_incoming(&self, node_id: &NodeId, edge_type: Option<EdgeType>) -> Result<Vec<GraphEdge>>;
    
    /// Traverse graph (BFS/DFS)
    async fn traverse(
        &self, 
        start: &NodeId, 
        direction: Direction, 
        max_depth: usize
    ) -> Result<Vec<GraphNode>>;
}

/// Metadata storage
#[async_trait]
pub trait MetadataStore: Send + Sync {
    async fn put(&self, hash: &ContentHash, key: &str, value: Value) -> Result<()>;
    async fn get(&self, hash: &ContentHash, key: &str) -> Result<Option<Value>>;
    async fn get_all(&self, hash: &ContentHash) -> Result<HashMap<String, Value>>;
}
```

### Unified Store Facade

```rust
/// Combined storage interface
pub struct Storage {
    pub chunks: Arc<dyn ChunkStore>,
    pub vectors: Arc<dyn VectorStore>,
    pub graph: Arc<dyn GraphStore>,
    pub metadata: Arc<dyn MetadataStore>,
}

impl Storage {
    /// Create storage from configuration
    pub fn from_config(config: &StorageConfig) -> Result<Self>;
}
```

### Configuration

```toml
[storage]
# Backend type: "sqlite" | "qdrant" | "hybrid"
backend = "sqlite"

[storage.sqlite]
path = "~/.codemate/index.db"
wal_mode = true

[storage.qdrant]
url = "http://localhost:6333"
collection_name = "codemate_vectors"
api_key = ""  # Optional

# Hybrid: SQLite for chunks/graph, Qdrant for vectors
[storage.hybrid]
chunks = "sqlite"
vectors = "qdrant"
graph = "sqlite"
metadata = "sqlite"
```

---

## Backend Implementations

### MVP: SQLite (Sprint 1)

| Store | Implementation |
|-------|----------------|
| ChunkStore | SQLite `chunks` table |
| VectorStore | sqlite-vec extension |
| GraphStore | SQLite `graph_nodes` + `graph_edges` |
| MetadataStore | SQLite `chunk_metadata` |

### Future: Qdrant (Sprint 5+)

| Store | Implementation |
|-------|----------------|
| ChunkStore | Qdrant payload storage or SQLite |
| VectorStore | Qdrant collections with HNSW |
| GraphStore | SQLite (Qdrant not optimized for graphs) |
| MetadataStore | Qdrant payloads or SQLite |

---

## Trade-offs

| Aspect | SQLite | Qdrant |
|--------|--------|--------|
| Setup complexity | None (embedded) | Requires server |
| Portability | Single file | Client-server |
| Vector search at 10K | ✅ Fast | ✅ Fast |
| Vector search at 1M | ⚠️ Slower | ✅ Fast |
| Graph queries | ✅ SQL joins | ❌ Not supported |
| Transactions | ✅ ACID | ⚠️ Limited |

---

## Consequences

### Positive
- Clean separation of concerns
- Easy to add new backends
- Test with mocks/in-memory stores
- Can optimize per-workload (hybrid mode)

### Negative
- Slight abstraction overhead
- Must maintain multiple implementations
- Some backend-specific features may not fit trait interface

---

## Open Questions

1. **Transaction support**: Should traits include transaction boundaries?
2. **Streaming**: Large result sets need iterator support
3. **Schema migrations**: How to handle across different backends?

---

## References

- [CodeMate Design Document](../draft/semantic-code-engine-design.md)
- [Qdrant Documentation](https://qdrant.tech/documentation/)
- [sqlite-vec](https://github.com/asg017/sqlite-vec)
