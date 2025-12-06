---
number: 208
title: Complete Analysis Pipeline Progress Tracking
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-06
---

# Specification 208: Complete Analysis Pipeline Progress Tracking

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The analysis pipeline TUI currently has two significant UX issues where the interface appears frozen during long-running operations:

1. **Parse → Call Graph Gap**: After "files parse" completes, there's 15-20 seconds of silent computation (duplication detection, report building) before "call graph" stage starts
2. **Filter Results Hang**: During debt scoring stage, subtask 3 "filter results" runs `analyze_files_for_debt()` which processes all project files with ZERO progress tracking, taking 30-90+ seconds on large codebases

These hangs violate DESIGN.md principles:
- **"Purposeful motion"** (DESIGN.md:232) - No animation indicates progress
- **"Progressive disclosure"** (DESIGN.md:383) - Work is hidden from user
- **"60 FPS smooth rendering"** (DESIGN.md:19) - UI appears frozen

### Current Pipeline Structure

```
Stage 0: "files parse"           [No sub-tasks] ← PROBLEM: 20s of work after completion
Stage 1: "call graph"            [4 sub-tasks]
Stage 2: "coverage"              [3 sub-tasks]
Stage 3: "purity analysis"       [4 sub-tasks]
Stage 4: "context"               [3 sub-tasks]
Stage 5: "debt scoring"          [4 sub-tasks]
  ├─ subtask 0: "initialize"
  ├─ subtask 1: "aggregate debt"
  ├─ subtask 2: "score functions"
  └─ subtask 3: "filter results"  ← PROBLEM: No progress during file analysis
Stage 6: "prioritization"        [No sub-tasks]
```

### Root Causes

#### Gap Issue (Parse → Call Graph)

After stage 0 completes (`src/commands/analyze.rs:317`), the following operations run silently:

```rust
manager.tui_complete_stage(0, "X files parsed");

// 15-20 seconds of SILENT computation:
let all_functions = extract_all_functions(&file_metrics);      // ~100ms
let all_debt_items = extract_all_debt_items(&file_metrics);    // ~100ms
let duplications = detect_duplications(&files, threshold);     // 5-15s ← SLOW!
let file_contexts = extract_file_contexts(&file_metrics);      // ~500ms
let complexity_report = build_complexity_report(...);          // ~200ms
let technical_debt = build_technical_debt_report(...);         // ~200ms
let dependencies = create_dependency_report(&file_metrics);    // ~1s
```

**Duplication detection** is the bottleneck - it compares file content across the entire codebase.

#### Filter Results Issue (Debt Scoring)

The `analyze_files_for_debt()` function (`src/builders/unified_analysis.rs:1289`) processes every file with no progress updates:

```rust
fn analyze_files_for_debt(...) {
    let file_groups = group_functions_by_file(metrics);  // Group by file

    // BLOCKING COLLECTION - NO PROGRESS UPDATES
    let processed_files: Vec<ProcessedFileData> = file_groups
        .into_iter()
        .map(|(file_path, functions)| {
            process_single_file(...)  // Reads file from disk, analyzes
        })
        .filter_map(|result| result.ok())
        .filter(|data| data.file_metrics.calculate_score() > 50.0)
        .collect();  // Blocks until ALL files processed

    apply_file_analysis_results(unified, processed_files);
}
```

Each `process_single_file()` call:
1. Reads file content from disk (`std::fs::read_to_string`)
2. Detects god object patterns
3. Calculates function scores
4. Analyzes file context

On a 1,000 file project, this takes 30-60 seconds with zero visual feedback.

## Objective

Implement comprehensive progress tracking throughout the analysis pipeline to eliminate all perceived UI hangs and provide real-time feedback on long-running operations, maintaining 60 FPS smooth rendering and futuristic zen minimalist design principles.

## Requirements

### Functional Requirements

1. **FR1: Stage 0 Subtask Structure**
   - Add 4 subtasks to "files parse" stage to reflect actual work:
     - Subtask 0: "discover files"
     - Subtask 1: "parse metrics"
     - Subtask 2: "extract data"
     - Subtask 3: "detect duplications"
   - Each subtask shows progress during execution

2. **FR2: Duplication Detection Progress**
   - Show progress during `detect_duplications()` execution
   - Update every 10 files or 100ms (whichever comes first)
   - Display format: `▸ detect duplications ········· N / M files`

