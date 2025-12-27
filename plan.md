# CodeMate Implementation Plan

**Project**: Semantic Code Understanding Engine  
**Version**: 0.1.0  
**Last Updated**: 2025-12-27

---

## Progress Summary

| Sprint | Focus | Completeness | Status |
|--------|-------|--------------|--------|
| Sprint 1 | MVP - Core Indexing & Basic Search | 0% | 🔜 Not Started |
| Sprint 2 | Git-Native Indexing | 0% | ⏳ Pending |
| Sprint 3 | Graph Index | 0% | ⏳ Pending |
| Sprint 4 | Query Layer | 0% | ⏳ Pending |
| Sprint 5 | External Interfaces | 0% | ⏳ Pending |

**Overall Progress**: 0% (Design Complete, Implementation Pending)

---

## Overview

This document outlines the implementation roadmap for CodeMate, organized into sprints. Each sprint delivers incremental, usable functionality.

---

## Sprint 1: MVP - Core Indexing & Basic Search

**Goal**: Index a local git repository and perform basic semantic search via CLI.

**Duration**: 2 weeks

### Deliverables

| Feature | Description | Priority |
|---------|-------------|----------|
| Project scaffolding | Rust workspace with cargo, CI setup | P0 |
| **Storage abstraction** | Trait-based storage layer (swappable backends) | P0 |
| File indexing | Parse files using tree-sitter, extract chunks | P0 |
| Content-addressable store | SQLite implementation of ChunkStore trait | P0 |
| Embedding generation | Generate embeddings using fastembed | P0 |
| Vector search | SQLite-vec implementation of VectorStore trait | P0 |
| CLI `index` command | `codemate index <path>` - index a directory | P0 |
| CLI `search` command | `codemate search <query>` - semantic search | P0 |

### Technical Scope

```
codemate/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── codemate-core/          # Core types, traits, storage
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── chunk.rs    # Chunk types
│   │   │   ├── storage/    # Storage abstraction
│   │   │   │   ├── mod.rs  # Trait definitions
│   │   │   │   ├── sqlite.rs   # SQLite backend
│   │   │   │   └── qdrant.rs   # Qdrant backend (future)
│   │   │   └── schema.sql  # SQLite schema
│   │   └── Cargo.toml
│   ├── codemate-parser/        # Tree-sitter parsing
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── languages/  # Language-specific extractors
│   │   └── Cargo.toml
│   ├── codemate-embeddings/    # Embedding generation
│   │   ├── src/
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   └── codemate-cli/           # CLI application
│       ├── src/
│       │   └── main.rs
│       └── Cargo.toml
└── tests/                  # Integration tests
```

### MVP User Stories

1. **As a developer**, I can run `codemate index .` to index my project
2. **As a developer**, I can run `codemate search "authentication"` to find relevant code
3. **As a developer**, I can see file path, line numbers, and code snippets in results

### Out of Scope for MVP

- Git history indexing (current HEAD only)
- Graph relationships (CALLS, IMPORTS)
- Full-text search (BM25)
- External symbol database
- HTTP API / MCP server
- Multi-repository support

---

## Sprint 2: Git-Native Indexing

**Goal**: Add git-aware indexing with commit tracking and blame attribution.

**Duration**: 2 weeks

### Deliverables

| Feature | Description | Priority |
|---------|-------------|----------|
| Git connector | Use git2 to read repository data | P0 |
| Commit walking | Index current branch history | P0 |
| Location index | Track chunk locations across commits | P0 |
| Blame attribution | Map chunks to original authors | P1 |
| CLI `history` command | Show chunk evolution over time | P1 |

---

## Sprint 3: Graph Index

**Goal**: Extract and query code relationships (call graph, imports).

**Duration**: 2 weeks

### Deliverables

| Feature | Description | Priority |
|---------|-------------|----------|
| Call graph extraction | CALLS edges from function calls | P0 |
| Import dependency extraction | IMPORTS edges | P0 |
| Graph storage | SQLite schema per PRD-edge-versioning | P0 |
| Symbol FQN | Implement FQN format per PRD-symbol-fqn-format | P0 |
| CLI `graph callers` | Find callers of a function | P1 |
| CLI `graph deps` | Show dependencies of a file | P1 |

---

## Sprint 4: Query Layer

**Goal**: Full query DSL with filters and multi-modal search.

**Duration**: 2 weeks

### Deliverables

| Feature | Description | Priority |
|---------|-------------|----------|
| Query parser | Parse DSL: `author:`, `after:`, `lang:`, etc. | P0 |
| Full-text search | FTS5 integration for symbol/comment search | P0 |
| Result fusion | RRF combining vector + FTS results | P1 |
| Branch filtering | `in:main,develop` filter | P1 |

---

## Sprint 5: External Interfaces

**Goal**: HTTP API and MCP server for tool integration.

**Duration**: 2 weeks

### Deliverables

| Feature | Description | Priority |
|---------|-------------|----------|
| HTTP API | axum server with JSON endpoints | P0 |
| MCP server | Claude Code integration via rmcp | P0 |
| Similarity queries | Per PRD-similarity-edges | P1 |
| External symbols | Per PRD-external-symbol-database | P2 |

---

## Technology Stack Summary

| Layer | Technology | Notes |
|-------|------------|-------|
| Language | Rust | Performance, memory safety |
| Async | Tokio | Concurrent I/O |
| **Storage (MVP)** | SQLite + WAL | Embedded, portable |
| **Storage (Alt)** | Qdrant | Swappable via config |
| Vector search | sqlite-vec / Qdrant | Per ADR-storage-abstraction |
| Full-text | SQLite FTS5 | Integrated BM25 |
| Parsing | tree-sitter | Multi-language |
| Git | git2 (libgit2) | Full git API |
| Embeddings | fastembed (ONNX) | Fast, no Python |
| CLI | clap | Feature-rich |
| HTTP | axum | Fast, async |
| MCP | rmcp | Claude integration |

---

## MVP Success Criteria

- [ ] `codemate index .` completes on a 10K LOC repo in < 60 seconds
- [ ] `codemate search <query>` returns results in < 500ms
- [ ] Top-5 search results are semantically relevant
- [ ] CLI output is readable and includes file paths + line numbers
- [ ] SQLite database is < 100MB for 10K LOC repo

---

## Risk & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| fastembed Rust bindings unstable | High | Fallback to Python subprocess |
| sqlite-vec performance | Medium | Benchmark early, switch to Qdrant if needed |
| tree-sitter language coverage | Low | Start with Rust, Python, TypeScript |

---

## Next Actions (Sprint 1)

1. [ ] Initialize Rust workspace with cargo
2. [ ] Set up GitHub repo with CI (cargo test, clippy)
3. [ ] Implement `codemate-core` with SQLite schema
4. [ ] Implement `codemate-parser` with tree-sitter for Rust
5. [ ] Implement `codemate-embeddings` with fastembed
6. [ ] Implement `codemate-cli` with `index` and `search` commands
7. [ ] Write integration tests for end-to-end workflow
8. [ ] Document CLI usage in README

---

## References

- [CodeMate Design Document](design/draft/semantic-code-engine-design.md)
- [PRD Index](PRD.md)
- [ADR: Storage Abstraction](design/decision/ADR-storage-abstraction.md)
- [PRD: Symbol FQN Format](design/decision/PRD-symbol-fqn-format.md)
- [PRD: Edge Versioning](design/decision/PRD-edge-versioning.md)
- [PRD: Similarity Edges](design/decision/PRD-similarity-edges.md)
- [PRD: External Symbol Database](design/decision/PRD-external-symbol-database.md)
