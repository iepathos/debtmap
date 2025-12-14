---
number: 195
title: Eliminate Redundant File I/O in Phase 3 Analysis
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 195: Eliminate Redundant File I/O in Phase 3 Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: none

## Context

The parallel unified analysis pipeline currently reads and parses files multiple times across different phases:

1. **Phase 1 (File Parsing)**: Files are read from disk, parsed into ASTs, and function metrics are extracted
2. **Phase 3 (File Analysis)**: Files are re-read from disk to:
   - Count total lines (for accurate file metrics)
   - Re-parse AST for god object detection

This redundant I/O creates a significant performance bottleneck, especially for large codebases. Analysis of a moderately-sized project shows Phase 3 can take 300+ seconds due to:
- Disk I/O for every file (even small ones)
- Redundant AST parsing with `syn::parse_file()`
- God object detection running on all files regardless of size

### Current Code Flow

```rust
// Phase 3: analyze_file_parallel (src/builders/parallel_unified_analysis.rs:915)
fn analyze_file_parallel(...) -> Option<FileDebtItem> {
    // 1. Aggregate existing function metrics (FAST - data already available)
    let file_metrics = file_analysis::aggregate_file_metrics(&functions_owned, coverage_data);

    // 2. Read file AGAIN just for line count (SLOW - redundant I/O)
    if let Ok(content) = std::fs::read_to_string(file_path) {
        let actual_line_count = content.lines().count();

        // 3. Parse AST AGAIN for god object detection (SLOW - redundant parsing)
        file_metrics.god_object_analysis = file_analysis::analyze_god_object(&content, ...);
    }
}
```

### Data Already Available

Phase 1 already has:
- File content (read from disk)
- Parsed AST (from `syn::parse_file`)
- Function metrics including line positions and lengths
- Total line count (content.lines().count() is trivial when content is available)

## Objective

Eliminate redundant file I/O and AST parsing in Phase 3 by:
1. Capturing file line counts during Phase 1 when files are first read
2. Passing this data to Phase 3 to avoid re-reading files for line counts
3. Adding early filtering to skip god object detection for small files that don't need it

## Requirements

### Functional Requirements

1. **Add `total_lines` field to `core::FileMetrics`**
   - New field: `pub total_lines: usize`
   - Set during initial file analysis when content is available
   - Propagate through analysis pipeline

2. **Capture line counts at source**
   - In `src/analyzers/rust.rs`: Set `total_lines` when creating `FileMetrics`
   - In `src/analyzers/batch.rs`: Capture line count when validating files
   - Line count = `content.lines().count()`

3. **Build file line count index**
   - Create `HashMap<PathBuf, usize>` mapping file paths to line counts
   - Build this index during Phase 1 from `FileMetrics` results
   - Pass to Phase 3 for O(1) lookup

4. **Remove redundant file reads in Phase 3**
   - Use line count from index instead of reading file
   - Only read files when god object detection is actually needed

5. **Add early filtering for god object detection**
   - Skip god object analysis for files that don't meet thresholds:
     - function_count <= 30 AND estimated_lines <= 1000
   - These small files cannot be god objects by definition
   - Only read/parse files that exceed thresholds

### Non-Functional Requirements

1. **Performance**: Reduce Phase 3 execution time by 80%+ for typical codebases
2. **Memory**: Minimal additional memory (line count index is small)
3. **Accuracy**: No change to analysis results - only performance optimization
4. **Backwards Compatibility**: No changes to output format or CLI interface

## Acceptance Criteria

- [ ] `core::FileMetrics` has `total_lines: usize` field
- [ ] `total_lines` is set in `src/analyzers/rust.rs` during initial analysis
- [ ] `total_lines` is set in `src/analyzers/batch.rs` during file validation
- [ ] All existing `FileMetrics` construction sites updated to include `total_lines`
- [ ] Phase 3 uses cached line counts instead of reading files
- [ ] Files with <= 30 functions AND <= 1000 lines skip god object detection
- [ ] God object detection still works correctly for large files
- [ ] All existing tests pass
- [ ] Performance benchmark shows significant improvement (target: 80% reduction in Phase 3 time)

## Technical Details

### Implementation Approach

#### Step 1: Add `total_lines` to `core::FileMetrics`

```rust
// src/core/mod.rs
pub struct FileMetrics {
    pub path: PathBuf,
    pub language: Language,
    pub complexity: ComplexityMetrics,
    pub debt_items: Vec<DebtItem>,
    pub dependencies: Vec<Dependency>,
    pub duplications: Vec<DuplicationBlock>,
    pub total_lines: usize,  // NEW FIELD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_scope: Option<ast::ModuleScopeAnalysis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<ast::ClassDef>>,
}
```

#### Step 2: Set `total_lines` at source

