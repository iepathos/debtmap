---
number: 219
title: TUI Context Stage Subsections
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-01-09
---

# Specification 219: TUI Context Stage Subsections

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

The TUI progression display currently shows 9 pipeline stages, but only the purity analysis stage (stage 6) has subsections that expand to show detailed progress. The context loading stage (stage 7) loads 3 independent context providers but shows only a single spinner with no visibility into which providers are being loaded.

Currently in `src/utils/risk_analyzer.rs:123-179`, the `build_context_aggregator` function loads three distinct providers:
- **critical_path** - Critical path analysis
- **dependency** - Dependency graph analysis
- **git_history** - Git commit history analysis

These providers are loaded sequentially, and users have no visibility into which provider is currently being processed. This creates a "black box" effect where users don't know if the stage is hanging or making progress.

## Objective

Add subsections to the TUI context stage (stage 7) to show real-time progress for each of the 3 context providers being loaded, matching the pattern established by the purity analysis stage.

## Requirements

### Functional Requirements

- Display 3 subsections under the context stage when it's active:
  1. "critical path" - Critical path provider
  2. "dependencies" - Dependency analysis provider
  3. "git history" - Git history provider
- Update each subsection status as Pending → Active → Completed
- Show subsections only when the context stage is active
- Hide subsections when context is disabled (--no-context flag)
- Add minimum 150ms visibility pause between subsection completions

### Non-Functional Requirements

- Maintain 60 FPS TUI rendering performance
- No impact on analysis performance (pauses only for visibility)
- Follow existing subsection rendering pattern from purity analysis
- Gracefully handle provider loading failures

## Acceptance Criteria

- [ ] Context stage shows 3 subsections when active
- [ ] Each subsection transitions Pending → Active → Completed correctly
- [ ] Subsections are visible for at least 150ms each
- [ ] Subsections hidden when context disabled
- [ ] TUI renders at 60 FPS during context loading
- [ ] Provider failures don't crash the TUI display
- [ ] Subsection names match provider names from error messages

## Technical Details

### Implementation Approach

1. **Update TUI App Structure** (`src/tui/app.rs:114-151`):
   - Modify `create_default_stages()` to create context stage with subsections
   - Change from `PipelineStage::new("context")` to `PipelineStage::with_subtasks()`
   - Add 3 SubTask entries for the providers

2. **Instrument Context Loading** (`src/utils/risk_analyzer.rs:123-179`):
   - Add TUI subsection updates to `build_context_aggregator`
   - Update subsection status before/after each provider creation
   - Add 150ms visibility pauses between provider loads
   - Handle case where context is disabled

3. **Update Main Analysis Flow** (`src/builders/unified_analysis.rs:340-385`):
   - Keep existing stage start/complete calls
   - Ensure context provider loop updates subsections
   - Handle both enabled and disabled context states

### Architecture Changes

No structural changes required. This extends the existing TUI subsection pattern to the context stage.

### Data Structures

Modify `create_default_stages()` in `src/tui/app.rs`:

```rust
PipelineStage::with_subtasks(
    "context",
    vec![
        SubTask {
            name: "critical path".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "dependencies".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "git history".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
    ],
),
```

### APIs and Interfaces

New TUI update calls in `build_context_aggregator`:

```rust
// Subtask 0: Critical path provider
if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(6, 0, crate::tui::app::StageStatus::Active, None);
}
// ... create critical path provider ...
if let Some(manager) = crate::progress::ProgressManager::global() {
    manager.tui_update_subtask(6, 0, crate::tui::app::StageStatus::Completed, None);
    std::thread::sleep(std::time::Duration::from_millis(150));
}

// Similar for subtasks 1 (dependencies) and 2 (git history)
```

## Dependencies

- **Prerequisites**: None (extends existing TUI infrastructure)
- **Affected Components**:
  - `src/tui/app.rs` - Stage definition
  - `src/utils/risk_analyzer.rs` - Context loading
  - `src/builders/unified_analysis.rs` - Integration point
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

- Test that context stage has 3 subsections defined
- Verify subsection status transitions
- Test that subsections hidden when context disabled

### Integration Tests

- Run full analysis with --context flag and verify subsections appear
- Run with --no-context and verify subsections don't interfere
- Test with missing git repository (git_history provider fails)
- Verify TUI cleanup works with context subsections

### Manual Testing

- Visual verification of subsection display in terminal
- Confirm 150ms minimum visibility per subsection
- Test on small and large projects
- Verify subsections expand/collapse correctly

## Documentation Requirements

### Code Documentation

- Add comments explaining subsection indices (0=critical_path, 1=dependencies, 2=git_history)
- Document the visibility pause rationale
- Add module-level comment about context provider mapping

### User Documentation

No user documentation updates needed - TUI is self-documenting through visual display.

### Architecture Updates

Update `docs/TUI_ARCHITECTURE.md` to document the context stage subsections as an example alongside purity analysis.

## Implementation Notes

### Provider Ordering

The subsection order should match the loading order in `build_context_aggregator`:
1. Critical path (index 0)
2. Dependencies (index 1)
3. Git history (index 2)

### Error Handling

If a provider fails to create (e.g., git_history when not in a git repo), still mark the subsection as Completed rather than leaving it hanging. The warning message will appear in the console separately.

### Disabled Context Case

When `enable_context` is false, the context stage should:
- Not show subsections (empty sub_tasks vec would work)
- Complete immediately with metric "skipped"
- This matches existing behavior from line 382-383 in unified_analysis.rs

### Performance Considerations

The 150ms pauses add ~450ms total to context loading (3 providers × 150ms). This is acceptable because:
- Context loading already takes several hundred milliseconds for git history parsing
- User feedback value outweighs the small delay
- Can be disabled by setting pauses to 0ms if needed

## Migration and Compatibility

No breaking changes. This is purely a visual enhancement to the TUI. Users who don't use the TUI (--quiet mode or non-TTY) see no difference.

## Alternative Approaches Considered

### Dynamic Subsection Creation

Instead of hardcoding 3 subsections, dynamically create them based on enabled providers.

**Rejected because**:
- Adds complexity to TUI state management
- Providers are known at compile time
- Harder to maintain consistent subsection indices
- No significant benefit over static definition

### Progress Bars for Provider Loading

Show progress bars (e.g., "45/100 commits") for each provider.

**Deferred because**:
- Requires refactoring provider creation to report progress
- Much larger scope than visual feedback improvement
- Can be added in a future spec if needed
- Current approach provides immediate value with minimal changes
