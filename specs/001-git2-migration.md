---
number: 001
title: Migrate Git Operations from Subprocess to git2 Library
category: foundation
priority: high
status: draft
dependencies: []
created: 2026-01-22
---

# Specification 001: Migrate Git Operations from Subprocess to git2 Library

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: none

## Context

The current git history analysis system relies on subprocess calls to the `git` command-line tool via `std::process::Command`. This approach has several reliability issues:

1. **Path normalization inconsistencies**: Different git commands may succeed or fail depending on whether paths are absolute or relative
2. **Silent failures**: When git commands fail (e.g., file not in history), they return empty output which is parsed as zero values
3. **Inconsistent results**: Author count may succeed while age calculation fails, leading to contradictory metrics (e.g., "1 author" but "0 days age")
4. **Parsing fragility**: Subprocess output parsing is error-prone and can break with different git versions or locale settings
5. **Performance overhead**: Each git operation spawns a new process, adding latency

### Observed Bug

When analyzing files in `../diogenes`, the git history showed:
- Age: 0 days
- Change Frequency: 0.00/month
- Bug Density: 0%
- Authors: 1

This is contradictory - if there's 1 author, commits must exist, so age shouldn't be 0. The root cause is inconsistent path handling between different subprocess calls.

### Current Implementation Scope

Git subprocess calls exist in these locations:

| File | Operations | Count |
|------|-----------|-------|
| `src/risk/context/git_history.rs` | rev-parse, rev-list, log, blame | 11 |
| `src/risk/context/git_history/batched.rs` | git log --numstat | 1 |
| `src/risk/context/git_history/function_level.rs` | git log -S, git log -G | 3 |
| `src/risk/context/git_history/blame_cache.rs` | git blame --porcelain | 1 |
| Tests and benchmarks | Various setup commands | ~20 |

## Objective

Replace all production git subprocess calls with the `git2` library (Rust bindings to libgit2) to provide:

1. **Reliable path handling**: libgit2 handles path resolution internally
2. **Proper error handling**: Rust `Result` types instead of parsing failures
3. **Consistent API**: Single library for all git operations
4. **Better performance**: No process spawning overhead
5. **Type safety**: Strongly typed git objects instead of string parsing

## Requirements

### Functional Requirements

1. **Repository Access**
   - Open git repositories by path
   - Discover repository root from any subdirectory
   - Handle bare repositories gracefully (error with clear message)

2. **Commit History Analysis**
   - Count commits touching a specific file
   - Get first and last commit dates for a file
   - List all commits with file changes (equivalent to `git log --numstat`)
   - Count unique authors for a file

3. **Commit Message Analysis**
   - Retrieve commit messages for bug-fix detection
   - Parse commit metadata (author, date, hash)

4. **Blame Analysis**
   - Get line-by-line blame information for files
   - Extract author information per line
   - Handle uncommitted changes gracefully

5. **Function-Level Analysis**
   - Find commit that introduced a function (pickaxe search equivalent)
   - Track modifications to specific code patterns

### Non-Functional Requirements

1. **Performance**: Must be at least as fast as subprocess approach (likely faster)
2. **Memory**: Should not significantly increase memory usage
3. **Compatibility**: Must work on macOS and Linux
4. **Thread Safety**: Must be safe for parallel analysis with rayon
5. **Error Handling**: All errors must be informative and recoverable

## Acceptance Criteria

- [ ] Add `git2` dependency to Cargo.toml
- [ ] Create `src/risk/context/git2_provider.rs` module with pure git2 wrapper functions
- [ ] Implement `Git2Repository` struct with methods for all required operations
- [ ] Migrate `GitHistoryProvider` to use git2 instead of subprocess calls
- [ ] Migrate `BatchedGitHistory` to use git2 for commit iteration
- [ ] Migrate `FileBlameCache` to use git2 blame API
- [ ] Migrate `function_level.rs` to use git2 for pickaxe and regex search
- [ ] All existing tests pass without modification to test assertions
- [ ] Fix the diogenes path resolution bug (validated manually)
- [ ] Performance benchmark shows no regression (ideally improvement)
- [ ] Test coverage for new git2 wrapper functions >= 80%

