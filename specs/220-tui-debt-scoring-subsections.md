---
number: 220
title: TUI Debt Scoring Stage Subsections
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-01-09
---

# Specification 220: TUI Debt Scoring Stage Subsections

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The debt scoring stage (stage 8) is one of the most complex and time-consuming operations in debtmap analysis. It currently shows only a single spinner "Computing technical debt priorities" with no visibility into the multiple distinct phases happening inside.

Looking at `src/builders/unified_analysis.rs:538-700+`, the `create_unified_analysis_with_exclusions_and_timing` function performs several sequential operations:

1. **Data flow initialization** (line 599) - Creates UnifiedAnalysis with data flow graph
2. **Purity population** (line 602) - Populates purity analysis from metrics
3. **Test function detection** (line 605) - Identifies test-only functions
4. **Debt aggregation** (line 609-621) - Aggregates debt items to functions
5. **Function scoring loop** (line 624-643) - Scores each function (main computational work)
6. **Error analysis** (line 646-651) - Analyzes error swallowing patterns
7. **Aggregation & filtering** (line 653+) - Aggregates and filters results

For large codebases with thousands of functions, the scoring loop can take several seconds with no feedback about progress. This creates uncertainty about whether the process is progressing or stuck.

## Objective

Add subsections to the TUI debt scoring stage (stage 8) to show real-time progress through the 4 major phases of debt scoring, with the scoring phase showing actual progress (current/total functions).

## Requirements

### Functional Requirements

- Display 4 subsections under the debt scoring stage when active:
  1. "initialize" - Data flow graph and purity setup
  2. "aggregate debt" - Aggregating debt items to functions
  3. "score functions" - Main scoring loop with progress
  4. "filter results" - Final aggregation and filtering
- Update each subsection status as Pending → Active → Completed
- Show progress information for "score functions" subsection (e.g., "1234/5678")
- Update scoring progress at throttled rate (every 100 functions or 100ms)
- Add minimum 150ms visibility pause between subsections (except scoring which is naturally slow)

### Non-Functional Requirements

- Maintain 60 FPS TUI rendering performance
- Minimize performance impact of progress updates (<1% overhead)
- Throttle scoring updates to avoid excessive TUI refresh
- Handle both sequential and parallel scoring modes
- Support edge cases (0 functions, all functions skipped)

## Acceptance Criteria

- [ ] Debt scoring stage shows 4 subsections when active
- [ ] Each subsection transitions Pending → Active → Completed correctly
- [ ] "score functions" subsection shows progress (current/total)
- [ ] Progress updates throttled to reasonable frequency
- [ ] Works correctly with parallel and sequential scoring
- [ ] Handles edge cases (0 functions, empty projects) gracefully
- [ ] TUI renders at 60 FPS during scoring
- [ ] Progress updates add <1% overhead to scoring time

## Technical Details

### Implementation Approach

1. **Update TUI App Structure** (`src/tui/app.rs:114-151`):
   - Modify `create_default_stages()` to create debt scoring stage with subsections
   - Change from `PipelineStage::new("debt scoring")` to `PipelineStage::with_subtasks()`
   - Add 4 SubTask entries for the phases

2. **Instrument Sequential Scoring** (`src/builders/unified_analysis.rs:538-700+`):
   - Add subsection updates before/after each phase
   - Update "score functions" subsection in the main loop
   - Throttle progress updates to every 100 functions or 100ms
   - Add visibility pauses for fast phases

3. **Instrument Parallel Scoring** (`src/builders/parallel_unified_analysis.rs`):
   - Apply same subsection instrumentation to parallel path
   - Use atomic counters for thread-safe progress tracking
   - Aggregate progress from worker threads

4. **Handle Edge Cases**:
   - 0 functions: Show "0/0" and complete immediately
   - All skipped: Track attempted vs completed counts
   - Early termination: Mark remaining subsections as completed

### Architecture Changes

No structural changes required. This extends the existing TUI subsection pattern to the debt scoring stage.

### Data Structures

Modify `create_default_stages()` in `src/tui/app.rs`:

```rust
PipelineStage::with_subtasks(
    "debt scoring",
    vec![
        SubTask {
            name: "initialize".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "aggregate debt".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "score functions".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "filter results".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
    ],
),
```

### APIs and Interfaces

New TUI update pattern in `create_unified_analysis_with_exclusions_and_timing`:

```rust
// Subtask 0: Initialize
if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(7, 0, crate::tui::app::StageStatus::Active, None);
}

let mut unified = UnifiedAnalysis::new(call_graph.clone());
unified.populate_purity_analysis(metrics);
let test_only_functions = call_graph.find_test_only_functions();

if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(7, 0, crate::tui::app::StageStatus::Completed, None);
    std::thread::sleep(std::time::Duration::from_millis(150));
}

// Subtask 1: Aggregate debt
if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(7, 1, crate::tui::app::StageStatus::Active, None);
}

let mut debt_aggregator = DebtAggregator::new();
// ... aggregation logic ...

if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(7, 1, crate::tui::app::StageStatus::Completed, None);
    std::thread::sleep(std::time::Duration::from_millis(150));
}

// Subtask 2: Score functions (with progress)
let scorable_count = metrics.len();
if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(7, 2, crate::tui::app::StageStatus::Active, Some((0, scorable_count)));
}

let mut scored = 0;
let mut last_update = std::time::Instant::now();
for (idx, metric) in metrics.iter().enumerate() {
    // ... scoring logic ...

    // Throttled progress update
    if idx % 100 == 0 || last_update.elapsed() > Duration::from_millis(100) {
        if let Some(manager) = crate::progress::ProgressManager::global() {
            manager.tui_update_subtask(7, 2, crate::tui::app::StageStatus::Active, Some((idx + 1, scorable_count)));
        }
        last_update = std::time::Instant::now();
    }
}

if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(7, 2, crate::tui::app::StageStatus::Completed, Some((scorable_count, scorable_count)));
    std::thread::sleep(std::time::Duration::from_millis(150));
}

// Subtask 3: Filter results
if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(7, 3, crate::tui::app::StageStatus::Active, None);
}

// ... filtering and aggregation ...

if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(7, 3, crate::tui::app::StageStatus::Completed, None);
}
```

### Progress Throttling Strategy

Update progress only when EITHER condition is true:
- Every 100 functions processed (avoids excessive updates for large codebases)
- At least 100ms elapsed since last update (ensures visual feedback for small codebases)

This balances:
- Performance (doesn't update on every iteration)
- Responsiveness (updates frequently enough to show progress)
- Resource usage (TUI renders at constant 60 FPS regardless)

## Dependencies

- **Prerequisites**: None (extends existing TUI infrastructure)
- **Affected Components**:
  - `src/tui/app.rs` - Stage definition
  - `src/builders/unified_analysis.rs` - Sequential scoring instrumentation
  - `src/builders/parallel_unified_analysis.rs` - Parallel scoring instrumentation
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

- Test that debt scoring stage has 4 subsections defined
- Verify subsection status transitions
- Test progress update throttling logic
- Verify edge cases (0 functions, all skipped)

### Integration Tests

- Run analysis on small project (<100 functions) and verify all subsections appear
- Run analysis on large project (>1000 functions) and verify throttled updates
- Test parallel mode with multiple threads
- Verify subsection completion on errors/early termination

### Performance Tests

- Benchmark overhead of progress updates vs baseline
- Verify <1% performance impact
- Test with various codebase sizes (10, 100, 1000, 10000 functions)
- Profile TUI rendering during scoring loop

### Manual Testing

- Visual verification of subsection progression in terminal
- Confirm progress updates appear smooth and responsive
- Test on debtmap's own codebase (~500 functions)
- Verify subsections expand/collapse correctly

## Documentation Requirements

### Code Documentation

- Add comments explaining subsection indices and phases
- Document throttling strategy and rationale
- Add comments about parallel mode progress aggregation
- Document edge case handling

### User Documentation

No user documentation updates needed - TUI is self-documenting through visual display.

### Architecture Updates

Update `docs/TUI_ARCHITECTURE.md`:
- Document debt scoring subsections as example of complex multi-phase stage
- Explain progress throttling pattern for large loops
- Show before/after example of subsection display

## Implementation Notes

### Parallel Mode Considerations

In parallel mode (`parallel_unified_analysis.rs`), progress tracking requires thread-safe counters:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

let scored_count = Arc::new(AtomicUsize::new(0));
let total_count = metrics.len();

// In worker thread:
scored_count.fetch_add(1, Ordering::Relaxed);

// In progress update thread:
let current = scored_count.load(Ordering::Relaxed);
manager.tui_update_subtask(7, 2, StageStatus::Active, Some((current, total_count)));
```

### Skipped Functions

Functions are skipped if they're test-only, framework exclusions, or clean dispatchers. The progress should show "attempted to score" vs "total functions", not "successfully scored":

- Total = `metrics.len()` (all functions considered)
- Current = index in iteration (functions attempted)
- Some functions may be skipped but index still increments

### Error Handling

If scoring encounters errors and terminates early:
1. Mark current subsection as Completed with partial progress
2. Mark remaining subsections as Completed (don't leave them Pending)
3. Stage overall completes with error metric (e.g., "partial - 456 items")

### Phase Breakdown Rationale

The 4 subsections align with conceptual phases:
1. **Initialize** - Setup work (data structures, detection)
2. **Aggregate** - Pre-scoring data aggregation
3. **Score** - Main computational loop
4. **Filter** - Post-scoring cleanup and organization

Alternative considered: 6 subsections (one per step in code) - rejected as too granular and some steps are instantaneous.

## Migration and Compatibility

No breaking changes. This is purely a visual enhancement to the TUI. Users who don't use the TUI (--quiet mode or non-TTY) see no difference.

The scoring performance may see a small overhead (<1%) from progress updates, but this is negligible compared to the scoring computation itself.

## Future Enhancements

### Deferred to Future Specs

1. **Parallel worker visualization** - Show progress per thread in parallel mode
2. **ETA estimation** - Calculate and display estimated time remaining
3. **Scoring performance metrics** - Show functions/second rate
4. **Memory usage tracking** - Display memory consumption during scoring

These are out of scope for this spec but could be valuable future improvements.