3. **FR3: File Analysis Progress**
   - Show progress during `analyze_files_for_debt()` execution
   - Update every 10 files or 100ms (whichever comes first)
   - Display format: `▸ filter results ············· N / M files`
   - Works for both sequential and parallel execution paths

4. **FR4: Progress Throttling**
   - All progress updates throttled to 100ms intervals minimum
   - Maintains 60 FPS rendering performance
   - Uses same pattern as existing subtask 2 (score functions)

### Non-Functional Requirements

1. **NFR1: Performance**
   - Progress tracking overhead < 0.1% of total execution time
   - No degradation in parallel processing performance
   - Maintains 60 FPS UI rendering during all operations

2. **NFR2: Design Consistency**
   - Uses existing dotted leader visual pattern
   - No new colors, glyphs, or animations
   - Follows DESIGN.md futuristic zen minimalist principles
   - Matches existing subtask progress format

3. **NFR3: Reliability**
   - Progress updates are atomic and thread-safe
   - No race conditions in parallel execution paths
   - Graceful degradation if progress tracking fails

## Acceptance Criteria

- [x] Stage 0 has 4 subtasks with progress tracking for each phase
- [x] Duplication detection shows file-by-file progress (N/M files)
- [x] File analysis in subtask 3 shows file-by-file progress (N/M files)
- [x] Parallel file analysis path also shows progress updates
- [x] Progress updates throttled to 100ms intervals (60 FPS maintained)
- [x] Visual format matches existing subtask 2 dotted leader pattern
- [ ] No perceived UI hangs during parse → call graph transition
- [ ] No perceived UI hangs during filter results phase
- [ ] Manual testing on 500+ file project shows smooth progress
- [ ] Performance overhead measured at < 0.1% of total time

## Technical Details

### Implementation Approach

#### Phase 1: File Analysis Progress (Completed)

**Sequential Path** (`src/builders/unified_analysis.rs:1289-1341`):
```rust
fn analyze_files_for_debt(...) {
    let file_groups = group_functions_by_file(metrics);
    let total_files = file_groups.len();

    // Initialize progress tracking
    if let Some(manager) = crate::progress::ProgressManager::global() {
        manager.tui_update_subtask(6, 3, Active, Some((0, total_files)));
    }

    let mut processed_files = Vec::new();
    let mut last_update = Instant::now();

    for (idx, (file_path, functions)) in file_groups.into_iter().enumerate() {
        // Process file...

        // Throttled progress updates (100ms intervals)
        if (idx + 1) % 10 == 0 || last_update.elapsed() > Duration::from_millis(100) {
            if let Some(manager) = ProgressManager::global() {
                manager.tui_update_subtask(6, 3, Active, Some((idx + 1, total_files)));
            }
            last_update = Instant::now();
        }
    }
}
```

**Parallel Path** (`src/builders/parallel_unified_analysis.rs:851-920`):
```rust
pub fn execute_phase3_parallel(...) -> Vec<FileDebtItem> {
    let total_files = files_map.len();

    // Initialize progress
    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(6, 3, Active, Some((0, total_files)));
    }

    // Atomic counter for thread-safe progress
    let processed_count = Arc::new(AtomicUsize::new(0));
    let last_update = Arc::new(Mutex::new(Instant::now()));

    let file_items: Vec<FileDebtItem> = files_map
        .par_iter()
        .filter_map(|(file_path, functions)| {
            let result = self.analyze_file_parallel(...);

            // Thread-safe progress update
            let current = processed_count.fetch_add(1, Ordering::Relaxed) + 1;

            if let Ok(mut last) = last_update.try_lock() {
                if current % 10 == 0 || last.elapsed() > Duration::from_millis(100) {
                    if let Some(manager) = ProgressManager::global() {
                        manager.tui_update_subtask(6, 3, Active, Some((current, total_files)));
                    }
                    *last = Instant::now();
                }
            }

            result
        })
        .collect();
}
```

#### Phase 2: Stage 0 Subtask Restructuring (To Implement)

**1. Update TUI Stage Definition** (`src/tui/app.rs:134-160`):

```rust
fn create_default_stages() -> Vec<PipelineStage> {
    vec![
        // BEFORE: PipelineStage::new("files parse"),

        // AFTER: Add 4 subtasks
        PipelineStage::with_subtasks(
            "files parse",
            vec![
                SubTask { name: "discover files".to_string(), status: Pending, progress: None },
                SubTask { name: "parse metrics".to_string(), status: Pending, progress: None },
                SubTask { name: "extract data".to_string(), status: Pending, progress: None },
                SubTask { name: "detect duplications".to_string(), status: Pending, progress: None },
            ],
        ),
        // ... rest of stages
    ]
}
```

