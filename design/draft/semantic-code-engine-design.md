# Semantic Code Understanding Engine

## Architecture Design Document

**Version**: 0.1.0 (Draft)
**Date**: December 2025
**Status**: Design Phase

---

## 1. Executive Summary

The Semantic Code Understanding Engine (CodeMate) is a next-generation code intelligence platform that combines AST-aware code analysis, git-native temporal understanding, and multi-modal semantic search. Unlike existing tools (osgrep, demongrep, qmd), CodeMate treats git history as a first-class citizen and implements a content-addressable storage model that enables unprecedented code archaeology capabilities.

### Key Differentiators

| Capability | osgrep | demongrep | qmd | CodeMate |
|------------|--------|-----------|-----|------|
| AST-aware chunking | ✅ | ✅ | ❌ | ✅ |
| Git history search | ❌ | ❌ | ❌ | ✅ |
| Cross-branch search | ❌ | ❌ | ❌ | ✅ |
| Author/blame tracking | ❌ | ❌ | ❌ | ✅ |
| Temporal queries | ❌ | ❌ | ❌ | ✅ |
| Graph relationships | ❌ | ❌ | ❌ | ✅ |
| Content deduplication | Partial | Partial | Partial | ✅ |
| Multi-source federation | ❌ | ❌ | Partial | ✅ |

---

## 2. Design Principles

### 2.1 Content-Addressable Everything

Every piece of content is identified by its cryptographic hash (SHA-256). This enables:
- **Deduplication**: Same code across branches/commits stored once
- **Immutability**: Content is append-only, never mutated
- **Git alignment**: Leverages git's native blob hashing
- **Efficient storage**: O(1) lookup, natural caching

### 2.2 Separation of Content and Location

```
WHAT (Content Layer)     vs.    WHERE (Location Layer)
─────────────────────           ─────────────────────
Content hash                    Repository
Embedding vectors               Branch
Semantic metadata               Commit
                                File path
                                Line range
                                Author
                                Timestamp
```

### 2.3 Git as Source of Truth

- Git objects (blobs, trees, commits) are the canonical content source
- CodeMate indexes git, never replaces it
- All temporal metadata derived from git history
- Branch relationships extracted from git graph

### 2.4 Lazy Materialization

- Embeddings computed on-demand, cached permanently
- Full content fetched from git at query time
- Index stores pointers, not content duplicates

---

## 3. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        SEMANTIC CODE UNDERSTANDING ENGINE                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                         QUERY LAYER                                  │    │
│  │  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────────────┐ │    │
│  │  │ CLI/TUI   │  │ HTTP API  │  │ MCP Server│  │ Language Server   │ │    │
│  │  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────────┬─────────┘ │    │
│  │        └──────────────┴──────────────┴──────────────────┘           │    │
│  │                              │                                       │    │
│  │  ┌───────────────────────────▼───────────────────────────────────┐  │    │
│  │  │                    QUERY PROCESSOR                             │  │    │
│  │  │  • Query expansion (LLM)    • Temporal filter parsing          │  │    │
│  │  │  • Multi-modal routing      • Result fusion (RRF)              │  │    │
│  │  └───────────────────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                        SEMANTIC LAYER                                │    │
│  │                                                                      │    │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────┐  │    │
│  │  │  VECTOR INDEX   │  │  GRAPH INDEX    │  │  FULL-TEXT INDEX    │  │    │
│  │  │  (Embeddings)   │  │  (Relations)    │  │  (BM25)             │  │    │
│  │  │                 │  │                 │  │                     │  │    │
│  │  │  • Code chunks  │  │  • Call graph   │  │  • Symbol names     │  │    │
│  │  │  • Symbols      │  │  • Import deps  │  │  • Comments         │  │    │
│  │  │  • Comments     │  │  • Type refs    │  │  • Strings          │  │    │
│  │  │  • Metadata     │  │  • Git ancestry │  │  • Identifiers      │  │    │
│  │  └────────┬────────┘  └────────┬────────┘  └──────────┬──────────┘  │    │
│  │           └────────────────────┴─────────────────────┘              │    │
│  │                              │                                       │    │
│  │  ┌───────────────────────────▼───────────────────────────────────┐  │    │
│  │  │                  CONTENT-ADDRESSABLE STORE                     │  │    │
│  │  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │  │    │
│  │  │  │ Chunk Store │  │ Vector Store│  │ Metadata Store          │ │  │    │
│  │  │  │ hash→content│  │ hash→embed  │  │ hash→(author,date,...)  │ │  │    │
│  │  │  └─────────────┘  └─────────────┘  └─────────────────────────┘ │  │    │
│  │  └───────────────────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                        CONTENT LAYER                                 │    │
│  │                                                                      │    │
│  │  ┌─────────────────────────────────────────────────────────────┐    │    │
│  │  │                    LOCATION INDEX                            │    │    │
│  │  │  chunk_hash → [(repo, branch, commit, path, range, meta)]   │    │    │
│  │  └─────────────────────────────────────────────────────────────┘    │    │
│  │                              │                                       │    │
│  │  ┌───────────────────────────▼───────────────────────────────────┐  │    │
│  │  │                    SOURCE CONNECTORS                           │  │    │
│  │  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────┐   │  │    │
│  │  │  │   Git   │  │  GitHub │  │  GitLab │  │  Local Files    │   │  │    │
│  │  │  │ (local) │  │  (API)  │  │  (API)  │  │  (watch)        │   │  │    │
│  │  │  └─────────┘  └─────────┘  └─────────┘  └─────────────────┘   │  │    │
│  │  └───────────────────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Core Components

