# PRD: Edge Versioning

**Product**: CodeMate (Semantic Code Understanding Engine)  
**Component**: Graph Store  
**Version**: 1.0  
**Date**: 2025-12-25  
**Status**: Approved  
**Discussion Log**: [discussion-edge-versioning.md](../draft/discussion-edge-versioning.md)

---

## Overview

This PRD defines the temporal versioning strategy for graph edges. Edges represent code relationships (CALLS, IMPORTS, REFERENCES) that change over time as code evolves.

## Requirements

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-001 | Current-state queries must be O(1) lookup | P0 |
| FR-002 | Support "when was edge created" queries | P0 |
| FR-003 | Support "when was edge deleted" queries | P1 |
| FR-004 | Support point-in-time snapshot queries | P2 |
| FR-005 | Edge history must survive re-indexing | P1 |

### Non-Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-001 | Storage overhead < 10% of edge count | P1 |
| NFR-002 | Current-state queries < 1ms | P0 |
| NFR-003 | Temporal queries < 100ms | P1 |

---

## Specification

### Architecture: Hybrid Current + History

```
┌─────────────────────────────────────┐
│         graph_edges (current)       │  ← Fast current-state
├─────────────────────────────────────┤
│ source_id | target_id | edge_type   │
│ created_commit | properties         │
└──────────────────┬──────────────────┘
                   │ references
                   ▼
┌─────────────────────────────────────┐
│         edge_history (changes)      │  ← Append-only log
├─────────────────────────────────────┤
│ source | target | type | event      │
│ commit_hash | authored_at           │
└─────────────────────────────────────┘
```

### Schema

```sql
-- Current state (mutable)
CREATE TABLE graph_edges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id       TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    edge_type       TEXT NOT NULL,
    created_commit  TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    properties      TEXT,
    FOREIGN KEY (source_id) REFERENCES graph_nodes(node_id),
    FOREIGN KEY (target_id) REFERENCES graph_nodes(node_id),
    UNIQUE (source_id, target_id, edge_type)
);

-- Change history (append-only)
CREATE TABLE edge_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    edge_source     TEXT NOT NULL,
    edge_target     TEXT NOT NULL,
    edge_type       TEXT NOT NULL,
    event_type      TEXT NOT NULL CHECK (event_type IN ('created', 'deleted')),
    commit_hash     TEXT NOT NULL,
    authored_at     TEXT NOT NULL,
    author_email    TEXT,
    properties      TEXT,
    recorded_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_edges_source ON graph_edges(source_id, edge_type);
CREATE INDEX idx_edges_target ON graph_edges(target_id, edge_type);
CREATE INDEX idx_history_commit ON edge_history(commit_hash);
CREATE INDEX idx_history_time ON edge_history(authored_at);
CREATE INDEX idx_history_edge ON edge_history(edge_source, edge_target, edge_type);
```

### Event Types

| Event | Meaning |
|-------|---------|
| `created` | Edge appeared in codebase |
| `deleted` | Edge removed from codebase |

### API

```rust
pub trait EdgeStore {
    // Current state
    fn get_outgoing(&self, node: &NodeId, edge_type: EdgeType) -> Vec<Edge>;
    fn get_incoming(&self, node: &NodeId, edge_type: EdgeType) -> Vec<Edge>;
    
    // Temporal queries
    fn edge_created_at(&self, edge: &EdgeId) -> Option<CommitInfo>;
    fn edge_deleted_at(&self, edge: &EdgeId) -> Option<CommitInfo>;
    fn edges_at_commit(&self, node: &NodeId, commit: &CommitHash) -> Vec<Edge>;
}
```

---

## Acceptance Criteria

- [ ] Current-state queries do not join edge_history
- [ ] Edge creation records history event
- [ ] Edge deletion records history event and removes from graph_edges
- [ ] Temporal CLI commands work: `codemate history`, `codemate blame`
- [ ] Re-indexing preserves history

---

## Future Considerations

- Edge property modification tracking
- Branch-specific edge views
- History retention policies

---

## References

- [Discussion Log](../draft/discussion-edge-versioning.md)
- [CodeMate Design Doc](../draft/semantic-code-engine-design.md) §4.3 Graph Index
