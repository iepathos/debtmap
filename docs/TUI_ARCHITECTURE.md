# TUI Architecture Documentation

## Overview

Debtmap's Terminal User Interface (TUI) provides real-time visualization of the analysis pipeline with a zen minimalist design. This document explains the data flow, rendering loop, and how to extend the TUI with new pipeline stages.

## Design Philosophy

The TUI follows the **zen minimalist** principle:
- **Single-screen view**: All 9 pipeline stages visible at once
- **Hierarchical expansion**: Active stages show sub-tasks with progress
- **Rich context without clutter**: Metrics, percentages, and timing
- **60 FPS smooth animations**: Progress bars and sliding indicators
- **Responsive layout**: Adapts gracefully to terminal size

## Architecture Components

### 1. App State (`src/tui/app.rs`)

The `App` struct holds all TUI state:

```rust
pub struct App {
    pub stages: Vec<PipelineStage>,      // 9 pipeline stages
    pub overall_progress: f64,            // 0.0 to 1.0
    pub current_stage: usize,             // Active stage index
    pub elapsed_time: Duration,           // Total time elapsed
    pub animation_frame: usize,           // 0-59 for 60 FPS

    // Live statistics
    pub functions_count: usize,
    pub debt_count: usize,
    pub coverage_percent: f64,
    pub thread_count: usize,
}
```

**Key Methods:**
- `start_stage(index)` - Mark stage as active
- `complete_stage(index, metric)` - Mark stage complete with summary
- `update_stage_metric(index, metric)` - Update stage metric while running
- `update_subtask(stage, subtask, status, progress)` - Update sub-task state
- `set_overall_progress(progress)` - Update overall pipeline progress
- `tick()` - Advance animation frame (call at 60 FPS)

### 2. TUI Manager (`src/tui/mod.rs`)

The `TuiManager` handles terminal setup and rendering:

```rust
pub struct TuiManager {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: App,
}
```

**Lifecycle:**
1. `new()` - Initialize terminal (raw mode + alternate screen)
2. `render()` - Draw a single frame
3. `cleanup()` - Restore terminal (called automatically on drop)

### 3. Progress Manager Integration (`src/progress.rs`)

The `ProgressManager` bridges pipeline execution to TUI state:

```rust
impl ProgressManager {
    pub fn tui_start_stage(&self, stage_index: usize);
    pub fn tui_complete_stage(&self, stage_index: usize, metric: String);
    pub fn tui_update_metric(&self, stage_index: usize, metric: String);
    pub fn tui_update_subtask(&self, stage_index, subtask_index, status, progress);
    pub fn tui_set_progress(&self, progress: f64);
    pub fn tui_update_stats(&self, functions, debt, coverage, threads);
    pub fn tui_render(&self);
    pub fn tui_cleanup(&self);
}
```

## Data Flow

### Pipeline → TUI Updates

```
┌─────────────────────────────────────────────────────────────────┐
│                      Pipeline Execution                         │
│  (analyze.rs, unified_analysis.rs)                              │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ ProgressManager::tui_*() calls
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│                     ProgressManager                              │
│  • Locks TuiManager                                              │
│  • Updates App state                                             │
│  • Calls render()                                                │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ app_mut() access
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│                        App State                                 │
│  • Updates stage status                                          │
│  • Updates metrics                                               │
│  • Advances animation frame                                      │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         │ render()
                         ↓
┌─────────────────────────────────────────────────────────────────┐
│                      Renderer                                    │
│  • Builds ratatui widgets                                        │
│  • Draws to terminal                                             │
└─────────────────────────────────────────────────────────────────┘
```

### Example: Stage Completion

When a pipeline stage completes:

```rust
// In unified_analysis.rs (or any pipeline code):
if let Some(manager) = ProgressManager::global() {
    manager.tui_complete_stage(2, format!("{} functions", count));
    manager.tui_set_progress(0.33); // ~3/9 stages complete
}
```

This:
1. Locks the `TuiManager` mutex
2. Calls `app.complete_stage(2, metric)`
3. Calls `app.set_overall_progress(0.33)`
4. Renders the updated frame via `tui.render()`

## Rendering Loop

### Frame Rate: 60 FPS

