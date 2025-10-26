---
number: 130
title: Real-time Progress Feedback for Long-running Analysis Phases
category: optimization
priority: high
status: draft
dependencies: [128]
created: 2025-01-25
---

# Specification 130: Real-time Progress Feedback for Long-running Analysis Phases

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 128 (indicatif integration)

## Context

Spec 128 introduced indicatif progress bars to debtmap, but several critical issues prevent real-time feedback during the longest analysis phases:

1. **94-second call graph building hang**: Phase 3 enhanced analysis processes 385+ files sequentially with NO progress indicator, causing a perceived freeze where users see maximum CPU usage but no output
2. **Misleading trait resolution progress**: Shows "🔍 0/0 traits" because the progress bar is created with length 0, making it appear broken
3. **Insufficient progress granularity**: Heavy CPU-bound rayon parallel iterators don't tick progress bars frequently enough during intensive computation

User experience issue: After "Call graph complete 385/385 files (100%)" message, the system hangs for 90+ seconds with all cores maxed but zero feedback, causing users to think the process is frozen.

**Actual timeline observed**:
```
🔗 Call graph complete 385/385 files (100%) - 0s
[DEBUG] Resolving trait patterns... done          # 94 seconds HIDDEN HERE
 [OK]
Resolving trait method calls...🔍 0/0 traits - 0s  # Broken display
```

## Objective

Provide continuous, accurate progress feedback during all analysis phases, eliminating perceived freezes and giving users visibility into what debtmap is doing during long-running operations.

## Requirements

### Functional Requirements

1. **Enhanced Call Graph Progress**: Add progress tracking to Phase 3 enhanced analysis (the 94-second sequential processing bottleneck)
2. **Fixed Trait Resolution Display**: Replace "0/0 traits" progress bar with appropriate spinner or correct length calculation
3. **Steady Progress Updates**: Enable steady tick on CPU-intensive progress bars to force visual updates every 100ms regardless of manual ticks
4. **Clear Phase Messaging**: Display what analysis phase is running (basic calls, trait dispatch, function pointers, framework patterns)

### Non-Functional Requirements

- Progress bars must update visually at least every 200ms during active work
- Progress messages must accurately reflect current operation
- Memory overhead from progress tracking must be < 1MB
- Progress bar rendering must not slow analysis by more than 2%

## Acceptance Criteria

- [ ] Phase 3 enhanced analysis shows progress bar: `🔧 Enhanced analysis {pos}/{len} files - {eta}`
- [ ] Progress bar updates visibly during all 385 files of sequential enhanced analysis
- [ ] Trait resolution shows spinner with message "Resolving trait method calls" instead of "0/0 traits"
- [ ] Function analysis progress bar has steady tick enabled (100ms intervals)
- [ ] No perceived freeze periods longer than 500ms during verbose analysis
- [ ] All progress bars complete with accurate final counts
- [ ] Progress output is clean with no duplicate or overlapping messages
- [ ] Running `debtmap analyze . -vv` on 386-file project shows continuous progress updates

## Technical Details

### Implementation Approach

#### Fix 1: Add Progress to Phase 3 Enhanced Analysis

**Location**: `src/builders/parallel_call_graph.rs:194-202`

**Current code** (no progress):
```rust
for (file_path, parsed) in &workspace_files {
    enhanced_builder
        .analyze_basic_calls(file_path, parsed)?
        .analyze_trait_dispatch(file_path, parsed)?
        .analyze_function_pointers(file_path, parsed)?
        .analyze_framework_patterns(file_path, parsed)?;
}
```

**Proposed implementation**:
```rust
// Create progress bar for enhanced analysis
let progress = ProgressManager::global()
    .map(|pm| {
        let pb = pm.create_bar(
            workspace_files.len() as u64,
            "🔧 {msg} {pos}/{len} files ({percent}%) - {eta}",
        );
        pb.set_message("Enhanced call graph analysis");
        pb
    })
    .unwrap_or_else(indicatif::ProgressBar::hidden);

// Process files with progress tracking
for (file_path, parsed) in &workspace_files {
    enhanced_builder
        .analyze_basic_calls(file_path, parsed)?
        .analyze_trait_dispatch(file_path, parsed)?
        .analyze_function_pointers(file_path, parsed)?
        .analyze_framework_patterns(file_path, parsed)?;
    progress.inc(1);
}

progress.finish_with_message("Enhanced analysis complete");
```

