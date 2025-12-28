# CodeMate CLI Documentation

CodeMate is an intelligent code intelligence engine with a feature-rich command-line interface. This document provides a comprehensive reference for all available commands and their arguments.

## Global Options

| Option | Shorthand | Description |
|--------|-----------|-------------|
| `--verbose` | `-v` | Enable verbose output (debug logging) |
| `--version` | `-V` | Show version information |
| `--help` | `-h` | Show help information |

---

## Commands

### `index`
Index a directory to build the semantic and relational graph.

**Usage:** `codemate index [PATH] [OPTIONS]`

| Argument/Option | Shorthand | Default | Description |
|-----------------|-----------|---------|-------------|
| `PATH` | - | `.` | The directory to index. |
| `--database` | `-d` | `.codemate/index.db` | Path to the SQLite database. |
| `--git` | - | `false` | Enable git-aware indexing (tracks commit history and authors). |
| `--max-commits` | - | `100` | Maximum number of commits to index (only used with `--git`). |

---

### `search`
Perform a hybrid (semantic + lexical) search for code.

**Usage:** `codemate search <QUERY> [OPTIONS]`

| Argument/Option | Shorthand | Default | Description |
|-----------------|-----------|---------|-------------|
| `QUERY` | - | - | Search query. Supports filters (e.g., `lang:rust author:Alice`). |
| `--database` | `-d` | `.codemate/index.db` | Path to the SQLite database. |
| `--limit` | `-l` | `10` | Maximum number of results to return. |
| `--threshold` | `-t` | `0.5` | Minimum similarity threshold for semantic results (0.0 to 1.0). |

---

### `stats`
Show statistics about the indexed database.

**Usage:** `codemate stats [OPTIONS]`

| Option | Shorthand | Default | Description |
|--------|-----------|---------|-------------|
| `--database` | `-d` | `.codemate/index.db` | Path to the SQLite database. |

---

### `history`
Show the evolution of a specific code chunk or file.

**Usage:** `codemate history <TARGET> [OPTIONS]`

| Argument/Option | Shorthand | Default | Description |
|-----------------|-----------|---------|-------------|
| `TARGET` | - | - | File path or content hash to show history for. |
| `--database` | `-d` | `.codemate/index.db` | Path to the SQLite database. |
| `--limit` | `-l` | `20` | Maximum history entries to show. |

---

### `graph`
Explore code relationships (call graph, dependencies).

**Usage:** `codemate graph [OPTIONS] <SUBCOMMAND>`

#### Subcommands:

##### `callers`
Find symbols that call or reference a specific symbol.
- `symbol`: The symbol name to find callers for.

##### `deps`
Find outgoing dependencies of a file.
- `file_path`: The file path to find dependencies for.

##### `tree`
Visualize recursive dependency tree/forest in the terminal.
- `symbol`: (Optional) Symbol name to start the tree from.
- `--all`, `-a`: Visualize the entire dependency forest (all entry points).
- `--depth`, `-d`: (Default: `3`) Maximum recursion depth.

---

## Query DSL Reference
The `search` command supports a simple DSL for filtering results:

- `lang:<language>`: Filter by programming language (e.g., `lang:rust`, `lang:python`).
- `author:<name>`: Filter by commit author.
- `file:<pattern>`: Filter by file path pattern.
- `after:<ISO-8601>`: Filter results after a certain date.
- `before:<ISO-8601>`: Filter results before a certain date.
- `limit:<number>`: Override the default result limit.

**Example:** `codemate search "database connection lang:rust author:Stanley"`