The TUI targets 60 FPS for smooth animations:

```rust
pub fn tick(&mut self) {
    self.elapsed_time = self.start_time.elapsed();
    self.animation_frame = (self.animation_frame + 1) % 60;
    self.last_update = Instant::now();
}
```

**Rendering Trigger:**
- Every TUI update method calls `render()` automatically
- This ensures the display is updated whenever state changes
- No explicit render loop needed - driven by pipeline progress

### Rendering Performance

Target: **<16ms per frame** (60 FPS = ~16.67ms per frame)

The rendering is fast because:
- Pure functional rendering (no state mutation in render code)
- Ratatui's efficient diffing algorithm
- Minimal widget tree depth
- Static layout computation

## Pipeline Stages

### Current 9-Stage Pipeline

The TUI displays these stages in order:

| Index | Name             | Description                              |
|-------|------------------|------------------------------------------|
| 0     | files            | File discovery                           |
| 1     | parse            | AST parsing                              |
| 2     | call graph       | Function call graph construction         |
| 3     | trait resolution | Rust trait method resolution             |
| 4     | coverage         | Test coverage data loading               |
| 5     | purity analysis  | Function purity and side effect analysis |
| 6     | context          | Project context loading                  |
| 7     | debt scoring     | Technical debt calculation               |
| 8     | prioritization   | Final prioritization and ranking         |

### Stage with Sub-Tasks: Purity Analysis

The purity analysis stage (index 5) has hierarchical sub-tasks:

```rust
PipelineStage::with_subtasks(
    "purity analysis",
    vec![
        SubTask { name: "data flow graph", .. },
        SubTask { name: "initial detection", .. },
        SubTask { name: "propagation", .. },
        SubTask { name: "side effects", .. },
    ],
)
```

Sub-tasks can show:
- Status (Pending, Active, Completed)
- Optional progress (current/total)

### Stage with Sub-Tasks: Coverage

The coverage stage (index 4) demonstrates the progress callback pattern with hierarchical sub-tasks:

```rust
PipelineStage::with_subtasks(
    "coverage",
    vec![
        SubTask { name: "load data", .. },
        SubTask { name: "parse coverage", .. },
        SubTask { name: "map to functions", .. },
    ],
)
```

**Before Display (when stage starts):**
```
coverage                     [..................] 0%
  • load data               [..................] pending
  • parse coverage          [..................] pending
  • map to functions        [..................] pending
```

**During Execution (active sub-task with progress):**
```
coverage                     [█████.............] 33%
  ✓ load data               [██████████████████] complete
  ➤ parse coverage          [████████..........] 45% (450/1000 lines)
  • map to functions        [..................] pending
```

**After Completion (all sub-tasks done):**
```
✓ coverage                  [██████████████████] 100% - 1000 lines, 85% covered
  ✓ load data               [██████████████████] complete
  ✓ parse coverage          [██████████████████] complete
  ✓ map to functions        [██████████████████] complete
```

**Implementation Pattern:**

This demonstrates the canonical pattern for progress callbacks with sub-tasks:

```rust
// In unified_analysis.rs or coverage.rs:

// 1. Start the stage
if let Some(manager) = ProgressManager::global() {
    manager.tui_start_stage(4);
}

// 2. Execute first sub-task
if let Some(manager) = ProgressManager::global() {
    manager.tui_update_subtask(4, 0, StageStatus::Active, None);
}
let data = load_coverage_data()?;
if let Some(manager) = ProgressManager::global() {
    manager.tui_update_subtask(4, 0, StageStatus::Completed, None);
}

// 3. Execute second sub-task with progress tracking
if let Some(manager) = ProgressManager::global() {
    manager.tui_update_subtask(4, 1, StageStatus::Active, None);
}

let total_lines = data.line_count();
for (idx, line) in data.lines().enumerate() {
    parse_coverage_line(line)?;

    // Update progress every N iterations to avoid excessive rendering
    if idx % 100 == 0 {
        if let Some(manager) = ProgressManager::global() {
            manager.tui_update_subtask(
                4,
                1,
                StageStatus::Active,
                Some((idx, total_lines))
            );
        }
    }
}

if let Some(manager) = ProgressManager::global() {
    manager.tui_update_subtask(4, 1, StageStatus::Completed, None);
}

// 4. Execute final sub-task
if let Some(manager) = ProgressManager::global() {
    manager.tui_update_subtask(4, 2, StageStatus::Active, None);
}
let coverage_map = map_coverage_to_functions(&data)?;
if let Some(manager) = ProgressManager::global() {
    manager.tui_update_subtask(4, 2, StageStatus::Completed, None);
}

// 5. Complete the stage with summary metric
if let Some(manager) = ProgressManager::global() {
    let summary = format!(
        "{} lines, {}% covered",
        total_lines,
        coverage_map.coverage_percent()
    );
    manager.tui_complete_stage(4, summary);
    manager.tui_set_progress(0.55); // ~5/9 stages complete
}
```

