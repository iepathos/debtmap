---
number: 123
title: Fix Debtmap LOC Counting Discrepancy
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-05
---

# Specification 123: Fix Debtmap LOC Counting Discrepancy

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap reports wildly different LOC (Lines of Code) counts depending on whether coverage data is provided, even though it's analyzing the same codebase:

- **Without coverage**: 29,724 LOC
- **With coverage**: 35,336 LOC (19% more)

This 5,612-line discrepancy suggests the coverage integration and base analysis are using different file/line accounting systems. This undermines trust in metrics and makes debt density calculations unreliable.

### Current Behavior

The LOC count changes based on analysis mode:

```bash
# No coverage
📏 DEBT DENSITY: 200.1 per 1K LOC (29724 total LOC)

# With coverage
📏 DEBT DENSITY: 1640.9 per 1K LOC (35336 total LOC)
```

The same codebase should report identical LOC counts regardless of whether coverage data is present.

### Root Cause Hypotheses

1. **File Set Mismatch**: Coverage parser may be including/excluding different files than tree-sitter analyzer
2. **Line Counting Methods**: Coverage (lcov) may count physical lines while tree-sitter counts logical lines
3. **Double Counting**: Coverage integration may be counting some lines twice
4. **Test File Inclusion**: Coverage may be including test files that base analysis excludes

## Objective

Ensure consistent LOC counting across all analysis modes:
1. Same LOC count whether coverage is provided or not
2. Clear definition of what constitutes a "line of code"
3. Unified file filtering logic (test files, generated files, etc.)
4. Transparent reporting of file/line accounting

## Requirements

### Functional Requirements

1. **Unified Line Counting**: Single source of truth for LOC calculation used by both base analysis and coverage integration
2. **File Set Consistency**: Same files analyzed regardless of coverage presence
3. **Accounting Transparency**: Report which files/lines are included/excluded and why
4. **Validation Mode**: CLI flag to compare LOC counts between modes and identify discrepancies

### Non-Functional Requirements

1. **Accuracy**: LOC counts must be within 1% between coverage/no-coverage modes
2. **Performance**: Line counting should not add more than 5% overhead
3. **Debuggability**: Clear logging when files are included/excluded
4. **Determinism**: Same input always produces same LOC count

## Acceptance Criteria

- [ ] LOC count identical (±1%) with and without coverage data
- [ ] `--validate-loc` flag reports file-by-file accounting
- [ ] Documentation clearly defines what counts as a "line of code"
- [ ] Coverage parser uses same file filtering as base analyzer
- [ ] Integration test verifies LOC consistency across modes
- [ ] Debug logs show which files contribute to LOC count
- [ ] Test files excluded consistently in both modes (or included consistently)

## Technical Details

### Current Architecture (Problematic)

```rust
// Base analysis (tree-sitter)
fn count_loc_base() -> usize {
    files.iter()
        .filter(|f| !is_test_file(f))  // Filter 1
        .filter(|f| !is_generated(f))  // Filter 2
        .map(|f| count_tree_sitter_nodes(f))
        .sum()
}

// Coverage integration (lcov)
fn count_loc_with_coverage() -> usize {
    lcov_records.iter()
        .filter(|r| r.is_executable())  // Different filter!
        .map(|r| r.line_count)          // Different counting!
        .sum()
}
```

### Proposed Architecture

```rust
// Single source of truth
pub struct LocCounter {
    config: LocCountingConfig,
}

impl LocCounter {
    /// Count lines using unified logic
    pub fn count_file(&self, path: &Path) -> LocCount {
        let content = fs::read_to_string(path)?;

        LocCount {
            physical_lines: content.lines().count(),
            code_lines: self.count_code_lines(&content),
            comment_lines: self.count_comment_lines(&content),
            blank_lines: self.count_blank_lines(&content),
        }
    }

    /// Determine if file should be included
    pub fn should_include(&self, path: &Path) -> bool {
        !self.is_test_file(path) &&
        !self.is_generated(path) &&
        !self.is_excluded_by_config(path)
    }
}

// Both analyzers use same counter
let counter = LocCounter::new(config);
let base_loc = counter.count_project(&files);
let coverage_loc = counter.count_coverage_files(&lcov_data);
assert_eq!(base_loc, coverage_loc);
```

