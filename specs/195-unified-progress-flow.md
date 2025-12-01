---
number: 195
title: Unified Progress Flow Display
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-11-30
---

# Specification 195: Unified Progress Flow Display

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

Debtmap's current progress output during analysis is confusing and repetitive:

```
\ Found 511 files to analyze
Analyzed 511 files 511/511 files (100%) - 0s
Call graph complete 511/511 files (100%) - 0s
Analyzed 511 files, 148769 unresolved calls (0s) 511/511 files (100%) - 0s
Resolved 48768/148769 calls (2s) 148769/148769 calls (100%) - 0s
```

**Problems identified**:

1. **Repetitive information** - "511/511 files (100%)" appears 4 times
2. **Unclear phases** - What's the difference between "Analyzed 511 files" appearing twice?
3. **Inconsistent format** - Each line uses different format patterns
4. **No phase awareness** - Users don't understand what's happening or how many steps remain
5. **Missing time estimates** - No indication of total progress or time remaining
6. **Visual noise** - Spinner character (`\`) not clearly separated from content

This creates a poor first impression and makes it difficult for users to understand:
- What analysis phase is currently running
- How many phases remain
- Whether the tool is stuck or working normally
- How long large codebases will take to analyze

## Objective

Create a unified progress flow display that:

1. **Shows clear analysis phases** with numbered steps (1/4, 2/4, etc.)
2. **Uses consistent formatting** across all progress indicators
3. **Provides phase context** so users understand what's happening
4. **Eliminates repetition** of completion percentages and counts
5. **Includes time tracking** for duration and estimates
6. **Maintains clean visual hierarchy** with clear separation

**Success Metric**: Users can glance at progress and immediately understand current phase, progress, and time elapsed.

## Requirements

### Functional Requirements

1. **Phase-Based Progress Display**
   - Display analysis as numbered phases: "1/4", "2/4", "3/4", "4/4"
   - Each phase has clear name: "Discovering files", "Analyzing complexity", etc.
   - Show phase-specific metrics (file count, call count, etc.)
   - Update progress within each phase

2. **Consistent Line Format**
   - Standard format: `{phase} {name}... {metrics} {duration}`
   - Example: `1/4 Discovering files...        511 found`
   - Align metrics in columns for visual consistency
   - Include duration for completed phases

3. **Analysis Phases**
   - **Phase 1**: Discovering files (find all source files matching criteria)
   - **Phase 2**: Analyzing complexity (parse and analyze each file)
   - **Phase 3**: Building call graph (construct cross-file dependencies)
   - **Phase 4**: Resolving dependencies (resolve function calls and imports)

4. **Progress Indicators**
   - Show count/total for countable operations: "511/511 files"
   - Show percentage for long-running operations: "(100%)"
   - Include spinner for active phases
   - Clear completion indicator for finished phases

5. **Time Tracking**
   - Show elapsed time for each phase: "2s", "15s", etc.
   - Show total elapsed time at end
   - Provide time estimates for known phases (optional enhancement)

### Non-Functional Requirements

1. **Performance**
   - Progress updates should not slow down analysis
   - Limit update frequency to reasonable rate (e.g., 10 updates/sec max)
   - Minimize allocations in hot loop

2. **Terminal Compatibility**
   - Work in standard terminals without special features
   - Degrade gracefully in non-interactive environments (CI/CD)
   - Support both ANSI color terminals and plain text

3. **Clarity**
   - Phase names should be self-explanatory
   - Metrics should be immediately understandable
   - Visual hierarchy should guide eye naturally

4. **Consistency**
   - Same format used for all phases
   - Predictable information placement
   - Uniform spacing and alignment

## Acceptance Criteria

- [ ] Progress display shows numbered phases (1/4, 2/4, 3/4, 4/4)
- [ ] Phase names clearly describe what's happening
- [ ] File discovery phase shows count of files found
- [ ] Complexity analysis phase shows count/total with percentage
- [ ] Call graph phase shows count/total with percentage
- [ ] Dependency resolution phase shows count/total with percentage
- [ ] Each completed phase shows duration in seconds
- [ ] Progress lines use consistent format across all phases
- [ ] No repetitive information (percentages shown once per phase)
- [ ] Spinner indicates active work in progress
- [ ] Completion checkmark (✓) indicates finished phases
- [ ] Total analysis time shown at end
- [ ] Progress output works in CI/CD (non-interactive) environments
- [ ] Visual alignment consistent across all phases
- [ ] Documentation updated with progress output examples
- [ ] User testing confirms improved clarity vs. current output

## Technical Details

### Implementation Approach

**Phase 1: Create Progress Tracker**

```rust
// src/io/progress.rs (new module)

use std::time::{Duration, Instant};
use indicatif::{ProgressBar, ProgressStyle};

pub struct AnalysisProgress {
    phases: Vec<AnalysisPhase>,
    current_phase: usize,
    start_time: Instant,
}

#[derive(Debug)]
struct AnalysisPhase {
    name: &'static str,
    status: PhaseStatus,
    start_time: Option<Instant>,
    duration: Option<Duration>,
    progress: PhaseProgress,
}

#[derive(Debug)]
enum PhaseStatus {
    Pending,
    InProgress,
    Complete,
}

#[derive(Debug)]
enum PhaseProgress {
    Indeterminate,
    Count(usize), // Just a count (e.g., files found)
    Progress { current: usize, total: usize }, // Current/Total
}

impl AnalysisProgress {
    pub fn new() -> Self {
        Self {
            phases: vec![
                AnalysisPhase::new("Discovering files", PhaseProgress::Indeterminate),
                AnalysisPhase::new("Analyzing complexity", PhaseProgress::Indeterminate),
                AnalysisPhase::new("Building call graph", PhaseProgress::Indeterminate),
                AnalysisPhase::new("Resolving dependencies", PhaseProgress::Indeterminate),
            ],
            current_phase: 0,
            start_time: Instant::now(),
        }
    }

    pub fn start_phase(&mut self, phase_index: usize) {
        self.current_phase = phase_index;
        self.phases[phase_index].status = PhaseStatus::InProgress;
        self.phases[phase_index].start_time = Some(Instant::now());
        self.render();
    }

    pub fn update_progress(&mut self, progress: PhaseProgress) {
        self.phases[self.current_phase].progress = progress;
        self.render();
    }

    pub fn complete_phase(&mut self) {
        let phase = &mut self.phases[self.current_phase];
        phase.status = PhaseStatus::Complete;
        if let Some(start) = phase.start_time {
            phase.duration = Some(start.elapsed());
        }
        self.render();
    }

    pub fn finish(&self) {
        let total_duration = self.start_time.elapsed();
        eprintln!("\nAnalysis complete in {:.1}s", total_duration.as_secs_f64());
    }

    fn render(&self) {
        // Render current progress state
        let phase = &self.phases[self.current_phase];
        let phase_num = self.current_phase + 1;
        let total_phases = self.phases.len();

        let indicator = match phase.status {
            PhaseStatus::InProgress => "→",
            PhaseStatus::Complete => "✓",
            PhaseStatus::Pending => " ",
        };

        let progress_str = match &phase.progress {
            PhaseProgress::Indeterminate => String::new(),
            PhaseProgress::Count(n) => format!("{} found", n),
            PhaseProgress::Progress { current, total } => {
                let pct = (*current as f64 / *total as f64 * 100.0) as usize;
                format!("{}/{} ({}%)", current, total, pct)
            }
        };

        let duration_str = phase.duration
            .map(|d| format!(" - {}s", d.as_secs()))
            .unwrap_or_default();

        eprint!(
            "\r{} {}/{} {}...{:30}{}",
            indicator,
            phase_num,
            total_phases,
            phase.name,
            progress_str,
            duration_str
        );

        // Move to next line when phase completes
        if matches!(phase.status, PhaseStatus::Complete) {
            eprintln!();
        }
    }
}

impl AnalysisPhase {
    fn new(name: &'static str, progress: PhaseProgress) -> Self {
        Self {
            name,
            status: PhaseStatus::Pending,
            start_time: None,
            duration: None,
            progress,
        }
    }
}
```

**Phase 2: Integrate into Analysis Pipeline**

```rust
// src/commands/analyze.rs

pub async fn analyze_command(args: AnalyzeArgs) -> Result<()> {
    let mut progress = AnalysisProgress::new();

    // Phase 1: Discover files
    progress.start_phase(0);
    let files = discover_files(&args.path)?;
    progress.update_progress(PhaseProgress::Count(files.len()));
    progress.complete_phase();

    // Phase 2: Analyze complexity
    progress.start_phase(1);
    let mut results = Vec::new();
    for (idx, file) in files.iter().enumerate() {
        let result = analyze_file(file)?;
        results.push(result);
        progress.update_progress(PhaseProgress::Progress {
            current: idx + 1,
            total: files.len(),
        });
    }
    progress.complete_phase();

    // Phase 3: Build call graph
    progress.start_phase(2);
    let call_graph = build_call_graph(&results, &mut progress)?;
    progress.complete_phase();

    // Phase 4: Resolve dependencies
    progress.start_phase(3);
    resolve_dependencies(&mut call_graph, &mut progress)?;
    progress.complete_phase();

    progress.finish();

    // Display results
    // ...

    Ok(())
}
```

**Phase 3: Handle CI/CD Environments**

```rust
// src/io/progress.rs

impl AnalysisProgress {
    pub fn new() -> Self {
        let is_interactive = atty::is(atty::Stream::Stderr);

        Self {
            phases: create_phases(),
            current_phase: 0,
            start_time: Instant::now(),
            interactive: is_interactive,
        }
    }

    fn render(&self) {
        if !self.interactive {
            // In CI/CD, print complete lines instead of overwriting
            let phase = &self.phases[self.current_phase];
            let phase_num = self.current_phase + 1;
            let total_phases = self.phases.len();

            match phase.status {
                PhaseStatus::Complete => {
                    let duration = phase.duration.unwrap();
                    eprintln!(
                        "✓ {}/{} {} - {}s",
                        phase_num,
                        total_phases,
                        phase.name,
                        duration.as_secs()
                    );
                }
                PhaseStatus::InProgress => {
                    // Only print on start, not every update
                    if let Some(start) = phase.start_time {
                        if start.elapsed().as_millis() < 100 {
                            eprintln!("→ {}/{} {}...", phase_num, total_phases, phase.name);
                        }
                    }
                }
                _ => {}
            }
        } else {
            // Interactive mode: use carriage return to overwrite line
            // ... existing render logic ...
        }
    }
}
```

### Example Output

**Interactive Terminal**:
```
→ 1/4 Discovering files...        511 found
✓ 1/4 Discovering files...        511 found - 0s
→ 2/4 Analyzing complexity...     511/511 (100%) - 2s
✓ 2/4 Analyzing complexity...     511/511 (100%) - 2s
→ 3/4 Building call graph...      511/511 (100%) - 1s
✓ 3/4 Building call graph...      511/511 (100%) - 1s
→ 4/4 Resolving dependencies...   148769/148769 (100%) - 3s
✓ 4/4 Resolving dependencies...   148769/148769 (100%) - 3s

Analysis complete in 6.2s
```

**CI/CD (Non-Interactive)**:
```
→ 1/4 Discovering files...
✓ 1/4 Discovering files - 0s
→ 2/4 Analyzing complexity...
✓ 2/4 Analyzing complexity - 2s
→ 3/4 Building call graph...
✓ 3/4 Building call graph - 1s
→ 4/4 Resolving dependencies...
✓ 4/4 Resolving dependencies - 3s

Analysis complete in 6.2s
```

### Architecture Changes

New modules:
- `src/io/progress.rs` - Progress tracking and rendering

Modified files:
- `src/commands/analyze.rs` - Integrate progress tracking
- `src/analyzers/mod.rs` - Report progress during analysis
- `src/analysis/call_graph.rs` - Report progress during graph building

### Data Structures

```rust
// src/io/progress.rs

pub struct AnalysisProgress {
    phases: Vec<AnalysisPhase>,
    current_phase: usize,
    start_time: Instant,
    interactive: bool,
}

struct AnalysisPhase {
    name: &'static str,
    status: PhaseStatus,
    start_time: Option<Instant>,
    duration: Option<Duration>,
    progress: PhaseProgress,
}

enum PhaseStatus {
    Pending,
    InProgress,
    Complete,
}

enum PhaseProgress {
    Indeterminate,
    Count(usize),
    Progress { current: usize, total: usize },
}
```

### APIs and Interfaces

```rust
// src/io/progress.rs

impl AnalysisProgress {
    /// Create new progress tracker
    pub fn new() -> Self;

    /// Start a specific phase (0-indexed)
    pub fn start_phase(&mut self, phase_index: usize);

    /// Update progress for current phase
    pub fn update_progress(&mut self, progress: PhaseProgress);

    /// Mark current phase as complete
    pub fn complete_phase(&mut self);

    /// Finish all analysis and show total time
    pub fn finish(&self);
}

impl PhaseProgress {
    /// Create indeterminate progress (unknown total)
    pub fn indeterminate() -> Self;

    /// Create count-only progress (no total, just count)
    pub fn count(n: usize) -> Self;

    /// Create fractional progress (current/total)
    pub fn progress(current: usize, total: usize) -> Self;
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/commands/analyze.rs` - Main analysis command
  - `src/analyzers/mod.rs` - File analysis loop
  - `src/analysis/call_graph.rs` - Call graph construction
  - All analyzer modules that report progress
- **External Dependencies**:
  - `atty` - Detect interactive terminal (already in dependencies)
  - Consider `indicatif` for advanced progress bars (optional)

## Testing Strategy

### Unit Tests

```rust
// src/io/progress.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_lifecycle() {
        let mut progress = AnalysisProgress::new();

        progress.start_phase(0);
        assert_eq!(progress.current_phase, 0);
        assert!(matches!(
            progress.phases[0].status,
            PhaseStatus::InProgress
        ));

        progress.update_progress(PhaseProgress::Count(100));
        assert!(matches!(
            progress.phases[0].progress,
            PhaseProgress::Count(100)
        ));

        progress.complete_phase();
        assert!(matches!(
            progress.phases[0].status,
            PhaseStatus::Complete
        ));
        assert!(progress.phases[0].duration.is_some());
    }

    #[test]
    fn test_format_phase_progress_count() {
        let progress = PhaseProgress::Count(511);
        let formatted = format_progress(&progress);
        assert_eq!(formatted, "511 found");
    }

    #[test]
    fn test_format_phase_progress_fractional() {
        let progress = PhaseProgress::Progress {
            current: 256,
            total: 511,
        };
        let formatted = format_progress(&progress);
        assert_eq!(formatted, "256/511 (50%)");
    }

    #[test]
    fn test_duration_formatting() {
        let duration = Duration::from_secs(125);
        let formatted = format_duration(&duration);
        assert_eq!(formatted, "125s");
    }
}
```

### Integration Tests

```rust
// tests/progress_display_test.rs

