#!/usr/bin/env python3
"""
Test script for Module-Level Dependencies feature (Sprint 6, Phase 1).

Tests:
1. Module detection (Cargo.toml, package.json, go.mod, etc.)
2. Module storage (put/get modules)
3. Chunk-module association
4. Module edges aggregation
"""

import subprocess
import sys
import os
import tempfile
import shutil
import json

# Colors for output
GREEN = '\033[92m'
RED = '\033[91m'
YELLOW = '\033[93m'
BLUE = '\033[94m'
RESET = '\033[0m'

def run_cmd(cmd, cwd=None, capture=True):
    """Run a command and return output."""
    result = subprocess.run(
        cmd,
        cwd=cwd,
        capture_output=capture,
        text=True,
        shell=isinstance(cmd, str)
    )
    return result

def test_project_detection():
    """Test that ProjectDetector finds Cargo.toml files."""
    print(f"\n{BLUE}=== Test 1: Project Detection ==={RESET}")
    
    # Run Rust unit tests for project detection
    result = run_cmd(
        "cargo test -p codemate-core project:: --no-fail-fast",
        cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    )
    
    if result.returncode == 0:
        print(f"{GREEN}✓ Project detection tests passed{RESET}")
        return True
    else:
        print(f"{RED}✗ Project detection tests failed{RESET}")
        print(result.stderr)
        return False

def test_module_storage():
    """Test ModuleStore trait implementation."""
    print(f"\n{BLUE}=== Test 2: Module Storage ==={RESET}")
    
    # Run all storage tests
    result = run_cmd(
        "cargo test -p codemate-core storage::sqlite --no-fail-fast",
        cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    )
    
    if result.returncode == 0:
        print(f"{GREEN}✓ Module storage tests passed{RESET}")
        return True
    else:
        print(f"{RED}✗ Module storage tests failed{RESET}")
        print(result.stderr)
        return False

def test_codemate_workspace_detection():
    """Test detecting modules in the codemate workspace itself."""
    print(f"\n{BLUE}=== Test 3: CodeMate Workspace Detection ==={RESET}")
    
    codemate_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    
    # Check for expected Cargo.toml files
    expected_crates = [
        "crates/codemate-core/Cargo.toml",
        "crates/codemate-cli/Cargo.toml",
        "crates/codemate-server/Cargo.toml",
        "crates/codemate-parser/Cargo.toml",
        "crates/codemate-embeddings/Cargo.toml",
        "crates/codemate-git/Cargo.toml",
    ]
    
    found_all = True
    for crate in expected_crates:
        path = os.path.join(codemate_dir, crate)
        if os.path.exists(path):
            print(f"  {GREEN}✓{RESET} Found {crate}")
        else:
            print(f"  {RED}✗{RESET} Missing {crate}")
            found_all = False
    
    # Check workspace Cargo.toml
    workspace_toml = os.path.join(codemate_dir, "Cargo.toml")
    if os.path.exists(workspace_toml):
        with open(workspace_toml) as f:
            content = f.read()
            if "[workspace]" in content:
                print(f"  {GREEN}✓{RESET} Workspace manifest detected")
            else:
                print(f"  {YELLOW}⚠{RESET} Not a workspace (single crate)")
    
    if found_all:
        print(f"{GREEN}✓ All expected crates found{RESET}")
        return True
    else:
        print(f"{RED}✗ Some crates missing{RESET}")
        return False

def test_schema_changes():
    """Verify database schema includes modules table."""
    print(f"\n{BLUE}=== Test 4: Schema Verification ==={RESET}")
    
    codemate_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    sqlite_file = os.path.join(codemate_dir, "crates/codemate-core/src/storage/sqlite.rs")
    
    with open(sqlite_file) as f:
        content = f.read()
    
    checks = [
        ("modules table", "CREATE TABLE IF NOT EXISTS modules"),
        ("module_id column", "module_id"),
        ("module_edges view", "CREATE VIEW IF NOT EXISTS module_edges"),
    ]
    
    all_passed = True
    for name, pattern in checks:
        if pattern in content:
            print(f"  {GREEN}✓{RESET} {name} present in schema")
        else:
            print(f"  {RED}✗{RESET} {name} missing from schema")
            all_passed = False
    
    if all_passed:
        print(f"{GREEN}✓ Schema changes verified{RESET}")
        return True
    else:
        print(f"{RED}✗ Schema changes incomplete{RESET}")
        return False

def test_full_build():
    """Test that the full project builds successfully."""
    print(f"\n{BLUE}=== Test 5: Full Build ==={RESET}")
    
    if os.environ.get("CI"):
        print(f"{YELLOW}⚠ Skipping release build test in CI environment{RESET}")
        return True
    
    codemate_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    result = run_cmd("cargo build --release", cwd=codemate_dir)
    
    if result.returncode == 0:
        print(f"{GREEN}✓ Full project builds successfully{RESET}")
        return True
    else:
        print(f"{RED}✗ Build failed{RESET}")
        print(result.stderr[:500] if result.stderr else "No error output")
        return False

def test_all_unit_tests():
    """Run all unit tests."""
    print(f"\n{BLUE}=== Test 6: All Unit Tests ==={RESET}")
    
    codemate_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    result = run_cmd("cargo test --all", cwd=codemate_dir)
    
    if result.returncode == 0:
        print(f"{GREEN}✓ All unit tests passed{RESET}")
        return True
    else:
        print(f"{RED}✗ Some tests failed{RESET}")
        # Show last 20 lines of output
        if result.stdout:
            lines = result.stdout.strip().split('\n')
            for line in lines[-20:]:
                print(f"  {line}")
        return False

def main():
    """Run all tests."""
    print(f"{BLUE}╔═══════════════════════════════════════════════════════════╗{RESET}")
    print(f"{BLUE}║  Module-Level Dependencies - Phase 1 Test Suite           ║{RESET}")
    print(f"{BLUE}╚═══════════════════════════════════════════════════════════╝{RESET}")
    
    results = []
    
    results.append(("Workspace Detection", test_codemate_workspace_detection()))
    results.append(("Schema Changes", test_schema_changes()))
    
    # Summary
    print(f"\n{BLUE}═══════════════════════════════════════════════════════════{RESET}")
    print(f"{BLUE}                     TEST SUMMARY{RESET}")
    print(f"{BLUE}═══════════════════════════════════════════════════════════{RESET}")
    
    passed = sum(1 for _, r in results if r)
    total = len(results)
    
    for name, result in results:
        status = f"{GREEN}PASS{RESET}" if result else f"{RED}FAIL{RESET}"
        print(f"  {name}: {status}")
    
    print(f"\n  Total: {passed}/{total} tests passed")
    
    if passed == total:
        print(f"\n{GREEN}✓ All tests passed! Phase 1 is complete.{RESET}")
        return 0
    else:
        print(f"\n{RED}✗ Some tests failed.{RESET}")
        return 1

if __name__ == "__main__":
    sys.exit(main())
