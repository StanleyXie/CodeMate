# PRD: Symbol Fully-Qualified Name Format

**Product**: CodeMate (Semantic Code Understanding Engine)  
**Component**: Graph Store  
**Version**: 1.0  
**Date**: 2025-12-25  
**Status**: Approved  
**Discussion Log**: [discussion-symbol-fqn-format.md](../draft/discussion-symbol-fqn-format.md)

---

## Overview

This PRD defines the format for fully-qualified names (FQN) used to identify code symbols across the CodeMate system. FQNs are the primary identifier for graph nodes, cross-repository search, and external symbol resolution.

## Requirements

### Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-001 | FQN must uniquely identify a symbol within a language | P0 |
| FR-002 | FQN must preserve language-specific module semantics | P0 |
| FR-003 | FQN must be parseable to extract language and components | P0 |
| FR-004 | FQN must be suitable for use as database primary key | P0 |
| FR-005 | FQN format must be extensible to new languages | P1 |

### Non-Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| NFR-001 | FQN parsing must complete in < 1μs | P1 |
| NFR-002 | FQN should be human-readable | P2 |

---

## Specification

### Format

```
<fqn> ::= <language> ":" <native_path>
<language> ::= "rust" | "python" | "typescript" | "go" | "java" | ...
<native_path> ::= language-specific path string
```

### Language Conventions

| Language | Native Path Format | Example |
|----------|-------------------|---------|
| Rust | `<crate>::<path>::<item>` | `rust:myapp::auth::handlers::authenticate` |
| Python | `<package>.<module>.<class>.<method>` | `python:myapp.auth.handlers.AuthHandler.login` |
| TypeScript | `<module>#<export>[.<member>]` | `typescript:@org/auth#AuthService.validate` |
| Go | `<package>.<symbol>` | `go:github.com/org/repo/auth.Authenticate` |
| Java | `<fqcn>[#<method>]` | `java:com.org.auth.AuthService#authenticate` |

### Schema

```sql
CREATE TABLE symbols (
    fqn             TEXT PRIMARY KEY,
    language        TEXT NOT NULL,
    kind            TEXT NOT NULL,     -- function, class, method, trait, const
    short_name      TEXT NOT NULL,     -- Terminal component
    signature       TEXT,
    doc_summary     TEXT,
    is_external     BOOLEAN DEFAULT FALSE,
    first_seen_at   TEXT NOT NULL,
    last_seen_at    TEXT NOT NULL
);

CREATE INDEX idx_symbols_name ON symbols(short_name);
CREATE INDEX idx_symbols_lang_kind ON symbols(language, kind);
```

### Parsing API

```rust
pub struct Fqn {
    pub language: Language,
    pub native_path: String,
}

impl Fqn {
    pub fn parse(s: &str) -> Result<Self, FqnParseError>;
    pub fn short_name(&self) -> &str;
    pub fn parent(&self) -> Option<Fqn>;
    pub fn components(&self) -> Vec<&str>;
}
```

---

## Acceptance Criteria

- [ ] FQN parser handles all 5 initial languages
- [ ] Round-trip: `parse(fqn.to_string()) == fqn`
- [ ] Graph edges use FQN for source/target IDs
- [ ] Search results include parsed FQN components
- [ ] External symbol database uses same FQN format

---

## Future Considerations

- Repository-scoped FQN: `github.com/org/repo:rust:crate::foo`
- Version-qualified FQN for external symbols
- Symbol visibility annotations

---

## References

- [Discussion Log](../draft/discussion-symbol-fqn-format.md)
- [CodeMate Design Doc](../draft/semantic-code-engine-design.md) §4.3 Graph Index