#### Fix 2: Replace Trait Resolution Progress with Spinner

**Location**: `src/builders/unified_analysis.rs:1408-1411`

**Current code** (broken "0/0 traits"):
```rust
let progress = crate::progress::ProgressManager::global()
    .map(|pm| pm.create_bar(0, crate::progress::TEMPLATE_TRAIT_RESOLUTION))
    .unwrap_or_else(indicatif::ProgressBar::hidden);
```

**Proposed implementation**:
```rust
let progress = crate::progress::ProgressManager::global()
    .map(|pm| pm.create_spinner("Resolving trait method calls"))
    .unwrap_or_else(indicatif::ProgressBar::hidden);
```

**Alternative** (if trait count can be determined upfront):
```rust
// Get trait count from registry before creating progress bar
let trait_count = trait_registry.unresolved_call_count();
let progress = crate::progress::ProgressManager::global()
    .map(|pm| {
        let pb = pm.create_bar(trait_count as u64, crate::progress::TEMPLATE_TRAIT_RESOLUTION);
        pb.set_message("Resolving trait method calls");
        pb
    })
    .unwrap_or_else(indicatif::ProgressBar::hidden);
```

#### Fix 3: Enable Steady Tick for CPU-intensive Progress Bars

**Location**: `src/builders/parallel_unified_analysis.rs:648-655`

**Current code**:
```rust
let pb = pm.create_bar(
    metrics.len() as u64,
    crate::progress::TEMPLATE_FUNCTION_ANALYSIS,
);
pb.set_message("Analyzing functions");
pb
```

**Proposed implementation**:
```rust
let pb = pm.create_bar(
    metrics.len() as u64,
    crate::progress::TEMPLATE_FUNCTION_ANALYSIS,
);
pb.set_message("Analyzing functions");
pb.enable_steady_tick(std::time::Duration::from_millis(100));
pb
```

**Note**: Steady tick forces indicatif to redraw the progress bar every 100ms even if no manual `.inc()` or `.tick()` calls occur. This is essential for CPU-bound parallel work where rayon may batch updates.

### Architecture Changes

**No architectural changes required** - all fixes are localized improvements to existing progress tracking infrastructure introduced in spec 128.

### Performance Considerations

- **Progress bar overhead**: Each `.inc(1)` call is ~50ns, negligible for 385 iterations
- **Steady tick overhead**: Spawns a background thread that redraws every 100ms, minimal CPU impact
- **Memory impact**: Progress bars add ~1KB per active bar, total < 10KB

### Integration Points

- **ProgressManager (spec 128)**: Use existing global progress manager
- **indicatif crate**: Use `enable_steady_tick()` API
- **MultiProgress**: Progress bars automatically coordinate through ProgressManager

## Dependencies

- **Prerequisites**: Spec 128 (indicatif progress infrastructure)
- **Affected Components**:
  - `src/builders/parallel_call_graph.rs` (enhanced analysis progress)
  - `src/builders/unified_analysis.rs` (trait resolution spinner)
  - `src/builders/parallel_unified_analysis.rs` (steady tick)
- **External Dependencies**: None (uses existing indicatif APIs)

## Testing Strategy

### Unit Tests

No new unit tests required - progress display is a UI concern validated through integration testing.

### Integration Tests

**Test: Enhanced Analysis Progress Visibility**
```rust
#[test]
fn test_enhanced_analysis_shows_progress() {
    // Set up test project with 10 files
    let project = create_test_project_with_files(10);

    // Run analysis and capture stderr
    let output = run_analysis_with_verbosity(&project, 2);

    // Verify progress messages appear
    assert!(output.contains("Enhanced call graph analysis"));
    assert!(output.contains("10/10 files"));
    assert!(output.contains("Enhanced analysis complete"));
}
```

**Test: Trait Resolution Shows Spinner**
```rust
#[test]
fn test_trait_resolution_spinner() {
    let project = create_test_rust_project();
    let output = run_analysis_with_verbosity(&project, 2);

    // Should show spinner, not "0/0 traits"
    assert!(output.contains("Resolving trait method calls"));
    assert!(!output.contains("0/0 traits"));
}
```

