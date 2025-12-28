# Feature Development with Git Worktree

Follow these best practices when developing new features or fixes for CodeMate using `git worktree`.

## 1. Directory Structure
Maintain all worktrees as sibling directories to the main repository to keep the primary path clean and avoid nested `.git` issues.

```bash
# Recommended layout
Workspace/
├── AG-Hub/ (Main branch - main)
├── feat-query-dsl/ (Worktree for Sprint 4)
└── fix-parser-crash/ (Worktree for hotfix)
```

## 2. Creating a Feature Worktree
Use the `feature/` prefix for new development.

```bash
# From inside the main AG-Hub repo
git worktree add ../feat-query-dsl feature/query-dsl
```

## 3. Development Workflow

1.  **Independent Builds**: Each worktree has its own `target/` directory by default. This avoids lock contention when building multiple features simultaneously.
2.  **Environment Sync**: If changes involve environment variables, ensure the sibling directory has the correct `.env` (or symlink if appropriate).
3.  **Local Indexing**: Use a worktree-specific database for validation to avoid corrupting the main index.
    ```bash
    ./target/debug/codemate index . --database .codemate/feature_test.db
    ```

## 4. Completion and Cleanup
Once the feature is merged into `main`:

1.  Return to the main repo: `cd ../AG-Hub`
2.  Remove the worktree:
    ```bash
    git worktree remove ../feat-query-dsl
    ```
3.  Delete the branch if not already done:
    ```bash
    git branch -d feature/query-dsl
    ```

## 5. Rules
- **One Feature Per Worktree**: Never mix multiple features in a single worktree.
- **Always Verify**: Run the multi-language validation script (`./tests/validate_langs.sh`) before pushing.
- **Main stays clean**: Only use the main `AG-Hub` directory for releases, documentation updates, and orchestration.
