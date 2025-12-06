---
number: 222
title: TUI Call Graph Stage Subsections
category: optimization
priority: low
status: draft
dependencies: []
created: 2025-01-09
---

# Specification 222: TUI Call Graph Stage Subsections

**Category**: optimization
**Priority**: low
**Status**: draft
**Dependencies**: None

## Context

The call graph building stage (stage 3) is currently a monolithic operation showing only a single spinner with no insight into the multiple phases happening internally. For large codebases, this can take 5-10+ seconds with no feedback.

Looking at the implementation:
- **Sequential mode** (`src/builders/call_graph.rs:process_rust_files_for_call_graph`) - Single-threaded AST traversal
- **Parallel mode** (`src/builders/parallel_call_graph.rs:237-250`) - Multi-threaded call graph construction

The parallel mode in particular has distinct phases:
1. **File discovery** - Finding Rust source files
2. **AST parsing** - Parsing syntax trees for each file
3. **Edge extraction** - Building call relationships from ASTs
4. **Cross-module linking** - Resolving cross-file function calls

These phases are already somewhat separated in the code but not exposed to the user.

## Objective

Add subsections to the TUI call graph stage (stage 3) to show real-time progress through the 4 major phases of call graph construction, with file processing showing actual progress counts.

## Requirements

### Functional Requirements

- Display 4 subsections under the call graph stage when active:
  1. "discover files" - Finding Rust source files with progress
  2. "parse ASTs" - Parsing syntax trees with progress
  3. "extract calls" - Building call edges with progress
  4. "link modules" - Resolving cross-file references
- Update each subsection status as Pending → Active → Completed
- Show progress information where applicable (e.g., "234/511 files")
- Update progress at throttled rate (every 10 files or 100ms)
- Support both sequential and parallel modes
- Add minimum 150ms visibility pause between subsections

### Non-Functional Requirements

- Maintain 60 FPS TUI rendering performance
- Minimize overhead of progress tracking (<2% of build time)
- Throttle updates appropriately for performance
- Handle both parallel and sequential modes identically from TUI perspective
- Support edge cases (empty project, single file, thousands of files)

## Acceptance Criteria

- [ ] Call graph stage shows 4 subsections when active
- [ ] Each subsection transitions Pending → Active → Completed correctly
- [ ] File processing subsections show progress (current/total)
- [ ] Progress updates throttled appropriately
- [ ] Works identically in parallel and sequential modes
- [ ] TUI renders at 60 FPS during call graph building
- [ ] Progress updates add <2% overhead to build time
- [ ] Handles edge cases (0 files, 1 file, 10k+ files) gracefully

## Technical Details

### Implementation Approach

1. **Update TUI App Structure** (`src/tui/app.rs:114-151`):
   - Modify `create_default_stages()` to create call graph stage with subsections
   - Change from `PipelineStage::new("call graph")` to `PipelineStage::with_subtasks()`
   - Add 4 SubTask entries for the phases

2. **Instrument Sequential Mode** (`src/builders/call_graph.rs`):
   - Add subsection updates for each phase
   - Track file discovery progress
   - Update progress during AST parsing loop
   - Report edge extraction and linking phases

3. **Instrument Parallel Mode** (`src/builders/parallel_call_graph.rs`):
   - Add subsection updates for each phase
   - Use atomic counters for thread-safe progress across workers
   - Aggregate progress from parallel workers
   - Report unified progress to TUI

4. **Refactor for Progress Visibility**:
   - May need to split large functions to expose phase boundaries
   - Add progress callback parameters to internal functions
   - Use shared progress state for parallel workers

### Architecture Changes

**Moderate refactoring required**: Call graph building is currently less structured than other stages. To expose subsections, we need to:

1. Extract file discovery as separate phase (currently implicit in file iteration)
2. Separate AST parsing from edge extraction (currently interleaved)
3. Make cross-module linking explicit (currently part of finalization)

This is more invasive than the other subsection specs, hence the "low" priority.

### Data Structures

Modify `create_default_stages()` in `src/tui/app.rs`:

```rust
PipelineStage::with_subtasks(
    "call graph",
    vec![
        SubTask {
            name: "discover files".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "parse ASTs".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "extract calls".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "link modules".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
    ],
),
```

### APIs and Interfaces

**Progress Callback Pattern**:

```rust
struct CallGraphProgress {
    phase: CallGraphPhase,
    current: usize,
    total: usize,
}

enum CallGraphPhase {
    DiscoveringFiles,
    ParsingASTs,
    ExtractingCalls,
    LinkingModules,
}

// In parallel mode:
pub fn build_call_graph_parallel<F>(
    project_path: &Path,
    base_graph: CallGraph,
    num_threads: Option<usize>,
    mut progress_callback: F,
) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)>
where
    F: FnMut(CallGraphProgress) + Send + Sync,
{
    // Phase 1: Discover files
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::DiscoveringFiles,
        current: 0,
        total: 0,
    });

    let files = discover_rust_files(project_path)?;
    let file_count = files.len();

    // Phase 2: Parse ASTs (parallel)
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::ParsingASTs,
        current: 0,
        total: file_count,
    });

    let parsed_count = Arc::new(AtomicUsize::new(0));
    let asts: Vec<_> = files.par_iter().map(|file| {
        let ast = parse_file(file)?;
        let count = parsed_count.fetch_add(1, Ordering::Relaxed) + 1;

        // Throttled callback
        if count % 10 == 0 || count == file_count {
            progress_callback(CallGraphProgress {
                phase: CallGraphPhase::ParsingASTs,
                current: count,
                total: file_count,
            });
        }

        Ok(ast)
    }).collect::<Result<_>>()?;

    // Phase 3: Extract calls (parallel)
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::ExtractingCalls,
        current: 0,
        total: file_count,
    });

    // ... similar pattern for call extraction ...

    // Phase 4: Link modules
    progress_callback(CallGraphProgress {
        phase: CallGraphPhase::LinkingModules,
        current: 0,
        total: 0, // Indeterminate
    });

    // ... cross-module resolution ...

    Ok((graph, exclusions, used_funcs))
}
```

**TUI Integration** (`src/builders/unified_analysis.rs`):

```rust
let (framework_exclusions, function_pointer_used_functions) = if parallel {
    build_parallel_call_graph_with_subsections(project_path, &mut call_graph, jobs)?
} else {
    build_sequential_call_graph_with_subsections(
        project_path,
        &mut call_graph,
        verbose_macro_warnings,
        show_macro_stats,
    )?
};

fn build_parallel_call_graph_with_subsections(
    project_path: &Path,
    call_graph: &mut CallGraph,
    jobs: usize,
) -> Result<(HashSet<FunctionId>, HashSet<FunctionId>)> {
    parallel_call_graph::build_call_graph_parallel(
        project_path,
        call_graph.clone(),
        if jobs == 0 { None } else { Some(jobs) },
        |progress| {
            if let Some(manager) = crate::progress::ProgressManager::global() {
                match progress.phase {
                    CallGraphPhase::DiscoveringFiles => {
                        manager.tui_update_subtask(2, 0, StageStatus::Active, None);
                    }
                    CallGraphPhase::ParsingASTs => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(2, 0, StageStatus::Completed, None);
                            std::thread::sleep(Duration::from_millis(150));
                            manager.tui_update_subtask(2, 1, StageStatus::Active, Some((0, progress.total)));
                        } else {
                            manager.tui_update_subtask(2, 1, StageStatus::Active, Some((progress.current, progress.total)));
                        }
                    }
                    CallGraphPhase::ExtractingCalls => {
                        if progress.current == 0 {
                            manager.tui_update_subtask(2, 1, StageStatus::Completed, None);
                            std::thread::sleep(Duration::from_millis(150));
                            manager.tui_update_subtask(2, 2, StageStatus::Active, Some((0, progress.total)));
                        } else {
                            manager.tui_update_subtask(2, 2, StageStatus::Active, Some((progress.current, progress.total)));
                        }
                    }
                    CallGraphPhase::LinkingModules => {
                        manager.tui_update_subtask(2, 2, StageStatus::Completed, None);
                        std::thread::sleep(Duration::from_millis(150));
                        manager.tui_update_subtask(2, 3, StageStatus::Active, None);
                    }
                }
            }
        },
    ).map(|(g, e, u)| {
        *call_graph = g;
        (e, u)
    })
}
```

## Dependencies

- **Prerequisites**: None (extends existing TUI infrastructure)
- **Affected Components**:
  - `src/tui/app.rs` - Stage definition
  - `src/builders/parallel_call_graph.rs` - Parallel mode refactoring
  - `src/builders/call_graph.rs` - Sequential mode refactoring
  - `src/builders/unified_analysis.rs` - Integration point
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