### Implementation Strategy

1. **Phase 1: Audit Current Counting**
   - Add debug logging to both LOC counting paths
   - Run with/without coverage and compare file lists
   - Identify which files cause the discrepancy
   - Document current behavior in test cases

2. **Phase 2: Extract Unified Counter**
   - Create `LocCounter` module with single counting logic
   - Define canonical "line of code" definition
   - Implement file filtering rules once
   - Add configuration for counting preferences

3. **Phase 3: Integrate Everywhere**
   - Replace base analysis LOC counting with `LocCounter`
   - Replace coverage LOC counting with `LocCounter`
   - Ensure both code paths use identical configuration
   - Add assertions to catch future divergence

4. **Phase 4: Validation & Reporting**
   - Add `--validate-loc` CLI flag
   - Report file-by-file accounting
   - Show included/excluded files with reasons
   - Add integration test for consistency

### File Changes Required

**New Files:**
- `src/metrics/loc_counter.rs`: Unified LOC counting logic
- `tests/integration/loc_consistency.rs`: Consistency tests

**Modified Files:**
- `src/analysis/mod.rs`: Use `LocCounter` for base analysis
- `src/analysis/coverage/mod.rs`: Use `LocCounter` for coverage
- `src/cli/commands/analyze.rs`: Add `--validate-loc` flag
- `src/config.rs`: Add LOC counting configuration options

### Data Structures

```rust
pub struct LocCount {
    pub physical_lines: usize,    // Raw line count
    pub code_lines: usize,         // Executable code
    pub comment_lines: usize,      // Comments
    pub blank_lines: usize,        // Whitespace
}

pub struct LocCountingConfig {
    pub include_tests: bool,
    pub include_generated: bool,
    pub count_comments: bool,
    pub count_blanks: bool,
    pub exclude_patterns: Vec<String>,
}

pub struct LocReport {
    pub total: LocCount,
    pub by_file: HashMap<PathBuf, LocCount>,
    pub included_files: Vec<PathBuf>,
    pub excluded_files: HashMap<PathBuf, ExclusionReason>,
}

pub enum ExclusionReason {
    TestFile,
    Generated,
    ConfigPattern(String),
    NoExecutableCode,
}
```

## Dependencies

**Prerequisites**: None

**Affected Components**:
- Base code analysis pipeline
- Coverage integration module
- Metrics calculation and reporting
- CLI validation commands

**External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_loc_counter_consistent() {
    let counter = LocCounter::default();
    let file = Path::new("src/example.rs");

    // Count twice, should be identical
    let count1 = counter.count_file(file).unwrap();
    let count2 = counter.count_file(file).unwrap();
    assert_eq!(count1, count2);
}

#[test]
fn test_file_filtering_identical() {
    let counter = LocCounter::default();
    let files = discover_files(".")?;

    let included_base = files.iter()
        .filter(|f| counter.should_include(f))
        .collect::<Vec<_>>();

    let included_coverage = files.iter()
        .filter(|f| counter.should_include(f))
        .collect::<Vec<_>>();

    assert_eq!(included_base, included_coverage);
}

#[test]
fn test_test_file_exclusion() {
    let counter = LocCounter::default();
    assert!(!counter.should_include(Path::new("tests/integration_test.rs")));
    assert!(!counter.should_include(Path::new("src/foo_test.rs")));
}
```

### Integration Tests

```rust
#[test]
fn test_loc_consistency_across_modes() {
    let project = TestProject::new()
        .with_source_file("src/lib.rs", "fn main() {}")
        .with_test_file("tests/test.rs", "#[test] fn t() {}")
        .with_coverage_data(lcov_data);

    let analysis_no_cov = analyze_project(&project, None);
    let analysis_with_cov = analyze_project(&project, Some(&lcov_data));

    assert_eq!(analysis_no_cov.total_loc, analysis_with_cov.total_loc);
}

