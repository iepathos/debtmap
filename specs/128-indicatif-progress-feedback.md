---
number: 128
title: Indicatif Progress Feedback for All Analysis Phases
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-25
---

# Specification 128: Indicatif Progress Feedback for All Analysis Phases

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently provides inconsistent progress feedback during analysis, with a critical gap after the call graph building phase completes. Users see "Building call graph... 100%" followed by 5+ minutes of silence where CPUs are maxed but no visual feedback is provided. This creates a poor user experience where users cannot tell if:

1. The analysis is still running or has hung
2. How much longer the analysis will take
3. Which phase is currently executing
4. Whether progress is being made

**Current Progress Feedback Issues**:

1. **Call Graph Building** (✓ Good)
   - Location: `src/builders/parallel_call_graph.rs:230-265`
   - Custom progress implementation with percentage updates
   - Shows: `🔗 Building call graph... 380/383 files (99%) - 3 remaining`
   - Updates every 5% or at completion

2. **Trait Resolution** (⚠️ Minimal)
   - Location: `src/builders/unified_analysis.rs:369-372`
   - Only shows start/end: `"Resolving trait method calls..."` → `" [OK]"`
   - No incremental progress during processing

3. **Coverage Loading** (⚠️ Minimal)
   - Location: `src/builders/unified_analysis.rs:390-411`
   - Only shows start/end: `"Loading coverage data..."` → `" [OK]"`
   - No feedback during file parsing

4. **Unified Analysis Creation** (❌ Critical Gap)
   - Location: `src/builders/unified_analysis.rs:415-446`
   - Shows: `"Creating unified analysis... "` then **5+ minutes of silence**
   - Three parallel phases with NO incremental feedback:
     - Phase 1: Initialization (data flow, purity, test detection, debt aggregation)
     - Phase 2: Function analysis (2000+ functions × complex debt classification)
     - Phase 3: File analysis (god object detection, coverage analysis)
   - Users see maxed CPUs but no visual confirmation of progress

5. **Phase 2 Function Processing** (❌ Worst Bottleneck)
   - Location: `src/builders/parallel_unified_analysis.rs:637-640`
   - Parallel iteration over ALL function metrics: `metrics.par_iter().filter_map(...).collect()`
   - For each function: debt classification, impact calculation, recommendation generation
   - Typical timing: 2000 functions × 100ms = 200 seconds with ZERO feedback
   - Only shows start/end messages (lines 599-600, 619-624)

6. **Phase 3 File Analysis** (⚠️ No Progress)
   - Location: `src/builders/parallel_unified_analysis.rs:704-709`
   - Parallel file processing: `files_map.par_iter().filter_map(...).collect()`
   - Reads every source file, analyzes god objects
   - Only shows start/end messages (lines 690-691, 713-718)

**Real-World Impact**:
- Medium project (384 files, ~2000 functions): 5+ minutes of silence after call graph
- Users interrupt analysis thinking it has hung
- No way to estimate completion time
- Poor user experience compared to modern CLI tools

**Current Custom Progress Implementation Limitations**:
- Custom callback-based progress (used in call graph builder)
- Requires manual progress tracking and update logic
- No built-in support for parallel iterators
- No time estimation or ETA
- Inconsistent formatting across phases
- Difficult to maintain and extend

## Objective

Replace all manual progress implementations with `indicatif` library to provide consistent, professional, and informative progress feedback across all analysis phases. Users should see continuous visual feedback with progress bars, ETAs, and clear phase labels throughout the entire analysis process.

## Requirements

### Functional Requirements

1. **Replace Custom Progress with Indicatif**
   - Remove callback-based progress from `ParallelCallGraphBuilder`
   - Implement `indicatif::ProgressBar` for all parallel iterations
   - Use `indicatif::MultiProgress` for concurrent phase tracking
   - Maintain existing progress display behavior (percentage, counts, remaining)

2. **Call Graph Building Progress**
   - Convert custom progress callback to `ProgressBar`
   - Show file processing progress with same information format
   - Template: `{spinner} {msg} {pos}/{len} files ({percent}%) - {eta}`
   - Update frequency: Every file processed (let indicatif handle throttling)

3. **Trait Resolution Progress**
   - Add progress bar for trait method resolution
   - Track progress through trait implementations
   - Show count of traits analyzed and remaining
   - Template: `{spinner} {msg} {pos}/{len} traits - {eta}`

