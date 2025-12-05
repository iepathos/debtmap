---
number: 206
title: Batched Git History Analysis for Performance
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-05
---

# Specification 206: Batched Git History Analysis for Performance

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The `--context` flag enables context-aware risk analysis by loading three context providers: critical_path, dependency, and git_history. The git_history provider currently spawns **~5-7 subprocess calls per file**, resulting in 2,300-3,300 total subprocesses for a typical codebase with 469 files.

**Current Performance**:
- Without `--context`: ~30 seconds (acceptable)
- With `--context`: ~260 seconds (unacceptable due to subprocess overhead)

**Root Cause**:
Each file analysis calls git multiple times:
```rust
// Per-file git queries (current implementation)
git rev-list --count HEAD -- <file>       // Count commits
git log --oneline --grep="fix" -- <file>  // Count bug fixes
git log -1 --format=%cI -- <file>        // Last modified
git shortlog -sn -- <file>               // Author count
git log --follow -- <file>               // Stability metrics
```

Even with parallel processing across files, subprocess fork/exec overhead dominates performance.

**Performance Analysis**:
- 20 git commands (5 files × 4 queries): ~10+ seconds
- 2,300+ git commands (469 files × 5 queries): ~260 seconds
- Subprocess overhead accounts for 90%+ of execution time

## Objective

Optimize git history analysis by implementing a **batched query approach** that:
1. Runs 3-5 comprehensive git queries once (upfront I/O)
2. Parses results into lookup maps (pure transformation)
3. Allows parallel file processing via instant HashMap lookups (no I/O)

**Target Performance**: Reduce `--context` analysis time from 260s to <10s (25x+ speedup)

## Requirements

### Functional Requirements

1. **Batched Git Queries**
   - Run one comprehensive git log query to fetch all commit history
   - Parse output into structured data (commits, authors, files changed)
   - Build lookup maps: `HashMap<PathBuf, FileHistory>`
   - Preserve all existing metrics (churn rate, bug fixes, stability, etc.)

2. **Stillwater Philosophy Compliance**
   - **Pure Core**: All data parsing and map building must be pure functions
   - **Imperative Shell**: Git subprocess calls isolated to initialization phase
   - **Functional Pipeline**: Transform git log output → parsed commits → aggregated maps
   - **Composition**: Build complex transformations from simple, testable pieces

3. **API Compatibility**
   - Maintain existing `GitHistoryProvider` interface
   - Preserve `FileHistory` struct and all fields
   - Drop-in replacement for current implementation
   - No breaking changes to context provider API

4. **Data Accuracy**
   - All metrics must match current implementation exactly
   - Bug fix counting logic preserved (word boundary matching, exclusion filters)
   - Stability calculations unchanged
   - Author counting logic identical

### Non-Functional Requirements

1. **Performance**
   - Target: < 10 seconds for 469-file codebase with `--context`
   - Minimum: 10x speedup over current implementation
   - Acceptable: Single-digit second overhead for git log parsing

2. **Memory Efficiency**
   - Stream git log output, don't load entire history into memory
   - Build maps incrementally during parsing
   - Total memory usage < 50MB for typical codebase (10K commits, 500 files)

3. **Maintainability**
   - Clear separation: I/O vs parsing vs aggregation
   - Pure functions for all transformations
   - Comprehensive documentation of git log format
   - Unit tests for parsing edge cases

4. **Error Handling**
   - Graceful fallback if git commands fail
   - Preserve context in error messages
   - Handle corrupted git output safely

## Acceptance Criteria

- [ ] Batched implementation reduces `--context` analysis time by at least 10x
- [ ] All existing git history metrics produce identical results
- [ ] Pure functions handle parsing with no I/O
- [ ] Existing tests pass without modification
- [ ] New unit tests cover parsing edge cases (merge commits, renames, binary files)
- [ ] Memory usage remains under 50MB for typical codebases
- [ ] Error handling preserves helpful context
- [ ] API compatibility maintained (drop-in replacement)

## Technical Details

### Implementation Approach

**Architecture**: Follow Stillwater's "Pure Core, Imperative Shell" pattern

```rust
// Imperative Shell: Single upfront git query
pub struct BatchedGitHistory {
    file_histories: HashMap<PathBuf, FileHistory>,
}

impl BatchedGitHistory {
    // I/O boundary: Fetch all data once
    pub fn new(repo_root: &Path) -> Result<Self> {
        let raw_log = Self::fetch_git_log(repo_root)?;  // I/O
        let commits = Self::parse_log(&raw_log)?;        // Pure
        let file_histories = Self::build_maps(commits); // Pure
        Ok(Self { file_histories })
    }

    // Pure lookup (no I/O after construction)
    pub fn get_file_history(&self, path: &Path) -> Option<&FileHistory> {
        self.file_histories.get(path)
    }
}

// Pure Core: All transformations are pure functions
impl BatchedGitHistory {
    // Pure: String → Vec<CommitInfo>
    fn parse_log(raw_log: &str) -> Result<Vec<CommitInfo>> {
        raw_log
            .split(":::")
            .filter_map(|entry| Self::parse_commit(entry).ok())
            .collect()
    }

    // Pure: Vec<CommitInfo> → HashMap<PathBuf, FileHistory>
    fn build_maps(commits: Vec<CommitInfo>) -> HashMap<PathBuf, FileHistory> {
        commits
            .into_iter()
            .fold(HashMap::new(), |mut acc, commit| {
                for file in commit.files {
                    acc.entry(file.clone())
                        .or_insert_with(FileHistory::default)
                        .add_commit(&commit);
                }
                acc
            })
    }
}
```