**Key Principles:**

1. **Sub-task granularity**: Each sub-task represents a distinct phase with clear start/end
2. **Progress tracking**: Use `Some((current, total))` for long-running sub-tasks
3. **Update frequency**: Balance responsiveness with rendering overhead (update every N iterations)
4. **Completion metrics**: Provide meaningful summary when stage completes
5. **Overall progress**: Update global progress bar to reflect pipeline advancement

This pattern ensures users get rich, real-time feedback about long-running operations while maintaining clean separation between business logic and UI concerns.

## Adding a New Pipeline Stage

To add a new stage to the TUI:

### Step 1: Update App Initialization

Edit `src/tui/app.rs` in `create_default_stages()`:

```rust
fn create_default_stages() -> Vec<PipelineStage> {
    vec![
        PipelineStage::new("files"),
        PipelineStage::new("parse"),
        // ... existing stages ...
        PipelineStage::new("your new stage"),  // Add here
    ]
}
```

### Step 2: Update Pipeline Integration

In your pipeline code (e.g., `unified_analysis.rs`):

```rust
// Start the new stage
if let Some(manager) = ProgressManager::global() {
    manager.tui_start_stage(9); // Use correct index
}

// Your stage logic here...
let result = perform_new_stage_work();

// Complete the stage
if let Some(manager) = ProgressManager::global() {
    manager.tui_complete_stage(9, format!("{} items", result.count));
    manager.tui_set_progress(0.95); // Adjust progress percentage
}
```

### Step 3: Update Progress Calculations

Update overall progress percentages throughout the pipeline to account for the new stage:

```rust
// Old: 9 stages, each is ~11% (0.11, 0.22, 0.33, ...)
// New: 10 stages, each is ~10% (0.10, 0.20, 0.30, ...)
manager.tui_set_progress(0.10 * (stage_index + 1) as f64);
```

### Step 4: Add Tests

Add a test case in `tests/tui_integration_test.rs`:

```rust
#[test]
fn test_new_stage_updates() {
    let mut app = App::new();

    // Verify new stage exists
    assert_eq!(app.stages[9].name, "your new stage");

    // Test stage lifecycle
    app.start_stage(9);
    assert_eq!(app.stages[9].status, StageStatus::Active);

    app.complete_stage(9, "completed");
    assert_eq!(app.stages[9].status, StageStatus::Completed);
}
```

## Adding Sub-Tasks to an Existing Stage

To add hierarchical sub-tasks to a stage:

### Step 1: Define Sub-Tasks

```rust
PipelineStage::with_subtasks(
    "your stage name",
    vec![
        SubTask {
            name: "subtask 1".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
        SubTask {
            name: "subtask 2".to_string(),
            status: StageStatus::Pending,
            progress: None,
        },
    ],
)
```

### Step 2: Update Sub-Tasks During Execution

```rust
// Start subtask
if let Some(manager) = ProgressManager::global() {
    manager.tui_update_subtask(
        stage_index,
        subtask_index,
        StageStatus::Active,
        Some((current, total)),
    );
}

// Complete subtask
if let Some(manager) = ProgressManager::global() {
    manager.tui_update_subtask(
        stage_index,
        subtask_index,
        StageStatus::Completed,
        None,
    );
}
```

## TTY Detection and Fallback

The TUI automatically detects non-interactive environments:

