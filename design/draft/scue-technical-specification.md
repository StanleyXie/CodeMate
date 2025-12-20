# SCUE Technical Architecture & Specification

## Development Blueprint v0.1

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Module Specifications](#2-module-specifications)
3. [Data Models & Schema](#3-data-models--schema)
4. [API Specifications](#4-api-specifications)
5. [Algorithm Details](#5-algorithm-details)
6. [Technology Decisions](#6-technology-decisions)
7. [Open Questions & Research Areas](#7-open-questions--research-areas)

---

## 1. System Overview

### 1.1 Architecture Diagram (ASCII)

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                    QUERY LAYER                                       │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐    ┌────────────────────────────┐  │
│  │   CLI   │ │  HTTP   │ │   MCP   │ │   LSP   │───▶│      Query Processor       │  │
│  │  (clap) │ │ (axum)  │ │ (rmcp)  │ │(tower)  │    │  Parse │ Expand │ Route    │  │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘    └────────────┬───────────────┘  │
│                                                                   │                  │
│  ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐  │                  │
│  │  Neural Reranker │ │  Query Expander  │ │ Embedding Service│◀─┘                  │
│  │  (cross-encoder) │ │    (Ollama)      │ │   (fastembed)    │                     │
│  └──────────────────┘ └──────────────────┘ └──────────────────┘                     │
└───────────────────────────────────────┬─────────────────────────────────────────────┘
                                        │
┌───────────────────────────────────────▼─────────────────────────────────────────────┐
│                                  SEMANTIC LAYER                                      │
│  ┌────────────────┐ ┌────────────────┐ ┌────────────────┐ ┌────────────────────┐    │
│  │  Vector Index  │ │  Graph Index   │ │   FTS Index    │ │   AST Pipeline     │    │
│  │  (sqlite-vec)  │ │   (SQLite)     │ │    (FTS5)      │ │  (tree-sitter)     │    │
│  │                │ │                │ │                │ │                    │    │
│  │  ANN Search    │ │  Edges/Nodes   │ │  BM25 Ranking  │ │  Parse → Extract   │    │
│  │  384-768 dims  │ │  Traversal     │ │  Porter Stem   │ │  → Chunk → Hash    │    │
│  └───────┬────────┘ └───────┬────────┘ └───────┬────────┘ └─────────┬──────────┘    │
│          │                  │                  │                    │               │
│          └──────────────────┴────────┬─────────┴────────────────────┘               │
│  ┌───────────────────────────────────▼───────────────────────────────────────────┐  │
│  │                     CONTENT-ADDRESSABLE STORE (CAS)                            │  │
│  │  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────────────┐   │  │
│  │  │   Chunk Store   │ │  Vector Store   │ │       Metadata Store            │   │  │
│  │  │  hash → content │ │  hash → embed   │ │  hash → {key: value, ...}       │   │  │
│  │  └─────────────────┘ └─────────────────┘ └─────────────────────────────────┘   │  │
│  └───────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                      │
│  ┌───────────────────────────────────────────────────────────────────────────────┐  │
│  │                           LOCATION INDEX                                       │  │
│  │   hash → [(repo, branch, commit, path, byte_range, line_range, author, ts)]   │  │
│  └───────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                      │
│  ┌──────────────────────┐ ┌──────────────────────────────────────────────────────┐  │
│  │    Git Pipeline      │ │                  Graph Builder                        │  │
│  │    (libgit2/git2)    │ │   Extract: calls, imports, types, inheritance        │  │
│  │  Walk │ Blame │ Diff │ │   Build: edges between chunk nodes                   │  │
│  └──────────────────────┘ └──────────────────────────────────────────────────────┘  │
└───────────────────────────────────────┬─────────────────────────────────────────────┘
                                        │
┌───────────────────────────────────────▼─────────────────────────────────────────────┐
│                                  CONTENT LAYER                                       │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────────┐  │
│  │ Git (Local) │ │ GitHub API  │ │ GitLab API  │ │ Local Files │ │    Custom     │  │
│  │   libgit2   │ │   octocrab  │ │   gitlab-rs │ │   notify    │ │  Connectors   │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘ └───────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Core Design Principles

1. **Content-Addressable**: All content identified by SHA-256 hash
2. **Git-Native**: Git history is first-class, not an afterthought
3. **Separation of Concerns**: Content (WHAT) vs Location (WHERE) vs Semantics (MEANING)
4. **Lazy Evaluation**: Compute embeddings on-demand, cache permanently
5. **Single Binary**: Distribute as one executable with embedded dependencies

---

## 2. Module Specifications

### 2.1 Module: `scue-core`

**Purpose**: Core types, traits, and utilities shared across all modules.

```rust
// crate: scue-core

/// Content hash (SHA-256, 32 bytes)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn compute(content: &[u8]) -> Self;
    pub fn from_git_blob(blob_id: git2::Oid) -> Self;
    pub fn to_hex(&self) -> String;
    pub fn from_hex(s: &str) -> Result<Self>;
}

/// A semantic chunk of code
#[derive(Clone)]
pub struct Chunk {
    pub hash: ContentHash,
    pub content: String,
    pub kind: ChunkKind,
    pub language: Language,
    pub symbol_name: Option<String>,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub byte_range: Range<usize>,
    pub line_range: Range<usize>,
}

#[derive(Clone, Copy)]
pub enum ChunkKind {
    Function, Method, Class, Struct, Enum, Trait, 
    Interface, Impl, Module, Constant, TypeAlias, Block, FileHeader,
}

#[derive(Clone, Copy)]
pub enum Language {
    Rust, Python, TypeScript, JavaScript, Go, Java, Unknown,
}

/// Location of a chunk in git history
#[derive(Clone)]
pub struct ChunkLocation {
    pub content_hash: ContentHash,
    pub repo_uri: String,
    pub branch: Option<String>,
    pub commit_hash: String,
    pub blob_hash: String,
    pub file_path: PathBuf,
    pub byte_range: Range<usize>,
    pub line_range: Range<usize>,
    pub author: Option<Author>,
    pub authored_at: Option<DateTime<Utc>>,
    pub commit_message: Option<String>,
}

#[derive(Clone)]
pub struct Author {
    pub name: String,
    pub email: String,
}

/// Search result with location context
#[derive(Clone)]
pub struct SearchResult {
    pub chunk: Chunk,
    pub locations: Vec<ChunkLocation>,
    pub score: f32,
    pub score_components: ScoreBreakdown,
}
```

**Dependencies**: git2, chrono, serde

---

### 2.2 Module: `scue-parser`

**Purpose**: AST parsing, chunk extraction, and graph edge detection.

```rust
// crate: scue-parser

pub struct ParserConfig {
    pub max_chunk_lines: usize,      // Default: 100
    pub max_chunk_bytes: usize,      // Default: 8192
    pub overlap_lines: usize,        // Default: 10
    pub extract_docstrings: bool,    // Default: true
    pub extract_signatures: bool,    // Default: true
}

pub struct SemanticParser {
    config: ParserConfig,
    parsers: HashMap<Language, Parser>,
}

impl SemanticParser {
    pub fn new(config: ParserConfig) -> Self;
    pub fn parse_file(&mut self, path: &Path, content: &str) -> Result<ParseResult>;
    pub fn parse_content(&mut self, content: &str, lang: Language) -> Result<ParseResult>;
}

pub struct ParseResult {
    pub chunks: Vec<Chunk>,
    pub edges: Vec<GraphEdge>,
    pub file_imports: Vec<Import>,
    pub parse_errors: Vec<ParseError>,
}

/// Edge in the code graph
#[derive(Clone)]
pub struct GraphEdge {
    pub source: EdgeEndpoint,
    pub target: EdgeEndpoint,
    pub kind: EdgeKind,
    pub weight: f32,
}

#[derive(Clone)]
pub enum EdgeEndpoint {
    Chunk(ContentHash),
    Symbol(String),
    File(PathBuf),
    External(String),
}

#[derive(Clone, Copy)]
pub enum EdgeKind {
    Calls, Imports, Extends, Implements, References, Contains, TypeOf,
}
```

**Language-Specific Extractors**:

```rust
pub trait LanguageExtractor: Send + Sync {
    fn language(&self) -> Language;
    fn grammar(&self) -> &TSLanguage;
    fn extract_definitions(&self, tree: &Tree, source: &str) -> Vec<Definition>;
    fn extract_calls(&self, tree: &Tree, source: &str) -> Vec<CallSite>;
    fn extract_imports(&self, tree: &Tree, source: &str) -> Vec<Import>;
    fn extract_docstring(&self, node: &Node, source: &str) -> Option<String>;
    fn extract_signature(&self, node: &Node, source: &str) -> Option<String>;
}

// Implementations
pub struct RustExtractor;
pub struct PythonExtractor;
pub struct TypeScriptExtractor;
pub struct JavaScriptExtractor;
pub struct GoExtractor;
```

**Dependencies**: tree-sitter + language grammars, scue-core

**Open Questions**:
- [ ] How to handle macros in Rust (they can expand to arbitrary code)?
- [ ] Should we parse generated code (e.g., .proto → .rs)?
- [ ] How to handle mixed-language files (e.g., JSX, Vue SFC)?

---

### 2.3 Module: `scue-git`

**Purpose**: Git repository traversal, blame integration, and history indexing.

```rust
// crate: scue-git

pub struct GitIndexConfig {
    pub branches: Vec<BranchPattern>,
    pub max_depth: Option<usize>,
    pub since: Option<DateTime<Utc>>,
    pub exclude_paths: Vec<GlobPattern>,
    pub include_paths: Vec<GlobPattern>,
}

pub struct GitIndex {
    repo: Repository,
    config: GitIndexConfig,
}

impl GitIndex {
    pub fn open(path: &Path, config: GitIndexConfig) -> Result<Self>;
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>>;
    pub fn walk_commits(&self, branch: &str) -> Result<CommitWalker>;
    pub fn commit_diff(&self, commit: &Commit) -> Result<Vec<DiffEntry>>;
    pub fn get_blob(&self, oid: Oid) -> Result<Vec<u8>>;
    pub fn blame_file(&self, commit: Oid, path: &Path) -> Result<BlameResult>;
    pub fn blame_lines(&self, commit: Oid, path: &Path, lines: Range<usize>) -> Result<Vec<BlameHunk>>;
}

pub struct BranchInfo {
    pub name: String,
    pub head_commit: Oid,
    pub is_remote: bool,
}

pub struct BlameHunk {
    pub line_range: Range<usize>,
    pub original_commit: Oid,
    pub original_path: PathBuf,
    pub author: Author,
    pub authored_at: DateTime<Utc>,
}

/// Incremental update support
impl GitIndex {
    pub fn incremental_update(
        &self,
        last_indexed_commits: &HashMap<String, Oid>,
    ) -> Result<IncrementalUpdate>;
}

pub struct IncrementalUpdate {
    pub blobs_to_index: Vec<BlobToIndex>,
    pub paths_removed: Vec<(String, PathBuf)>,
    pub paths_renamed: Vec<RenameEntry>,
    pub new_branch_heads: HashMap<String, Oid>,
}
```

**Dependencies**: git2 (libgit2), glob, scue-core

**Open Questions**:
- [ ] How to handle very large repositories (>100K commits)?
- [ ] Should we index submodules? How deep?
- [ ] How to handle shallow clones (missing history)?
- [ ] Performance of blame on large files - cache strategy?

---

### 2.4 Module: `scue-storage`

**Purpose**: Content-addressable storage, vector index, and location tracking.

```rust
// crate: scue-storage

pub struct Storage {
    conn: Connection,
    db_path: PathBuf,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn open_in_memory() -> Result<Self>;
    
    // Chunk operations
    pub fn store_chunk(&self, chunk: &Chunk) -> Result<()>;
    pub fn get_chunk(&self, hash: &ContentHash) -> Result<Option<Chunk>>;
    pub fn chunk_exists(&self, hash: &ContentHash) -> Result<bool>;
    
    // Vector operations
    pub fn store_embedding(&self, hash: &ContentHash, embedding: &[f32], model: &str) -> Result<()>;
    pub fn get_embedding(&self, hash: &ContentHash) -> Result<Option<Vec<f32>>>;
    pub fn vector_search(&self, query: &[f32], limit: usize, filter: Option<&SearchFilter>) -> Result<Vec<VectorResult>>;
    
    // Location operations
    pub fn store_location(&self, location: &ChunkLocation) -> Result<i64>;
    pub fn get_locations(&self, hash: &ContentHash) -> Result<Vec<ChunkLocation>>;
    pub fn get_locations_by_author(&self, email: &str) -> Result<Vec<ChunkLocation>>;
    pub fn get_locations_in_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ChunkLocation>>;
    
    // Metadata operations
    pub fn store_metadata(&self, hash: &ContentHash, key: &str, value: &serde_json::Value) -> Result<()>;
    pub fn get_all_metadata(&self, hash: &ContentHash) -> Result<HashMap<String, serde_json::Value>>;
    
    // Graph operations
    pub fn store_edge(&self, edge: &GraphEdge) -> Result<i64>;
    pub fn get_edges_from(&self, source: &EdgeEndpoint, kind: Option<EdgeKind>) -> Result<Vec<GraphEdge>>;
    pub fn get_edges_to(&self, target: &EdgeEndpoint, kind: Option<EdgeKind>) -> Result<Vec<GraphEdge>>;
    
    // FTS operations
    pub fn fts_search(&self, query: &str, limit: usize) -> Result<Vec<FtsResult>>;
}
```

**Schema Summary**:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DATABASE SCHEMA                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  chunks (hash PK, content, kind, language, symbol_name, signature, ...)     │
│  embeddings (hash PK/FK, model, dimensions, vector, created_at)             │
│  locations (id PK, hash FK, repo, branch, commit, path, lines, author, ts)  │
│  metadata (hash PK/FK, key PK, value JSON, source, created_at)              │
│  graph_nodes (id PK, node_type, properties JSON)                            │
│  graph_edges (id PK, source FK, target FK, edge_type, weight)               │
│  chunks_fts (FTS5: symbol_name, signature, docstring, content)              │
│  vec_chunks (sqlite-vec: hash PK, embedding float[N])                       │
│  index_state (repo PK, branch PK, commit_hash, indexed_at)                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Dependencies**: rusqlite, sqlite-vec, serde_json, scue-core

**Open Questions**:
- [ ] sqlite-vec vs. alternatives (usearch, faiss)?
- [ ] Should we support multiple embedding models simultaneously?
- [ ] Store all content vs. fetch from git on demand?

---

### 2.5 Module: `scue-embed`

**Purpose**: Embedding generation using ONNX Runtime.

```rust
// crate: scue-embed

#[derive(Clone, Copy)]
pub enum Model {
    JinaCodeV2,        // 768 dims, best for code
    AllMiniLML6V2,     // 384 dims, fast
    BGESmallENV1_5,    // 384 dims, good quality
    NomicEmbedTextV1,  // 768 dims, long context
}

impl Model {
    pub fn dimensions(&self) -> usize;
    pub fn max_tokens(&self) -> usize;
}

pub struct EmbeddingService {
    model: TextEmbedding,
    model_id: Model,
    batch_size: usize,
}

impl EmbeddingService {
    pub fn new(model: Model) -> Result<Self>;
    pub fn embed(&self, text: &str) -> Result<Vec<f32>>;
    pub fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>>;
    pub fn embed_query(&self, query: &str) -> Result<Vec<f32>>;
    pub fn embed_code(&self, chunk: &Chunk) -> Result<Vec<f32>>;
}

/// Cached embedding service
pub struct CachedEmbeddingService {
    inner: EmbeddingService,
    storage: Arc<Storage>,
}

impl CachedEmbeddingService {
    pub fn embed_chunk(&self, chunk: &Chunk) -> Result<Vec<f32>>;
    pub fn embed_chunks_batch(&self, chunks: &[Chunk]) -> Result<Vec<Vec<f32>>>;
}
```

**Dependencies**: fastembed, scue-core, scue-storage

**Open Questions**:
- [ ] Which embedding model provides best code search quality?
- [ ] Should we support GPU acceleration?
- [ ] How to handle embedding model upgrades?

---

### 2.6 Module: `scue-search`

**Purpose**: Query processing, multi-modal search, and result fusion.

```rust
// crate: scue-search

pub struct SearchConfig {
    pub default_limit: usize,           // Default: 20
    pub vector_weight: f32,             // Default: 0.5
    pub fts_weight: f32,                // Default: 0.3
    pub rrf_k: usize,                   // Default: 60
    pub enable_query_expansion: bool,   // Default: true
    pub enable_reranking: bool,         // Default: true
    pub rerank_top_n: usize,            // Default: 50
}

pub struct ParsedQuery {
    pub text: String,
    pub filters: QueryFilters,
    pub modifiers: QueryModifiers,
}

pub struct QueryFilters {
    pub repos: Vec<String>,
    pub branches: Vec<String>,
    pub paths: Vec<GlobPattern>,
    pub languages: Vec<Language>,
    pub authors: Vec<String>,
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub chunk_kinds: Vec<ChunkKind>,
}

pub struct QueryParser;

impl QueryParser {
    /// Parse: "rate limiting author:alice after:2024-01-01 in:main,dev"
    pub fn parse(query: &str) -> Result<ParsedQuery>;
}

pub struct SearchEngine {
    storage: Arc<Storage>,
    embedder: Arc<CachedEmbeddingService>,
    query_expander: Option<QueryExpander>,
    reranker: Option<Reranker>,
    config: SearchConfig,
}

impl SearchEngine {
    pub async fn search(&self, query: &str) -> Result<SearchResults>;
    pub async fn search_parsed(&self, query: &ParsedQuery) -> Result<SearchResults>;
}

/// Query expansion using Ollama
pub struct QueryExpander {
    client: reqwest::Client,
    ollama_url: String,
    model: String,
}

impl QueryExpander {
    pub async fn expand(&self, query: &str) -> Result<Vec<String>>;
}

/// Cross-encoder reranker
pub struct Reranker {
    client: reqwest::Client,
    ollama_url: String,
    model: String,
}

impl Reranker {
    pub async fn rerank(&self, query: &str, docs: &[(ContentHash, &str)]) -> Result<HashMap<ContentHash, f32>>;
}
```

**Dependencies**: scue-core, scue-storage, scue-embed, reqwest, futures

---

### 2.7 Module: `scue-graph`

**Purpose**: Code relationship graph queries.

```rust
// crate: scue-graph

pub struct GraphEngine {
    storage: Arc<Storage>,
}

impl GraphEngine {
    pub fn find_callers(&self, symbol: &str, depth: usize) -> Result<CallGraph>;
    pub fn find_callees(&self, symbol: &str, depth: usize) -> Result<CallGraph>;
    pub fn find_dependencies(&self, file: &Path, depth: usize) -> Result<DependencyTree>;
    pub fn find_dependents(&self, file: &Path, depth: usize) -> Result<DependencyTree>;
    pub fn find_similar(&self, hash: &ContentHash, threshold: f32) -> Result<Vec<SimilarChunk>>;
    pub fn find_type_hierarchy(&self, type_name: &str) -> Result<TypeHierarchy>;
}
```

---

## 3. API Specifications

### 3.1 CLI Interface

```
scue - Semantic Code Understanding Engine

USAGE:
    scue <COMMAND>

COMMANDS:
    index       Index a git repository
    search      Semantic search across indexed code
    graph       Query code relationships
    history     View code evolution over time
    status      Show index status

EXAMPLES:
    scue index .
    scue search "authentication middleware"
    scue search "db conn" author:alice after:2024-01-01
    scue graph callers "authenticate"
    scue history "UserService.create"
```

### 3.2 MCP Server Tools

```json
{
  "tools": [
    { "name": "scue_search", "description": "Semantic code search" },
    { "name": "scue_callers", "description": "Find callers of a function" },
    { "name": "scue_deps", "description": "Find dependencies of a file" },
    { "name": "scue_history", "description": "View symbol history" },
    { "name": "scue_blame", "description": "Find who wrote code" }
  ]
}
```

---

## 4. Algorithm Details

### 4.1 Reciprocal Rank Fusion (RRF)

```
Input: lists (ranked result lists), weights, k=60
Output: Fused ranked list

1. For each (list, weight) in zip(lists, weights):
     For rank, item in enumerate(list):
       scores[item] += weight / (k + rank + 1)
       
2. Top-rank bonus:
     scores[list[0]] += 0.05
     scores[list[1]] += 0.02
     scores[list[2]] += 0.02
     
3. Sort by score descending
```

### 4.2 Position-Aware Rerank Blending

```
For each item at RRF position i:
  if i <= 3:  rrf_weight = 0.75    # Trust retrieval
  elif i <= 10: rrf_weight = 0.60
  else: rrf_weight = 0.40          # Trust reranker
  
  blended = rrf_weight * (1/i) + (1 - rrf_weight) * rerank_score
```

---

## 5. Technology Decisions

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Performance, memory safety, single binary |
| Database | SQLite | Embedded, portable, well-tested |
| Vector Search | sqlite-vec | Single file, good enough perf |
| FTS | SQLite FTS5 | Built-in, BM25, porter stemmer |
| AST Parsing | tree-sitter | Multi-language, incremental |
| Git | libgit2 (git2) | Full API, blame support |
| Embeddings | fastembed | ONNX, no Python |
| CLI | clap | Feature-rich |
| HTTP | axum | Async, tower ecosystem |

---

## 6. Open Questions & Research Areas

### 6.1 High Priority (Blocking)

| Question | Potential Approaches |
|----------|---------------------|
| **Embedding model selection** | Benchmark jina-code-v2, nomic-embed, codebert |
| **Chunk size optimization** | Experiment with 50-150 line ranges |
| **sqlite-vec scalability** | Benchmark at 1M+ vectors |
| **Incremental graph updates** | Version edges by commit? Periodic rebuild? |

### 6.2 Medium Priority

| Question | Potential Approaches |
|----------|---------------------|
| **Cross-repo symbol resolution** | Namespace by repo, use type signatures |
| **Large file handling** | Aggressive chunking, skip generated files |
| **Blame performance** | Cache results, incremental blame |
| **Query expansion quality** | Fine-tune prompts, A/B test |
| **Multi-language projects** | Language-prefixed symbols |

### 6.3 Research Spikes Needed

**Spike 1: Embedding Model Benchmark (3-5 days)**
- Create 100 query test dataset with ground truth
- Evaluate: MRR@10, Recall@10, latency, memory
- Output: Model recommendation

**Spike 2: sqlite-vec Scale Testing (2-3 days)**
- Generate 100K, 500K, 1M vectors
- Benchmark insert throughput, query latency
- Compare with usearch/hnswlib if needed

**Spike 3: AST Chunking Quality (3-4 days)**
- Analyze real codebases for function size distribution
- Test chunk sizes: 50, 75, 100, 150 lines
- Measure retrieval precision

**Spike 4: Git History Indexing Performance (3-5 days)**
- Profile on repos of varying sizes
- Measure time, memory, storage per commit
- Design incremental update strategy

---

## 7. Module Dependency Graph

```
                                 ┌─────────────────┐
                                 │    scue-cli     │
                                 └────────┬────────┘
                    ┌─────────────────────┼─────────────────────┐
                    ▼                     ▼                     ▼
           ┌───────────────┐    ┌───────────────┐    ┌───────────────┐
           │  scue-server  │    │  scue-search  │    │   scue-mcp    │
           └───────┬───────┘    └───────┬───────┘    └───────┬───────┘
                   └────────────────────┼────────────────────┘
                    ┌───────────────────┴───────────────────┐
                    ▼                                       ▼
           ┌───────────────┐                      ┌───────────────┐
           │  scue-embed   │                      │  scue-graph   │
           └───────┬───────┘                      └───────┬───────┘
                   └───────────────────┬───────────────────┘
                                       ▼
                              ┌───────────────┐
                              │ scue-storage  │
                              └───────┬───────┘
           ┌──────────────────────────┼──────────────────────────┐
           ▼                          ▼                          ▼
  ┌───────────────┐          ┌───────────────┐          ┌───────────────┐
  │  scue-parser  │          │   scue-git    │          │  scue-core    │
  └───────────────┘          └───────────────┘          └───────────────┘
```

---

## 8. File Structure

```
scue/
├── Cargo.toml                 # Workspace manifest
├── crates/
│   ├── scue-core/            # Types, traits, utilities
│   ├── scue-parser/          # AST parsing, chunking
│   ├── scue-git/             # Git operations
│   ├── scue-storage/         # Database, indexes
│   ├── scue-embed/           # Embeddings
│   ├── scue-search/          # Search engine
│   ├── scue-graph/           # Graph queries
│   ├── scue-server/          # HTTP API
│   ├── scue-mcp/             # MCP server
│   └── scue-cli/             # CLI binary
├── tests/integration/
└── benches/
```

---

## 9. Estimated Storage Sizes

| Component | Per-Chunk | 100K Chunks | 1M Chunks |
|-----------|-----------|-------------|-----------|
| chunks | ~2KB | ~200MB | ~2GB |
| embeddings (384d) | ~1.5KB | ~150MB | ~1.5GB |
| embeddings (768d) | ~3KB | ~300MB | ~3GB |
| locations | ~500B | ~50MB | ~500MB |
| graph_edges | ~200B | ~20MB | ~200MB |
| **Total (384d)** | | **~480MB** | **~4.8GB** |

---

*Document End - Version 0.1.0*
