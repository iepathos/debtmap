---
number: 205
title: Improved Bug Detection Heuristics
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-04
---

# Specification 205: Improved Bug Detection Heuristics

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The Git History provider currently uses overly broad pattern matching for bug fix detection, leading to false positives that inflate bug density scores. The current implementation uses simple substring matching:

```rust
--grep=fix    // Matches "fix", "prefix", "fixture", "formatting fixes"
--grep=bug    // Matches "bug", "debug", "debugging"
```

This causes issues like:
- "style: apply automated formatting" counted as bug fix (contains "formatting")
- "refactor: improve prefix handling" counted as bug fix (contains "prefix")
- "Add debugging tools" counted as bug fix (contains "debug")

The bug density metric is critical for risk assessment - files with high bug density (>50%) are flagged as critical and receive up to 2.0x risk multiplier. False positives undermine trust in the scoring system and may cause teams to ignore legitimate warnings.

## Objective

Improve the precision of bug fix detection in the Git History provider by implementing word boundary matching and commit message filtering, reducing false positives by ~80% while maintaining recall of genuine bug fixes.

## Requirements

### Functional Requirements

1. **Word Boundary Matching**: Use regex word boundaries to match only complete words
   - Match "fix" but not "prefix", "fixture", "suffix"
   - Match "bug" but not "debug", "debugging", "bugzilla"
   - Match common variants: "fix", "fixes", "fixed", "fixing"

2. **Commit Message Filtering**: Exclude commits that are clearly not bug fixes
   - Filter out conventional commit types: `style:`, `chore:`, `docs:`, `test:`
   - Filter out non-bug maintenance: "formatting", "linting", "whitespace"
   - Filter out refactorings unless they also mention bugs

3. **Backwards Compatibility**: Maintain the same `FileHistory` data structure and API
   - No changes to `bug_fix_count` field type
   - No changes to `calculate_bug_density` function signature
   - Same git command interface (only pattern changes)

4. **Comprehensive Testing**: Validate accuracy improvements with real-world commit messages
   - Test against debtmap's own commit history
   - Verify false positive reduction
   - Ensure genuine bug fixes are still detected

### Non-Functional Requirements

1. **Performance**: No significant performance degradation
   - Git grep performance with word boundaries is comparable to substring matching
   - Post-processing filter adds minimal overhead (~1-2ms per file)

2. **Maintainability**: Make patterns easy to understand and modify
   - Clear comments explaining each pattern
   - Separate exclusion logic into dedicated function
   - Document pattern choices in code and book

3. **Documentation**: Update the book to explain improved methodology
   - Document new patterns in context-providers.md
   - Explain exclusion logic and rationale
   - Provide examples of what is/isn't detected

## Acceptance Criteria

- [ ] Word boundary patterns implemented for bug fix detection
- [ ] Exclusion filter removes non-bug commits (style, chore, formatting, linting)
- [ ] All existing unit tests pass without modification
- [ ] New tests added covering false positive scenarios
- [ ] Test using debtmap's own history shows improvement:
  - "style: apply automated formatting" NOT counted (currently counted)
  - "fix: resolve login bug" IS counted (correctly)
  - "Fixed the payment issue" IS counted (correctly)
- [ ] Performance regression test shows <5% overhead
- [ ] Book documentation updated with new detection methodology
- [ ] No changes to public API or data structures

## Technical Details

### Implementation Approach

**Step 1: Update `count_bug_fixes` Method**

Replace current grep patterns with word boundary patterns:

