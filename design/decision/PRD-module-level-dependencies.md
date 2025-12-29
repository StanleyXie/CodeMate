# PRD: Module-Level Dependency Analysis

**Status**: Proposed  
**Author**: CodeMate Team  
**Date**: 2025-12-29

## Problem Statement

Currently, CodeMate's `graph tree` command displays dependencies at the **function/symbol level**, which produces verbose output with hundreds of nodes. Users need a higher-level view to understand:

1. How modules/crates depend on each other
2. Which packages are entry points vs. libraries
3. Cross-module coupling and architectural boundaries

## Goals

1. **Detect project structure** - Identify crates, packages, and modules from file system markers
2. **Aggregate dependencies** - Roll up function-level edges to module-level edges
3. **Visualize at multiple levels** - Support C4 model abstraction levels (Container, Component, Code)

## Non-Goals

- External dependency analysis (crates.io, npm packages) - handled by PRD-external-symbol-database
- Runtime dependency analysis (dynamic dispatch)
- Build system integration (Cargo, npm commands)

## User Stories

### US1: View Crate Dependencies
```bash
codemate graph modules --database project.db

# Output:
[crate] codemate-cli
        → codemate-core (47 deps)
        → codemate-embeddings (12 deps)
        → codemate-parser (23 deps)

[crate] codemate-server
        → codemate-core (35 deps)
```

### US2: Drill Down to Module Level
```bash
codemate graph modules --level=module --database project.db

# Output:
[module] codemate-core::storage
         → codemate-core::chunk (15 deps)
         → codemate-embeddings (8 deps)
```

### US3: Filter by Language
```bash
codemate graph modules --lang=rust --database project.db
```

## Technical Design

### Project Detection

| Language | Project Marker | Module Marker |
|----------|---------------|---------------|
| Rust | `Cargo.toml` | `mod.rs`, `lib.rs`, directory |
| Python | `pyproject.toml`, `setup.py` | `__init__.py` |
| TypeScript/JS | `package.json` | `index.ts/js` |
| Go | `go.mod` | Directory with `.go` files |
| Java | `pom.xml`, `build.gradle` | Package directory |
| Terraform | Root `.tf` directory | Subdirectories |

### Database Schema Changes

```sql
-- New table for modules
CREATE TABLE modules (
    id TEXT PRIMARY KEY,              -- e.g., "codemate-core"
    name TEXT NOT NULL,
    path TEXT NOT NULL,               -- Relative path from index root
    language TEXT NOT NULL,
    project_type TEXT NOT NULL,       -- 'crate', 'package', 'module', 'workspace'
    parent_id TEXT,                   -- For nested modules
    FOREIGN KEY (parent_id) REFERENCES modules(id)
);

-- Link chunks to their containing module
ALTER TABLE chunks ADD COLUMN module_id TEXT REFERENCES modules(id);

-- Aggregated module-level edges (computed)
CREATE VIEW module_edges AS
SELECT 
    src_chunk.module_id AS source_module,
    tgt_chunk.module_id AS target_module,
    COUNT(*) AS edge_count
FROM edges e
JOIN chunks src_chunk ON e.source_hash = src_chunk.content_hash
JOIN chunks tgt_chunk ON e.target_query IN (SELECT symbol_name FROM chunks WHERE content_hash = tgt_chunk.content_hash)
WHERE src_chunk.module_id != tgt_chunk.module_id
GROUP BY src_chunk.module_id, tgt_chunk.module_id;
```

### API Changes

#### CLI
```bash
# New subcommand
codemate graph modules [OPTIONS] --database <DB>

Options:
  --level <LEVEL>     Abstraction level: crate|module [default: crate]
  --lang <LANG>       Filter by language
  --format <FORMAT>   Output format: tree|dot|json [default: tree]
```

#### REST API
```
POST /api/v1/graph/modules
{
  "level": "crate",
  "language": "rust"
}
```

#### MCP Tool
```json
{
  "name": "get_module_dependencies",
  "parameters": {
    "level": "crate",
    "language": "rust"
  }
}
```

## Implementation Phases

### Phase 1: Module Detection (Week 1)
- [ ] Add `modules` table to schema
- [ ] Implement project marker detection during indexing
- [ ] Associate chunks with their containing module

### Phase 2: Module Graph (Week 2)
- [ ] Implement `module_edges` view
- [ ] Add `codemate graph modules` CLI command
- [ ] Add REST and MCP endpoints

### Phase 3: Visualization (Week 3)
- [ ] Add DOT/Graphviz output format
- [ ] Add JSON output for programmatic use
- [ ] Add cycle detection at module level

## Success Metrics

- Module detection accuracy > 95% for supported languages
- Tree output line count reduced by 10x for large projects
- All existing tests continue to pass

## Open Questions

1. **How to handle monorepos?** - Detect workspace markers (Cargo.toml with [workspace])
2. **What about vendored dependencies?** - Exclude common vendor paths (node_modules, vendor/)
3. **Naming convention for anonymous modules?** - Use directory path as fallback

## References

- [C4 Model](https://c4model.com/) - Software architecture visualization levels
- [Pydeps](https://github.com/thebjorn/pydeps) - Python module dependency visualization
- [cargo-depgraph](https://crates.io/crates/cargo-depgraph) - Rust crate dependency graphs
- [Tach](https://gauge.sh) - Python module boundary enforcement
