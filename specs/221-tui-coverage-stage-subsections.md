---
number: 221
title: TUI Coverage Stage Subsections
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-01-09
---

# Specification 221: TUI Coverage Stage Subsections

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The coverage loading stage (stage 5) currently shows a single spinner "Loading coverage data" with no visibility into the parsing process. For large LCOV files (hundreds of files, thousands of functions), this can take several seconds with no feedback about progress.

Looking at `src/risk/lcov.rs:225-280`, the `parse_lcov_file_with_progress` function performs distinct operations:
1. Opens and initializes the LCOV file reader
2. Iterates through records, processing each source file
3. Aggregates function coverage data per file
4. Computes final coverage statistics

The function already accepts a progress bar parameter but only uses it to set a static message. There's infrastructure for progress tracking (file_count at line 239) but it's not exposed to the user.

## Objective

Add subsections to the TUI coverage stage (stage 5) to show real-time progress through the 3 major phases of coverage loading: initialization, file processing (with progress), and statistics computation.

## Requirements

### Functional Requirements

- Display 3 subsections under the coverage stage when active:
  1. "open file" - Opening and validating LCOV file
  2. "parse coverage" - Processing coverage records with progress
  3. "compute stats" - Computing final statistics
- Update each subsection status as Pending → Active → Completed
- Show progress information for "parse coverage" subsection (e.g., "45/128 files")
- Update progress at throttled rate (every 10 files or 50ms)
- Handle case where no coverage file is provided (skip stage)
- Add minimum 150ms visibility pause between subsections

### Non-Functional Requirements

- Maintain 60 FPS TUI rendering performance
- Minimize overhead of progress tracking (<1% of parse time)
- Throttle updates to avoid excessive TUI refresh
- Handle malformed LCOV files gracefully
- Support edge cases (empty LCOV, single file, thousands of files)

## Acceptance Criteria

- [ ] Coverage stage shows 3 subsections when coverage file provided
- [ ] Coverage stage completes with "skipped" when no coverage file
- [ ] Each subsection transitions Pending → Active → Completed correctly
- [ ] "parse coverage" subsection shows file count progress
- [ ] Progress updates throttled appropriately
- [ ] TUI renders at 60 FPS during coverage parsing
- [ ] Progress updates add <1% overhead to parse time
- [ ] Handles malformed LCOV files without crashing TUI

## Technical Details

### Implementation Approach

1. **Update TUI App Structure** (`src/tui/app.rs:114-151`):
   - Modify `create_default_stages()` to create coverage stage with subsections
   - Change from `PipelineStage::new("coverage")` to `PipelineStage::with_subtasks()`
   - Add 3 SubTask entries for the phases

2. **Enhance LCOV Parser** (`src/risk/lcov.rs:225-280`):
   - Add progress callback parameter to `parse_lcov_file_with_progress`
   - Invoke callback with file count updates as files are processed
   - Separate initialization, parsing, and statistics phases

3. **Instrument Coverage Loading** (`src/builders/unified_analysis.rs:250-315`):
   - Add subsection updates before/after coverage loading
   - Pass progress callback to LCOV parser
   - Handle skipped coverage case (no file provided)
   - Add visibility pauses between phases

### Architecture Changes

**Minor API change**: `parse_lcov_file_with_progress` signature changes from accepting a `ProgressBar` to accepting a progress callback:

```rust
// Before
pub fn parse_lcov_file_with_progress(path: &Path, progress: &ProgressBar) -> Result<LcovData>

// After
pub fn parse_lcov_file_with_progress<F>(path: &Path, progress_callback: F) -> Result<LcovData>
where
    F: Fn(usize, usize) // (current_file, total_files_seen_so_far)
```

This allows the caller to update TUI subsections instead of just a progress bar message.

### Data Structures

Modify `create_default_stages()` in `src/tui/app.rs`:

```rust
PipelineStage::with_subtasks(
    "coverage",
    vec![
        SubTask {
            name: "open file".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "parse coverage".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "compute stats".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
    ],
),
```