## Technical Details

### Implementation Approach

#### Phase 1: Foundation Layer

Create a new module `src/risk/context/git2_provider.rs` that wraps git2 operations:

```rust
use git2::{Repository, Commit, Blame, BlameOptions};
use std::path::Path;
use anyhow::Result;

/// Thread-safe wrapper around git2::Repository
pub struct Git2Repository {
    repo: Repository,
}

impl Git2Repository {
    /// Open a repository, discovering the root from any subdirectory
    pub fn open(path: &Path) -> Result<Self>;

    /// Count commits touching a file
    pub fn count_file_commits(&self, file_path: &Path) -> Result<usize>;

    /// Get file age in days since first commit
    pub fn file_age_days(&self, file_path: &Path) -> Result<u32>;

    /// Get unique author emails for a file
    pub fn file_authors(&self, file_path: &Path) -> Result<HashSet<String>>;

    /// Get all commits with file changes (for batched analysis)
    pub fn file_commits_with_stats(&self, file_path: &Path) -> Result<Vec<CommitStats>>;

    /// Get blame information for a file
    pub fn blame_file(&self, file_path: &Path) -> Result<BlameData>;

    /// Find commit that introduced a string (pickaxe search)
    pub fn find_introduction(&self, file_path: &Path, pattern: &str) -> Result<Option<Oid>>;
}
```

#### Phase 2: Batched History Migration

Replace `BatchedGitHistory::fetch_git_log()` with git2 repository walking:

```rust
impl BatchedGitHistory {
    pub fn new_with_git2(repo: &Git2Repository) -> Result<Self> {
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;

        let commits = revwalk
            .filter_map(|oid| repo.find_commit(oid.ok()?).ok())
            .map(|commit| parse_commit_with_diff(&commit))
            .collect();

        let file_histories = Self::build_file_maps(commits);
        Ok(Self { file_histories })
    }
}
```

#### Phase 3: Direct Query Migration

Replace individual git commands in `GitHistoryProvider` with git2 methods:

| Current Method | git2 Equivalent |
|---------------|-----------------|
| `count_commits()` | `revwalk` with path filter |
| `count_bug_fixes()` | `revwalk` + message parsing |
| `get_last_modified()` | `revwalk` first commit |
| `count_unique_authors()` | `revwalk` + author collection |
| `get_file_age_days()` | `revwalk` oldest commit |

#### Phase 4: Blame Migration

Replace `git blame --porcelain` with git2 blame API:

```rust
impl FileBlameCache {
    fn fetch_file_blame_git2(&self, repo: &Git2Repository, file_path: &Path) -> Result<String> {
        let blame = repo.blame_file(
            file_path,
            Some(BlameOptions::new().track_copies_same_file(true))
        )?;

        // Convert blame hunks to our internal format
        parse_git2_blame(&blame)
    }
}
```

#### Phase 5: Function-Level Migration

Replace `-S` and `-G` searches with git2 diff walking:

```rust
/// Find the commit that introduced a function
fn find_function_introduction(
    repo: &Git2Repository,
    file_path: &Path,
    function_name: &str,
) -> Result<Option<Oid>> {
    // Walk commits in reverse chronological order
    // Check each commit's diff for the function signature
    // Return the oldest commit containing the function
}
```

### Architecture Changes

```
Before:
┌──────────────────┐     subprocess      ┌─────────────┐
│ GitHistoryProvider├────────────────────►│  git CLI    │
├──────────────────┤                     └─────────────┘
│ BatchedGitHistory│
├──────────────────┤
│ function_level   │
├──────────────────┤
│ blame_cache      │
└──────────────────┘

After:
┌──────────────────┐                     ┌─────────────┐
│ GitHistoryProvider├────┐               │  git2 crate │
├──────────────────┤    │               └──────┬──────┘
│ BatchedGitHistory│    │                      │
├──────────────────┼────┼──►┌────────────────┐ │
│ function_level   │    │   │ Git2Repository ├─┘
├──────────────────┤    │   │ (wrapper)      │
│ blame_cache      ├────┘   └────────────────┘
└──────────────────┘
```

### Data Structures