#[test]
fn test_progress_in_real_analysis() {
    let temp_dir = create_temp_codebase(100); // 100 test files

    let output = run_debtmap_analyze(&temp_dir).unwrap();

    // Verify all phases shown
    assert!(output.contains("1/4 Discovering files"));
    assert!(output.contains("2/4 Analyzing complexity"));
    assert!(output.contains("3/4 Building call graph"));
    assert!(output.contains("4/4 Resolving dependencies"));

    // Verify completion indicators
    assert!(output.contains("✓ 1/4"));
    assert!(output.contains("✓ 2/4"));
    assert!(output.contains("✓ 3/4"));
    assert!(output.contains("✓ 4/4"));

    // Verify total time shown
    assert!(output.contains("Analysis complete in"));
}

#[test]
fn test_progress_in_ci_environment() {
    env::set_var("CI", "true");

    let output = run_debtmap_analyze_non_interactive().unwrap();

    // Verify newlines instead of carriage returns
    assert!(!output.contains('\r'));
    assert!(output.lines().count() >= 8); // At least 2 lines per phase

    env::remove_var("CI");
}
```

## Documentation Requirements

### Code Documentation

```rust
/// Progress tracker for multi-phase analysis operations.
///
/// Displays progress through 4 analysis phases:
/// 1. File discovery
/// 2. Complexity analysis
/// 3. Call graph construction
/// 4. Dependency resolution
///
/// Adapts output for interactive terminals (overwriting lines)
/// vs CI/CD environments (new line per update).
///
/// # Example
/// ```
/// let mut progress = AnalysisProgress::new();
///
/// progress.start_phase(0);
/// progress.update_progress(PhaseProgress::Count(511));
/// progress.complete_phase();
///
/// progress.start_phase(1);
/// for i in 0..511 {
///     progress.update_progress(PhaseProgress::progress(i + 1, 511));
/// }
/// progress.complete_phase();
///
/// progress.finish();
/// ```
pub struct AnalysisProgress { ... }
```

### User Documentation

Update README.md:

```markdown
## Analysis Progress