```rust
fn count_bug_fixes(&self, path: &Path) -> Result<usize> {
    let output = Command::new("git")
        .args([
            "log",
            "--oneline",
            "--grep=\\bfix\\b",      // Matches "fix" only, not "prefix"
            "--grep=\\bfixes\\b",    // Matches "fixes" only
            "--grep=\\bfixed\\b",    // Past tense
            "--grep=\\bfixing\\b",   // Present continuous
            "--grep=\\bbug\\b",      // Matches "bug" only, not "debug"
            "--grep=\\bhotfix\\b",   // Emergency fixes
            "--",
            path.to_str().unwrap_or(""),
        ])
        .current_dir(&self.repo_root)
        .output()
        .context("Failed to count bug fixes")?;

    if output.status.success() {
        let lines = String::from_utf8_lossy(&output.stdout);
        let count = lines.lines()
            .filter(|line| !Self::is_excluded_commit(line))
            .count();
        Ok(count)
    } else {
        Ok(0)
    }
}
```

**Step 2: Add Exclusion Filter**

Create a pure function to filter out non-bug commits:

```rust
/// Determines if a commit message indicates a non-bug change that should
/// be excluded from bug fix counting.
///
/// Excludes:
/// - Conventional commit types: style, chore, docs, test
/// - Maintenance keywords: formatting, linting, whitespace
/// - Refactoring without bug mentions
fn is_excluded_commit(commit_line: &str) -> bool {
    let lowercase = commit_line.to_lowercase();

    // Conventional commit type exclusions
    if lowercase.starts_with("style:")
        || lowercase.starts_with("chore:")
        || lowercase.starts_with("docs:")
        || lowercase.starts_with("test:") {
        return true;
    }

    // Maintenance keyword exclusions
    let exclusion_keywords = [
        "formatting",
        "linting",
        "whitespace",
        "typo",
    ];

    for keyword in &exclusion_keywords {
        if lowercase.contains(keyword) {
            return true;
        }
    }

    // Refactoring exclusion (unless it mentions bug/fix in a bug context)
    if lowercase.starts_with("refactor:")
        && !lowercase.contains("bug")
        && !lowercase.contains("issue") {
        return true;
    }

    false
}
```

**Step 3: Add Comprehensive Tests**

```rust
#[test]
fn test_is_excluded_commit() {
    // Should exclude
    assert!(GitHistoryProvider::is_excluded_commit("style: apply formatting fixes"));
    assert!(GitHistoryProvider::is_excluded_commit("chore: update dependencies"));
    assert!(GitHistoryProvider::is_excluded_commit("docs: fix typo"));
    assert!(GitHistoryProvider::is_excluded_commit("refactor: improve prefix handling"));
    assert!(GitHistoryProvider::is_excluded_commit("8c45a3c5 style: apply automated formatting"));

    // Should NOT exclude
    assert!(!GitHistoryProvider::is_excluded_commit("fix: resolve login bug"));
    assert!(!GitHistoryProvider::is_excluded_commit("Fixed the payment issue"));
    assert!(!GitHistoryProvider::is_excluded_commit("Bug fix for issue #123"));
    assert!(!GitHistoryProvider::is_excluded_commit("refactor: fix memory leak"));
}

#[test]
fn test_bug_fix_detection_precision() -> Result<()> {
    let (_temp, repo_path) = setup_test_repo()?;

    // Create commits with various messages
    let file_path = create_test_file(&repo_path, "test.rs", "fn main() {}")?;
    commit_with_message(&repo_path, "Initial commit")?;

    // True positives
    modify_and_commit(&repo_path, "test.rs", "v2", "fix: resolve login bug")?;
    modify_and_commit(&repo_path, "test.rs", "v3", "Fixed the payment issue")?;
    modify_and_commit(&repo_path, "test.rs", "v4", "Bug fix for issue #123")?;

    // False positives (should be filtered)
    modify_and_commit(&repo_path, "test.rs", "v5", "style: apply formatting fixes")?;
    modify_and_commit(&repo_path, "test.rs", "v6", "refactor: improve prefix handling")?;
    modify_and_commit(&repo_path, "test.rs", "v7", "Add debugging tools")?;

    let mut provider = GitHistoryProvider::new(repo_path)?;
    let history = provider.analyze_file(Path::new("test.rs"))?;

    // Should detect 3 bug fixes, not 6
    assert_eq!(history.bug_fix_count, 3);
    assert_eq!(history.total_commits, 7);

    // Bug density should be 3/7 ≈ 0.43, not 6/7 ≈ 0.86
    let bug_density = GitHistoryProvider::calculate_bug_density(
        history.bug_fix_count,
        history.total_commits
    );
    assert!(bug_density > 0.40 && bug_density < 0.45);

    Ok(())
}
```