#[test]
fn test_validate_loc_command() {
    let output = run_command("debtmap analyze . --validate-loc");

    assert!(output.contains("LOC Consistency: PASS"));
    assert!(output.contains("Total Files: "));
    assert!(output.contains("Included: "));
    assert!(output.contains("Excluded: "));
}
```

### Regression Tests

- Compare LOC counts with debtmap 0.2.5 for known projects
- Verify test file exclusion works consistently
- Test with various coverage file formats (lcov, cobertura)

## Documentation Requirements

### Code Documentation

```rust
/// Counts lines of code using a consistent methodology across all analysis modes.
///
/// # Line Counting Rules
///
/// - **Physical Lines**: Raw line count from file
/// - **Code Lines**: Lines containing executable code (excludes comments, blanks)
/// - **Comment Lines**: Lines that are primarily comments
/// - **Blank Lines**: Lines containing only whitespace
///
/// # File Filtering
///
/// Files are excluded if they match:
/// - Test file patterns: `*_test.rs`, `tests/**/*`
/// - Generated file markers: `@generated`, `DO NOT EDIT`
/// - Custom exclusion patterns from config
///
/// # Examples
///
/// ```rust
/// let counter = LocCounter::default();
/// let count = counter.count_file(Path::new("src/lib.rs"))?;
/// println!("Code lines: {}", count.code_lines);
/// ```
pub struct LocCounter { /* ... */ }
```

### User Documentation

Add section to debtmap guide:

```markdown
## Understanding LOC Counts

Debtmap counts "lines of code" consistently across all analysis modes:

- **Physical Lines**: Total lines in file
- **Code Lines**: Lines containing executable code (used for debt density)
- Excludes: comments, blank lines, test files, generated files

### Validating LOC Counts

Use `--validate-loc` to see detailed accounting:

```bash
debtmap analyze . --validate-loc
```

Output shows:
- Files included/excluded with reasons
- LOC breakdown by file
- Consistency validation between modes
```

### Architecture Updates

Document LOC counting in `ARCHITECTURE.md`:

```markdown
## Metrics: Lines of Code

LOC counting uses a unified `LocCounter` module to ensure consistency:

1. **Single Source of Truth**: Both base analysis and coverage use same counter
2. **Canonical Definition**: Code lines = executable code (excludes comments, blanks)
3. **File Filtering**: Test files and generated code excluded by default
4. **Validation**: `--validate-loc` ensures consistency across modes
```

## Implementation Notes

### LOC Definition Choices

**What counts as "code"?**
- ✅ Function declarations, statements, expressions
- ✅ Imports and module declarations
- ❌ Comments (even inline)
- ❌ Blank lines
- ❌ Closing braces on their own line (debatable)

**Why this definition?**
- Aligns with industry standards (SonarQube, cloc)
- Focuses on maintainable code volume
- Excludes formatting choices (blank lines)

### Edge Cases

1. **Multi-line Comments**: Each physical line counts as comment
2. **Code + Comment on Same Line**: Counts as code line
3. **Macro-Generated Code**: Counts if in source, not if expanded
4. **Multi-file Modules**: Each file counted separately

### Performance Considerations

- Cache LOC counts per file (invalidate on modification)
- Parse files once, extract both AST and LOC
- Use rayon for parallel counting on large projects

## Migration and Compatibility

### Breaking Changes

**Minor Breaking Change**: LOC counts may change slightly for existing projects

**Mitigation**:
- Document expected LOC calculation changes in CHANGELOG
- Provide `--legacy-loc` flag to use old counting (temporary)
- Show migration guide in error message if debt density changes >20%

### Configuration Migration

New config section:

```toml
[debtmap.loc]
# Include test files in LOC count (default: false)
include_tests = false

# Include generated files in LOC count (default: false)
include_generated = false

# Count comments as code lines (default: false)
count_comments = false

# Additional exclusion patterns
exclude_patterns = ["*.generated.rs", "build.rs"]
```

### Rollout Plan

1. **Version 0.2.6**: Ship fix with `--validate-loc` flag
2. **Warn on Discrepancy**: If LOC changes >10%, show warning
3. **Documentation**: Update all examples with new LOC counts
4. **CI/CD**: Update baseline metrics in automated workflows
