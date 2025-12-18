---
number: 195
title: Function-Level Git History Analysis
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-06-18
---

# Specification 195: Function-Level Git History Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The current git history analysis operates at the **file level**, which causes inaccurate risk scoring for individual functions. When a file has multiple commits including bug fixes, the bug density is incorrectly attributed to ALL functions in that file, even functions that were never modified.

**Current Problem:**

```
File: src/analysis/call_graph/framework_patterns.rs
- Total commits to file: 8
- Bug fix commits: 3 (37.5% bug density)
- Function `get_exclusions`: Created once, NEVER modified

Result: get_exclusions incorrectly scores 100.0 (critical) due to 2.24x contextual risk multiplier
Correct: get_exclusions should have 0% bug density (0 modifications, 0 bug fixes)
```

**Root Cause:**

The `GitHistoryProvider.gather()` method ignores the `line_range` from `AnalysisTarget` and only looks at file-level history. The `BatchedGitHistory` tracks commit counts and bug fixes per file, not per function.

**Key Insight:**

This function-level analysis only runs for debt items already identified by static analysis (complexity, coverage, etc.). We're analyzing maybe 10-100 functions, not thousands, making the additional git subprocess calls acceptable.

## Objective

Implement function-level git history analysis that:

1. **Identifies when a function was introduced** using `git log -S "fn function_name"`
2. **Counts only commits that modified the function** after its introduction
3. **Calculates accurate bug density** based on function-specific commits
4. **Falls back to file-level analysis** when function-level tracking fails

Result: Functions that were never modified after introduction have 0% bug density, not the file's bug density.

## Requirements

### Functional Requirements

1. **Function Introduction Detection**
   - Use `git log -S "fn {function_name}"` to find the commit that introduced the function
   - Handle multiple matches (function renamed, signature changed)
   - Use the oldest matching commit as the introduction point
   - Pure function to parse git output

2. **Function-Specific Commit Tracking**
   - Use `git log {intro_commit}..HEAD -S "{function_name}"` to find modifications
   - Count total modifications after introduction
   - Count bug fix commits using existing `is_bug_fix()` logic
   - Pure function to calculate metrics from commit list

3. **Accurate Bug Density Calculation**
   - `bug_density = bug_fix_count / total_function_commits`
   - If function never modified: `bug_density = 0.0`
   - If function never modified: `change_frequency = 0.0`
   - Pure function to calculate density

4. **Fallback Behavior**
   - If function name detection fails, fall back to file-level analysis
   - If git commands fail, fall back to file-level analysis
   - Log warnings when falling back for debugging

5. **Integration with Existing System**
   - Update `GitHistoryProvider.gather()` to use function-level when `function_name` is provided
   - Preserve existing file-level analysis for non-function targets
   - No breaking changes to public API

### Non-Functional Requirements

1. **Performance**
   - Max 2 git subprocess calls per function (introduction + modifications)
   - Cache results to avoid duplicate queries for same function
   - Total analysis time increase < 5 seconds for typical projects

2. **Accuracy**
   - Functions introduced and never modified: 0% bug density
   - Functions with modifications: accurate per-function bug density
   - No false attribution from other parts of the file

3. **Maintainability**
   - Pure functions for parsing and calculation
   - I/O operations isolated to thin wrapper functions
   - Following Stillwater philosophy: pure core, imperative shell

## Acceptance Criteria

- [ ] `git log -S` used to detect function introduction commit
- [ ] Only commits after introduction are counted for the function
- [ ] Functions never modified have bug_density = 0.0 and change_frequency = 0.0
- [ ] Existing `is_bug_fix()` and `is_excluded_commit()` logic reused
- [ ] Fallback to file-level analysis when function detection fails
- [ ] Pure functions for parsing git output (testable without git)
- [ ] Unit tests for parsing functions
- [ ] Integration test with real git repository
- [ ] No performance regression > 5 seconds for analysis
- [ ] `get_exclusions` example case scores correctly (not 100.0)

## Technical Details

### Implementation Approach

Following Stillwater philosophy: **Pure Core, Imperative Shell**

