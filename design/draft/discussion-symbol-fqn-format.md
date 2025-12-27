# Symbol FQN Format - Design Discussion

**Topic**: Fully-Qualified Name format for code symbols  
**Date**: 2025-12-25  
**Status**: Decision Made  
**Related PRD**: [PRD-symbol-fqn-format.md](../decision/PRD-symbol-fqn-format.md)

---

## Problem Statement

CodeMate indexes code symbols (functions, classes, methods) across multiple languages. Each language has its own module/namespace system with different conventions. We need a consistent way to identify symbols for:
- Graph edges (CALLS, REFERENCES, IMPORTS)
- Cross-repository search
- External symbol resolution

---

## Options Considered

### Option 1: Unified Format (Rejected)

Force all languages into a single format like `package::module::symbol`.

**Pros:**
- Single parsing logic
- Uniform storage format

**Cons:**
- Loses language-specific semantics
- Awkward mappings (Python's `__init__.py`, TS's `index.ts`)
- Breaks tooling compatibility
- Cannot represent some concepts (Rust's `pub(crate)`, Python's relative imports)

### Option 2: Raw Language-Specific (Rejected)

Store each language's native FQN as-is.

**Pros:**
- Perfect fidelity
- Works with existing tools

**Cons:**
- Cannot distinguish `foo.bar.baz` (Python) from `foo.bar.baz` (TypeScript)
- Cross-language queries impossible
- Collision risk between languages

### Option 3: Language-Prefixed Format (Selected ✓)

Two-layer format: `<language>:<native_fqn>`

**Pros:**
- Preserves language semantics
- Enables cross-language queries
- Extensible to new languages
- Compatible with existing tooling

**Cons:**
- Slightly more complex parsing
- Requires language normalization layer

---

## Decision: Language-Prefixed Format

**Format**: `<language>:<native_fqn>`

### Language-Specific Conventions

| Language | Format | Example |
|----------|--------|---------|
| Rust | `rust:<path>::<item>` | `rust:crate::auth::middleware::authenticate` |
| Python | `python:<dotted.path>` | `python:myapp.auth.handlers.AuthHandler.login` |
| TypeScript | `typescript:<module>#<export>` | `typescript:@org/auth/middleware#authenticate` |
| Go | `go:<package>.<symbol>` | `go:github.com/org/repo/auth.Authenticate` |
| Java | `java:<fqcn>#<method>` | `java:com.org.auth.AuthService#authenticate` |

### Schema

```sql
-- Symbol registry for FQN resolution
CREATE TABLE symbols (
    fqn             TEXT PRIMARY KEY,  -- language:native_path
    language        TEXT NOT NULL,
    kind            TEXT NOT NULL,     -- function, class, method, trait, etc.
    short_name      TEXT NOT NULL,     -- Last component (authenticate)
    signature       TEXT,              -- Full signature if available
    doc_summary     TEXT,              -- First line of docstring
    is_external     BOOLEAN DEFAULT FALSE,
    created_at      TEXT NOT NULL
);

CREATE INDEX idx_symbols_name ON symbols(short_name);
CREATE INDEX idx_symbols_lang ON symbols(language);
```

---

## Open Questions for Future Discussion

### 1. Repository Context in FQN?

Should FQN include repository context for multi-repo setups?

**Current thinking**: No, keep FQN as language-level identifier. Use `chunk_locations.repo_uri` for repository context.

**Alternative**: `github.com/org/repo:rust:crate::foo::bar`

**Trade-off**: Longer FQNs vs. simpler cross-repo queries

### 2. Versioning of External Symbols

How to handle different versions of the same library?

**Example**: `react:React.useState` (v17) vs `react:React.useState` (v18)

**Options**:
- Version suffix: `react@18:React.useState`
- Separate symbol entries with version metadata
- Ignore versions (symbols are conceptually the same)

### 3. Private/Internal Symbol Visibility

Should we track symbol visibility (pub, private, internal)?

**Use cases**:
- "Show only public API" queries
- Dependency analysis (external deps should only use public symbols)

---

## Discussion Log

### 2025-12-25 - Initial Decision

**Participants**: AI Assistant, User

**Summary**:
- Reviewed three options for FQN format
- Selected language-prefixed format for best balance of fidelity and usability
- Noted open questions for future discussion

**Key insight**: Trying to force all languages into unified format loses too much semantic information. Better to embrace language differences with a thin wrapper.
