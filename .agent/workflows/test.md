---
description: How to run tests for CodeMate
---

# CodeMate Testing Workflow

## After completing a task (Unit Tests)
// turbo
1. Run unit tests to verify no regressions:
```bash
cd /Users/stanleyxie/Workspace/Projects/research/AG-Hub/codemate
source ~/.cargo/env && cargo test --lib
```

## Before merge or functionality commit (E2E Tests)
// turbo
1. Run full E2E test suite:
```bash
cd /Users/stanleyxie/Workspace/Projects/research/AG-Hub/codemate
source ~/.cargo/env && cargo test -p codemate-core --test e2e_tests
```

// turbo
2. Run all tests:
```bash
cd /Users/stanleyxie/Workspace/Projects/research/AG-Hub/codemate
source ~/.cargo/env && cargo test --all
```

## Full pre-commit workflow
// turbo
1. Format code:
```bash
cd /Users/stanleyxie/Workspace/Projects/research/AG-Hub/codemate
source ~/.cargo/env && cargo fmt
```

// turbo
2. Lint check:
```bash
cd /Users/stanleyxie/Workspace/Projects/research/AG-Hub/codemate
source ~/.cargo/env && cargo clippy
```

// turbo
3. Run all tests:
```bash
cd /Users/stanleyxie/Workspace/Projects/research/AG-Hub/codemate
source ~/.cargo/env && cargo test --all
```
