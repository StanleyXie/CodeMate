# Discussion: Module-Level Dependency Analysis

**Date**: 2025-12-29  
**Participants**: CodeMate Team

## Context

During tree graph optimization work, we identified that function-level dependency trees are too verbose for understanding high-level architecture. Users need module/crate-level views.

## Key Decisions

### Decision 1: Project Detection Strategy

**Options Considered:**
1. **File marker detection** - Look for `Cargo.toml`, `package.json`, etc.
2. **Directory-based heuristics** - Treat each directory as a module
3. **Explicit configuration** - Require user to define module boundaries

**Decision**: **Option 1 (File marker detection)**

**Rationale**: 
- Works out-of-box without configuration
- Aligns with language conventions
- Can be supplemented with Option 3 for edge cases

---

### Decision 2: Database Schema Approach

**Options Considered:**
1. **Separate `modules` table** - Explicit module entities with foreign keys
2. **Computed from file paths** - Derive module from chunk file path at query time
3. **Metadata column** - Add `module_id` column to chunks only

**Decision**: **Option 1 (Separate modules table)**

**Rationale**:
- Enables storing module metadata (name, type, parent)
- Better query performance with pre-computed relationships
- Supports hierarchical modules (workspace → crate → module)

---

### Decision 3: CLI Command Structure

**Options Considered:**
1. **New subcommand**: `codemate graph modules`
2. **Flag on existing**: `codemate graph tree --level=module`
3. **Separate command**: `codemate modules`

**Decision**: **Option 1 (New subcommand)**

**Rationale**:
- Clear separation of concerns
- Consistent with existing `graph tree`, `graph deps`, `graph callers`
- Allows module-specific options without cluttering other commands

---

### Decision 4: C4 Model Level Mapping

| C4 Level | CodeMate Equivalent | Command |
|----------|-------------------|---------|
| Context | Entire indexed repo | N/A |
| Container | Crates/Packages | `graph modules --level=crate` |
| Component | Modules within crate | `graph modules --level=module` |
| Code | Functions/structs | `graph tree` (current) |

---

## Open Questions (Resolved)

### Q1: How to name modules without explicit names?

**Resolution**: Use the directory path relative to project root.
- Example: `crates/codemate-core/src/storage` → `codemate-core::storage`

### Q2: How to handle test modules?

**Resolution**: Include by default with a `--no-tests` flag to exclude.
- Test detection: files in `tests/`, `*_test.rs`, `test_*.py`

### Q3: What about external dependencies?

**Resolution**: Out of scope for this PRD. External symbols are tracked but not resolved to external modules. See PRD-external-symbol-database for future work.

---

## Implementation Notes

### Phase 1 Focus
- Start with Rust (Cargo.toml) as it's the primary language
- Add Python support in parallel
- Other languages can follow the same pattern

### Performance Considerations
- Module detection should happen during indexing, not query time
- Pre-compute module edges as a materialized view or separate table
- Incremental updates when files change

---

## References

- [PRD-module-level-dependencies](decision/PRD-module-level-dependencies.md)
- [C4 Model](https://c4model.com/)
- Current tree optimization work in `crates/codemate-cli/src/commands/graph.rs`