4. **Coverage Loading Progress**
   - Add progress bar for lcov file parsing
   - Track progress through coverage entries
   - Show file-level or line-level progress
   - Template: `{spinner} {msg} {pos}/{len} files - {eta}`

5. **Unified Analysis Multi-Phase Progress**
   - Use `MultiProgress` to show all three phases concurrently
   - Phase 1 (Initialization): Show 4 concurrent tasks
     - Data flow graph creation
     - Purity analysis
     - Test detection
     - Debt aggregation
   - Phase 2 (Function Analysis): Primary bottleneck
     - Show function processing progress: `{pos}/{len} functions`
     - Include throughput: `{per_sec} functions/sec`
     - Show ETA for completion
   - Phase 3 (File Analysis):
     - Show file processing progress: `{pos}/{len} files`
     - Include god object detection status

6. **Parallel Iterator Integration**
   - Wrap `rayon::ParallelIterator` with progress tracking
   - Use `indicatif::ParallelProgressIterator` for seamless integration
   - Maintain parallelism performance (no serialization bottlenecks)
   - Handle concurrent updates safely (indicatif handles internal locking)

7. **Progress Bar Styling**
   - Consistent visual style across all progress bars
   - Use appropriate progress bar templates for each phase
   - Include spinner for indeterminate progress
   - Use color coding (via `colored` crate integration)
   - Clear phase labels: "Call Graph", "Trait Resolution", "Coverage", etc.

8. **Quiet Mode Support**
   - Respect `DEBTMAP_QUIET` environment variable
   - Respect `--quiet` CLI flag
   - Disable all progress output in quiet mode
   - Maintain existing quiet mode logic

9. **Verbosity Integration**
   - Level 0 (default): Show main progress bars only
   - Level 1 (`-v`): Show sub-phase progress and timing details
   - Level 2 (`-vv`): Show detailed per-phase metrics and statistics
   - Use `MultiProgress` to manage hierarchical progress display

### Non-Functional Requirements

1. **Performance**
   - Progress updates must not significantly impact analysis performance (<2% overhead)
   - Use indicatif's built-in update throttling (default 15 updates/sec)
   - Leverage lock-free progress tracking where possible
   - Maintain full parallelism (no forced serialization)

2. **User Experience**
   - Progress feedback must be smooth and non-flickering
   - Terminal width should be respected (use `{wide_bar}` template)
   - Progress bars should clear cleanly on completion
   - ETAs should be reasonably accurate (let indicatif handle estimation)

3. **Maintainability**
   - Centralize progress bar creation and configuration
   - Use builder pattern for consistent progress bar setup
   - Document progress tracking points in code
   - Make it easy to add progress to new analysis phases

4. **Compatibility**
   - Work correctly in CI environments (non-TTY detection)
   - Handle terminal resize gracefully
   - Support both colored and non-colored output
   - Maintain backward compatibility with existing flags

## Acceptance Criteria

- [ ] **Indicatif dependency added** to Cargo.toml
- [ ] **Call graph progress** converted from custom callback to `ProgressBar`
- [ ] **Trait resolution progress** shows incremental progress with ETA
- [ ] **Coverage loading progress** shows file parsing progress with ETA
- [ ] **Phase 1 initialization** shows 4 concurrent task progress bars via `MultiProgress`
- [ ] **Phase 2 function analysis** shows continuous progress during parallel iteration
  - Progress bar updates as functions are processed
  - Shows throughput (functions/sec)
  - Shows accurate ETA
  - No 5+ minute silence period
- [ ] **Phase 3 file analysis** shows file processing progress with god object detection status
- [ ] **Parallel iterator integration** uses `ParallelProgressIterator` without serialization
- [ ] **Quiet mode** disables all progress output when enabled
- [ ] **Verbosity levels** control progress detail (-v, -vv)
- [ ] **Terminal width** is respected and bars don't overflow
- [ ] **CI/non-TTY** detection works correctly (fallback to simple logging)
- [ ] **Performance overhead** is less than 2% compared to no progress tracking
- [ ] **No visual artifacts** - progress bars clear cleanly, no flickering
- [ ] **Consistent styling** across all progress bars with clear phase labels
- [ ] **Unit tests** added for progress bar creation and configuration
- [ ] **Integration tests** verify progress in real analysis runs
- [ ] **Documentation** updated with progress feedback architecture