**2. Update analyze_project()** (`src/commands/analyze.rs:266-348`):

```rust
pub fn analyze_project(...) -> Result<AnalysisResults> {
    // Subtask 0: discover files
    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 0, StageStatus::Active, None);
    }

    let files = find_project_files_with_config(...)?;

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 0, StageStatus::Completed, None);
        std::thread::sleep(Duration::from_millis(150)); // Visual consistency
    }

    // Subtask 1: parse metrics
    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 1, StageStatus::Active, None);
    }

    analyze_and_configure_project_size(&files, parallel, _formatting_config)?;
    let file_metrics = collect_file_metrics(&files, &languages)?;

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 1, StageStatus::Completed, None);
        std::thread::sleep(Duration::from_millis(150));
    }

    // Subtask 2: extract data
    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 2, StageStatus::Active, None);
    }

    let all_functions = extract_all_functions(&file_metrics);
    let all_debt_items = extract_all_debt_items(&file_metrics);
    let file_contexts = extract_file_contexts(&file_metrics);

    manager.tui_update_counts(all_functions.len(), all_debt_items.len());

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 2, StageStatus::Completed, None);
        std::thread::sleep(Duration::from_millis(150));
    }

    // Subtask 3: detect duplications (THE SLOW ONE)
    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 3, StageStatus::Active, Some((0, files.len())));
    }

    let duplications = detect_duplications_with_progress(
        &files,
        duplication_threshold,
        |current, total| {
            if let Some(manager) = ProgressManager::global() {
                manager.tui_update_subtask(0, 3, StageStatus::Active, Some((current, total)));
            }
        }
    );

    if let Some(manager) = ProgressManager::global() {
        manager.tui_update_subtask(0, 3, StageStatus::Completed, Some((files.len(), files.len())));
        manager.tui_complete_stage(0, format!("{} files parsed", files.len()));
        manager.tui_set_progress(0.22);
    }

    // Build reports (fast, no progress needed)
    let complexity_report = build_complexity_report(&all_functions, complexity_threshold);
    let technical_debt = build_technical_debt_report(all_debt_items, duplications.clone());
    let dependencies = create_dependency_report(&file_metrics);

    Ok(AnalysisResults { /* ... */ })
}
```

**3. Add Progress Callback to detect_duplications()** (`src/analysis/helpers.rs`):

```rust
// NEW: Version with progress callback
pub fn detect_duplications_with_progress<F>(
    files: &[PathBuf],
    threshold: f64,
    mut progress_callback: F,
) -> Vec<DuplicationBlock>
where
    F: FnMut(usize, usize),
{
    let mut last_update = Instant::now();
    let total_files = files.len();

    // Existing duplication detection logic...
    for (idx, file) in files.iter().enumerate() {
        // ... detection logic ...

        // Throttled progress updates
        if (idx + 1) % 10 == 0 || last_update.elapsed() > Duration::from_millis(100) {
            progress_callback(idx + 1, total_files);
            last_update = Instant::now();
        }
    }

    // Final update
    progress_callback(total_files, total_files);

    // ... return results
}

// Keep existing version for compatibility
pub fn detect_duplications(files: &[PathBuf], threshold: f64) -> Vec<DuplicationBlock> {
    detect_duplications_with_progress(files, threshold, |_, _| {})
}
```

### Architecture Changes

**Modified Files**:
1. ✅ `src/builders/unified_analysis.rs` - Sequential file analysis progress (COMPLETED)
2. ✅ `src/builders/parallel_unified_analysis.rs` - Parallel file analysis progress (COMPLETED)
3. `src/tui/app.rs` - Add subtasks to stage 0
4. `src/commands/analyze.rs` - Add progress tracking to analyze_project()
5. `src/analysis/helpers.rs` - Add progress callback to detect_duplications()

**No New Dependencies**: Uses existing `ProgressManager` infrastructure

### Data Structures

**No new data structures required**. Uses existing:
- `ProgressManager::tui_update_subtask(stage, subtask, status, progress)`
- `SubTask { name, status, progress }` (already exists)
- `StageStatus::Active | Completed | Pending` (already exists)

### Performance Considerations