### Git Command Design

**Single Comprehensive Query**:
```bash
git log --all --numstat --format=":::%H:::%cI:::%s:::%an:::%ae" HEAD
```

**Output Format**:
```
:::abc123:::2025-01-15T10:30:00Z:::fix: resolve memory leak:::John Doe:::john@example.com
10      5       src/main.rs
15      2       src/lib.rs

:::def456:::2025-01-14T14:20:00Z:::feat: add caching:::Jane Smith:::jane@example.com
25      0       src/cache.rs
```

**Parsing Strategy**:
1. Split on ":::" markers to separate commits
2. Extract commit metadata (hash, date, message, author)
3. Parse numstat lines (additions, deletions, file path)
4. Aggregate per-file statistics functionally

### Data Structures

```rust
#[derive(Debug, Clone)]
struct CommitInfo {
    hash: String,
    date: DateTime<Utc>,
    message: String,
    author: String,
    files: Vec<FileChange>,
}

#[derive(Debug, Clone)]
struct FileChange {
    path: PathBuf,
    additions: usize,
    deletions: usize,
}

#[derive(Debug, Clone, Default)]
struct FileHistory {
    total_commits: usize,
    bug_fix_count: usize,
    authors: HashSet<String>,
    last_modified: Option<DateTime<Utc>>,
    first_seen: Option<DateTime<Utc>>,
    total_churn: usize,  // additions + deletions
}

impl FileHistory {
    // Pure: Accumulate commit data
    fn add_commit(&mut self, commit: &CommitInfo) {
        self.total_commits += 1;

        if is_bug_fix(&commit.message) {
            self.bug_fix_count += 1;
        }

        self.authors.insert(commit.author.clone());

        self.last_modified = Some(
            self.last_modified
                .map(|d| d.max(commit.date))
                .unwrap_or(commit.date)
        );

        self.first_seen = Some(
            self.first_seen
                .map(|d| d.min(commit.date))
                .unwrap_or(commit.date)
        );

        for file in &commit.files {
            self.total_churn += file.additions + file.deletions;
        }
    }

    // Pure: Calculate derived metrics
    fn calculate_metrics(&self) -> CalculatedMetrics {
        let age_days = self.first_seen
            .map(|first| (Utc::now() - first).num_days() as u32)
            .unwrap_or(0);

        let change_frequency = if age_days > 0 {
            (self.total_commits as f64 / age_days as f64) * 30.0
        } else {
            0.0
        };

        let bug_density = if self.total_commits > 0 {
            self.bug_fix_count as f64 / self.total_commits as f64
        } else {
            0.0
        };

        let stability_score = calculate_stability(
            self.total_commits,
            self.total_churn,
            age_days
        );

        CalculatedMetrics {
            change_frequency,
            bug_density,
            stability_score,
            author_count: self.authors.len(),
            age_days,
        }
    }
}
```

### Functional Pipeline Composition

```rust
// Compose small, testable pure functions
fn process_git_history(repo_root: &Path) -> Result<HashMap<PathBuf, FileHistory>> {
    fetch_git_log(repo_root)?           // I/O: Get raw data
        |> parse_log                    // Pure: String → Vec<CommitInfo>
        |> validate_commits             // Pure: Filter invalid entries
        |> build_file_maps              // Pure: Aggregate by file
        |> calculate_all_metrics        // Pure: Derive final metrics
        |> Ok
}

// Each function is independently testable
#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_log() {
        let input = ":::abc123:::2025-01-15T10:30:00Z:::fix bug:::Author";
        let commits = parse_log(input).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].hash, "abc123");
    }

    #[test]
    fn test_bug_fix_detection() {
        assert!(is_bug_fix("fix: memory leak"));
        assert!(is_bug_fix("hotfix for crash"));
        assert!(!is_bug_fix("refactor: cleanup"));
        assert!(!is_bug_fix("style: formatting"));
    }
}
```

### Integration with Existing Code

**Minimal Changes Required**:

1. **New Module**: `src/risk/context/git_history_batched.rs`
   - Contains `BatchedGitHistory` implementation
   - All pure parsing functions
   - Helper functions for metric calculation

2. **Modified**: `src/risk/context/git_history.rs`
   - Change `GitHistoryProvider::new()` to use batched implementation
   - Keep existing API unchanged
   - Maintain backward compatibility

3. **Tests**: `src/risk/context/git_history_batched_tests.rs`
   - Unit tests for parsing functions
   - Integration tests comparing old vs new results
   - Performance benchmarks

**Migration Strategy**:
- Implement batched version alongside current implementation
- Add feature flag `batched_git_history` (default: true)
- Run both implementations in tests to verify identical results
- Remove old implementation after validation period