## Technical Details

### Implementation Approach

#### 1. Dependency Addition

Add `indicatif` to `Cargo.toml`:

```toml
[dependencies]
# Progress feedback
indicatif = { version = "0.17", features = ["rayon"] }
```

The `rayon` feature enables `ParallelProgressIterator` trait for seamless parallel iteration progress.

#### 2. Progress Bar Configuration Module

Create `src/progress.rs` module for centralized progress management:

```rust
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;

pub struct ProgressConfig {
    pub quiet_mode: bool,
    pub verbosity: u8,
}

impl ProgressConfig {
    pub fn from_env() -> Self {
        Self {
            quiet_mode: std::env::var("DEBTMAP_QUIET").is_ok(),
            verbosity: /* parse from CLI args */,
        }
    }

    pub fn should_show_progress(&self) -> bool {
        !self.quiet_mode && atty::is(atty::Stream::Stderr)
    }
}

pub struct ProgressManager {
    multi: MultiProgress,
    config: ProgressConfig,
}

impl ProgressManager {
    pub fn new(config: ProgressConfig) -> Self {
        Self {
            multi: MultiProgress::new(),
            config,
        }
    }

    pub fn create_bar(&self, len: u64, template: &str) -> ProgressBar {
        if !self.config.should_show_progress() {
            return ProgressBar::hidden();
        }

        let pb = self.multi.add(ProgressBar::new(len));
        pb.set_style(ProgressStyle::default_bar()
            .template(template)
            .expect("Invalid template")
            .progress_chars("█▓▒░  "));
        pb
    }

    pub fn create_spinner(&self, msg: &str) -> ProgressBar {
        if !self.config.should_show_progress() {
            return ProgressBar::hidden();
        }

        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .expect("Invalid template"));
        pb.set_message(msg.to_string());
        pb
    }
}
```

#### 3. Call Graph Progress Conversion

Replace callback in `src/builders/parallel_call_graph.rs`:

**Before** (lines 230-265):
```rust
if show_progress {
    config = config.with_progress(|processed, total| {
        // Custom progress logic
    });
}
```

**After**:
```rust
pub fn build_call_graph_parallel(
    project_path: &Path,
    base_graph: CallGraph,
    num_threads: Option<usize>,
    progress_manager: &ProgressManager,
) -> Result<(CallGraph, HashSet<FunctionId>, HashSet<FunctionId>)> {
    let progress = progress_manager.create_bar(
        file_count as u64,
        "🔗 {msg} {pos}/{len} files ({percent}%) - {eta}"
    );
    progress.set_message("Building call graph");

    // Use ParallelProgressIterator
    use indicatif::ParallelProgressIterator;

    files.into_par_iter()
        .progress_with(progress.clone())
        .map(|file| process_file(file))
        .collect()
}
```

#### 4. Phase 2 Function Analysis Progress

Update `src/builders/parallel_unified_analysis.rs`:

**Current** (lines 637-640):
```rust
metrics
    .par_iter()
    .filter_map(|metric| self.process_single_metric(metric, test_only_functions, context))
    .collect()
```

**Enhanced**:
```rust
use indicatif::ParallelProgressIterator;

let progress = progress_manager.create_bar(
    metrics.len() as u64,
    "⚙️  {msg} {pos}/{len} functions ({percent}%) - {per_sec} functions/sec - {eta}"
);
progress.set_message("Analyzing functions");

let items: Vec<UnifiedDebtItem> = metrics
    .par_iter()
    .progress_with(progress.clone())
    .filter_map(|metric| self.process_single_metric(metric, test_only_functions, context))
    .collect();

progress.finish_with_message("Function analysis complete");
```

#### 5. Multi-Phase Progress Display

For Phase 1 initialization with concurrent tasks:

```rust
pub fn execute_phase1_parallel(&mut self, progress_manager: &ProgressManager) {
    let pb_data_flow = progress_manager.create_spinner("Data flow graph");
    let pb_purity = progress_manager.create_spinner("Purity analysis");
    let pb_tests = progress_manager.create_spinner("Test detection");
    let pb_debt = progress_manager.create_spinner("Debt aggregation");

    rayon::scope(|s| {
        s.spawn(|_| {
            // Data flow graph creation
            pb_data_flow.finish_with_message("Data flow graph ✓");
        });

        s.spawn(|_| {
            // Purity analysis
            pb_purity.finish_with_message("Purity analysis ✓");
        });

        // ... other tasks
    });
}
```

