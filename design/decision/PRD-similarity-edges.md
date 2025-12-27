# PRD: Similarity Edges

**Product**: CodeMate (Semantic Code Understanding Engine)  
**Component**: Graph Store / Vector Search  
**Version**: 1.0  
**Date**: 2025-12-25  
**Status**: Approved  
**Discussion Log**: [discussion-similarity-edges.md](../draft/discussion-similarity-edges.md)

---

## Overview

This PRD defines the strategy for computing, caching, and querying semantic similarity between code chunks. Similarity enables duplicate detection, "find similar" functionality, and refactoring suggestions.

## Requirements

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-001 | Find top-K similar chunks to a given chunk | P0 |
| FR-002 | Configurable similarity threshold | P0 |
| FR-003 | Results reflect current embeddings | P0 |
| FR-004 | CLI command for duplicate detection | P1 |
| FR-005 | Similarity cache for repeated queries | P1 |

### Non-Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-001 | Cached queries return in < 10ms | P0 |
| NFR-002 | Uncached queries return in < 500ms | P1 |
| NFR-003 | Cache size configurable, default < 100MB | P1 |

---

## Specification

### Architecture: Query-Time + LRU Cache

```
Query: "Find similar to chunk:abc123"
           │
           ▼
    ┌──────────────┐     HIT     ┌──────────────┐
    │ Cache Lookup │────────────▶│ Return Cache │
    └──────────────┘             └──────────────┘
           │ MISS
           ▼
    ┌──────────────┐
    │ Vector ANN   │  ← sqlite-vec k-NN search
    │ Search       │
    └──────────────┘
           │
           ▼
    ┌──────────────┐
    │ Cache Store  │  ← LRU eviction
    └──────────────┘
           │
           ▼
    ┌──────────────┐
    │ Return       │
    └──────────────┘
```

### Schema

```sql
-- Ephemeral cache (can be dropped without data loss)
CREATE TABLE similarity_cache (
    source_hash     TEXT NOT NULL,
    target_hash     TEXT NOT NULL,
    similarity      REAL NOT NULL,
    model_id        TEXT NOT NULL,      -- Embedding model version
    computed_at     TEXT NOT NULL,
    PRIMARY KEY (source_hash, target_hash)
);

CREATE INDEX idx_sim_source ON similarity_cache(source_hash, similarity DESC);
CREATE INDEX idx_sim_time ON similarity_cache(computed_at);
```

### Configuration

```toml
[similarity]
default_threshold = 0.85       # Minimum cosine similarity
default_limit = 10             # Max results per query
cache_max_entries = 100000     # ~10MB memory
cache_ttl_seconds = 3600       # 1 hour TTL
```

### API

```rust
pub struct SimilarityQuery {
    pub source: ContentHash,
    pub threshold: f32,
    pub limit: usize,
}

pub struct SimilarityResult {
    pub target: ContentHash,
    pub similarity: f32,
    pub from_cache: bool,
}

pub trait SimilarityService {
    async fn find_similar(&self, query: SimilarityQuery) -> Vec<SimilarityResult>;
    async fn find_duplicates(&self, threshold: f32) -> Vec<DuplicateGroup>;
    fn invalidate_cache(&self, hash: &ContentHash);
    fn clear_cache(&self);
}
```

### CLI Commands

```bash
# Find similar chunks
codemate similar <chunk-id> [--threshold 0.9] [--limit 20]

# Detect duplicates
codemate duplicates detect [--threshold 0.95] [--output json|table]

# Cache management
codemate cache status
codemate cache clear --type similarity
```

---

## Acceptance Criteria

- [ ] `codemate similar` returns results ordered by similarity
- [ ] Repeated queries hit cache
- [ ] Cache respects TTL and max entries
- [ ] `codemate duplicates` identifies near-identical chunks
- [ ] Cache invalidated when embeddings re-computed

---

## Future Considerations

- Cross-language similarity
- Similarity as graph edge type
- Incremental cache invalidation on embedding changes

---

## References

- [Discussion Log](../draft/discussion-similarity-edges.md)
- [CodeMate Design Doc](../draft/semantic-code-engine-design.md) §4.1 Vector Store