## Dependencies

**External**: None (uses existing git CLI)

**Internal**:
- Existing `git_history.rs` module (will be refactored)
- `chrono` crate (already a dependency)
- Standard library `HashMap`, `HashSet`

## Testing Strategy

### Unit Tests

1. **Parsing Functions** (Pure, easy to test)
   - Test commit parsing with various git log formats
   - Test numstat parsing with edge cases (binary files, renames)
   - Test bug fix detection heuristics
   - Test date parsing with timezones

2. **Aggregation Logic** (Pure, deterministic)
   - Test map building with multiple commits per file
   - Test metric calculations (churn rate, stability)
   - Test edge cases (empty history, single commit)

3. **Integration Tests**
   - Run batched implementation on debtmap's own git history
   - Compare results with current implementation
   - Verify identical metrics for all files

### Performance Tests

1. **Benchmark Suite**
   ```rust
   #[bench]
   fn bench_current_implementation(b: &mut Bencher) {
       b.iter(|| {
           // Current per-file queries
       });
   }

   #[bench]
   fn bench_batched_implementation(b: &mut Bencher) {
       b.iter(|| {
           // New batched implementation
       });
   }
   ```

2. **Target Metrics**
   - Batched implementation should be 10x+ faster
   - Memory usage should be < 50MB
   - Parsing should complete in single-digit seconds

### User Acceptance

- Run `debtmap analyze . --context` on debtmap itself
- Verify output is identical to current implementation
- Confirm performance improvement (< 10s vs ~260s)
- Test on various repository sizes (small, medium, large)

## Documentation Requirements

### Code Documentation

1. **Module-level docs**
   - Explain batched approach and philosophy alignment
   - Document git log format and parsing strategy
   - Provide examples of usage

2. **Function docs**
   - All pure functions need examples showing transformations
   - Document edge cases and assumptions
   - Explain algorithmic complexity

3. **Design Rationale**
   - Comment explaining Stillwater philosophy application
   - Justification for data structure choices
   - Performance characteristics

### User Documentation

No user-facing documentation changes needed (transparent optimization).

### Architecture Updates

Add section to ARCHITECTURE.md:

```markdown
## Git History Analysis

The git history context provider uses a batched query approach:

1. **Imperative Shell**: Single git log query fetches all commit data
2. **Pure Core**: Parse and aggregate data using pure functions
3. **Functional Pipeline**: Compose transformations for clarity

This design provides 25x+ speedup while maintaining functional purity.
```

## Implementation Notes

### Parsing Challenges

1. **Binary Files**: numstat shows `-` instead of numbers
   - Handle with `str::parse().unwrap_or(0)`

2. **File Renames**: Git shows `old.rs => new.rs`
   - Parse rename syntax and track both paths
   - Credit commits to final filename

3. **Merge Commits**: May have empty numstat
   - Include in commit count but skip churn calculation

4. **Large Repositories**: Millions of commits
   - Stream parsing instead of loading all into memory
   - Build maps incrementally

### Performance Optimization

1. **Lazy Calculation**: Don't calculate metrics until needed
2. **Caching**: Memoize expensive calculations
3. **Parallel Aggregation**: Use rayon for map building if beneficial
4. **Memory Pooling**: Reuse allocations where possible

### Error Handling Strategy

```rust
// Pure error handling with context
fn parse_commit(entry: &str) -> Result<CommitInfo> {
    let parts: Vec<&str> = entry.split(":::").collect();

    let hash = parts.get(1)
        .context("Missing commit hash")?;

    let date = parts.get(2)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .context("Invalid commit date")?;

    // ... etc

    Ok(CommitInfo { hash, date, ... })
}
```

## Migration and Compatibility

### Backward Compatibility

- Maintain existing `GitHistoryProvider` API
- Preserve `FileHistory` struct exactly
- No changes to context provider interface
- Existing code using git history continues to work

### Feature Flag

```toml
[features]
default = ["batched_git_history"]
batched_git_history = []
```

```rust
#[cfg(feature = "batched_git_history")]
use batched::BatchedGitHistory as GitHistoryImpl;

#[cfg(not(feature = "batched_git_history"))]
use legacy::GitHistoryProvider as GitHistoryImpl;
```

### Migration Path

1. **Phase 1**: Implement batched version with feature flag
2. **Phase 2**: Run both implementations in tests, verify identical results
3. **Phase 3**: Enable batched version by default
4. **Phase 4**: Remove legacy implementation after validation

### Breaking Changes

None. This is a pure performance optimization with no API changes.

## Success Metrics

1. **Performance**: `--context` analysis < 10 seconds (vs 260s baseline)
2. **Accuracy**: 100% identical results to current implementation
3. **Memory**: Peak usage < 50MB for typical codebases
4. **Reliability**: No regressions in error handling
5. **Maintainability**: Code complexity reduced via pure functions

## References

- Stillwater Philosophy: `../stillwater/PHILOSOPHY.md`
- Current Implementation: `src/risk/context/git_history.rs`
- Context Provider Interface: `src/risk/context/mod.rs`
- Git Log Documentation: `man git-log`
