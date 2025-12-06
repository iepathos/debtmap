---
number: 227
title: Combine Files and Parse Stages in Analysis Progression View
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-05
---

# Specification 227: Combine Files and Parse Stages in Analysis Progression View

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

The current analysis progression view displays file discovery and parsing as two separate stages:
1. "files" (Stage 0) - Discovering files
2. "parse" (Stage 1) - Analyzing complexity (parsing)

In practice, these two stages are tightly coupled - file discovery is quick and parsing immediately follows. Separating them creates unnecessary visual complexity and doesn't provide meaningful user value. Users care about "files being parsed" as a single conceptual unit rather than two separate steps.

This affects both:
- **CLI progress display** (src/io/progress.rs) - Shows "1/3 Discovering files" and "2/3 Analyzing complexity"
- **TUI interface** (src/tui/app.rs) - Shows separate "files" and "parse" stages in the pipeline view

## Objective

Combine the "files" and "parse" stages into a single "files parse" stage to create a cleaner, more meaningful analysis progression view that better represents the user's mental model of the analysis process.

## Requirements

### Functional Requirements

- Merge "Discovering files" (stage 0) and "Analyzing complexity" (stage 1) into single "files parse" stage
- Maintain ability to show file discovery count during early part of stage
- Transition smoothly from discovery count to parsing progress within the same stage
- Update phase numbering: combined stage becomes 1/2, call graph becomes 2/2
- Preserve all existing progress information (counts, percentages, timing)

### Non-Functional Requirements

- No degradation in progress visibility or user feedback
- Maintain compatibility with both CLI and TUI progress displays
- Preserve performance characteristics of progress updates
- Keep progress update throttling behavior intact

## Acceptance Criteria

- [ ] CLI progress display shows "1/2 files parse..." instead of separate "1/3 Discovering files" and "2/3 Analyzing complexity"
- [ ] TUI pipeline view shows single "files parse" stage instead of separate "files" and "parse" stages
- [ ] Progress metrics transition from "N found" to "N/M (X%)" within the same stage display
- [ ] Total phase count updated from 3 to 2 in CLI display
- [ ] Stage indices updated in src/commands/analyze.rs to reflect new numbering
- [ ] All tests updated to expect new stage names and numbering
- [ ] Integration tests pass with new progress display format
- [ ] TUI integration tests updated for new stage structure

## Technical Details

### Implementation Approach

**1. Update CLI Progress Display (src/io/progress.rs)**

Modify the `AnalysisProgress::new()` method to combine the first two phases:

```rust
phases: vec![
    AnalysisPhase::new("files parse", PhaseProgress::Indeterminate),
    AnalysisPhase::new("Building call graph", PhaseProgress::Indeterminate),
],
```

**2. Update TUI Pipeline Stages (src/tui/app.rs)**

Modify `create_default_stages()` to combine stages:

```rust
fn create_default_stages() -> Vec<PipelineStage> {
    vec![
        PipelineStage::new("files parse"),
        PipelineStage::new("call graph"),
        PipelineStage::with_subtasks("coverage", vec![...]),
        // ... rest of stages
    ]
}
```

**3. Update Stage Indices (src/commands/analyze.rs)**

Update the stage transition logic:

```rust
// Before: Phase 0 = files, Phase 1 = parse, Phase 2 = call graph
// After:  Phase 0 = files parse, Phase 1 = call graph

// Start combined files parse stage
io::progress::AnalysisProgress::with_global(|p| p.start_phase(0));
if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_start_stage(0); // files parse stage
}

// ... file discovery updates to phase 0 ...

// Continue phase 0 for parsing (no phase transition needed)

// ... parsing progress updates to phase 0 ...

// Phase 1: Building call graph (was Phase 2)
io::progress::AnalysisProgress::with_global(|p| p.start_phase(1));
if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_start_stage(1); // call graph stage (index decreased by 1)
}
```

**4. Update Test Expectations**

Update all test assertions expecting the old stage names and numbering:

- `tests/progress_display_integration_test.rs` - Update phase name assertions
- `tests/tui_integration_test.rs` - Update stage count and names
- Any other tests checking progress output

### Architecture Changes

**Before**:
```
Stage 0: files         → Discovering files
Stage 1: parse         → Analyzing complexity
Stage 2: call graph    → Building call graph
(3 total stages in CLI, indices 0-2)
```