- Test that call graph stage has 4 subsections defined
- Verify subsection status transitions
- Test progress callback invocation
- Test parallel and sequential modes produce same subsection behavior
- Verify edge cases (0 files, 1 file, all parsing failures)

### Integration Tests

- Run analysis on small project (<10 files) and verify subsections
- Run analysis on large project (500+ files) and verify throttling
- Test parallel mode with different thread counts (1, 2, 4, 8)
- Test sequential mode
- Verify subsections complete correctly even on parse errors

### Performance Tests

- Benchmark call graph build time with vs without progress updates
- Verify <2% overhead from progress tracking
- Test with various codebase sizes (10, 100, 1000 files)
- Profile atomic counter overhead in parallel mode
- Compare parallel mode performance with/without progress

### Manual Testing

- Visual verification of subsection progression
- Confirm progress updates appear smooth in both modes
- Test on debtmap's own codebase (~500 Rust files)
- Verify subsections expand/collapse correctly
- Test with very large projects (1000+ files)

## Documentation Requirements

### Code Documentation

- Add comments explaining CallGraphPhase enum and mapping to subsections
- Document progress callback pattern and throttling
- Add examples of subsection instrumentation
- Document parallel mode atomic counter usage

### User Documentation

No user documentation updates needed - TUI is self-documenting through visual display.

### Architecture Updates

Update `docs/TUI_ARCHITECTURE.md`:
- Document call graph subsections as example of progress callback pattern
- Explain parallel mode progress aggregation
- Show code example of subsection instrumentation

## Implementation Notes

### Why Low Priority?

This spec is marked "low" priority (vs "high" for context and debt scoring) because:

1. **Higher implementation complexity** - Requires refactoring call graph builder
2. **Less critical user value** - Call graph builds relatively fast on most projects
3. **More invasive changes** - Touches core analysis infrastructure
4. **Parallel mode complexity** - Requires thread-safe progress aggregation

The context and debt scoring subsections provide better ROI (easier implementation, higher user value).

### Refactoring Required

Current call graph building is monolithic. To add subsections requires:

1. **Extract file discovery** - Currently implicit, needs to be explicit phase
2. **Separate parsing from extraction** - Currently interleaved in same loop
3. **Make linking explicit** - Currently part of graph finalization
4. **Add progress hooks** - Need callback points in each phase

This is roughly 200-300 lines of refactoring across multiple files.

### Parallel Mode Challenges

Progress tracking in parallel mode requires:
- **Atomic counters** for thread-safe increments
- **Progress aggregation** across worker threads
- **Callback synchronization** to avoid race conditions
- **Overhead minimization** to not slow down parallel speedup

The existing parallel implementation uses rayon which makes this easier, but still requires careful design.

### Alternative: Coarse-Grained Subsections

Instead of 4 fine-grained subsections, use 2 coarse-grained:
1. "build graph" - Discovery, parsing, extraction (with progress)
2. "resolve links" - Cross-module linking

This would be easier to implement (less refactoring) but provide less insight.

## Migration and Compatibility

### Breaking Changes

**API changes**: Call graph builder functions gain optional progress callback parameter. Existing callers can pass no-op callback.

### Migration Path

1. Add progress callback parameter as Option<F> (backward compatible)
2. Update primary call site (unified_analysis.rs) to provide callback
3. In future, make callback required (breaking change)

Or keep both APIs:
```rust
// With progress
pub fn build_call_graph_parallel_with_progress<F>(...)

// Without progress (convenience wrapper)
pub fn build_call_graph_parallel(...) {
    build_call_graph_parallel_with_progress(..., |_| {})
}
```

## Future Enhancements

### Deferred to Future Specs

1. **Per-thread progress visualization** - Show progress for each parallel worker
2. **Call graph statistics** - Show edge count, node count in subsection metrics
3. **Incremental call graph building** - Only re-parse changed files
4. **Call graph caching** - Cache parsed ASTs between runs

These are out of scope but could provide additional value.

## Alternative Approaches Considered

### Phase Merging

Merge "parse ASTs" and "extract calls" into single "analyze files" subsection since they're tightly coupled.

**Rejected because**: Users want to see what's actually happening. Parsing and extraction are conceptually distinct even if implementation interleaves them.

### Progress Estimation

Use file size to estimate progress more accurately (larger files take longer to parse).

**Deferred because**: Adds complexity for marginal benefit. File count is simpler and still provides useful feedback.

### Streaming Progress

Show files as they complete rather than aggregate counts.

**Rejected because**: TUI would need scrolling list UI, much more complex than simple progress counter.