```
       Git Commands (I/O Shell)
      ╱                        ╲
 git log -S              git log range
     ↓                         ↓
   Raw Output              Raw Output
     ↓                         ↓
┌─────────────────────────────────────────┐
│           Pure Core Functions           │
│                                         │
│  parse_introduction_log() → Option<Hash>│
│  parse_modifications_log() → Vec<Commit>│
│  filter_bug_fixes() → Vec<Commit>       │
│  calculate_function_metrics() → Metrics │
│                                         │
│  (No I/O, easily testable)              │
└─────────────────────────────────────────┘
```

### Data Structures

```rust
/// History data for a specific function
#[derive(Debug, Clone, Default)]
pub struct FunctionHistory {
    /// Commit hash where function was introduced
    pub introduction_commit: Option<String>,
    /// Total commits that modified this function
    pub total_commits: usize,
    /// Bug fix commits that modified this function
    pub bug_fix_count: usize,
    /// Authors who modified this function
    pub authors: HashSet<String>,
    /// When function was last modified
    pub last_modified: Option<DateTime<Utc>>,
    /// When function was introduced
    pub introduced: Option<DateTime<Utc>>,
}

impl FunctionHistory {
    /// Calculate bug density for this function
    pub fn bug_density(&self) -> f64 {
        if self.total_commits == 0 {
            return 0.0; // Never modified = no bugs
        }
        self.bug_fix_count as f64 / self.total_commits as f64
    }

    /// Calculate change frequency (modifications per month)
    pub fn change_frequency(&self) -> f64 {
        let age_days = self.age_days();
        if age_days == 0 || self.total_commits == 0 {
            return 0.0;
        }
        (self.total_commits as f64 / age_days as f64) * 30.0
    }

    fn age_days(&self) -> u32 {
        self.introduced
            .map(|d| (Utc::now() - d).num_days().max(0) as u32)
            .unwrap_or(0)
    }
}
```

### Git Commands

**1. Find Function Introduction:**

```bash
# Returns commit hash(es) where function signature was added
git log -S "fn get_exclusions" --format="%H" --reverse -- src/file.rs | head -1
```

**2. Find Function Modifications:**

```bash
# Returns commits that modified the function after introduction
git log {intro_commit}..HEAD -S "get_exclusions" --format=":::%H:::%cI:::%s:::%ae" -- src/file.rs
```

### Pure Functions (Testable Without Git)

```rust
/// Parse git log output to find introduction commit
///
/// Pure function - parses string input, returns Option<String>
fn parse_introduction_commit(git_output: &str) -> Option<String> {
    git_output
        .lines()
        .next()
        .filter(|line| !line.is_empty())
        .map(|s| s.trim().to_string())
}

/// Parse git log output to extract commit information
///
/// Pure function - parses formatted git log output
fn parse_modification_commits(git_output: &str) -> Vec<CommitInfo> {
    git_output
        .lines()
        .filter(|line| line.starts_with(":::"))
        .filter_map(parse_commit_line)
        .collect()
}

/// Parse a single commit line from formatted output
fn parse_commit_line(line: &str) -> Option<CommitInfo> {
    let parts: Vec<&str> = line.split(":::").collect();
    if parts.len() < 5 {
        return None;
    }
    Some(CommitInfo {
        hash: parts[1].to_string(),
        date: DateTime::parse_from_rfc3339(parts[2]).ok()?,
        message: parts[3].to_string(),
        author: parts[4].to_string(),
    })
}

/// Filter commits to only bug fixes
///
/// Pure function - uses existing is_bug_fix() logic
fn filter_bug_fix_commits(commits: &[CommitInfo]) -> Vec<&CommitInfo> {
    commits
        .iter()
        .filter(|c| is_bug_fix(&c.message))
        .collect()
}

/// Calculate function history from parsed commits
///
/// Pure function - aggregates commit data into history
fn calculate_function_history(
    introduction_commit: Option<String>,
    introduction_date: Option<DateTime<Utc>>,
    modification_commits: &[CommitInfo],
) -> FunctionHistory {
    let bug_fixes = filter_bug_fix_commits(modification_commits);

    FunctionHistory {
        introduction_commit,
        total_commits: modification_commits.len(),
        bug_fix_count: bug_fixes.len(),
        authors: modification_commits.iter().map(|c| c.author.clone()).collect(),
        last_modified: modification_commits.iter().map(|c| c.date).max(),
        introduced: introduction_date,
    }
}
```