**After**:
```
Stage 0: files parse   → files parse (discovery + parsing)
Stage 1: call graph    → Building call graph
(2 total stages in CLI, indices 0-1)
```

### Data Structures

No changes to data structures needed. The `PhaseProgress` enum already supports the required transition:
- Start with `PhaseProgress::Count(n)` during file discovery
- Transition to `PhaseProgress::Progress { current, total }` during parsing
- All within the same phase index (0)

### APIs and Interfaces

**Modified public methods** (behavior changes only, signatures unchanged):
- `AnalysisProgress::new()` - Returns 2 phases instead of 3
- `App::create_default_stages()` - Returns stages with combined "files parse" entry

**Call site updates required**:
- `src/commands/analyze.rs` - Update phase indices (0, 1 instead of 0, 1, 2)
- All progress update calls - Use phase 0 for both discovery and parsing

## Dependencies

**Prerequisites**: None

**Affected Components**:
- `src/io/progress.rs` - CLI progress display logic
- `src/tui/app.rs` - TUI pipeline stage definitions
- `src/commands/analyze.rs` - Stage transition coordination
- `tests/progress_display_integration_test.rs` - Progress output tests
- `tests/tui_integration_test.rs` - TUI stage tests

**External Dependencies**: None

## Testing Strategy

### Unit Tests

**src/io/progress.rs**:
- [ ] Test phase count is 2 (was 3)
- [ ] Test first phase name is "files parse"
- [ ] Test progress transitions within same phase (Count → Progress)
- [ ] Test format_progress handles both discovery and parsing displays

**src/tui/app.rs**:
- [ ] Test default stages contains "files parse" as first stage
- [ ] Test stage count is 8 (files parse + call graph + 6 others)
- [ ] Test start_stage(0) activates "files parse"
- [ ] Test complete_stage(0, metric) marks "files parse" complete

### Integration Tests

**tests/progress_display_integration_test.rs**:
- [ ] Update assertions to expect "1/2 files parse" instead of "1/3 Discovering files"
- [ ] Update assertions to expect "2/2 Building call graph" instead of "3/3 Building call graph"
- [ ] Verify "Analyzing complexity" no longer appears as separate phase
- [ ] Test progress transitions smoothly within files parse stage

**tests/tui_integration_test.rs**:
- [ ] Update stage count expectations
- [ ] Verify "files parse" appears in TUI output
- [ ] Verify "files" and "parse" no longer appear as separate stages
- [ ] Test stage activation and completion with new indices

### User Acceptance

- [ ] Run analysis on small codebase, verify CLI output shows cleaner progression
- [ ] Run analysis with TUI enabled, verify stage list is more concise
- [ ] Verify no loss of progress information during file discovery or parsing
- [ ] Confirm elapsed time tracking still accurate per stage

## Documentation Requirements

### Code Documentation

- [ ] Update doc comment example in `src/io/progress.rs` to show new 2-stage output
- [ ] Update doc comment in `src/tui/mod.rs` if it references stage count
- [ ] Add code comment explaining why files and parse are combined

### User Documentation

- [ ] Update any user-facing documentation mentioning analysis stages
- [ ] Update screenshots or examples showing progress output (if any)

### Architecture Updates

- [ ] Update ARCHITECTURE.md if it documents the analysis pipeline stages
- [ ] Document the rationale for combining stages (user mental model alignment)

## Implementation Notes

### Progress Display Behavior

The combined "files parse" stage should:
1. Start with indeterminate progress during initial file discovery
2. Show "N found" when file count is known
3. Transition to "N/M (X%)" as parsing begins
4. Maintain the same throttling behavior (10 updates/sec max)

### Backward Compatibility

This is a user-facing display change only. The internal analysis logic remains unchanged. No data format changes or API breakages.

### Performance Considerations

- No performance impact expected
- Progress update frequency unchanged
- TUI rendering performance unchanged

## Migration and Compatibility

### Breaking Changes

**User-facing**:
- Progress output format changes from 3 phases to 2 phases
- Users scripting/parsing progress output will see different text

**Internal**:
- Stage indices shift down by 1 after the combined stage
- Test assertions must be updated

### Migration Steps

No migration required for users. This is a display-only change.

For developers:
1. Update any custom code referencing stage indices (rare)
2. Update test expectations
3. Re-run test suite to verify

### Compatibility Considerations

- CI/CD output parsers may need updates if they depend on specific phase text
- Monitoring tools watching progress output may need adjustment
- No file format changes, no data compatibility issues