Debtmap shows clear progress through 4 analysis phases:

```
→ 1/4 Discovering files...        511 found
✓ 1/4 Discovering files...        511 found - 0s
→ 2/4 Analyzing complexity...     511/511 (100%) - 2s
✓ 2/4 Analyzing complexity...     511/511 (100%) - 2s
→ 3/4 Building call graph...      511/511 (100%) - 1s
✓ 3/4 Building call graph...      511/511 (100%) - 1s
→ 4/4 Resolving dependencies...   148769/148769 (100%) - 3s
✓ 4/4 Resolving dependencies...   148769/148769 (100%) - 3s

Analysis complete in 6.2s
```

### Phase Descriptions

- **1/4 Discovering files**: Scans directories for source files matching language filters
- **2/4 Analyzing complexity**: Parses and analyzes each file for metrics and patterns
- **3/4 Building call graph**: Constructs relationships between functions and modules
- **4/4 Resolving dependencies**: Resolves cross-file function calls and imports

Progress adapts to environment:
- **Interactive terminals**: Updates in-place with carriage returns
- **CI/CD environments**: Prints new lines for each update
```

## Implementation Notes

### Implementation Order

1. **Create progress module** with AnalysisProgress struct
2. **Implement phase tracking** and state management
3. **Add rendering logic** for both interactive and non-interactive modes
4. **Integrate into analyze command** at each phase
5. **Add progress updates** during long-running operations
6. **Test in both environments** (interactive terminal and CI/CD)
7. **Update documentation**

### Edge Cases

1. **Very fast analysis** - Phases completing in < 100ms, may not show progress
2. **Very large codebases** - Update frequency throttling to avoid slowdown
3. **Terminal width < 80** - Truncate phase names or metrics
4. **Non-UTF8 terminals** - Fall back to ASCII characters (-> instead of →)
5. **Ctrl+C interruption** - Clean up progress display before exit

### Performance Considerations

```rust
// Throttle progress updates to avoid overhead
impl AnalysisProgress {
    fn should_update(&self) -> bool {
        // Update at most 10 times per second
        self.last_update.elapsed() > Duration::from_millis(100)
    }