### 4.1 Content-Addressable Store (CAS)

The CAS is the heart of CodeMate, providing deduplication and immutable storage.

#### 4.1.1 Chunk Store

```sql
-- Chunks are content-addressed by SHA-256
CREATE TABLE chunks (
    content_hash    TEXT PRIMARY KEY,  -- SHA-256 of content
    content         BLOB,              -- Optional: cached content
    content_type    TEXT NOT NULL,     -- 'code', 'comment', 'docstring'
    language        TEXT,              -- 'rust', 'python', 'typescript'
    chunk_kind      TEXT NOT NULL,     -- 'function', 'class', 'method', 'block'
    symbol_name     TEXT,              -- Extracted identifier
    signature       TEXT,              -- Function/method signature
    byte_size       INTEGER NOT NULL,
    line_count      INTEGER NOT NULL,
    first_seen_at   TEXT NOT NULL,     -- ISO timestamp
    last_accessed   TEXT NOT NULL
);

CREATE INDEX idx_chunks_symbol ON chunks(symbol_name);
CREATE INDEX idx_chunks_kind ON chunks(chunk_kind, language);
```

#### 4.1.2 Vector Store

```sql
-- Embeddings stored separately, keyed by content hash
CREATE TABLE embeddings (
    content_hash    TEXT PRIMARY KEY,
    model_id        TEXT NOT NULL,     -- 'minilm-l6-v2', 'jina-code', etc.
    vector          BLOB NOT NULL,     -- Float32 array
    dimensions      INTEGER NOT NULL,
    embedded_at     TEXT NOT NULL,
    FOREIGN KEY (content_hash) REFERENCES chunks(content_hash)
);

-- sqlite-vec virtual table for ANN search
CREATE VIRTUAL TABLE vectors_vec USING vec0(
    content_hash TEXT PRIMARY KEY,
    embedding float[384]
);
```

#### 4.1.3 Metadata Store (Extensible)

```sql
-- Flexible key-value metadata per chunk
CREATE TABLE chunk_metadata (
    content_hash    TEXT NOT NULL,
    key             TEXT NOT NULL,
    value           TEXT NOT NULL,     -- JSON-encoded
    source          TEXT NOT NULL,     -- 'ast', 'git', 'user', 'inferred'
    created_at      TEXT NOT NULL,
    PRIMARY KEY (content_hash, key),
    FOREIGN KEY (content_hash) REFERENCES chunks(content_hash)
);

-- Example metadata keys:
-- 'complexity.cyclomatic' → 12
-- 'dependencies' → ['lodash', 'express']
-- 'test_coverage' → 0.85
-- 'security.cwe' → ['CWE-79', 'CWE-89']
-- 'ai.summary' → 'Handles user authentication...'
```

### 4.2 Location Index

The Location Index tracks WHERE each chunk appears across git history.