### APIs and Interfaces

**LCOV Parser Enhancement** (`src/risk/lcov.rs`):

```rust
pub fn parse_lcov_file_with_progress<F>(
    path: &Path,
    mut progress_callback: F,
) -> Result<LcovData>
where
    F: FnMut(CoverageProgress),
{
    // Report initialization
    progress_callback(CoverageProgress::Initializing);

    let reader = Reader::open_file(path)?;

    // Report parsing start
    progress_callback(CoverageProgress::Parsing { current: 0, total: 0 });

    let mut data = LcovData::default();
    let mut file_count = 0;

    for record in reader {
        let record = record?;
        match record {
            Record::SourceFile { .. } => {
                file_count += 1;
                // Throttle: update every 10 files
                if file_count % 10 == 0 {
                    progress_callback(CoverageProgress::Parsing {
                        current: file_count,
                        total: file_count, // Total unknown until end
                    });
                }
            }
            // ... process other records ...
        }
    }

    // Report statistics computation
    progress_callback(CoverageProgress::ComputingStats);

    // ... final statistics ...

    progress_callback(CoverageProgress::Complete);
    Ok(data)
}

pub enum CoverageProgress {
    Initializing,
    Parsing { current: usize, total: usize },
    ComputingStats,
    Complete,
}
```

**Coverage Loading Instrumentation** (`src/builders/unified_analysis.rs`):

```rust
// Subtask 0: Open file
if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(4, 0, crate::tui::app::StageStatus::Active, None);
}

let coverage_data = load_coverage_data_with_subsections(coverage_file)?;

// load_coverage_data_with_subsections handles subsection updates via callback

fn load_coverage_data_with_subsections(
    coverage_file: Option<PathBuf>,
) -> Result<Option<LcovData>> {
    match coverage_file {
        Some(path) => {
            risk::lcov::parse_lcov_file_with_progress(&path, |progress| {
                if let Some(manager) = crate::progress::ProgressManager::global() {
                    match progress {
                        CoverageProgress::Initializing => {
                            // Subtask 0 complete: open file
                            manager.tui_update_subtask(4, 0, StageStatus::Completed, None);
                            std::thread::sleep(Duration::from_millis(150));
                            // Subtask 1 start: parse coverage
                            manager.tui_update_subtask(4, 1, StageStatus::Active, None);
                        }
                        CoverageProgress::Parsing { current, total } => {
                            manager.tui_update_subtask(4, 1, StageStatus::Active, Some((current, total)));
                        }
                        CoverageProgress::ComputingStats => {
                            // Subtask 1 complete
                            manager.tui_update_subtask(4, 1, StageStatus::Completed, None);
                            std::thread::sleep(Duration::from_millis(150));
                            // Subtask 2 start: compute stats
                            manager.tui_update_subtask(4, 2, StageStatus::Active, None);
                        }
                        CoverageProgress::Complete => {
                            manager.tui_update_subtask(4, 2, StageStatus::Completed, None);
                        }
                    }
                }
            }).map(Some)
        }
        None => {
            // No coverage file provided - skip all subsections
            if let Some(manager) = crate::progress::ProgressManager::global() {
                manager.tui_update_subtask(4, 0, StageStatus::Completed, None);
                manager.tui_update_subtask(4, 1, StageStatus::Completed, None);
                manager.tui_update_subtask(4, 2, StageStatus::Completed, None);
            }
            Ok(None)
        }
    }
}
```

## Dependencies

- **Prerequisites**: None (extends existing TUI infrastructure)
- **Affected Components**:
  - `src/tui/app.rs` - Stage definition
  - `src/risk/lcov.rs` - LCOV parser API change
  - `src/builders/unified_analysis.rs` - Coverage loading instrumentation
  - `src/organization/behavioral_decomposition/mod.rs` - Also calls parse_lcov_file_with_progress
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