```rust
/// Statistics for a single commit
#[derive(Debug, Clone)]
pub struct CommitStats {
    pub hash: git2::Oid,
    pub date: DateTime<Utc>,
    pub message: String,
    pub author_email: String,
    pub files: Vec<FileStats>,
}

/// Statistics for a file change in a commit
#[derive(Debug, Clone)]
pub struct FileStats {
    pub path: PathBuf,
    pub additions: usize,
    pub deletions: usize,
}

/// Blame information for a file
#[derive(Debug, Clone)]
pub struct BlameData {
    pub lines: HashMap<usize, BlameLineInfo>,
}
```

### APIs and Interfaces

The public API of `GitHistoryProvider` and related structs will remain unchanged. Only the internal implementation switches from subprocess to git2.

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/risk/context/git_history.rs`
  - `src/risk/context/git_history/batched.rs`
  - `src/risk/context/git_history/function_level.rs`
  - `src/risk/context/git_history/blame_cache.rs`
- **External Dependencies**:
  - `git2` crate (adds ~2MB to binary due to libgit2)
  - May require `libgit2` system library on some platforms (usually bundled)

## Testing Strategy

### Unit Tests

- Test `Git2Repository` methods with a temporary git repository
- Verify path normalization works for absolute and relative paths
- Test error handling for non-existent files, non-git directories
- Test blame parsing for various scenarios (single author, multiple authors, uncommitted changes)

### Integration Tests

- Verify existing integration tests pass unchanged
- Add test for cross-directory analysis (the diogenes bug scenario)
- Test with repositories of varying sizes (empty, small, large)

### Performance Tests

- Benchmark before/after for `BatchedGitHistory::new()`
- Benchmark individual file analysis
- Verify parallel analysis still works correctly with rayon

### User Acceptance

- Manually verify the diogenes bug is fixed
- Run debtmap on several real-world repositories
- Verify output matches expected git history

## Documentation Requirements

- **Code Documentation**: Document all public methods in `Git2Repository`
- **User Documentation**: No changes needed (internal implementation)
- **Architecture Updates**: Update ARCHITECTURE.md if it exists

## Implementation Notes

### git2 Thread Safety

`git2::Repository` is not `Send` or `Sync`. For parallel analysis:

1. **Option A**: Create a new `Repository` per thread (simple, slight overhead)
2. **Option B**: Use `Arc<Mutex<Repository>>` with careful locking
3. **Option C**: Pre-compute all git data upfront (current batched approach)

Recommend **Option A** for simplicity, with **Option C** for the common path.

### Error Handling

Replace silent failures with explicit errors:

```rust
// Before (subprocess)
if output.status.success() {
    parse_output(&output.stdout)
} else {
    Ok(0) // Silent failure!
}

// After (git2)
self.repo
    .revwalk()?
    .push_head()
    .context("Failed to start revision walk")?
    .filter_paths(&[file_path])
    .count()
```

### Pickaxe Search Limitation

git2 does not have a direct equivalent to `git log -S` (pickaxe search). Options:

1. **Manual diff walking**: Walk commits and check each diff for the pattern
2. **Fall back to subprocess**: Keep subprocess for pickaxe only
3. **Use regex search**: Convert to `-G` style regex matching

Recommend **Option 1** for consistency, with optimization to stop early.

## Migration and Compatibility

- **Breaking Changes**: None (public API unchanged)
- **Migration Path**: Direct replacement, no user action required
- **Backwards Compatibility**: N/A (internal implementation change)
- **Rollback Plan**: Revert to subprocess implementation if issues arise

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| git2 API differences from CLI | Medium | Comprehensive testing against known outputs |
| Binary size increase (~2MB) | Low | Acceptable for reliability gains |
| Platform-specific issues | Low | CI tests on macOS and Linux |
| Performance regression | Medium | Benchmark before/after, optimize if needed |
| Thread safety issues | High | Use per-thread Repository instances |

## Success Metrics

1. **Bug Fix**: The diogenes git history bug no longer occurs
2. **Test Suite**: All existing tests pass
3. **Performance**: No regression in benchmarks (target: 10% improvement)
4. **Reliability**: No silent failures in git operations
5. **Code Quality**: git2 wrapper has >80% test coverage
