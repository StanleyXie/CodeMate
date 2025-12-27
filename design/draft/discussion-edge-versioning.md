# Edge Versioning - Design Discussion

**Topic**: Temporal versioning strategy for graph edges  
**Date**: 2025-12-25  
**Status**: Decision Made  
**Related PRD**: [PRD-edge-versioning.md](../decision/PRD-edge-versioning.md)

---

## Problem Statement

Graph edges (CALLS, IMPORTS, REFERENCES) represent code relationships. These relationships change over time as code evolves. We need to decide:
- Do edges represent "current" state or "all time" history?
- How to enable temporal queries ("When did X start calling Y?")?
- Storage vs. query performance trade-offs

---

## Options Considered

### Option 1: Current State Only (Rejected)

Edges represent the current state of the code. On each index update, delete old edges and create new ones.

**Pros:**
- Simple storage model
- Fast queries (no temporal filters)
- Minimal storage

**Cons:**
- Cannot answer "When did X start calling Y?"
- Lose history on re-indexing
- No git-native temporal awareness

### Option 2: Full Temporal Versioning (Rejected)

Each edge has `valid_from_commit` and `valid_to_commit`, creating an edge for every commit where relationship exists.

**Pros:**
- Complete historical accuracy
- Can query any point in time

**Cons:**
- O(edges × commits) storage explosion
- Complex query logic (temporal joins)
- Re-indexing is very expensive

### Option 3: Hybrid - Current State + Change History (Selected ✓)

Edges represent current state with creation metadata. Separate append-only history table tracks changes.

**Pros:**
- Fast current-state queries
- Temporal queries via join
- Storage efficient (only changes stored)
- Incremental updates possible

**Cons:**
- Two tables to maintain
- Slightly more complex schema

---

## Decision: Hybrid Current + History

### Schema Design

```sql
-- Primary edge table: current state
CREATE TABLE graph_edges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id       TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    edge_type       TEXT NOT NULL,
    
    -- Creation metadata
    created_commit  TEXT NOT NULL,     -- Commit where edge first appeared
    created_at      TEXT NOT NULL,     -- Timestamp of that commit
    
    properties      TEXT,              -- JSON: weight, confidence, etc.
    
    FOREIGN KEY (source_id) REFERENCES graph_nodes(node_id),
    FOREIGN KEY (target_id) REFERENCES graph_nodes(node_id),
    UNIQUE (source_id, target_id, edge_type)
);

-- Append-only edge history for temporal queries
CREATE TABLE edge_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    edge_source     TEXT NOT NULL,
    edge_target     TEXT NOT NULL,
    edge_type       TEXT NOT NULL,
    
    event_type      TEXT NOT NULL,     -- 'created', 'deleted'
    commit_hash     TEXT NOT NULL,
    authored_at     TEXT NOT NULL,
    author_email    TEXT,
    
    -- Optional: snapshot of properties at this point
    properties      TEXT,
    
    recorded_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_edge_history_commit ON edge_history(commit_hash);
CREATE INDEX idx_edge_history_time ON edge_history(authored_at);
CREATE INDEX idx_edge_history_lookup ON edge_history(edge_source, edge_target, edge_type);
```

### Query Examples

**Current state**: "What does function X call?"
```sql
SELECT target_id FROM graph_edges 
WHERE source_id = 'chunk:abc123' AND edge_type = 'CALLS';
```

**Temporal**: "When did X start calling Y?"
```sql
SELECT commit_hash, authored_at FROM edge_history
WHERE edge_source = 'chunk:abc123' 
  AND edge_target = 'chunk:def456'
  AND edge_type = 'CALLS'
  AND event_type = 'created'
ORDER BY authored_at ASC
LIMIT 1;
```

**Historical snapshot**: "What did X call as of commit Z?"
```sql
-- Get all edges that existed at commit Z
-- (created before Z and not deleted before Z)
WITH edge_state AS (
    SELECT edge_source, edge_target, edge_type,
           MAX(CASE WHEN event_type = 'created' THEN authored_at END) as created,
           MAX(CASE WHEN event_type = 'deleted' THEN authored_at END) as deleted
    FROM edge_history
    WHERE edge_source = 'chunk:abc123'
      AND authored_at <= (SELECT authored_at FROM commits WHERE hash = :commit_z)
    GROUP BY edge_source, edge_target, edge_type
)
SELECT * FROM edge_state
WHERE created IS NOT NULL 
  AND (deleted IS NULL OR deleted < created);
```

---

## Open Questions for Future Discussion

### 1. Edge Modification Events

Current design tracks 'created' and 'deleted'. Should we also track 'modified'?

**Use case**: Edge property changes (e.g., call weight/frequency changes)

**Trade-off**: More events vs. simpler model

### 2. History Retention Policy

How long to keep edge history?

**Options**:
- Forever (complete audit trail)
- Rolling window (last N months)
- Per-repository config

### 3. Branch-Specific Edges

Current design tracks commit-level changes. How to handle branch divergence?

**Example**: Feature branch adds edge, gets reverted on merge

**Options**:
- Ignore branches, only track commit ancestry
- Track branch context in edge_history
- Separate edge tables per branch (expensive)

---

## Discussion Log

### 2025-12-25 - Initial Decision

**Participants**: AI Assistant, User

**Summary**:
- Analyzed storage costs: full temporal would be O(edges × commits)
- For a 100K edge graph with 10K commits, that's 1B edge-commits
- Hybrid approach estimates ~5% overhead (only storing changes)
- Selected hybrid for practical balance

**Key insight**: Code relationships don't change frequently. Most edges are stable across many commits. Storing only deltas is dramatically more efficient.
