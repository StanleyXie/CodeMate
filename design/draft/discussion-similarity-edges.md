# Similarity Edges - Design Discussion

**Topic**: Strategy for computing and storing semantic similarity relationships  
**Date**: 2025-12-25  
**Status**: Decision Made  
**Related PRD**: [PRD-similarity-edges.md](../decision/PRD-similarity-edges.md)

---

## Problem Statement

CodeMate can identify semantically similar code chunks via embedding cosine similarity. This enables:
- Duplicate/clone detection
- "Find similar" functionality
- Refactoring suggestions

We need to decide when and how to compute these relationships.

---

## Options Considered

### Option 1: Full Pre-computation (Rejected)

Compute and store similarity edges for all chunk pairs above threshold.

**Pros:**
- Instant O(1) lookup
- No query-time computation
- Can pre-index for graph traversal

**Cons:**
- O(n²) storage explosion
  - 100K chunks = 5 billion pairs to check
  - Even at 0.1% similarity rate = 5M edges
- Stale immediately when embeddings change
- Expensive to maintain on updates

### Option 2: Pure Query-Time (Rejected)

Compute similarity on every query using vector ANN search.

**Pros:**
- Always fresh
- Minimal storage
- Works with any threshold

**Cons:**
- Latency on every query
- Cannot pre-build similarity graph
- Repeated computation for same queries

### Option 3: Query-Time with Intelligent Caching (Selected ✓)

Compute at query time, cache results, with optional pre-compute for specific use cases.

**Pros:**
- Fresh by default
- Popular queries become fast
- Storage scales with usage, not data size
- Optional pre-compute for "find duplicates" workflow

**Cons:**
- First query for a chunk is slower
- Cache management complexity

---

## Decision: Query-Time + LRU Cache + Optional Pre-compute

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    SIMILARITY QUERY FLOW                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Query: "Find similar to chunk:abc123"                          │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    CACHE CHECK                           │    │
│  │  Key: (chunk_hash, threshold, limit)                     │    │
│  │  Hit? → Return cached results                            │    │
│  └───────────────────────┬─────────────────────────────────┘    │
│       │ Miss                                                     │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    VECTOR SEARCH                         │    │
│  │  sqlite-vec ANN search for top-K similar                 │    │
│  │  Filter by threshold (default 0.85)                      │    │
│  └───────────────────────┬─────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    CACHE STORE                           │    │
│  │  LRU cache with configurable size                        │    │
│  │  TTL: configurable (default 1 hour)                      │    │
│  └───────────────────────┬─────────────────────────────────┘    │
│       │                                                          │
│       ▼                                                          │
│  Results: [(chunk:def456, 0.94), (chunk:ghi789, 0.91), ...]     │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Schema

```sql
-- Similarity cache table (can be cleared without data loss)
CREATE TABLE similarity_cache (
    source_hash     TEXT NOT NULL,
    target_hash     TEXT NOT NULL,
    similarity      REAL NOT NULL,
    threshold_used  REAL NOT NULL,
    computed_at     TEXT NOT NULL,
    PRIMARY KEY (source_hash, target_hash)
);

-- Index for cache lookups and LRU eviction
CREATE INDEX idx_similarity_source ON similarity_cache(source_hash, similarity DESC);
CREATE INDEX idx_similarity_time ON similarity_cache(computed_at);
```

### Configuration

```toml
[similarity]
# Default similarity threshold for "find similar" queries
default_threshold = 0.85

# Cache settings
cache_max_entries = 100000      # ~10MB for 100K pairs
cache_ttl_seconds = 3600        # 1 hour

# Pre-compute settings (for duplicate detection)
precompute_enabled = false
precompute_threshold = 0.95     # Higher threshold for duplicates
precompute_batch_size = 1000    # Chunks per batch
```

### CLI Commands

```bash
# Query-time similarity (uses cache)
codemate similar chunk:abc123 --threshold 0.9 --limit 10

# Pre-compute duplicates (optional workflow)
codemate duplicates detect --threshold 0.95 --output duplicates.json

# Cache management
codemate cache clear --type similarity
codemate cache stats
```

---

## Open Questions for Future Discussion

### 1. Cross-Language Similarity

Should similarity work across languages?

**Use case**: Find Python code similar to a Rust implementation

**Challenge**: Different embedding spaces per language may not be comparable

**Options**:
- Use universal code embedding model (e.g., StarCoder)
- Separate language-specific similarity
- Cross-language via code2text embedding

### 2. Similarity Graph Integration

Should cached similarities be queryable as graph edges?

**Use case**: "Find all chunks within 2 hops of similarity"

**Challenge**: Cache is ephemeral, graph expects persistent edges

**Options**:
- Promote high-confidence cached pairs to real edges
- Virtual edge type that queries cache on traversal
- Separate similarity graph (materialized view)

### 3. Incremental Cache Invalidation

When a chunk's embedding changes, how to invalidate cache?

**Current approach**: TTL-based expiration

**Alternative**: Track embedding version, invalidate on version change

---

## Discussion Log

### 2025-12-25 - Initial Decision

**Participants**: AI Assistant, User

**Summary**:
- Calculated storage costs for pre-computation: prohibitive at scale
- Noted that similarity is not frequently queried for most chunks
- Cache hit rate will be high for "hot" chunks (frequently viewed)
- Selected query-time + cache as default with opt-in pre-compute

**Key insight**: Similarity is fundamentally different from structural edges (CALLS, IMPORTS). Structural edges are discovered during indexing and are stable. Similarity is a derived property that depends on threshold and can be computed on demand.