```rust
pub fn should_show_progress(&self) -> bool {
    if self.quiet_mode {
        return false;
    }

    // Automatic TTY detection
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
}
```

**Behavior:**
- **TTY detected**: Full TUI with animations
- **Non-TTY (CI/pipes)**: Hidden progress bars, clean output
- **Quiet mode**: No progress output at all

This ensures debtmap works seamlessly in:
- Interactive terminals (full TUI)
- CI/CD pipelines (no TUI, clean logs)
- Piped output (no TUI interference)

## Layout and Rendering

### Adaptive Layout (`src/tui/layout.rs`)

The layout adapts to terminal size:

```rust
pub fn render_adaptive(frame: &mut Frame, app: &App) {
    let size = frame.size();

    if size.height < MIN_HEIGHT {
        render_compact(frame, app);
    } else {
        render_full(frame, app);
    }
}
```

**Minimum terminal size:** 24 rows × 80 columns

### Theme and Colors (`src/tui/theme.rs`)

Colors are defined in a central theme module:

```rust
pub struct Theme {
    pub primary: Color,
    pub success: Color,
    pub active: Color,
    pub pending: Color,
    pub accent: Color,
}
```

This enables easy theme customization in the future.

## Performance Considerations

### Rendering Performance

- **Target:** <16ms per frame (60 FPS)
- **Measurement:** Use `criterion` benchmarks in `benches/tui_render_bench.rs`
- **Optimization:** Minimize widget tree depth, use static layouts

### Memory Usage

- **App state:** <1KB (9 stages + stats)
- **Terminal buffer:** Proportional to terminal size
- **Total overhead:** <10KB typically

### Thread Safety

The TUI is accessed through `Arc<Mutex<Option<TuiManager>>>`:
- Thread-safe updates from pipeline
- Locks are held briefly during renders
- No contention under normal usage

## Testing Strategy

### Unit Tests

Located in `src/tui/app.rs`:
- Stage lifecycle
- Progress clamping
- Subtask updates
- Animation ticking

### Integration Tests

Located in `tests/tui_integration_test.rs`:
- Full pipeline simulation
- TUI manager lifecycle
- TTY detection
- Progress manager integration

### Manual Testing

```bash
# Test with real analysis
cargo run -- analyze . -v

# Test TTY detection (should disable TUI)
cargo run -- analyze . | cat

# Test quiet mode
cargo run -- analyze . --quiet
```

## Troubleshooting

### TUI Not Showing

**Check:**
1. Is TTY detected? Run `cargo run -- analyze .` (not piped)
2. Is quiet mode enabled? Remove `--quiet` or `DEBTMAP_QUIET` env var
3. Terminal size sufficient? Minimum 24×80

### Rendering Issues

**Common causes:**
1. Terminal emulator incompatibility - Test with standard terminals
2. Color support missing - Check `$TERM` environment variable
3. Performance issues - Profile render loop with `cargo flamegraph`

### Animation Stuttering

**Causes:**
1. Slow terminal emulator
2. Heavy CPU load from analysis
3. Frequent state updates

**Solutions:**
- Reduce update frequency in hot loops
- Batch state updates where possible
- Profile to identify bottlenecks

## Future Enhancements

Potential improvements to the TUI:

1. **User interaction:** Keyboard controls to pause/skip stages
2. **Detailed views:** Press key to see detailed stage info
3. **Color themes:** Multiple theme options (dark, light, high-contrast)
4. **Charts:** Real-time graphs of complexity distribution
5. **Performance view:** Live CPU/memory usage graphs
6. **Log viewer:** Integrated log display panel

## References

- **Ratatui documentation:** https://ratatui.rs/
- **Crossterm documentation:** https://docs.rs/crossterm/
- **TUI design principles:** https://github.com/ratatui-org/ratatui/blob/main/DESIGN.md

## Summary

The TUI architecture is:
- **Functional:** Pure rendering, immutable state updates
- **Composable:** Easy to add new stages or sub-tasks
- **Performant:** 60 FPS with minimal overhead
- **Robust:** Automatic fallback for non-interactive environments
- **Maintainable:** Clear separation of state, rendering, and pipeline integration

To extend the TUI, follow the patterns established in the existing code and add corresponding tests to ensure correctness.