### I/O Wrapper Functions

```rust
/// Get function history from git (I/O Shell)
///
/// This is the imperative shell that orchestrates git commands
pub fn get_function_history(
    repo_root: &Path,
    file_path: &Path,
    function_name: &str,
) -> Result<FunctionHistory> {
    // I/O: Find introduction commit
    let intro_output = run_git_log_introduction(repo_root, file_path, function_name)?;
    let intro_commit = parse_introduction_commit(&intro_output);

    // If no introduction found, function doesn't exist in history
    let Some(ref intro) = intro_commit else {
        return Ok(FunctionHistory::default());
    };

    // I/O: Get introduction date
    let intro_date = get_commit_date(repo_root, intro)?;

    // I/O: Find modifications after introduction
    let mods_output = run_git_log_modifications(repo_root, file_path, function_name, intro)?;
    let modification_commits = parse_modification_commits(&mods_output);

    // Pure: Calculate history from parsed data
    Ok(calculate_function_history(
        intro_commit,
        intro_date,
        &modification_commits,
    ))
}

/// Run git log -S to find function introduction (I/O)
fn run_git_log_introduction(
    repo_root: &Path,
    file_path: &Path,
    function_name: &str,
) -> Result<String> {
    let search_pattern = format!("fn {}", function_name);
    let output = Command::new("git")
        .args([
            "log",
            "-S", &search_pattern,
            "--format=%H",
            "--reverse",
            "--",
            &file_path.to_string_lossy(),
        ])
        .current_dir(repo_root)
        .output()
        .context("Failed to run git log -S")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run git log to find modifications after introduction (I/O)
fn run_git_log_modifications(
    repo_root: &Path,
    file_path: &Path,
    function_name: &str,
    intro_commit: &str,
) -> Result<String> {
    let range = format!("{}..HEAD", intro_commit);
    let output = Command::new("git")
        .args([
            "log",
            &range,
            "-S", function_name,
            "--format=:::%H:::%cI:::%s:::%ae",
            "--",
            &file_path.to_string_lossy(),
        ])
        .current_dir(repo_root)
        .output()
        .context("Failed to run git log range")?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
```

### Integration with GitHistoryProvider

```rust
impl ContextProvider for GitHistoryProvider {
    fn gather(&self, target: &AnalysisTarget) -> Result<Context> {
        // Try function-level analysis if function name is provided
        if !target.function_name.is_empty() {
            match self.gather_for_function(target) {
                Ok(context) => return Ok(context),
                Err(e) => {
                    log::debug!(
                        "Function-level git analysis failed for {}, falling back to file-level: {}",
                        target.function_name,
                        e
                    );
                }
            }
        }

        // Fall back to file-level analysis
        self.gather_for_file(target)
    }

    fn gather_for_function(&self, target: &AnalysisTarget) -> Result<Context> {
        let history = get_function_history(
            &self.repo_root,
            &target.file_path,
            &target.function_name,
        )?;

        let contribution = Self::classify_risk_contribution(
            history.change_frequency(),
            history.bug_density(),
        );

        Ok(Context {
            provider: self.name().to_string(),
            weight: self.weight(),
            contribution,
            details: ContextDetails::Historical {
                change_frequency: history.change_frequency(),
                bug_density: history.bug_density(),
                age_days: history.age_days(),
                author_count: history.authors.len(),
            },
        })
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/risk/context/git_history.rs` - Main implementation
  - `src/risk/context/git_history/batched.rs` - New function-level module
  - `src/risk/context/mod.rs` - Integration point
- **External Dependencies**: None (uses existing git CLI)

## Testing Strategy