### Manual Testing

**Test Case**: 386-file project analysis
```bash
debtmap analyze . --lcov target/coverage/lcov.info -vv
```

**Expected output**:
```
Analyzing 386 files (medium project)
Parallel processing enabled for better performance
🔗 Call graph complete 385/385 files (100%) - 0s
🔧 Enhanced call graph analysis 385/385 files (100%) - 94s
 [OK]
Resolving trait method calls...⠋ Resolving trait method calls
 [OK]
Loading coverage data... [OK]
Creating unified analysis... [OK]
Starting parallel phase 1 (initialization)...
⚙️ Analyzing functions 9020/9020 functions (100%) - 3,704/sec - 2.4s
Total parallel analysis time: 99.3s
```

### Performance Tests

**Benchmark**: Progress overhead measurement
```bash
# Run analysis with progress enabled (baseline)
time debtmap analyze large_project/ -vv

# Verify overhead is < 2% compared to spec 128 baseline
```

## Documentation Requirements

### Code Documentation

- Add inline comments explaining steady tick rationale
- Document why spinner is used for trait resolution
- Comment on the sequential nature of enhanced analysis

### User Documentation

Update `book/src/parallel-processing.md`:
```markdown
## Progress Feedback

Debtmap provides real-time progress feedback during all analysis phases:

- **Call graph building**: File-by-file progress
- **Enhanced analysis**: Sequential analysis of framework patterns and trait dispatch
- **Trait resolution**: Spinner during trait method call resolution
- **Function analysis**: Parallel processing with per-function updates
- **File analysis**: Per-file debt aggregation progress

Progress updates occur at least every 200ms during active work to provide continuous feedback.
```

### Architecture Updates

No ARCHITECTURE.md updates needed - this is an enhancement to existing progress system.

## Implementation Notes

### Steady Tick Best Practices

- Only enable on CPU-intensive progress bars where manual updates may be infrequent
- Use 100ms interval (10 updates/second) for good responsiveness without overhead
- Disable on I/O-bound operations where manual updates are frequent enough

### Progress Bar Template Guidelines

- Use bar template (`{pos}/{len}`) when total count is known upfront
- Use spinner template when total is unknown or varies dynamically
- Include `{eta}` for long-running operations (> 5 seconds)
- Include `{percent}` for user-friendly completion indication

### Verbosity Levels

Current verbosity behavior (maintained):
- **Level 0** (default): Main progress bars only
- **Level 1** (`-v`): Sub-phase progress and timing
- **Level 2** (`-vv`): Detailed per-phase metrics

All fixes apply at default verbosity level to ensure all users benefit.

## Migration and Compatibility

### Breaking Changes

None - this is a pure enhancement to visual feedback.

### Compatibility

- Progress bars automatically hide in non-TTY environments (CI, piped output)
- `DEBTMAP_QUIET` environment variable disables all progress as before
- Existing `--quiet` flag behavior unchanged

### Performance Impact

Expected timing changes:
- Enhanced analysis: No slowdown (progress overhead < 0.1%)
- Function analysis: Potential 0.5% slowdown from steady tick background thread
- Overall analysis: < 1% total impact, well within 2% requirement

## Risk Assessment

### Low Risk Areas
- Steady tick addition (well-tested indicatif API)
- Spinner replacement (simpler than progress bar)

### Medium Risk Areas
- Enhanced analysis progress bar placement (ensure no deadlocks with sequential processing)

### Mitigation Strategies
- Test with various project sizes (10, 100, 1000+ files)
- Verify no progress bar overlap or corruption
- Ensure graceful degradation when ProgressManager unavailable

## Success Metrics

- Zero user reports of "frozen" or "hanging" analysis
- Continuous progress updates every < 500ms during all phases
- User satisfaction with progress visibility (qualitative feedback)
- No regression in analysis performance (< 2% overhead)

## Future Enhancements

Potential follow-up improvements (not in scope):
- Parallel enhanced analysis (requires significant refactoring)
- More granular progress within enhanced analysis phases
- Progress percentage in window title for background analysis
- Desktop notifications for long-running analysis completion
