---
number: 2
title: Batched Git Blame Cache
category: optimization
priority: critical
status: draft
dependencies: []
created: 2025-12-21
---

# Specification 002: Batched Git Blame Cache

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: None

## Context

The `get_function_history` function in `src/risk/context/git_history/function_level.rs` is the largest performance bottleneck in debtmap analysis. Profiling shows it accounts for the majority of analysis time on large codebases.

### Current Implementation Problem

For each function analyzed, the code spawns **4 separate git subprocess calls**:

| Call | Purpose | Git Command |
|------|---------|-------------|
| 1 | Find introduction commit | `git log -S "fn func_name" --format=%H --reverse` |
| 2 | Get introduction date | `git log -1 --format=%cI <commit>` |
| 3 | Find modifications | `git log <intro>..HEAD -G func_name --format=...` |
| 4 | Get blame authors | `git blame -L start,end --porcelain <file>` |

For a codebase with 12,000 functions, this means **48,000 subprocess spawns** - the dominant cost being process creation overhead and git repository traversal.

### The Blame Problem Specifically

The `get_blame_authors` function (line 315-344) calls `git blame -L start,end` **separately for each function in a file**. For a file with 15 functions:

```
Function 1: lines 10-25   → git blame -L 10,25 src/analyzer.rs
Function 2: lines 30-45   → git blame -L 30,45 src/analyzer.rs
...
Function 15: lines 400-450 → git blame -L 400,450 src/analyzer.rs
```

That's **15 subprocess calls** to git, each one:
1. Spawns a new process
2. Opens and reads the git index
3. Traverses commit history for that file
4. Outputs only a small slice of the result

### Stillwater Philosophy Application

Following the Stillwater philosophy of "Pure Core, Imperative Shell":

```
       Batched Git Blame
      ╱                  ╲
 Pure Logic            Effects
     ↓                    ↓
  parse_blame()      fetch_file_blame()
  extract_authors()  (single subprocess)
  HashSet lookup     DashMap cache
```

- **Pure functions**: Parse blame output, extract authors for line ranges
- **I/O boundary**: Single git blame call per file, cached in DashMap
- **Composition**: Small, testable functions composed together

## Objective

Reduce git blame subprocess calls from N (functions per file) to 1 per file by implementing a per-file blame cache, following Stillwater's "pure core, imperative shell" pattern.

## Requirements

### Functional Requirements

1. **File-Level Blame Fetching**
   - Call `git blame --porcelain <file>` once per file (no `-L` flag)
   - Parse the complete blame output into a line-indexed data structure
   - Cache the result for subsequent function lookups

2. **Line Range Author Extraction**
   - Given a line range `(start, end)`, extract unique authors from cached blame data
   - Pure function with O(n) iteration over line range, O(1) lookups
   - Return `HashSet<String>` of author names

3. **Thread-Safe Caching**
   - Use `DashMap<PathBuf, HashMap<usize, BlameLineInfo>>` for concurrent access
   - Support parallel function analysis with `rayon`
   - Lock-free reads for cached files

4. **Graceful Degradation**
   - If blame fails for a file (binary, not tracked, etc.), return empty authors
   - Log warning but don't fail the entire analysis
   - Cache negative results to avoid repeated failures

### Non-Functional Requirements

1. **Performance Target**
   - Reduce blame calls by 10x or more (avg 10+ functions per file)
   - Sub-millisecond author lookups for cached files
   - Minimal memory overhead (blame data is small)

2. **Stillwater Compliance**
   - Pure parsing functions testable without git
   - I/O isolated to single fetch function
   - Clear separation of concerns

3. **Backward Compatibility**
   - Existing `get_blame_authors` signature preserved
   - No changes to `FunctionHistory` struct
   - Drop-in replacement for current implementation

## Acceptance Criteria

- [ ] `FileBlameCache` struct created with `DashMap` storage
- [ ] `parse_full_blame_output` pure function implemented and tested
- [ ] `fetch_file_blame` I/O function implemented
- [ ] `get_authors_for_range` pure lookup function implemented
- [ ] Cache integrated into `GitHistoryProvider`
- [ ] `get_function_history` uses cached blame lookup
- [ ] Unit tests for all pure parsing functions (no git required)
- [ ] Integration test verifying 10x reduction in blame calls
- [ ] All existing git_history tests pass

## Technical Details

### Data Structures