    pub fn update_progress(&mut self, progress: PhaseProgress) {
        self.phases[self.current_phase].progress = progress;

        if self.should_update() {
            self.render();
            self.last_update = Instant::now();
        }
    }
}
```

## Migration and Compatibility

### Breaking Changes

**None** - This replaces existing progress output with clearer format.

### Migration Path

No migration required. Progress output is ephemeral (not persisted).

## Success Metrics

- ✅ Progress shows 4 clear phases with numbers
- ✅ Phase names are self-explanatory
- ✅ No repetitive information
- ✅ Consistent formatting across all phases
- ✅ Durations shown for each phase
- ✅ Total analysis time displayed
- ✅ Works in interactive terminals
- ✅ Works in CI/CD environments
- ✅ User testing confirms improved clarity
- ✅ No performance regression from progress updates
- ✅ Documentation includes progress examples

## Follow-up Work

1. **Estimated time remaining** - Based on historical analysis times
2. **Progress persistence** - Cache progress for resume on interruption
3. **Parallel progress** - Show progress for parallel file analysis
4. **Color coding** - Green for complete, blue for in-progress
5. **Sub-phases** - Show detail within complex phases
6. **Progress API** - Allow plugins to add custom phases

## References

- Design Analysis: Debtmap Terminal Output (parent document)
- src/commands/analyze.rs - Current analysis orchestration
- indicatif crate - Progress bar library (potential dependency)
- UX Pattern: Progress Indicators (Nielsen Norman Group)