#### 6. Progress Bar Templates

**Standard Templates**:
- Call graph: `🔗 {msg} {pos}/{len} files ({percent}%) - {eta}`
- Trait resolution: `🔍 {msg} {pos}/{len} traits - {eta}`
- Coverage: `📊 {msg} {pos}/{len} files - {eta}`
- Function analysis: `⚙️  {msg} {pos}/{len} functions ({percent}%) - {per_sec}/sec - {eta}`
- File analysis: `📁 {msg} {pos}/{len} files ({percent}%) - {eta}`
- Spinner: `{spinner} {msg}`

### Architecture Changes

**New Module**:
- `src/progress.rs` - Centralized progress management

**Modified Modules**:
- `src/builders/parallel_call_graph.rs` - Replace custom progress with indicatif
- `src/builders/parallel_unified_analysis.rs` - Add progress to Phase 1, 2, 3
- `src/builders/unified_analysis.rs` - Integrate progress manager
- `src/commands/analyze.rs` - Initialize progress manager, pass to builders
- `src/main.rs` - Parse verbosity flags, create progress config

**Configuration Flow**:
1. `main.rs` - Parse CLI args, detect quiet mode, determine verbosity
2. Create `ProgressConfig` from environment and flags
3. Create `ProgressManager` from config
4. Pass `&ProgressManager` to all builder functions
5. Builders create progress bars as needed

### Data Structures

**ProgressConfig**:
```rust
pub struct ProgressConfig {
    pub quiet_mode: bool,
    pub verbosity: u8,
    pub use_colors: bool,
}
```

**ProgressManager**:
```rust
pub struct ProgressManager {
    multi: MultiProgress,
    config: ProgressConfig,
}
```

**Progress Templates** (constants):
```rust
pub const TEMPLATE_CALL_GRAPH: &str = "🔗 {msg} {pos}/{len} files ({percent}%) - {eta}";
pub const TEMPLATE_FUNCTION_ANALYSIS: &str = "⚙️  {msg} {pos}/{len} functions ({percent}%) - {per_sec}/sec - {eta}";
// ... other templates
```

### APIs and Interfaces

**ProgressManager Public API**:
```rust
impl ProgressManager {
    pub fn new(config: ProgressConfig) -> Self;
    pub fn create_bar(&self, len: u64, template: &str) -> ProgressBar;
    pub fn create_spinner(&self, msg: &str) -> ProgressBar;
    pub fn should_show_progress(&self) -> bool;
}
```

**Builder Function Signature Changes**:
```rust
// Before
pub fn build_call_graph_parallel(
    project_path: &Path,
    base_graph: CallGraph,
    num_threads: Option<usize>,
    show_progress: bool,
) -> Result<...>

// After
pub fn build_call_graph_parallel(
    project_path: &Path,
    base_graph: CallGraph,
    num_threads: Option<usize>,
    progress_manager: &ProgressManager,
) -> Result<...>
```

Similar changes for:
- `perform_unified_analysis_with_options`
- `create_unified_analysis_with_exclusions_and_timing`
- `execute_phase1_parallel`
- `execute_phase2_parallel`
- `execute_phase3_parallel`

## Dependencies

**External Dependencies**:
- `indicatif = { version = "0.17", features = ["rayon"] }` - Main progress library
- `atty = "0.2"` - TTY detection for progress display (likely already in deps)

**Internal Dependencies**:
- No spec dependencies - can be implemented independently
- Touches parallel processing modules (rayon integration)
- Integrates with existing CLI argument parsing

**Affected Components**:
- All parallel analysis builders
- CLI command handling
- Analysis entry points

## Testing Strategy

### Unit Tests

**Progress Configuration Tests** (`tests/progress_config_test.rs`):
```rust
#[test]
fn test_quiet_mode_disables_progress() {
    std::env::set_var("DEBTMAP_QUIET", "1");
    let config = ProgressConfig::from_env();
    assert!(!config.should_show_progress());
}

#[test]
fn test_non_tty_disables_progress() {
    // Mock non-TTY environment
    let manager = ProgressManager::new(ProgressConfig::default());
    let pb = manager.create_bar(100, TEMPLATE_CALL_GRAPH);
    assert!(pb.is_hidden());
}

#[test]
fn test_verbosity_levels() {
    let config = ProgressConfig { verbosity: 0, .. };
    // Test basic progress only

    let config = ProgressConfig { verbosity: 2, .. };
    // Test detailed progress
}
```

