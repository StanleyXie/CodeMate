#!/bin/bash
# CodeMate Cumulative Validation Suite (Sprints 1-4)

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Paths
BIN="./target/release/codemate"
TEST_DIR=".codemate/cumulative_test"
mkdir -p "$TEST_DIR"

log_info() { echo -e "${BLUE}==> $1${NC}"; }
log_success() { echo -e "${GREEN}  ✓ $1${NC}"; }
log_failure() { echo -e "${RED}  ✗ $1${NC}"; }

# Ensure binary exists
if [ ! -f "$BIN" ]; then
    echo "Building codemate-cli..."
    cargo build --release -p codemate-cli
fi

# -----------------------------------------------------------------------------
# Module 1: Multi-Language Parsing & Graph Verification
# -----------------------------------------------------------------------------
test_languages() {
    log_info "Module 1: Language & Graph Validation"
    local db_dir="$TEST_DIR/langs"
    mkdir -p "$db_dir"

    validate_lang() {
        local lang=$1; local file=$2; local content=$3; local sym=$4
        local db="$db_dir/${lang}.db"
        
        echo -e "$content" > "$file"
        $BIN index . --database "$db" > /dev/null
        
        # Verify Stats
        if [[ "$($BIN stats --database "$db")" == *"Chunks indexed:"* ]]; then
            log_success "$lang: indexing & stats verified"
        else
            log_failure "$lang: stats failed"; return 1
        fi

        # Verify Callers
        if [[ "$($BIN graph --database "$db" callers "$sym")" == *"Found "* ]]; then
            log_success "$lang: graph relationships verified"
        else
            log_failure "$lang: graph failed"; return 1
        fi
        rm "$file"
    }

    validate_lang "rust" "tmp.rs" "fn h() {}\nfn main() { h(); }" "h" || return 1
    validate_lang "python" "tmp.py" "def h(): pass\ndef main(): h()" "h" || return 1
    validate_lang "go" "tmp.go" "package main\nfunc h() {}\nfunc main() { h() }" "h" || return 1
    validate_lang "typescript" "tmp.ts" "function h() {}\nfunction main() { h(); }" "h" || return 1
    validate_lang "hcl" "tmp.tf" "resource \"aws_instance\" \"web\" {\n  ami = \"ami-123\"\n}\noutput \"ip\" {\n  value = aws_instance.web.public_ip\n  # custom_unique_logic\n}" "aws_instance.web" || return 1
    
    return 0
}

# -----------------------------------------------------------------------------
# Module 2: Query Layer (DSL, FTS5, RRF) Verification
# -----------------------------------------------------------------------------
test_query_layer() {
    log_info "Module 2: Query Layer Validation"
    local repo_dir="$TEST_DIR/query_repo"
    local db="$TEST_DIR/query.db"
    mkdir -p "$repo_dir"
    
    # Setup Mock Repo
    (
        cd "$repo_dir"
        git init -q
        git config user.name "Alice"
        git config user.email "alice@example.com"
        echo "fn alice_fun() {}" > alice.rs
        git add . && git commit -m "Alice commit" -q
        
        git config user.name "Bob"
        git config user.email "bob@example.com"
        echo "def bob_fun(): pass" > bob.py
        git add . && git commit -m "Bob commit" -q
    )

    # Index
    $BIN index "$repo_dir" --git --database "$db" > /dev/null

    # Case 1: RRF/Lexical Priority
    if [[ "$($BIN search "alice_fun" --database "$db")" == *"[1]"* ]]; then
        log_success "Lexical match (FTS5/RRF) prioritized"
    else
        log_failure "RRF prioritization failed"; return 1
    fi

    # Case 2: Language Filter
    if [[ "$($BIN search "fun lang:rust" --database "$db")" == *"alice_fun"* ]] && \
       [[ "$($BIN search "fun lang:rust" --database "$db")" != *"bob_fun"* ]]; then
        log_success "Language filtering verified"
    else
        log_failure "Language filtering failed"; return 1
    fi

    # Case 3: Author Filter
    if [[ "$($BIN search "fun author:Alice" --database "$db")" == *"Alice"* ]] && \
       [[ "$($BIN search "fun author:Alice" --database "$db")" != *"bob_fun"* ]]; then
        log_success "Author filtering verified"
    else
        log_failure "Author filtering failed"; return 1
    fi

    return 0
}

# -----------------------------------------------------------------------------
# Module 3: Graph Forest Validation
# -----------------------------------------------------------------------------
test_graph_forest() {
    log_info "Module 3: Graph Forest Validation"
    local db="$TEST_DIR/forest.db"
    
    # Create mixed language data
    echo "fn r_h() {}" > f.rs
    echo "fn r_m() { r_h(); }" >> f.rs
    echo "package main" > f.go
    echo "func g_h() {}" >> f.go
    echo "func g_m() { g_h() }" >> f.go

    $BIN index f.rs --database "$db" > /dev/null
    $BIN index f.go --database "$db" > /dev/null

    local out=$($BIN graph --database "$db" tree --all)
    
    if [[ "$out" == *"r_m"* ]] && [[ "$out" == *"r_h"* ]] && \
       [[ "$out" == *"g_m"* ]] && [[ "$out" == *"g_h"* ]]; then
        log_success "Multi-language dependency forest verified"
    else
        log_failure "Graph forest validation failed"; return 1
    fi

    rm f.rs f.go
    return 0
}

# -----------------------------------------------------------------------------
# Main Runner
# -----------------------------------------------------------------------------
log_info "Starting CodeMate Cumulative Validation..."

if test_languages && test_query_layer && test_graph_forest; then
    echo -e "\n${GREEN}=========================================${NC}"
    echo -e "${GREEN}🎉 ALL SYSTEMS VALIDATED SUCCESSFULLY 🎉${NC}"
    echo -e "${GREEN}=========================================${NC}"
    # Final cleanup
    rm -rf "$TEST_DIR"
    exit 0
else
    echo -e "\n${RED}=========================================${NC}"
    echo -e "${RED}🚨 VALIDATION FAILED 🚨${NC}"
    echo -e "${RED}=========================================${NC}"
    exit 1
fi