```rust
/// Information extracted from a single blame line
#[derive(Debug, Clone)]
pub struct BlameLineInfo {
    pub author: String,
    pub commit_hash: String,
}

/// Cache for file-level blame data
pub struct FileBlameCache {
    /// Maps file_path -> (line_number -> blame info)
    cache: DashMap<PathBuf, HashMap<usize, BlameLineInfo>>,
    repo_root: PathBuf,
}
```

### Pure Functions (Testable Without Git)

```rust
/// Parse git blame --porcelain output into line-indexed map
///
/// Pure function - parses string input, returns HashMap
pub fn parse_full_blame_output(blame_output: &str) -> HashMap<usize, BlameLineInfo> {
    // Parse porcelain format:
    // <commit_hash> <orig_line> <final_line> <num_lines>
    // author <name>
    // author-mail <email>
    // ...
    // \t<line content>
}

/// Extract unique authors for a line range from cached data
///
/// Pure function - O(n) iteration, O(1) lookups
pub fn extract_authors_for_range(
    blame_data: &HashMap<usize, BlameLineInfo>,
    start_line: usize,
    end_line: usize,
) -> HashSet<String> {
    (start_line..=end_line)
        .filter_map(|line| blame_data.get(&line))
        .map(|info| info.author.clone())
        .filter(|author| author != "Not Committed Yet")
        .collect()
}
```

### I/O Boundary (Imperative Shell)

```rust
impl FileBlameCache {
    /// Get or fetch blame data for entire file (I/O boundary)
    pub fn get_or_fetch(&self, file_path: &Path) -> Result<&HashMap<usize, BlameLineInfo>> {
        // Check cache first (lock-free read)
        if let Some(cached) = self.cache.get(file_path) {
            return Ok(cached.value());
        }

        // I/O: Single git blame call for entire file
        let blame_output = self.fetch_file_blame(file_path)?;

        // Pure: Parse the output
        let blame_data = parse_full_blame_output(&blame_output);

        // Cache for future lookups
        self.cache.insert(file_path.to_path_buf(), blame_data);

        Ok(self.cache.get(file_path).unwrap().value())
    }

    /// Fetch complete blame for a file (I/O)
    fn fetch_file_blame(&self, file_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(["blame", "--porcelain", &file_path.to_string_lossy()])
            .current_dir(&self.repo_root)
            .output()
            .context("Failed to run git blame")?;

        if !output.status.success() {
            // Return empty for untracked/binary files
            return Ok(String::new());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get authors for a function's line range (public API)
    pub fn get_authors(
        &self,
        file_path: &Path,
        start_line: usize,
        end_line: usize,
    ) -> Result<HashSet<String>> {
        let blame_data = self.get_or_fetch(file_path)?;
        Ok(extract_authors_for_range(blame_data, start_line, end_line))
    }
}
```

### Integration with GitHistoryProvider

```rust
pub struct GitHistoryProvider {
    repo_root: PathBuf,
    cache: Arc<DashMap<PathBuf, FileHistory>>,
    batched_history: Option<batched::BatchedGitHistory>,
    blame_cache: FileBlameCache,  // NEW: Add blame cache
}

impl GitHistoryProvider {
    pub fn new(repo_root: PathBuf) -> Result<Self> {
        // ... existing code ...

        let blame_cache = FileBlameCache::new(repo_root.clone());

        Ok(Self {
            repo_root,
            cache: Arc::new(DashMap::new()),
            batched_history,
            blame_cache,  // NEW
        })
    }
}
```

### Updated get_function_history

```rust
pub fn get_function_history(
    repo_root: &Path,
    file_path: &Path,
    function_name: &str,
    line_range: (usize, usize),
    blame_cache: &FileBlameCache,  // NEW: Accept cache reference
) -> Result<FunctionHistory> {
    // ... existing intro commit and modifications logic ...

    // NEW: Use cached blame instead of per-function call
    let (start, end) = line_range;
    let blame_authors = blame_cache.get_authors(file_path, start, end)?;

    Ok(calculate_function_history_with_authors(
        intro_commit,
        intro_date,
        &modification_commits,
        blame_authors,
    ))
}
```

## Testing Strategy

### Unit Tests (No Git Required)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_blame_output_single_author() {
        let blame_output = r#"abc123def456 1 1 3
author John Doe
author-mail <john@example.com>
author-time 1234567890
author-tz +0000
committer Jane Smith
committer-mail <jane@example.com>
committer-time 1234567890
committer-tz +0000
summary Initial commit
filename src/test.rs
	fn foo() {
abc123def456 2 2
	    bar();
abc123def456 3 3
	}