**Progress Bar Creation Tests**:
```rust
#[test]
fn test_progress_bar_templates() {
    let manager = ProgressManager::new(ProgressConfig::default());
    let pb = manager.create_bar(1000, TEMPLATE_FUNCTION_ANALYSIS);
    assert!(!pb.is_hidden());
    // Verify template formatting
}

#[test]
fn test_multi_progress_concurrent_bars() {
    let manager = ProgressManager::new(ProgressConfig::default());
    let pb1 = manager.create_bar(100, TEMPLATE_CALL_GRAPH);
    let pb2 = manager.create_bar(200, TEMPLATE_FUNCTION_ANALYSIS);
    // Verify both bars are tracked
}
```

### Integration Tests

**Call Graph Progress Test** (`tests/integration/call_graph_progress.rs`):
```rust
#[test]
fn test_call_graph_shows_progress() {
    // Run analysis on test project
    // Capture stderr output
    // Verify progress messages appear
    // Verify final completion message
}
```

**Phase 2 Function Analysis Test** (`tests/integration/function_analysis_progress.rs`):
```rust
#[test]
fn test_function_analysis_continuous_progress() {
    // Run analysis on medium-sized project
    // Monitor stderr for progress updates
    // Verify no 5+ minute silence gaps
    // Verify ETA appears and decreases
}
```

**Multi-Phase Progress Test**:
```rust
#[test]
fn test_parallel_phases_show_concurrent_progress() {
    // Run full analysis
    // Verify Phase 1, 2, 3 progress all appear
    // Verify progress bars don't overlap or flicker
}
```

### Performance Tests

**Progress Overhead Benchmark** (`benches/progress_overhead.rs`):
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_with_progress(c: &mut Criterion) {
    c.bench_function("function_analysis_with_progress", |b| {
        b.iter(|| {
            // Run Phase 2 with progress enabled
            analyze_functions_with_progress(black_box(&test_data))
        })
    });
}

fn bench_without_progress(c: &mut Criterion) {
    c.bench_function("function_analysis_without_progress", |b| {
        b.iter(|| {
            // Run Phase 2 with quiet mode (no progress)
            analyze_functions_without_progress(black_box(&test_data))
        })
    });
}

// Verify overhead < 2%
```

### User Acceptance Tests

**Manual Testing Checklist**:
- [ ] Run `debtmap analyze .` on small project (< 100 files)
  - Verify smooth progress display
  - Verify all phases show progress
  - Verify completion messages

- [ ] Run `debtmap analyze .` on medium project (300-500 files)
  - Verify no multi-minute silence gaps
  - Verify ETAs are reasonably accurate
  - Verify throughput metrics appear

- [ ] Run `debtmap analyze . --quiet`
  - Verify NO progress output
  - Verify analysis still completes correctly

- [ ] Run `debtmap analyze . -vv`
  - Verify detailed progress and timing info
  - Verify phase timing breakdown appears

- [ ] Run in CI environment (non-TTY)
  - Verify graceful fallback to simple logging
  - Verify no terminal control sequences in logs

## Documentation Requirements

### Code Documentation

**Module Documentation** (`src/progress.rs`):
```rust
//! Progress feedback infrastructure for debtmap analysis.
//!
//! This module provides centralized progress management using the `indicatif` library.
//! Progress bars are shown for all major analysis phases including call graph building,
//! trait resolution, coverage loading, and unified analysis.
//!
//! # Progress Behavior
//!
//! - **Quiet Mode**: No progress output (respects `DEBTMAP_QUIET` env var and `--quiet` flag)
//! - **Non-TTY**: Gracefully disables progress bars in CI and piped output
//! - **Verbosity Levels**:
//!   - Level 0 (default): Main progress bars only
//!   - Level 1 (-v): Sub-phase progress and timing
//!   - Level 2 (-vv): Detailed per-phase metrics
//!
//! # Examples
//!
//! ```rust
//! use debtmap::progress::{ProgressConfig, ProgressManager};
//!
//! let config = ProgressConfig::from_env();
//! let manager = ProgressManager::new(config);
//!
//! let progress = manager.create_bar(total_files as u64, TEMPLATE_CALL_GRAPH);
//! progress.set_message("Building call graph");
//!
//! // Process files...
//! for file in files {
//!     // Work...
//!     progress.inc(1);
//! }
//!
//! progress.finish_with_message("Call graph complete");
//! ```
```

**Function Documentation**:
- Document progress manager creation and configuration
- Explain progress bar template format
- Describe parallel iterator integration
- Document quiet mode and verbosity behavior

### User Documentation

**README.md Update**:
```markdown
## Progress Feedback

