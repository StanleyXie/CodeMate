# External Symbol Database - Design Discussion

**Topic**: Strategy for indexing and querying external/library symbols  
**Date**: 2025-12-25  
**Status**: Decision Made  
**Related PRD**: [PRD-external-symbol-database.md](../decision/PRD-external-symbol-database.md)

---

## Problem Statement

User code references external symbols from libraries, frameworks, and standard libraries. These symbols are not in the indexed repository but are essential for:
- Complete call graphs ("what calls React.useState?")
- Dependency impact analysis ("what uses lodash.merge?")
- Security auditing ("find deprecated crypto APIs")
- API usage patterns analysis

---

## Options Considered

### Option 1: Don't Track External Symbols (Rejected)

Only index first-party code. External references are unresolved.

**Pros:**
- Simplest implementation
- No maintenance burden

**Cons:**
- Incomplete call graphs
- Cannot answer "what uses library X?"
- Security use cases impossible
- Graph has dangling edges

### Option 2: Full External Index (Rejected)

Pre-build complete index of all popular libraries.

**Pros:**
- Most complete coverage
- Fast lookups

**Cons:**
- Massive maintenance burden
- Storage explosion (npm has 2M+ packages)
- Version combinatorics
- Stale quickly

### Option 3: Minimal Curated + Auto-Discovery (Selected ✓)

Small curated set of popular symbols + automatic discovery from project dependencies.

**Pros:**
- Covers 80% use cases with 20% effort
- Auto-scales with project
- Community can contribute definitions
- Version-aware via lockfiles

**Cons:**
- Some gaps in coverage
- Requires dependency parsing

---

## Decision: Curated Core + Auto-Discovery

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      EXTERNAL SYMBOL DATABASE                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    CURATED SYMBOL REGISTRY                           │    │
│  │  • Standard library symbols (Rust std, Python builtins)              │    │
│  │  • Top 100 packages per ecosystem                                    │    │
│  │  • Security-relevant APIs                                            │    │
│  │  • Community-contributed definitions                                 │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                              │                                               │
│                              ▼                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    AUTO-DISCOVERY LAYER                              │    │
│  │  Parse lockfiles → Extract dependencies → Generate symbol stubs      │    │
│  │  Cargo.lock, package-lock.json, poetry.lock, go.sum                  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                              │                                               │
│                              ▼                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                    UNIFIED SYMBOL TABLE                              │    │
│  │  external_symbols: fqn → (kind, signature, doc_url, version)         │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Schema

```sql
-- External symbol definitions
CREATE TABLE external_symbols (
    fqn             TEXT PRIMARY KEY,  -- e.g., "rust:std::collections::HashMap"
    language        TEXT NOT NULL,
    package_name    TEXT NOT NULL,     -- e.g., "std", "react", "lodash"
    package_version TEXT,              -- NULL for "any version"
    
    kind            TEXT NOT NULL,     -- function, class, method, constant, type
    short_name      TEXT NOT NULL,     -- Last component
    signature       TEXT,              -- Full type signature
    doc_url         TEXT,              -- Link to official docs
    
    -- Metadata
    source          TEXT NOT NULL,     -- 'curated', 'auto', 'user'
    deprecated      BOOLEAN DEFAULT FALSE,
    security_note   TEXT,              -- e.g., "Use bcrypt instead"
    
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- Mapping from project dependencies to external symbols
CREATE TABLE project_dependencies (
    repo_uri        TEXT NOT NULL,
    package_name    TEXT NOT NULL,
    package_version TEXT NOT NULL,
    language        TEXT NOT NULL,
    lockfile_path   TEXT NOT NULL,
    indexed_at      TEXT NOT NULL,
    PRIMARY KEY (repo_uri, package_name, language)
);

CREATE INDEX idx_external_package ON external_symbols(package_name, language);
CREATE INDEX idx_external_deprecated ON external_symbols(deprecated) WHERE deprecated = TRUE;
```

### Curated Definition Format

```toml
# external_symbols/rust/std.toml
[package]
name = "std"
language = "rust"
source = "curated"
doc_base = "https://doc.rust-lang.org/std/"

[[symbols]]
fqn = "rust:std::collections::HashMap"
kind = "struct"
short_name = "HashMap"
signature = "struct HashMap<K, V, S = RandomState>"
doc_path = "collections/struct.HashMap.html"

[[symbols]]
fqn = "rust:std::collections::HashMap::new"
kind = "function"
short_name = "new"
signature = "pub fn new() -> HashMap<K, V, RandomState>"
doc_path = "collections/struct.HashMap.html#method.new"
```

```toml
# external_symbols/typescript/react.toml
[package]
name = "react"
language = "typescript"
source = "curated"
doc_base = "https://react.dev/reference/react/"

[[symbols]]
fqn = "typescript:react#useState"
kind = "function"
short_name = "useState"
signature = "function useState<S>(initialState: S | (() => S)): [S, Dispatch<SetStateAction<S>>]"
doc_path = "useState"

[[symbols]]
fqn = "typescript:react#useEffect"
kind = "function"
short_name = "useEffect"
signature = "function useEffect(effect: EffectCallback, deps?: DependencyList): void"
doc_path = "useEffect"
```

### Priority Coverage

| Language | Priority Packages |
|----------|-------------------|
| Rust | std, tokio, serde, clap, anyhow, thiserror |
| Python | builtins, os, sys, typing, asyncio, dataclasses |
| TypeScript | react, next, express, lodash, axios, zod |
| Go | fmt, net/http, encoding/json, context, sync |

---

## Open Questions for Future Discussion

### 1. Version Resolution Strategy

When project uses `react@18.2.0` but curated has generic `react`:

**Options**:
- Ignore version, match on package name
- Exact version match only
- Semver compatibility check

### 2. Auto-Discovery Depth

How deep to parse into type definitions?

**Current**: Top-level exports only
**Alternative**: Full type tree (expensive)

### 3. Symbol Stub Quality

Auto-discovered symbols lack signatures/docs. Worth fetching?

**Options**:
- Stub only (name + kind)
- Parse type definitions from node_modules/target
- Fetch from registry APIs

### 4. Community Contribution Workflow

How to accept community symbol definitions?

**Options**:
- Pull requests to curated TOML files
- npm-style package registry
- Auto-generate from DefinitelyTyped, docs.rs

---

## Discussion Log

### 2025-12-25 - Initial Decision

**Participants**: AI Assistant, User

**Summary**:
- Acknowledged that full external indexing is impractical
- Curated top-100 covers most common use cases
- Auto-discovery from lockfiles fills project-specific gaps
- Security use case is high value for minimal effort

**Key insight**: External symbols follow power law distribution. A small curated set (React, lodash, std) covers the vast majority of actual usage. Auto-discovery handles the long tail.