```rust
// src/analyzers/rust.rs - in analyze_rust_file and create_file_metrics
FileMetrics {
    path,
    language: Language::Rust,
    total_lines: ast.source.lines().count(),  // NEW
    complexity: ComplexityMetrics { ... },
    // ...
}
```

#### Step 3: Build line count index in parallel analysis

```rust
// src/builders/parallel_unified_analysis.rs
impl ParallelUnifiedAnalysisBuilder {
    fn build_line_count_index(&self) -> HashMap<PathBuf, usize> {
        // Build from FileMetrics collected during Phase 1
        self.file_metrics
            .iter()
            .map(|fm| (fm.path.clone(), fm.total_lines))
            .collect()
    }
}
```

#### Step 4: Modify `analyze_file_parallel` to use cached data

```rust
fn analyze_file_parallel(
    &self,
    file_path: &Path,
    functions: &[&FunctionMetrics],
    coverage_data: Option<&LcovData>,
    no_god_object: bool,
    file_line_count: usize,  // NEW PARAMETER
) -> Option<FileDebtItem> {
    let functions_owned: Vec<FunctionMetrics> = functions.iter().map(|&f| f.clone()).collect();
    let mut file_metrics = file_analysis::aggregate_file_metrics(&functions_owned, coverage_data);

    // USE CACHED LINE COUNT - NO FILE READ
    file_metrics.total_lines = file_line_count;
    file_metrics.uncovered_lines =
        ((1.0 - file_metrics.coverage_percent) * file_line_count as f64) as usize;

    // EARLY FILTERING - Skip god object detection for small files
    let needs_god_object = !no_god_object
        && (file_metrics.function_count > 30 || file_line_count > 1000);

    if !needs_god_object {
        file_metrics.god_object_analysis = None;
        return build_file_item(file_metrics);
    }

    // Only read file for god object detection on large files
    if let Ok(content) = std::fs::read_to_string(file_path) {
        file_metrics.god_object_analysis =
            file_analysis::analyze_god_object(&content, file_path, coverage_data);
    }

    build_file_item(file_metrics)
}
```

### Architecture Changes

1. **Data Flow**: Line count flows from Phase 1 → index → Phase 3
2. **No new modules**: Changes are within existing modules
3. **Minimal API changes**: Only internal function signatures change

### Files to Modify

| File | Change |
|------|--------|
| `src/core/mod.rs` | Add `total_lines` field to `FileMetrics` |
| `src/analyzers/rust.rs` | Set `total_lines` when creating `FileMetrics` |
| `src/analyzers/batch.rs` | Set `total_lines` in `FileAnalysisResult` |
| `src/analyzers/mod.rs` | Update any `FileMetrics` construction |
| `src/analyzers/implementations.rs` | Update `FileMetrics` construction |
| `src/analyzers/context_aware.rs` | Update `FileMetrics` construction |
| `src/analyzers/effects.rs` | Update test `FileMetrics` construction |
| `src/builders/parallel_unified_analysis.rs` | Use cached line counts, add filtering |

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `core::FileMetrics` struct
  - All analyzer implementations that create `FileMetrics`
  - `ParallelUnifiedAnalysisBuilder::analyze_file_parallel`
- **External Dependencies**: None

## Testing Strategy

### Unit Tests
- Test `FileMetrics` serialization/deserialization with `total_lines`
- Test line count calculation matches expected values
- Test early filtering logic for god object detection

### Integration Tests
- Run full analysis on test project, verify results unchanged
- Verify god object detection still works for large files
- Verify small files skip god object detection

### Performance Tests
- Benchmark Phase 3 before and after optimization
- Measure time savings on different codebase sizes
- Target: 80% reduction in Phase 3 execution time

### Regression Tests
- All existing tests must pass
- Output format must remain unchanged
- No change to debt item detection or scoring

## Documentation Requirements

- **Code Documentation**: Add doc comments to new `total_lines` field
- **User Documentation**: None (internal optimization)
- **Architecture Updates**: Note in ARCHITECTURE.md about caching strategy

## Implementation Notes

### God Object Detection Thresholds

The thresholds for skipping god object detection (30 functions, 1000 lines) are conservative:
- Most legitimate god objects have 50+ functions or 2000+ lines
- Using lower thresholds ensures we don't miss edge cases
- Can be tuned based on real-world results

### Memory Considerations

The line count index is minimal:
- ~100 bytes per file (PathBuf + usize)
- 1000 files ≈ 100KB additional memory
- Negligible compared to AST storage

### Future Optimizations

This spec focuses on the immediate win. Future optimizations could include:
- Caching parsed ASTs for reuse (high memory cost)
- Incremental analysis (only re-analyze changed files)
- Parallel god object detection with work stealing

## Migration and Compatibility

- **Breaking Changes**: None for public API
- **Internal Changes**: `FileMetrics` struct gains new field
- **Serialization**: New field is additive, backwards compatible
- **Default Value**: `total_lines: 0` for legacy data
