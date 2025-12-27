# CodeMate Testing Workflow

This document describes the testing strategy and commands for CodeMate.

## Test Types

### 1. Unit Tests (Fast, Local)
Run after each task/feature completion to catch regressions early.

```bash
# Run all unit tests
cargo test --lib

# Run tests for a specific crate
cargo test -p codemate-core --lib
cargo test -p codemate-parser --lib
```

### 2. E2E/Integration Tests (Comprehensive)
Run before merge and major commits to verify full system behavior.

```bash
# Run all integration tests
cargo test --test e2e_tests

# Run all tests (unit + integration)
cargo test --all
```

### 3. Quick Smoke Test
Fast validation that basic functionality works.

```bash
# Build and run a quick check
cargo check && cargo test --lib -q
```

---

## When to Run Tests

| Trigger | Unit Tests | E2E Tests |
|---------|------------|-----------|
| After implementing a feature | ✅ | ❌ |
| After fixing a bug | ✅ | ✅ |
| Before committing | ✅ | ✅ |
| Before merge/PR | ✅ | ✅ |
| During code review | ❌ | ✅ |

---

## Test Commands Quick Reference

```bash
# 🔹 UNIT TESTS - Run after each task
source ~/.cargo/env && cargo test --lib

# 🔸 E2E TESTS - Run before merge/commit
source ~/.cargo/env && cargo test --test e2e_tests

# 🔹 FULL TEST SUITE
source ~/.cargo/env && cargo test --all

# 🔹 WATCH MODE (requires cargo-watch)
cargo watch -x "test --lib"
```

---

## Test Coverage

| Crate | Unit Tests | Integration |
|-------|------------|-------------|
| codemate-core | ✅ 8 tests | ✅ 5 tests |
| codemate-parser | ✅ 3 tests | ✅ (via e2e) |
| codemate-embeddings | ⚠️ Needs mocks | ⚠️ (via e2e) |
| codemate-cli | ⚠️ Needs fixtures | ⚠️ (via e2e) |

---

## Adding New Tests

### Unit Test Pattern
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feature_name() {
        // Arrange
        let input = ...;
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Async Test Pattern
```rust
#[tokio::test]
async fn test_async_feature() {
    let storage = SqliteStorage::in_memory().unwrap();
    // ... test async code
}
```

---

## Pre-Commit Checklist

Before committing:
1. [ ] `cargo fmt` - Format code
2. [ ] `cargo clippy` - Lint check
3. [ ] `cargo test --lib` - Unit tests
4. [ ] `cargo test --test e2e_tests` - E2E tests (for functionality changes)

Before merge:
1. [ ] All unit tests pass
2. [ ] All E2E tests pass
3. [ ] `cargo build --release` succeeds