Debtmap provides detailed progress feedback during analysis:

- **Call Graph Building**: File-by-file progress with ETA
- **Trait Resolution**: Trait implementation analysis progress
- **Coverage Loading**: LCOV file parsing progress
- **Function Analysis**: Continuous progress with throughput metrics
- **File Analysis**: God object detection progress

### Controlling Progress Output

```bash
# Default: Show progress bars with ETAs
debtmap analyze .

# Quiet mode: No progress output
debtmap analyze . --quiet
DEBTMAP_QUIET=1 debtmap analyze .

# Verbose: Show detailed phase timing
debtmap analyze . -v

# Very verbose: Show all metrics
debtmap analyze . -vv
```

### CI/Non-Interactive Environments

Progress bars are automatically disabled when:
- Output is not a TTY (piped or redirected)
- `--quiet` flag is used
- `DEBTMAP_QUIET` environment variable is set
```

### Architecture Documentation

**ARCHITECTURE.md Update**:
```markdown
## Progress Feedback Architecture

Debtmap uses `indicatif` for professional progress feedback across all analysis phases.

### Design Principles

1. **Centralized Management**: Single `ProgressManager` coordinates all progress bars
2. **Parallel-Friendly**: Uses `ParallelProgressIterator` for rayon integration
3. **Graceful Degradation**: Automatically disables in non-TTY environments
4. **Minimal Overhead**: < 2% performance impact via update throttling

### Progress Phases

1. **Call Graph Building** (`src/builders/parallel_call_graph.rs`)
   - File-level progress: `🔗 Building call graph... X/Y files (NN%)`

2. **Trait Resolution** (`src/builders/unified_analysis.rs`)
   - Trait-level progress: `🔍 Resolving traits... X/Y traits`

3. **Unified Analysis** (`src/builders/parallel_unified_analysis.rs`)
   - Phase 1: Multi-progress for 4 concurrent initialization tasks
   - Phase 2: Function-level progress with throughput metrics
   - Phase 3: File-level progress with god object detection

### Integration Points

Progress managers are created at command entry points and passed down:

```
main.rs
  └─> commands/analyze.rs (create ProgressManager)
       ├─> builders/parallel_call_graph.rs (call graph progress)
       └─> builders/unified_analysis.rs (create progress manager)
            └─> builders/parallel_unified_analysis.rs
                 ├─> Phase 1 progress (multi-spinner)
                 ├─> Phase 2 progress (function analysis bar)
                 └─> Phase 3 progress (file analysis bar)
```
```

## Implementation Notes

### indicatif Best Practices

1. **Update Throttling**: indicatif automatically throttles updates to ~15/sec. Don't manually throttle.

2. **Parallel Progress**: Use `ParallelProgressIterator` instead of manual progress updates:
   ```rust
   // Good - Automatic progress tracking
   use indicatif::ParallelProgressIterator;
   items.par_iter()
       .progress_with(pb)
       .map(|item| process(item))
       .collect()

   // Avoid - Manual tracking in parallel context (slower, more complex)
   items.par_iter()
       .map(|item| {
           let result = process(item);
           pb.inc(1);  // Lock contention!
           result
       })
       .collect()
   ```

3. **Hidden Progress Bars**: Use `ProgressBar::hidden()` for quiet mode rather than conditionals:
   ```rust
   // Good
   let pb = if quiet { ProgressBar::hidden() } else { create_bar(...) };
   pb.inc(1);  // Safe even when hidden

   // Avoid
   if !quiet {
       pb.inc(1);  // Requires conditional everywhere
   }
   ```

4. **MultiProgress Lifecycle**: Keep `MultiProgress` alive for entire operation:
   ```rust
   // Good - MultiProgress lives for entire analysis
   fn analyze(manager: &ProgressManager) {
       let pb1 = manager.create_bar(...);
       let pb2 = manager.create_bar(...);
       // Both bars display correctly
   }

   // Avoid - Short-lived MultiProgress
   fn analyze() {
       let multi = MultiProgress::new();
       let pb1 = multi.add(...);
   }  // multi dropped, bars disappear!
   ```

### Rayon Integration

The `rayon` feature of indicatif provides `ParallelProgressIterator`:

```rust
use indicatif::ParallelProgressIterator;