"#;

        let blame_data = parse_full_blame_output(blame_output);

        assert_eq!(blame_data.len(), 3);
        assert_eq!(blame_data.get(&1).unwrap().author, "John Doe");
        assert_eq!(blame_data.get(&2).unwrap().author, "John Doe");
        assert_eq!(blame_data.get(&3).unwrap().author, "John Doe");
    }

    #[test]
    fn test_parse_full_blame_output_multiple_authors() {
        // Test with multiple authors across different lines
    }

    #[test]
    fn test_extract_authors_for_range() {
        let mut blame_data = HashMap::new();
        blame_data.insert(1, BlameLineInfo { author: "Alice".into(), commit_hash: "abc".into() });
        blame_data.insert(2, BlameLineInfo { author: "Bob".into(), commit_hash: "def".into() });
        blame_data.insert(3, BlameLineInfo { author: "Alice".into(), commit_hash: "abc".into() });
        blame_data.insert(4, BlameLineInfo { author: "Charlie".into(), commit_hash: "ghi".into() });

        let authors = extract_authors_for_range(&blame_data, 1, 3);

        assert_eq!(authors.len(), 2);
        assert!(authors.contains("Alice"));
        assert!(authors.contains("Bob"));
        assert!(!authors.contains("Charlie"));
    }

    #[test]
    fn test_extract_authors_filters_not_committed() {
        let mut blame_data = HashMap::new();
        blame_data.insert(1, BlameLineInfo { author: "Alice".into(), commit_hash: "abc".into() });
        blame_data.insert(2, BlameLineInfo { author: "Not Committed Yet".into(), commit_hash: "000".into() });

        let authors = extract_authors_for_range(&blame_data, 1, 2);

        assert_eq!(authors.len(), 1);
        assert!(authors.contains("Alice"));
    }

    #[test]
    fn test_extract_authors_empty_range() {
        let blame_data = HashMap::new();
        let authors = extract_authors_for_range(&blame_data, 100, 200);
        assert!(authors.is_empty());
    }
}
```

### Integration Tests

```rust
#[test]
fn test_blame_cache_reduces_subprocess_calls() {
    // Create temp git repo with test file
    // Call get_authors for multiple function ranges in same file
    // Verify only 1 git blame call was made (check command count or timing)
}

#[test]
fn test_blame_cache_thread_safety() {
    // Use rayon to call get_authors from multiple threads
    // Verify no data races or duplicate fetches
}
```

## Dependencies

- **Prerequisites**: None (standalone optimization)
- **Affected Components**:
  - `src/risk/context/git_history/function_level.rs`
  - `src/risk/context/git_history.rs` (GitHistoryProvider)
- **External Dependencies**: None new (uses existing `dashmap`, `std::process::Command`)

## Documentation Requirements

- **Code Documentation**: Document pure vs I/O functions per Stillwater pattern
- **Architecture Updates**: Update git_history module docs to describe caching strategy

## Implementation Notes

### Git Blame Porcelain Format

The `--porcelain` format outputs structured data that's easy to parse:

```
<40-char-hash> <orig-line> <final-line> [<num-lines>]
author <author-name>
author-mail <author-email>
author-time <unix-timestamp>
author-tz <timezone>
committer <committer-name>
committer-mail <committer-email>
committer-time <unix-timestamp>
committer-tz <timezone>
summary <first-line-of-commit-message>
[previous <hash> <filename>]  # if line was copied
filename <filename>
\t<actual-line-content>
```

For subsequent lines from the same commit, only the hash line and content are repeated.

### Edge Cases

1. **Binary files**: `git blame` fails with non-zero exit - return empty authors
2. **Untracked files**: Same handling as binary
3. **Empty files**: Valid but no lines - return empty authors
4. **Files with uncommitted changes**: Lines show "Not Committed Yet" author - filter out

### Memory Considerations

Blame data is relatively small:
- ~100 bytes per line (author string + hash)
- 1000-line file = ~100KB cached data
- 100 files = ~10MB total cache

This is negligible compared to AST data already in memory.

## Migration and Compatibility

This is a pure performance optimization with no breaking changes:

- Existing `get_blame_authors` function signature preserved as wrapper
- `FunctionHistory` struct unchanged
- All existing tests continue to pass
- Cache is transparent to callers