- Test that coverage stage has 3 subsections defined
- Verify subsection status transitions
- Test progress callback invocation in LCOV parser
- Test "no coverage file" case (all subsections complete immediately)
- Test malformed LCOV file handling

### Integration Tests

- Run analysis with small LCOV file (1-5 files) and verify subsections
- Run analysis with large LCOV file (100+ files) and verify throttling
- Run analysis without coverage file and verify skip behavior
- Test with empty LCOV file
- Test with LCOV file containing only one source file

### Performance Tests

- Benchmark parse time with vs without progress updates
- Verify <1% overhead from progress callbacks
- Test with various LCOV sizes (10, 100, 1000, 10000 files)
- Profile callback invocation frequency

### Manual Testing

- Visual verification of subsection progression
- Confirm progress updates appear smooth
- Test on real coverage files from debtmap CI
- Verify subsections expand/collapse correctly

## Documentation Requirements

### Code Documentation

- Add comments explaining subsection phases
- Document CoverageProgress enum variants
- Add examples of progress callback usage
- Document throttling strategy (every 10 files)

### User Documentation

No user documentation updates needed - TUI is self-documenting through visual display.

### Architecture Updates

Update `docs/TUI_ARCHITECTURE.md`:
- Document coverage subsections as example of progress callback pattern
- Show before/after example of subsection display

## Implementation Notes

### Unknown Total Count

LCOV files don't have a header with total file count, so the total is only known at the end of parsing. The progress display will show "45/45", "56/56", etc., as files are discovered. This is acceptable because it still shows that progress is happening.

Alternative: Two-pass parsing (count files first, then parse) - rejected as it doubles I/O time.

### Throttling Strategy

Update progress every 10 files (not every file) to avoid excessive callback invocations. For small LCOV files (<10 files), this means only 1-2 updates, but the phases themselves provide feedback.

### Multiple Callers

The LCOV parser is called from multiple places:
- `src/builders/unified_analysis.rs` - Main analysis (needs subsections)
- `src/organization/behavioral_decomposition/mod.rs` - Behavioral decomposition (no subsections)

The progress callback should be optional (using a no-op callback by default) or provide both APIs:

```rust
// With progress
pub fn parse_lcov_file_with_progress<F>(path: &Path, callback: F) -> Result<LcovData>

// Without progress (convenience wrapper)
pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
    parse_lcov_file_with_progress(path, |_| {})
}
```

### Malformed LCOV Handling

If parsing fails mid-way through:
1. Current subsection should complete (even if error occurred)
2. Remaining subsections should be marked Completed
3. Error should propagate normally
4. TUI should not be left in inconsistent state

## Migration and Compatibility

### Breaking Changes

**API change**: `parse_lcov_file_with_progress` signature changes from `&ProgressBar` to callback. This affects:
- `src/builders/unified_analysis.rs` - Updated as part of this spec
- `src/organization/behavioral_decomposition/mod.rs` - Needs update to use no-op callback

### Migration Path

1. Add new callback-based API as `parse_lcov_file_with_callback`
2. Update all call sites to use new API
3. Remove old `parse_lcov_file_with_progress` that accepts ProgressBar
4. Rename `parse_lcov_file_with_callback` to `parse_lcov_file_with_progress`

Or keep both APIs for compatibility:
```rust
pub fn parse_lcov_file_with_progress<F>(path: &Path, callback: F) -> Result<LcovData>
pub fn parse_lcov_file_with_progressbar(path: &Path, pb: &ProgressBar) -> Result<LcovData> {
    parse_lcov_file_with_progress(path, |progress| {
        pb.set_message(format!("{:?}", progress));
    })
}
```

## Future Enhancements

### Deferred to Future Specs

1. **Line coverage parsing** - Show progress for line data (separate from file count)
2. **Coverage statistics display** - Show overall coverage % in subsection metric
3. **Parallel LCOV parsing** - Parse multiple LCOV files concurrently
4. **Streaming LCOV parsing** - Start analysis before coverage fully loaded

These are out of scope but could provide additional value.
