## Stage 1: Trace Duplicate Discovery
**Goal**: Confirm which file-discovery paths allow Git worktree internals into analysis.
**Success Criteria**: The TUI/CLI discovery path and the older pipeline discovery path are identified, with the `.git/worktrees` leak reproduced in code inspection.
**Tests**: Existing walker tests reviewed and focused discovery tests selected.
**Status**: Complete

## Stage 2: Exclude Git Metadata
**Goal**: Prevent file discovery from traversing Git metadata directories such as `.git/worktrees`.
**Success Criteria**: Discovery skips any path under `.git` while preserving normal source-file discovery and `.gitignore` handling.
**Tests**: Walker unit tests for `.git` exclusion and existing discovery behavior.
**Status**: Complete

## Stage 3: Add Ignore Regressions
**Goal**: Lock in the expected ignore behavior for Git metadata and `.gitignore` patterns.
**Success Criteria**: Regression tests prove `.git/worktrees` files are ignored and `.gitignore` rules exclude matching source files.
**Tests**: Focused `FileWalker` tests plus a pipeline discovery regression test.
**Status**: Complete

## Stage 4: Verify End-to-End
**Goal**: Run focused tests for the touched discovery paths and confirm the fix.
**Success Criteria**: Targeted cargo tests pass for walker and pipeline discovery coverage.
**Tests**: `cargo test test_walk_`, `cargo test test_find_files_`, and discovery-stage unit tests.
**Status**: Complete