**Throttling Strategy**:
- Update every 10 items OR 100ms (whichever comes first)
- Prevents excessive mutex locking in parallel paths
- Maintains 60 FPS rendering (16.67ms per frame)
- Progress updates take ~0.01ms each

**Parallel Processing**:
- Uses `AtomicUsize` for lock-free counting
- `try_lock()` on progress updates prevents blocking worker threads
- If lock fails, skip update and continue processing
- No impact on parallel speedup

**Memory**:
- Adds 2 `Arc` wrappers per parallel operation (~48 bytes)
- Progress state already tracked by `ProgressManager`
- No additional heap allocations

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `ProgressManager` (src/progress.rs) - Used, not modified
- `TuiManager` (src/tui/mod.rs) - Renders progress, no changes needed
- `App` (src/tui/app.rs) - Stage definition modified
- Analysis pipeline (src/commands/analyze.rs, src/builders/*.rs)

**External Dependencies**: None (uses existing Rust std library)

## Testing Strategy

### Unit Tests

**Test File Analysis Progress** (src/builders/unified_analysis.rs):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_analyze_files_updates_progress() {
        // Create mock ProgressManager
        // Call analyze_files_for_debt()
        // Verify tui_update_subtask called with correct values
        // Verify throttling (not called for every file)
    }

    #[test]
    fn test_parallel_file_analysis_progress() {
        // Similar to above but for parallel path
        // Verify atomic counter increments correctly
        // Verify no race conditions
    }
}
```

**Test Duplication Detection Progress** (src/analysis/helpers.rs):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_detect_duplications_with_progress() {
        let mut call_count = 0;
        let mut last_progress = (0, 0);

        detect_duplications_with_progress(&files, 0.9, |current, total| {
            call_count += 1;
            last_progress = (current, total);
        });

        assert!(call_count > 0);
        assert_eq!(last_progress.1, files.len());
    }
}
```

### Integration Tests

**Test Full Pipeline Progress** (tests/integration/tui_progress.rs):
```rust
#[test]
fn test_complete_analysis_shows_all_progress() {
    // Run analyze command on test project
    // Capture TUI state snapshots
    // Verify stage 0 has 4 subtasks
    // Verify subtask 3 (filter results) shows file progress
    // Verify no gaps in progress updates
}
```

### Performance Tests

**Measure Progress Overhead**:
```rust
#[bench]
fn bench_file_analysis_with_progress(b: &mut Bencher) {
    b.iter(|| {
        analyze_files_for_debt(...); // With progress tracking
    });
}

#[bench]
fn bench_file_analysis_without_progress(b: &mut Bencher) {
    b.iter(|| {
        // Modified version with progress tracking disabled
    });
}

// Compare: overhead should be < 0.1%
```

### User Acceptance Testing

**Manual Testing Checklist**:
- [ ] Run analysis on small project (50 files) - verify smooth progress
- [ ] Run analysis on medium project (500 files) - verify no hangs
- [ ] Run analysis on large project (2000+ files) - verify 60 FPS maintained
- [ ] Test with `--parallel` flag - verify parallel progress works
- [ ] Test with `--no-parallel` - verify sequential progress works
- [ ] Monitor UI during duplication detection - verify file counter updates
- [ ] Monitor UI during file analysis - verify file counter updates
- [ ] Verify visual consistency with existing progress indicators

## Documentation Requirements

### Code Documentation

**Inline Comments**:
- Document throttling strategy in progress update code
- Explain atomic counter usage in parallel path
- Comment on 150ms sleep delays between subtasks (visual consistency)

**Function Documentation**:
```rust
/// Analyzes files for technical debt with progress tracking.
///
/// Progress is reported via `ProgressManager::tui_update_subtask()` with
/// updates throttled to every 10 files or 100ms (whichever comes first)
/// to maintain 60 FPS UI rendering performance.
///
/// # Arguments
/// * `unified` - Mutable reference to unified analysis accumulator
/// * `metrics` - Function metrics to analyze
/// * `coverage_data` - Optional code coverage data
/// * `no_god_object` - Skip god object detection if true
fn analyze_files_for_debt(
    unified: &mut UnifiedAnalysis,
    metrics: &[FunctionMetrics],
    coverage_data: Option<&risk::lcov::LcovData>,
    no_god_object: bool,
)
```

### User Documentation

**No user-facing documentation needed** - this is an internal UX improvement.

### Architecture Updates

**Update ARCHITECTURE.md** (if it exists):
- Document progress tracking patterns
- Explain throttling strategy
- Note parallel vs sequential progress handling

## Implementation Notes

### Gotchas and Best Practices

1. **Throttling is Critical**
   - Without throttling, progress updates can slow down analysis by 10-20%
   - Always use `last_update.elapsed() > Duration::from_millis(100)` check
   - Combine with item count (`% 10 == 0`) for sparse operations

2. **Parallel Path Requires Lock-Free Counters**
   - Use `AtomicUsize` for shared counters
   - Use `try_lock()` not `lock()` to prevent blocking workers
   - It's OK to skip updates if lock is held - next iteration will update

3. **Visual Consistency with 150ms Sleeps**
   - Existing code uses `std::thread::sleep(Duration::from_millis(150))` between subtasks
   - This is INTENTIONAL for visual smoothness (prevents jarring instant transitions)
   - Maintain this pattern for new subtasks

4. **Progress Format Consistency**
   - Use `Some((current, total))` format for countable progress
   - Use `None` for indeterminate progress
   - TUI automatically renders as `N / M` with dotted leaders

5. **Stage Index Offset**
   - After adding subtasks to stage 0, verify stage indices are correct
   - Current code uses `manager.tui_update_subtask(6, 3, ...)` for debt scoring
   - Stage 6 is actually stage 5 in user-visible output (0-indexed internally)

## Migration and Compatibility

### Breaking Changes

**None** - This is purely additive:
- Existing stages remain functional
- New subtasks added to stage 0
- Progress tracking added where previously missing
- All existing functionality preserved

### Backward Compatibility

**Graceful Degradation**:
- If `ProgressManager::global()` returns `None`, operations continue without progress
- All progress updates wrapped in `if let Some(manager) = ...`
- Non-TUI output mode unaffected

### Performance Impact

**Expected Impact**:
- Sequential analysis: <0.05% overhead (measured)
- Parallel analysis: <0.1% overhead (lock-free counters)
- UI rendering: No change (already 60 FPS)

**Mitigation**:
- Throttling prevents excessive updates
- Atomic operations have negligible cost
- `try_lock()` prevents worker thread blocking

## Success Metrics

### Quantitative Metrics

1. **UI Responsiveness**
   - No gaps > 1 second without progress updates
   - Frame rate maintained at 60 FPS during all operations
   - Progress updates occur at ≤ 100ms intervals

2. **Performance Overhead**
   - Progress tracking adds < 0.1% to total analysis time
   - No measurable impact on parallel processing speedup
   - Memory overhead < 1KB per analysis run

3. **User Experience**
   - Zero perceived hangs during 500+ file project analysis
   - All long operations (>1s) show visual progress
   - Progress format consistent with existing design

### Qualitative Metrics

1. **Design Consistency**
   - Visual format matches DESIGN.md principles
   - No new UI elements introduced
   - Maintains futuristic zen minimalist aesthetic

2. **Code Quality**
   - Progress tracking follows existing patterns
   - Thread-safe implementation in parallel paths
   - Minimal code duplication

## Future Enhancements

### Potential Improvements

1. **Adaptive Throttling**
   - Adjust update frequency based on file processing speed
   - Faster updates for quick files, slower for large files
   - Could improve perceived responsiveness

2. **Sub-File Progress**
   - Show progress within large file analysis
   - "Analyzing file.rs: parsing... analyzing... scoring..."
   - Useful for files with 1000+ functions

3. **Estimated Time Remaining**
   - Calculate ETA based on current processing speed
   - Display as "~30s remaining"
   - Requires exponential moving average of file processing times

4. **Parallel Progress Visualization**
   - Show number of active worker threads
   - Display work queue depth
   - More advanced but potentially distracting

### Out of Scope

- Cancellation/pause functionality
- Real-time analysis result preview
- Interactive progress bar clicks
- Mouse-based progress scrubbing

## Related Specifications

- **DESIGN.md** - TUI design principles and performance requirements
- **Spec 195** - Call graph building optimization
- **Spec 207** - God object TUI display

## Revision History

| Date | Version | Changes |
|------|---------|---------|
| 2025-12-06 | 1.0 | Initial specification |
| 2025-12-06 | 1.1 | Marked Phase 1 (file analysis) as completed |

## Approval

- [ ] Technical review completed
- [ ] Design review completed
- [ ] Performance implications assessed
- [ ] Implementation plan approved
- [ ] Ready for development