```sql
-- Every occurrence of a chunk in git history
CREATE TABLE chunk_locations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    content_hash    TEXT NOT NULL,
    
    -- Git coordinates
    repo_uri        TEXT NOT NULL,     -- 'file:///path' or 'https://github.com/...'
    branch          TEXT,              -- 'main', 'feature/auth', NULL for detached
    commit_hash     TEXT NOT NULL,     -- Git commit SHA
    blob_hash       TEXT NOT NULL,     -- Git blob SHA (= content hash for whole files)
    
    -- File coordinates  
    file_path       TEXT NOT NULL,
    byte_start      INTEGER NOT NULL,
    byte_end        INTEGER NOT NULL,
    line_start      INTEGER NOT NULL,
    line_end        INTEGER NOT NULL,
    
    -- Git metadata (denormalized for query performance)
    author_name     TEXT,
    author_email    TEXT,
    authored_at     TEXT,              -- ISO timestamp
    committer_name  TEXT,
    committer_email TEXT,
    committed_at    TEXT,
    commit_message  TEXT,
    
    -- Indexing metadata
    indexed_at      TEXT NOT NULL,
    
    FOREIGN KEY (content_hash) REFERENCES chunks(content_hash)
);

-- Compound indexes for common query patterns
CREATE INDEX idx_locations_repo_branch ON chunk_locations(repo_uri, branch);
CREATE INDEX idx_locations_author ON chunk_locations(author_email, authored_at);
CREATE INDEX idx_locations_path ON chunk_locations(file_path, repo_uri);
CREATE INDEX idx_locations_time ON chunk_locations(authored_at);
CREATE INDEX idx_locations_commit ON chunk_locations(commit_hash);
CREATE INDEX idx_locations_hash ON chunk_locations(content_hash);
```

### 4.3 Graph Index

The Graph Index captures structural relationships in code.

```sql
-- Nodes: chunks, files, commits, branches, authors
CREATE TABLE graph_nodes (
    node_id         TEXT PRIMARY KEY,  -- Typed ID: 'chunk:abc123', 'commit:def456'
    node_type       TEXT NOT NULL,     -- 'chunk', 'file', 'commit', 'branch', 'author'
    properties      TEXT,              -- JSON blob
    created_at      TEXT NOT NULL
);

-- Edges: relationships between nodes
CREATE TABLE graph_edges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id       TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    edge_type       TEXT NOT NULL,     -- See edge types below
    properties      TEXT,              -- JSON blob (weight, metadata)
    created_at      TEXT NOT NULL,
    
    FOREIGN KEY (source_id) REFERENCES graph_nodes(node_id),
    FOREIGN KEY (target_id) REFERENCES graph_nodes(node_id)
);

CREATE INDEX idx_edges_source ON graph_edges(source_id, edge_type);
CREATE INDEX idx_edges_target ON graph_edges(target_id, edge_type);
CREATE INDEX idx_edges_type ON graph_edges(edge_type);
```

#### Edge Types

| Edge Type | Source | Target | Description |
|-----------|--------|--------|-------------|
| `CALLS` | chunk (function) | chunk (function) | Function call relationship |
| `IMPORTS` | chunk/file | chunk/file | Import/require dependency |
| `EXTENDS` | chunk (class) | chunk (class) | Class inheritance |
| `IMPLEMENTS` | chunk (class) | chunk (interface) | Interface implementation |
| `REFERENCES` | chunk | chunk | Symbol reference |
| `CONTAINS` | file | chunk | File contains chunk |
| `PARENT_COMMIT` | commit | commit | Git commit ancestry |
| `BRANCH_HEAD` | branch | commit | Branch points to commit |
| `AUTHORED` | author | chunk | Author wrote chunk |
| `MODIFIED` | commit | chunk | Commit modified chunk |
| `SIMILAR_TO` | chunk | chunk | Semantic similarity (>0.9) |