### Unit Tests (Pure Functions)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_introduction_commit_found() {
        let output = "abc123def456\n";
        let result = parse_introduction_commit(output);
        assert_eq!(result, Some("abc123def456".to_string()));
    }

    #[test]
    fn test_parse_introduction_commit_empty() {
        let output = "";
        let result = parse_introduction_commit(output);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_modification_commits() {
        let output = r#":::abc123:::2025-01-01T10:00:00Z:::fix: bug:::author@example.com
:::def456:::2025-01-02T10:00:00Z:::feat: feature:::author@example.com"#;

        let commits = parse_modification_commits(output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "fix: bug");
        assert_eq!(commits[1].message, "feat: feature");
    }

    #[test]
    fn test_filter_bug_fix_commits() {
        let commits = vec![
            CommitInfo { message: "fix: bug".to_string(), ..Default::default() },
            CommitInfo { message: "feat: feature".to_string(), ..Default::default() },
            CommitInfo { message: "hotfix: urgent".to_string(), ..Default::default() },
        ];

        let bug_fixes = filter_bug_fix_commits(&commits);
        assert_eq!(bug_fixes.len(), 2);
    }

    #[test]
    fn test_function_history_never_modified() {
        let history = FunctionHistory {
            total_commits: 0,
            bug_fix_count: 0,
            ..Default::default()
        };

        assert_eq!(history.bug_density(), 0.0);
        assert_eq!(history.change_frequency(), 0.0);
    }

    #[test]
    fn test_function_history_with_modifications() {
        let history = FunctionHistory {
            total_commits: 4,
            bug_fix_count: 1,
            introduced: Some(Utc::now() - chrono::Duration::days(30)),
            ..Default::default()
        };

        assert_eq!(history.bug_density(), 0.25);
        assert!(history.change_frequency() > 3.5 && history.change_frequency() < 4.5);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_function_level_git_history_integration() -> Result<()> {
    let temp = setup_test_repo()?;

    // Create file with function
    create_and_commit(&temp, "test.rs", "fn my_func() {}", "Initial commit")?;

    // Modify other parts of file (not the function)
    modify_and_commit(&temp, "test.rs", "fn my_func() {}\nfn other() {}", "fix: other bug")?;
    modify_and_commit(&temp, "test.rs", "fn my_func() {}\nfn other() { x }", "fix: another bug")?;

    let history = get_function_history(&temp, Path::new("test.rs"), "my_func")?;

    // my_func was introduced but never modified
    assert_eq!(history.total_commits, 0);
    assert_eq!(history.bug_fix_count, 0);
    assert_eq!(history.bug_density(), 0.0);

    Ok(())
}
```

## Documentation Requirements

- **Code Documentation**: Document all pure functions with examples
- **Architecture Updates**: Add function-level analysis pattern to ARCHITECTURE.md
- **User Documentation**: None (internal optimization)

## Implementation Notes

### Edge Cases

1. **Function renamed**: `git log -S` will show two commits (remove old name, add new name)
   - Take the oldest commit as introduction

2. **Function signature changed**: Same pattern as rename
   - Use function name only, not full signature

3. **Multiple functions with same name**: Rare in Rust (different modules)
   - File path scoping handles this

4. **Function moved between files**: Will appear as new introduction
   - Acceptable: different file context is different risk context

### Performance Considerations

- Each debt item requires 2 git commands (introduction + modifications)
- With 50 debt items, that's 100 git commands
- Git commands are fast (< 50ms each typically)
- Total overhead: < 5 seconds

### Caching Opportunity

Future optimization: Cache function introduction commits in `BatchedGitHistory` during initial load. Would require parsing diffs to extract function names.

## Migration and Compatibility

### Breaking Changes

**None** - Internal optimization. Public API unchanged.

### Behavior Changes

- Functions with no modifications will score lower (correct behavior)
- Functions with high modification rate will score appropriately
- File-level fallback ensures no analysis failures

## Success Metrics

- `get_exclusions` function: bug_density = 0.0 (was incorrectly ~37%)
- Functions introduced and never modified: bug_density = 0.0
- No increase in analysis time > 5 seconds
- All existing tests pass

## References

- **Stillwater PHILOSOPHY.md** - Pure core, imperative shell pattern
- **spec 187** - Function extraction patterns
- **git-scm.com** - `git log -S` (pickaxe) documentation