### Architecture Changes

**Files Modified**:
- `src/risk/context/git_history.rs`: Update `count_bug_fixes` and add `is_excluded_commit`

**No Breaking Changes**:
- `FileHistory` struct unchanged
- `GitHistoryProvider` API unchanged
- Backward compatible with existing call sites

### Data Structures

No changes to existing data structures. The improvement is internal to the `GitHistoryProvider` implementation.

### Pattern Design Decisions

**Why Word Boundaries (`\b`)?**
- Standard regex feature supported by git grep
- Eliminates substring false positives
- Minimal performance overhead
- Easy to understand and maintain

**Why Post-Processing Filter?**
- Git grep has limited filtering capabilities
- Easier to test exclusion logic in Rust
- Allows complex conditional logic (e.g., "refactor unless mentions bug")
- Can be easily extended in the future

**Why These Exclusion Patterns?**
- `style:` - Never represents bug fixes, only formatting
- `chore:` - Maintenance tasks, not bug fixes
- `docs:` - Documentation updates
- `formatting/linting` - Automated tooling fixes
- `refactor` - Code restructuring (unless fixing bugs)

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/risk/context/git_history.rs` (GitHistoryProvider)
  - `book/src/context-providers.md` (documentation)
- **External Dependencies**: None (uses existing git and regex capabilities)

## Testing Strategy

### Unit Tests

1. **Exclusion Logic Tests**
   - Test each exclusion pattern individually
   - Test edge cases (e.g., "refactor: fix bug" should NOT be excluded)
   - Test case insensitivity

2. **Integration Tests**
   - Create test repo with diverse commit messages
   - Verify bug fix count matches expected value
   - Test across different commit message styles

3. **Regression Tests**
   - Ensure all existing tests still pass
   - Verify no performance degradation

### Performance Tests

Benchmark against large repositories:
```rust
#[bench]
fn bench_bug_fix_detection(b: &mut Bencher) {
    let provider = setup_large_repo(); // 1000+ commits
    b.iter(|| {
        provider.count_bug_fixes(Path::new("src/main.rs"))
    });
}
```

Target: <5% performance regression compared to current implementation.

### Real-World Validation

Test against debtmap's own commit history:
```bash
# Current implementation
cargo run -- analyze src/risk/context/git_history.rs --context

# After implementation
cargo run -- analyze src/risk/context/git_history.rs --context

