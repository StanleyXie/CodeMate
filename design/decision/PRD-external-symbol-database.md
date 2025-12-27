# PRD: External Symbol Database

**Product**: CodeMate (Semantic Code Understanding Engine)  
**Component**: Graph Store  
**Version**: 1.0  
**Date**: 2025-12-25  
**Status**: Approved  
**Discussion Log**: [discussion-external-symbol-database.md](../draft/discussion-external-symbol-database.md)

---

## Overview

This PRD defines the strategy for indexing external symbols (stdlib, libraries, frameworks) that are referenced by indexed code but not present in the repository. This enables complete call graphs and dependency analysis.

## Requirements

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-001 | Resolve external symbol references to known signatures | P0 |
| FR-002 | Query "find all code calling library.function" | P0 |
| FR-003 | Auto-discover dependencies from lockfiles | P1 |
| FR-004 | Support community-contributed symbol definitions | P2 |
| FR-005 | Flag deprecated/insecure external APIs | P1 |

### Non-Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-001 | Curated symbols cover top 80% usage | P1 |
| NFR-002 | External symbol lookup < 1ms | P0 |
| NFR-003 | Storage < 50MB for curated definitions | P1 |

---

## Specification

### Architecture: Curated Core + Auto-Discovery

```
┌──────────────────────────────────────────────────────────────┐
│                     EXTERNAL SYMBOLS                          │
├──────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              CURATED REGISTRY                            │ │
│  │  external_symbols/rust/std.toml                          │ │
│  │  external_symbols/typescript/react.toml                  │ │
│  │  external_symbols/python/builtins.toml                   │ │
│  └───────────────────────┬─────────────────────────────────┘ │
│                          │                                    │
│                          ▼                                    │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              AUTO-DISCOVERY                              │ │
│  │  Cargo.lock → extract crate names                        │ │
│  │  package-lock.json → extract package names               │ │
│  │  poetry.lock → extract package names                     │ │
│  └───────────────────────┬─────────────────────────────────┘ │
│                          │                                    │
│                          ▼                                    │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              UNIFIED TABLE                               │ │
│  │  external_symbols → fqn, kind, signature, doc_url        │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
└──────────────────────────────────────────────────────────────┘
```

### Schema

```sql
CREATE TABLE external_symbols (
    fqn             TEXT PRIMARY KEY,
    language        TEXT NOT NULL,
    package_name    TEXT NOT NULL,
    package_version TEXT,              -- NULL = any version
    kind            TEXT NOT NULL,
    short_name      TEXT NOT NULL,
    signature       TEXT,
    doc_url         TEXT,
    source          TEXT NOT NULL CHECK (source IN ('curated', 'auto', 'user')),
    deprecated      BOOLEAN DEFAULT FALSE,
    security_note   TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE TABLE project_dependencies (
    repo_uri        TEXT NOT NULL,
    package_name    TEXT NOT NULL,
    package_version TEXT NOT NULL,
    language        TEXT NOT NULL,
    lockfile_path   TEXT NOT NULL,
    indexed_at      TEXT NOT NULL,
    PRIMARY KEY (repo_uri, package_name, language)
);

CREATE INDEX idx_ext_package ON external_symbols(package_name, language);
CREATE INDEX idx_ext_deprecated ON external_symbols(deprecated) WHERE deprecated;
```

### Definition Format

```toml
# external_symbols/rust/tokio.toml
[package]
name = "tokio"
language = "rust"
source = "curated"
doc_base = "https://docs.rs/tokio/latest/tokio/"

[[symbols]]
fqn = "rust:tokio::spawn"
kind = "function"
short_name = "spawn"
signature = "pub fn spawn<F>(future: F) -> JoinHandle<F::Output>"
doc_path = "fn.spawn.html"

[[symbols]]
fqn = "rust:tokio::runtime::Runtime::new"
kind = "function"
short_name = "new"
signature = "pub fn new() -> io::Result<Runtime>"
doc_path = "runtime/struct.Runtime.html#method.new"
```

### Priority Packages

| Language | Curated Packages |
|----------|------------------|
| Rust | std, tokio, serde, clap, anyhow, thiserror, tracing |
| Python | builtins, os, sys, typing, asyncio, collections, json |
| TypeScript | react, next, express, lodash, axios, zod, prisma |
| Go | fmt, net/http, encoding/json, context, sync, io |

### API

```rust
pub trait ExternalSymbolDb {
    fn lookup(&self, fqn: &Fqn) -> Option<ExternalSymbol>;
    fn find_callers(&self, fqn: &Fqn, repo: &RepoUri) -> Vec<ChunkLocation>;
    fn find_deprecated_usage(&self, repo: &RepoUri) -> Vec<DeprecatedUsage>;
    fn sync_lockfile(&self, repo: &RepoUri, lockfile: &Path) -> Result<()>;
}
```

### CLI Commands

```bash
# Sync project dependencies
codemate externals sync [--lockfile Cargo.lock]

# Query external symbol usage
codemate externals usage react.useState
codemate externals usage tokio::spawn --repo .

# Security check
codemate externals deprecated --severity high
```

---

## Acceptance Criteria

- [ ] Curated definitions load at startup
- [ ] Lockfile sync populates project_dependencies
- [ ] `codemate externals usage` returns calling code
- [ ] Graph edges cross first-party → external boundary
- [ ] Deprecated API warnings work

---

## Future Considerations

- Version-aware symbol resolution
- Deep type introspection from type definitions
- Community contribution workflow

---

## References

- [Discussion Log](../draft/discussion-external-symbol-database.md)
- [CodeMate Design Doc](../draft/semantic-code-engine-design.md) §4.3 Graph Index