### 4.4 AST Processing Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        AST PROCESSING PIPELINE                           │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Source File                                                             │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    LANGUAGE DETECTION                            │    │
│  │  Extension → tree-sitter grammar mapping                         │    │
│  │  .rs → tree-sitter-rust, .py → tree-sitter-python, etc.         │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    TREE-SITTER PARSE                             │    │
│  │  Source → CST (Concrete Syntax Tree)                             │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    SEMANTIC EXTRACTION                           │    │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐        │    │
│  │  │ Definition    │  │ Docstring     │  │ Reference     │        │    │
│  │  │ Extractor     │  │ Extractor     │  │ Extractor     │        │    │
│  │  │               │  │               │  │               │        │    │
│  │  │ • Functions   │  │ • /// docs    │  │ • Calls       │        │    │
│  │  │ • Classes     │  │ • """ docs    │  │ • Imports     │        │    │
│  │  │ • Methods     │  │ • /** docs    │  │ • Type refs   │        │    │
│  │  │ • Structs     │  │ • # comments  │  │ • Variables   │        │    │
│  │  │ • Traits      │  │               │  │               │        │    │
│  │  └───────────────┘  └───────────────┘  └───────────────┘        │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    CHUNK ASSEMBLY                                │    │
│  │  • Combine definition + docstring + body                         │    │
│  │  • Apply size limits (max 100 lines, 4KB)                        │    │
│  │  • Split oversized chunks with overlap                           │    │
│  │  • Compute content hash (SHA-256)                                │    │
│  │  • Extract signature, symbol name                                │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    GRAPH EXTRACTION                              │    │
│  │  • Build call graph from function calls                          │    │
│  │  • Extract import dependencies                                   │    │
│  │  • Map type references and inheritance                           │    │
│  │  • Link chunks to containing files                               │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  Chunks + Graph Edges + Metadata                                        │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 4.5 Git Indexing Pipeline

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        GIT INDEXING PIPELINE                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Git Repository                                                          │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    BRANCH ENUMERATION                            │    │
│  │  • List all branches (local + remote)                            │    │
│  │  • Filter by pattern (main, develop, feature/*)                  │    │
│  │  • Build branch→commit mapping                                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    COMMIT WALK                                   │    │
│  │  • Topological traversal from branch heads                       │    │
│  │  • Respect depth limits (last N commits, since date)             │    │
│  │  • Extract commit metadata (author, date, message)               │    │
│  │  • Build commit ancestry graph                                   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    BLOB EXTRACTION                               │    │
│  │  For each commit:                                                │    │
│  │  • Diff against parent to find changed files                     │    │
│  │  • Extract blob content for changed files                        │    │
│  │  • Check if blob hash already indexed (dedup)                    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    BLAME ATTRIBUTION                             │    │
│  │  For each chunk in file:                                         │    │
│  │  • Run git blame for line range                                  │    │
│  │  • Map lines to originating commits                              │    │
│  │  • Identify original author of each chunk                        │    │
│  │  • Track modification history                                    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │                    LOCATION INDEXING                             │    │
│  │  • Create chunk_locations entry for each occurrence              │    │
│  │  • Link to content_hash in CAS                                   │    │
│  │  • Store author, timestamp, commit metadata                      │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│       │                                                                  │
│       ▼                                                                  │
│  Location Index + Commit Graph + Author Graph                           │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 5. Query Capabilities

### 5.1 Basic Semantic Search

```
Query: "authentication middleware"

Pipeline:
1. Embed query → vector
2. ANN search in vector index
3. Fetch chunk metadata
4. Resolve locations (branch, commit, path)
5. Return ranked results
```

### 5.2 Temporal Queries

```
Query: "authentication middleware AFTER:2024-01-01 BEFORE:2024-06-01"

Pipeline:
1. Parse temporal filters
2. Embed query → vector
3. ANN search with pre-filter on authored_at
4. Fetch chunks within time range
5. Return results with temporal context
```

### 5.3 Author Queries

```
Query: "authentication AUTHOR:alice@example.com"

Pipeline:
1. Parse author filter
2. Embed query → vector
3. ANN search with pre-filter on author_email
4. Fetch chunks authored by alice
5. Return results with author attribution
```

### 5.4 Cross-Branch Queries

```
Query: "rate limiting IN:main,develop,feature/*"

Pipeline:
1. Parse branch patterns
2. Resolve matching branches
3. Embed query → vector
4. ANN search with pre-filter on branch
5. Deduplicate same content across branches
6. Return results with branch context
```

### 5.5 Graph Queries

```
Query: "What calls the authenticate() function?"

Pipeline:
1. Find chunk containing authenticate()
2. Graph traversal: CALLS edges → authenticate
3. Return calling functions with context
```

```
Query: "What are the dependencies of UserService?"

Pipeline:
1. Find UserService chunk
2. Graph traversal: IMPORTS edges from UserService
3. Return dependency tree
```

### 5.6 Temporal Evolution Queries

```
Query: "Show me how authenticate() changed over time"

Pipeline:
1. Find all versions of authenticate() across commits
2. Order by authored_at
3. Compute diffs between versions
4. Return timeline with changes highlighted
```

### 5.7 Code Archaeology Queries

```
Query: "Who originally wrote this function and when?"

Pipeline:
1. Find chunk by identifier or content
2. Follow AUTHORED edge to author
3. Find earliest commit containing chunk
4. Return author, date, commit context
```

---

## 6. Embedding Strategy

### 6.1 Multi-Modal Embeddings

| Content Type | Embedding Model | Dimensions | Use Case |
|--------------|-----------------|------------|----------|
| Code chunks | jina-code-v2 | 768 | Semantic code search |
| Symbols | minilm-l6-v2 | 384 | Symbol name search |
| Comments | bge-small-en | 384 | Documentation search |
| Commit messages | minilm-l6-v2 | 384 | Commit search |

### 6.2 Composite Embedding

For each chunk, compute weighted combination:

```
chunk_embedding = 0.7 * code_embed + 0.2 * signature_embed + 0.1 * docstring_embed
```

### 6.3 Expandable Metadata Embedding

Custom metadata can be embedded and searched:

```sql
-- Metadata embeddings table
CREATE TABLE metadata_embeddings (
    content_hash    TEXT NOT NULL,
    metadata_key    TEXT NOT NULL,
    vector          BLOB NOT NULL,
    model_id        TEXT NOT NULL,
    embedded_at     TEXT NOT NULL,
    PRIMARY KEY (content_hash, metadata_key)
);
```

Example: Embed AI-generated summaries for semantic search on descriptions.

---

## 7. Technology Stack

### 7.1 Core Runtime

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Language | Rust | Performance, memory safety, single binary |
| Async Runtime | Tokio | Concurrent git operations, I/O |
| CLI Framework | clap | Feature-rich argument parsing |

### 7.2 Parsing & Analysis

| Component | Technology | Rationale |
|-----------|------------|-----------|
| AST Parsing | tree-sitter (native) | Multi-language, incremental, fast |
| Git Operations | git2 (libgit2) | Full git API, blame support |
| Language Detection | file extension + magic bytes | Simple, reliable |

### 7.3 Storage & Indexing

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Primary Database | SQLite + WAL | Embedded, ACID, portable |
| Vector Search | sqlite-vec | Single-file, good performance |
| Full-Text Search | SQLite FTS5 | Integrated BM25 |
| Graph Storage | SQLite tables | Simple, queryable |

### 7.4 Embedding & ML

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Embedding Runtime | fastembed (ONNX) | Fast, no Python dependency |
| Query Expansion | Ollama (optional) | Local LLM, privacy |
| Reranking | Cross-encoder (optional) | Improved precision |

### 7.5 Interfaces

| Component | Technology | Rationale |
|-----------|------------|-----------|
| CLI | clap + colored | Rich terminal output |
| HTTP API | axum | Fast, async |
| MCP Server | rmcp | Claude Code integration |
| LSP | tower-lsp | Editor integration |

---

## 8. Data Flow

### 8.1 Indexing Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Git Repo    │────▶│ Git Walker  │────▶│ AST Parser  │────▶│ Chunk       │
│             │     │             │     │             │     │ Assembler   │
└─────────────┘     └─────────────┘     └─────────────┘     └──────┬──────┘
                                                                    │
                    ┌───────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Content     │◀────│ Dedup       │◀────│ Hash        │◀────│ Chunks      │
│ Store       │     │ Check       │     │ Compute     │     │             │
└──────┬──────┘     └─────────────┘     └─────────────┘     └─────────────┘
       │
       │  (if new)
       ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Embedding   │────▶│ Vector      │     │ Graph       │
│ Generator   │     │ Store       │     │ Builder     │
└─────────────┘     └─────────────┘     └──────┬──────┘
                                               │
       ┌───────────────────────────────────────┘
       │
       ▼
┌─────────────┐     ┌─────────────┐
│ Location    │     │ Graph       │
│ Index       │     │ Index       │
└─────────────┘     └─────────────┘
```

### 8.2 Query Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ User Query  │────▶│ Query       │────▶│ Filter      │────▶│ Query       │
│             │     │ Parser      │     │ Extractor   │     │ Expander    │
└─────────────┘     └─────────────┘     └─────────────┘     └──────┬──────┘
                                                                    │
                    ┌───────────────────────────────────────────────┘
                    │
                    ▼
               ┌─────────────────────────────────────────────────────┐
               │                  PARALLEL SEARCH                     │
               │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │
               │  │ Vector      │  │ FTS         │  │ Graph       │  │
               │  │ Search      │  │ Search      │  │ Search      │  │
               │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  │
               │         │                │                │         │
               │         └────────────────┴────────────────┘         │
               │                          │                          │
               └──────────────────────────┼──────────────────────────┘
                                          │
                                          ▼
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ Results     │◀────│ Content     │◀────│ Location    │◀────│ RRF         │
│             │     │ Resolver    │     │ Resolver    │     │ Fusion      │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
```

---

## 9. Query Language (DSL)

### 9.1 Basic Syntax

```
<query> ::= <text_query> [<filter>]*
<filter> ::= <key>:<value> | <key>:<op><value>
<key> ::= 'author' | 'branch' | 'path' | 'lang' | 'type' | 'after' | 'before' | 'in'
<op> ::= '>' | '<' | '>=' | '<=' | '~'
```

### 9.2 Examples

```bash
# Basic semantic search
codemate search "authentication middleware"

# Author filter
codemate search "database connection" author:alice@example.com

# Time range
codemate search "rate limiting" after:2024-01-01 before:2024-06-01

# Branch filter (glob patterns supported)
codemate search "feature flag" in:main,develop,feature/*

# Language filter
codemate search "async handler" lang:rust,typescript

# Chunk type filter
codemate search "validation" type:function,method

# Path filter (glob patterns)
codemate search "config" path:src/config/**

# Combined filters
codemate search "authentication" author:alice in:main after:2024-01-01 lang:rust type:function
```

### 9.3 Graph Queries

```bash
# Find callers
codemate graph callers "authenticate"

# Find callees
codemate graph callees "UserService.create"

# Find dependencies
codemate graph deps "src/auth/mod.rs"

# Find dependents (reverse deps)
codemate graph rdeps "src/utils/hash.rs"

# Find similar code
codemate graph similar "src/handlers/auth.rs:authenticate"
```

### 9.4 Temporal Queries

```bash
# Show history of a function
codemate history "authenticate" --since 2024-01-01

# Find when function was added
codemate origin "UserService.validate"

# Find who last modified
codemate blame "src/auth/middleware.rs:45-60"

# Compare across commits
codemate diff "authenticate" --from HEAD~10 --to HEAD
```

---

## 10. Extensibility

### 10.1 Custom Metadata Providers

```rust
trait MetadataProvider: Send + Sync {
    /// Provider identifier
    fn id(&self) -> &str;
    
    /// Extract metadata from a chunk
    async fn extract(&self, chunk: &Chunk) -> Result<Vec<(String, Value)>>;
    
    /// Optional: embed metadata for search
    fn embeddable_keys(&self) -> Vec<&str> { vec![] }
}

// Example: Complexity analyzer
struct ComplexityProvider;

impl MetadataProvider for ComplexityProvider {
    fn id(&self) -> &str { "complexity" }
    
    async fn extract(&self, chunk: &Chunk) -> Result<Vec<(String, Value)>> {
        let cyclomatic = analyze_cyclomatic(chunk)?;
        let cognitive = analyze_cognitive(chunk)?;
        Ok(vec![
            ("complexity.cyclomatic".into(), json!(cyclomatic)),
            ("complexity.cognitive".into(), json!(cognitive)),
        ])
    }
}
```

### 10.2 Custom Source Connectors

```rust
trait SourceConnector: Send + Sync {
    /// Connector identifier
    fn id(&self) -> &str;
    
    /// List content units (files, blobs)
    async fn list_units(&self, filter: &SourceFilter) -> Result<Vec<ContentUnit>>;
    
    /// Fetch content by unit ID
    async fn fetch_content(&self, unit_id: &str) -> Result<Vec<u8>>;
    
    /// Watch for changes (optional)
    fn watch(&self) -> Option<BoxStream<ChangeEvent>> { None }
}

// Example: GitHub connector
struct GitHubConnector {
    client: Octocrab,
    owner: String,
    repo: String,
}
```

### 10.3 Custom Embedding Models

```rust
trait EmbeddingModel: Send + Sync {
    /// Model identifier
    fn id(&self) -> &str;
    
    /// Embedding dimensions
    fn dimensions(&self) -> usize;
    
    /// Embed batch of texts
    async fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>>;
}

// Models are registered at startup
registry.register_model(FastEmbedModel::new("jina-code-v2")?);
registry.register_model(OllamaModel::new("nomic-embed-text")?);
```

---

## 11. Performance Considerations

### 11.1 Indexing Performance

| Operation | Target | Strategy |
|-----------|--------|----------|
| Git walk | 10K commits/sec | Parallel tree traversal |
| AST parse | 5K files/sec | tree-sitter incremental |
| Embedding | 100 chunks/sec | Batch processing, GPU optional |
| Dedup check | O(1) | Hash-based lookup |

### 11.2 Query Performance

| Operation | Target | Strategy |
|-----------|--------|----------|
| Vector search | <50ms | sqlite-vec ANN |
| FTS search | <20ms | SQLite FTS5 |
| Graph traversal | <100ms | Index on edges |
| Filter application | <10ms | Pre-filtering |

### 11.3 Storage Efficiency

| Metric | Target | Strategy |
|--------|--------|----------|
| Dedup ratio | 20-40% | Content-addressable |
| Index size | <2x source | Pointer-based locations |
| Vector storage | 1.5KB/chunk | Float32, 384 dims |

---

## 12. Implementation Phases

### Phase 1: Core Infrastructure (4-6 weeks)

- [ ] Content-addressable store
- [ ] SQLite schema and migrations
- [ ] Basic git connector (local repos)
- [ ] tree-sitter integration (Rust, Python, TypeScript, JavaScript)
- [ ] Chunk extraction pipeline
- [ ] CLI skeleton

### Phase 2: Semantic Search (3-4 weeks)

- [ ] fastembed integration
- [ ] sqlite-vec setup
- [ ] FTS5 configuration
- [ ] Basic semantic search
- [ ] Query parsing

### Phase 3: Git Intelligence (4-5 weeks)

- [ ] Full git history walking
- [ ] Branch enumeration and filtering
- [ ] Blame integration
- [ ] Location index population
- [ ] Temporal query support

### Phase 4: Graph Index (3-4 weeks)

- [ ] Call graph extraction
- [ ] Import dependency tracking
- [ ] Type reference mapping
- [ ] Graph query API

### Phase 5: Advanced Features (4-6 weeks)

- [ ] Query expansion (LLM)
- [ ] Reranking
- [ ] HTTP API
- [ ] MCP server
- [ ] Incremental updates
- [ ] Watch mode

### Phase 6: Polish & Ecosystem (ongoing)

- [ ] Additional language support
- [ ] GitHub/GitLab connectors
- [ ] VS Code extension
- [ ] Documentation
- [ ] Performance optimization

---

## 13. Example Queries & Outputs

### 13.1 Basic Search

```bash
$ codemate search "authentication middleware"

📄 src/middleware/auth.rs:45-89 (main @ abc1234)
   Score: 0.92 | Author: alice@example.com | 2024-03-15
   
   pub async fn authenticate(req: Request) -> Result<User, AuthError> {
       let token = extract_bearer_token(&req)?;
       let claims = verify_jwt(token)?;
       // ... 40 more lines
   }

📄 src/handlers/login.rs:23-67 (main @ def5678)
   Score: 0.87 | Author: bob@example.com | 2024-02-20
   
   async fn handle_login(payload: LoginRequest) -> Response {
       let user = authenticate_user(&payload.email, &payload.password).await?;
       // ... 35 more lines
   }
```

### 13.2 Temporal Search

```bash
$ codemate search "rate limiting" after:2024-01-01 before:2024-03-01

Found 3 results in time range 2024-01-01 to 2024-03-01:

📄 src/middleware/rate_limit.rs:12-45 (main @ 789abc)
   Score: 0.95 | Author: charlie@example.com | 2024-02-10
   
📄 src/config/limits.rs:5-20 (feature/rate-limits @ 456def)
   Score: 0.82 | Author: alice@example.com | 2024-01-25
   
📄 tests/rate_limit_test.rs:30-80 (main @ 123ghi)
   Score: 0.78 | Author: bob@example.com | 2024-02-28
```

### 13.3 Cross-Branch Search

```bash
$ codemate search "feature flag" in:main,develop,feature/*

Results across 5 branches:

📄 src/flags/mod.rs:10-50 [main, develop]
   Score: 0.94 | Same content in both branches
   
📄 src/flags/evaluator.rs:25-100 [feature/flags-v2]
   Score: 0.91 | Modified in feature branch
   
📄 src/flags/mod.rs:10-60 [feature/flags-v2]
   Score: 0.88 | Extended version in feature branch
```

### 13.4 Graph Query

```bash
$ codemate graph callers "authenticate"

Functions that call authenticate():

├── src/handlers/login.rs:handle_login (5 calls)
├── src/handlers/api.rs:protected_endpoint (3 calls)
├── src/middleware/auth.rs:auth_middleware (2 calls)
└── tests/auth_test.rs:test_auth_flow (8 calls)

Total: 18 call sites in 4 files
```

### 13.5 Code Archaeology

```bash
$ codemate history "authenticate" --since 2024-01-01

Timeline for authenticate():

2024-03-15 (abc1234) alice@example.com
  └── Added async support, error handling improvements
  
2024-02-10 (def5678) bob@example.com
  └── Refactored token extraction
  
2024-01-05 (789ghi) alice@example.com
  └── Initial implementation
  
3 versions found | 2 authors | First seen: 2024-01-05
```

---

## 14. Future Directions

### 14.1 AI-Powered Features

- **Code summarization**: Generate natural language descriptions
- **Similarity detection**: Find semantically similar code
- **Refactoring suggestions**: Identify code smell patterns
- **Documentation generation**: Auto-generate from code

### 14.2 Collaboration Features

- **Shared indexes**: Team-wide code search
- **Access control**: Respect git permissions
- **Audit logging**: Track searches and access

### 14.3 Integration Ecosystem

- **IDE plugins**: VS Code, IntelliJ, Vim/Neovim
- **CI/CD hooks**: Index on push, search in pipelines
- **Code review**: Search in PR context
- **Documentation**: Link code to docs

---

## Appendix A: Supported Languages (Phase 1)

| Language | tree-sitter Grammar | Chunk Types |
|----------|---------------------|-------------|
| Rust | tree-sitter-rust | function, struct, enum, trait, impl, mod |
| Python | tree-sitter-python | function, class, method |
| TypeScript | tree-sitter-typescript | function, class, method, interface, type |
| JavaScript | tree-sitter-javascript | function, class, method |

## Appendix B: Configuration File

```toml
# ~/.config/codemate/config.toml

[index]
# Default branches to index
branches = ["main", "develop", "release/*"]
# Maximum commit depth (0 = unlimited)
max_depth = 1000
# Exclude patterns
exclude = ["node_modules", "target", ".git", "vendor"]

[embedding]
# Default embedding model
model = "jina-code-v2"
# Batch size for embedding
batch_size = 32

[search]
# Default result limit
limit = 10
# Minimum score threshold
min_score = 0.5
# Enable query expansion
expand_queries = true

[ollama]
# Ollama server URL (for query expansion)
url = "http://localhost:11434"
model = "qwen2.5:0.5b"

[server]
# HTTP API port
port = 4444
# Enable MCP server
mcp = true
```

## Appendix C: Glossary

| Term | Definition |
|------|------------|
| **CAS** | Content-Addressable Store - storage where content is identified by its hash |
| **Chunk** | A semantic unit of code (function, class, method, etc.) |
| **Location** | A specific occurrence of a chunk in git history |
| **RRF** | Reciprocal Rank Fusion - algorithm for combining ranked lists |
| **ANN** | Approximate Nearest Neighbor - fast vector similarity search |
| **FTS** | Full-Text Search - keyword-based text search |

---

*Document End*