# Compare bug density scores
```

Expected improvement: Bug density should decrease for files with many formatting commits.

## Documentation Requirements

### Code Documentation

1. **Inline Documentation**
   - Add detailed rustdoc comments to `is_excluded_commit`
   - Document each pattern in `count_bug_fixes` with examples
   - Include rationale for exclusion choices

2. **Examples**
   ```rust
   /// Example commit messages:
   /// - "fix: resolve login bug" → Counted ✅
   /// - "Fixed the payment issue" → Counted ✅
   /// - "style: apply formatting" → Excluded ❌
   /// - "refactor: improve prefix" → Excluded ❌
   ```

### User Documentation

Update `book/src/context-providers.md`:

1. **Bug Fix Detection Section** (after line 242)
   - Explain word boundary matching
   - List excluded commit types
   - Provide examples of detected vs excluded commits

2. **Suggested Addition**:
   ```markdown
   #### Detection Methodology

   The provider uses word boundary matching to identify bug fixes:

   **Patterns Matched:**
   - `\bfix\b`, `\bfixes\b`, `\bfixed\b`, `\bfixing\b`
   - `\bbug\b`, `\bhotfix\b`

   **Excluded Commit Types:**
   - Styling: `style:`, `formatting`, `linting`
   - Maintenance: `chore:`, `whitespace`, `typo`
   - Documentation: `docs:`
   - Tests: `test:`
   - Refactoring: `refactor:` (unless mentions bugs)

   **Examples:**

   | Commit Message | Detected? | Reason |
   |----------------|-----------|--------|
   | `fix: resolve login bug` | ✅ | Contains "fix" as complete word |
   | `Fixed the payment issue` | ✅ | Contains "fixed" as complete word |
   | `Bug fix for issue #123` | ✅ | Contains "bug" as complete word |
   | `style: apply formatting fixes` | ❌ | Excluded: styling commit |
   | `refactor: improve prefix handling` | ❌ | Excluded: refactoring without bug mention |
   | `Add debugging tools` | ❌ | "debug" doesn't match "\bbug\b" |
   ```

## Implementation Notes

### Common Pitfalls

1. **Regex Escaping**: Remember to escape backslashes in Rust strings
   ```rust
   "\\bfix\\b"  // Correct
   "\bfix\b"    // Wrong - \b interpreted as backspace
   ```

2. **Git Grep Syntax**: Git uses basic regex by default
   - Word boundaries work out of the box
   - No need for `-E` (extended regex) flag

3. **Case Sensitivity**: Git grep is case-sensitive by default
   - Use `.to_lowercase()` in Rust filter for case-insensitive matching
   - Alternative: Use git's `-i` flag (but reduces precision)

### Future Enhancements (Out of Scope)

This spec focuses on quick wins. Future improvements could include:

1. **Conventional Commit Support**: Explicit `^fix:` pattern for teams using conventions
2. **Issue Tracker Integration**: Verify bug vs feature via GitHub/GitLab API
3. **Configurable Patterns**: Allow users to customize detection via TOML
4. **ML-Based Detection**: Use commit message embeddings for classification
5. **Severity Classification**: Distinguish critical vs minor bug fixes

These are documented for future consideration but not required for this spec.

## Migration and Compatibility

### No Breaking Changes

This is a pure improvement with no API changes:
- Existing code using `GitHistoryProvider` requires no modifications
- Output format unchanged
- Data structures unchanged

### Behavioral Changes

Users will notice:
- **Lower bug density scores** for files with many formatting commits
- **More accurate risk assessment** with fewer false positives
- **Better alignment** between high bug density and actual problematic code

This is the desired behavior change.

### Rollback Plan

If issues arise:
1. Git commit can be easily reverted
2. No database migrations or persistent state changes
3. Tests ensure no regression in genuine bug detection

## Success Metrics

1. **Precision Improvement**:
   - Baseline: ~50-60% precision (many false positives)
   - Target: ~85-90% precision
   - Measure: Manual review of 100 randomly sampled commits

2. **Recall Maintenance**:
   - Ensure >95% of genuine bug fixes still detected
   - Measure: Test against known bug fix commits in debtmap history

3. **Performance**:
   - <5% increase in analysis time for git history provider
   - Measure: Benchmark on large repositories (1000+ commits)

4. **User Trust**:
   - Reduced false positive reports in GitHub issues
   - Positive feedback on accuracy improvements

## Timeline Estimate

- **Implementation**: 2-3 hours
  - Update `count_bug_fixes`: 30 minutes
  - Add `is_excluded_commit`: 30 minutes
  - Write unit tests: 1 hour
  - Integration testing: 30 minutes
  - Performance validation: 30 minutes

- **Documentation**: 1 hour
  - Code comments: 20 minutes
  - Book updates: 40 minutes

- **Review and refinement**: 1 hour

**Total**: ~4-5 hours