// Automatic progress for parallel iterators
items.into_par_iter()
    .progress_with(pb)
    .map(|item| process(item))
    .collect()

// Combine with progress_count for automatic length detection
items.into_par_iter()
    .progress_count(items.len() as u64)
    .map(|item| process(item))
    .collect()
```

### Template Gotchas

1. **Wide Bars**: Use `{wide_bar}` instead of `{bar}` to respect terminal width
2. **Per-Second Metrics**: `{per_sec}` requires time tracking - works automatically with indicatif
3. **ETA Accuracy**: ETAs become accurate after ~5% completion (let indicatif warm up)
4. **Template Validation**: `template()` returns `Result` - always `.expect()` with descriptive message

### Performance Considerations

1. **Lock-Free Increments**: indicatif uses atomic counters for `inc()` - no mutex contention
2. **Update Throttling**: Built-in 60ms throttle prevents excessive terminal writes
3. **Parallel Safety**: `ProgressBar` is `Clone` + `Send` + `Sync` - safe to share across threads
4. **Hidden Bar Overhead**: `ProgressBar::hidden()` has near-zero overhead (atomic check only)

### CI/Non-TTY Detection

Use `atty` crate for reliable TTY detection:

```rust
use atty::Stream;

fn should_show_progress() -> bool {
    !quiet_mode && atty::is(Stream::Stderr)
}
```

This automatically handles:
- Piped output: `debtmap analyze . | tee log.txt`
- Redirected output: `debtmap analyze . 2> progress.log`
- CI environments: GitHub Actions, GitLab CI, etc.

## Migration and Compatibility

### Breaking Changes

**Function Signatures**:
- `build_call_graph_parallel()` - `show_progress: bool` → `progress_manager: &ProgressManager`
- `perform_unified_analysis_with_options()` - Add `progress_manager: &ProgressManager` parameter

These are internal API changes - no user-facing breaking changes.

### Migration Strategy

1. **Phase 1**: Add indicatif dependency and progress module
2. **Phase 2**: Convert call graph progress (lowest risk)
3. **Phase 3**: Add Phase 2 function analysis progress (highest impact)
4. **Phase 4**: Add Phase 1 and Phase 3 progress
5. **Phase 5**: Add trait resolution and coverage progress
6. **Phase 6**: Remove old custom progress code

### Backward Compatibility

**Environment Variables**: Maintain existing behavior:
- `DEBTMAP_QUIET=1` - Disables all progress (existing)
- `DEBTMAP_PARALLEL=1` - Enables parallel mode (existing)

**CLI Flags**: Maintain existing flags:
- `--quiet` - Disables progress (existing)
- `-v`, `-vv` - Verbosity levels (existing)

**Output Format**: Progress bars write to stderr, analysis output to stdout (unchanged)

### Rollback Plan

If issues arise, can disable indicatif progress with:
```rust
// Temporary rollback - return hidden progress bars
pub fn create_bar(&self, len: u64, template: &str) -> ProgressBar {
    ProgressBar::hidden()  // Disable all progress temporarily
}
```

This allows quick rollback without removing indicatif code.

## Success Metrics

**User Experience Metrics**:
- Zero reports of "analysis appears hung" after 100% call graph
- Positive user feedback on progress visibility
- Reduction in GitHub issues related to progress feedback

**Performance Metrics**:
- Progress overhead < 2% as measured by benchmarks
- No increase in memory usage (indicatif is lightweight)
- No degradation in parallel performance

**Code Quality Metrics**:
- Reduced LOC for progress management (centralized vs scattered)
- Improved consistency (single template format)
- Better testability (mocked progress for tests)

## Future Enhancements

**Potential Future Work** (not in scope for this spec):
1. **Progress Persistence**: Save/restore progress for interrupted analyses
2. **Web UI**: Expose progress via WebSocket for browser-based monitoring
3. **Structured Logging**: JSON progress events for programmatic consumption
4. **Custom Templates**: User-configurable progress bar templates
5. **Progress Hooks**: Callbacks for progress milestones (25%, 50%, 75%, 100%)
