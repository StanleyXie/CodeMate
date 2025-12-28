#!/bin/bash
# CodeMate Multi-Language Validation Script
# Validates Chunks and Graph Edge extraction for supported languages.

set -e

BIN="./target/release/codemate"
DB_DIR=".codemate/test_langs"
mkdir -p "$DB_DIR"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

function log_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

function log_failure() {
    echo -e "${RED}✗ $1${NC}"
}

function validate() {
    local lang=$1
    local file=$2
    local content=$3
    local target_symbol=$4
    local database="$DB_DIR/${lang}_test.db"

    echo "--- Validating $lang ---"
    
    # 1. Create file
    echo -e "$content" > "$file"
    
    # 2. Index
    $BIN index . --database "$database" > /dev/null
    log_success "$lang indexing complete"
    
    # 3. Verify Stats
    local stats=$($BIN stats --database "$database")
    if [[ "$stats" == *"Chunks indexed:"* ]]; then
        log_success "$lang stats verified"
    else
        log_failure "$lang stats failed"
        echo "$stats"
        exit 1
    fi

    # 4. Verify Search (using a unique keyword from content)
    local search=$($BIN search "custom_unique_logic" --database "$database")
    if [[ "$search" == *"Found"* ]]; then
        log_success "$lang search verified"
    else
        log_failure "$lang search failed"
        echo "$search"
        exit 1
    fi

    # 5. Verify Callers (Incoming edges)
    local callers=$($BIN graph --database "$database" callers "$target_symbol")
    if [[ "$callers" == *"Found "* && "$callers" == *"caller(s)"* ]]; then
        log_success "$lang graph callers verified"
    else
        log_failure "$lang graph callers failed"
        echo "$callers"
        exit 1
    fi

    # 6. Verify Deps (Outgoing edges)
    local deps=$($BIN graph --database "$database" deps "$file")
    if [[ "$deps" == *"Found "* && "$deps" == *"code chunk(s)"* ]]; then
        log_success "$lang graph deps verified"
    else
        log_failure "$lang graph deps failed"
        echo "$deps"
        exit 1
    fi
}

# Ensure binary exists
if [ ! -f "$BIN" ]; then
    echo "Building codemate-cli..."
    cargo build --release -p codemate-cli
fi

# RUST
validate "rust" "test_rs.rs" "fn helper() { /* custom_unique_logic */ }\nfn main() { helper(); }" "helper"

# PYTHON
validate "python" "test_py.py" "def helper():\n    \"\"\"custom_unique_logic\"\"\"\n    pass\ndef main(): helper()" "helper"

# GO
validate "go" "test_go.go" "package main\nfunc helper() { /* custom_unique_logic */ }\nfunc main() { helper() }" "helper"

# TYPESCRIPT
validate "typescript" "test_ts.ts" "function helper() { /* custom_unique_logic */ }\nfunction main() { helper(); }" "helper"

echo -e "\n${GREEN}All languages validated successfully!${NC}"
# Cleanup
rm test_rs.rs test_py.py test_go.go test_ts.ts
rm -rf "$DB_DIR"
