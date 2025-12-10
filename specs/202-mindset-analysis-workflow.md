---
number: 202
title: Analysis Workflow State Machine with Mindset
category: foundation
priority: high
status: draft
dependencies: [201]
created: 2025-12-10
---

# Specification 202: Analysis Workflow State Machine with Mindset

**Category**: foundation
**Priority**: high
**Status**: draft
**Dependencies**: Spec 201 (Premortem Config Validation)

## Context

The main analysis workflow in `src/builders/unified_analysis.rs` is a ~260-line function (`perform_unified_analysis_computation`) with implicit phase progression tracked via progress callbacks. This creates several problems:

**Current State:**
```rust
// src/builders/unified_analysis.rs:134-390
pub fn perform_unified_analysis_computation<'a>(
    options: UnifiedAnalysisOptions<'a>,
) -> anyhow::Result<Vec<DebtRecommendation>> {
    // Phase 1: Call graph (implicit)
    progress_manager.start("Building call graph");
    let call_graph = build_call_graph(&options.results);
    progress_manager.complete();

    // Phase 2: Coverage (implicit, conditional)
    let coverage = if let Some(path) = options.coverage_file {
        progress_manager.start("Loading coverage");
        let cov = load_coverage(path)?;
        progress_manager.complete();
        Some(cov)
    } else {
        None
    };

    // Phase 3: Purity analysis (implicit, depends on coverage)
    progress_manager.start("Analyzing purity");
    let purity_results = analyze_purity(&metrics, &call_graph);
    progress_manager.complete();

    // ... 8 more implicit phases
}
```

**Problems:**
1. **No explicit state** - Can't tell what phase the analysis is in
2. **No checkpoint/resume** - Long analysis must restart from scratch
3. **Hard to test** - Can't test individual phase transitions
4. **Implicit dependencies** - Coverage must complete before purity, but not explicit
5. **Progress is side-effect** - Pure analysis mixed with progress reporting

**With mindset state machine:**
```rust
state_enum! {
    enum AnalysisPhase {
        Initialized,
        CallGraphBuilding,
        CallGraphComplete,
        CoverageLoading,
        CoverageComplete,
        PurityAnalyzing,
        PurityComplete,
        // ...
        Complete,
    }
    final: [Complete]
}

// Pure guards check if transition is valid
fn can_start_purity(state: &AnalysisState) -> bool {
    state.call_graph.is_some()  // Explicit dependency
}

// Effectful actions are separate
fn run_purity_analysis<Env>(state: &mut AnalysisState, env: &mut Env) -> Result<()>
where
    Env: ProgressReporter,
{
    env.report_progress("Analyzing function purity");
    state.purity_results = Some(compute_purity(&state.metrics, state.call_graph.as_ref().unwrap()));
    Ok(())
}
```

## Objective

Use **mindset** to implement an explicit state machine for the analysis workflow:

1. **Explicit phases** - Each analysis phase is an enum variant
2. **Pure guards** - Transition validation is pure (testable)
3. **Effectful actions** - Side effects isolated via environment traits
4. **Checkpoint support** - Save/restore state for resume capability
5. **Clear dependencies** - Phase prerequisites explicit in guards

**Success Metric**: Analysis can be paused/resumed and individual phase transitions are unit testable.

## Requirements

### Functional Requirements

1. **Define Analysis Phase Enum**
   - `Initialized` - Configuration validated, ready to start
   - `CallGraphBuilding` - Building function call graph
   - `CallGraphComplete` - Call graph built successfully
   - `CoverageLoading` - Loading LCOV coverage data
   - `CoverageComplete` - Coverage loaded (or skipped)
   - `PurityAnalyzing` - Analyzing function purity
   - `PurityComplete` - Purity analysis complete
   - `ContextLoading` - Loading context providers
   - `ContextComplete` - Context loaded
   - `ScoringInProgress` - Computing debt scores
   - `ScoringComplete` - Scores computed
   - `FilteringInProgress` - Filtering and ranking
   - `Complete` - Analysis finished

2. **Implement Pure Guards**
   - `can_start_coverage` - True after call graph complete
   - `can_start_purity` - True after call graph complete
   - `can_start_context` - True after purity complete
   - `can_start_scoring` - True after all dependencies complete
   - Each guard is a pure function taking `&AnalysisState`

3. **Implement Effectful Actions**
   - `build_call_graph` - Requires `FileSystem` environment
   - `load_coverage` - Requires `FileSystem` environment
   - `analyze_purity` - Pure computation, but reports progress
   - `compute_scores` - Pure computation
   - Actions use environment traits for side effects

4. **State Persistence**
   - Serialize state to JSON for checkpointing
   - Resume from checkpoint on restart
   - Validate checkpoint before resume

5. **Progress Reporting**
   - Report phase transitions via environment
   - Report progress within phases
   - Support both TUI and CLI progress

### Non-Functional Requirements

1. **Testability** - Guards are pure, testable without I/O
2. **Performance** - State machine overhead < 1% of analysis time
3. **Debuggability** - State can be inspected/logged
4. **Extensibility** - Easy to add new phases

## Acceptance Criteria

- [ ] `AnalysisPhase` enum with all phases defined
- [ ] `AnalysisState` struct holding phase and accumulated results
- [ ] Pure guard functions for each transition
- [ ] Action functions with environment traits for I/O
- [ ] `ProgressReporter` trait for progress side effects
- [ ] Checkpoint serialization to JSON
- [ ] Resume from checkpoint
- [ ] Unit tests for all guards (pure)
- [ ] Integration tests for full workflow
- [ ] Existing analysis behavior unchanged (regression tests pass)

## Technical Details

### Implementation Approach

**Phase 1: Define State Types**

```rust
// src/analysis/workflow/state.rs
use mindset::{State, state_enum};
use serde::{Deserialize, Serialize};

state_enum! {
    /// Analysis workflow phases.
    ///
    /// Each phase represents a distinct step in the analysis pipeline.
    /// Transitions between phases are validated by pure guard functions.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    enum AnalysisPhase {
        /// Initial state after config validation.
        Initialized,

        /// Building the function call graph.
        CallGraphBuilding,

        /// Call graph built successfully.
        CallGraphComplete,

        /// Loading LCOV coverage data.
        CoverageLoading,

        /// Coverage loaded (or skipped if not provided).
        CoverageComplete,

        /// Analyzing function purity.
        PurityAnalyzing,

        /// Purity analysis complete.
        PurityComplete,

        /// Loading context providers.
        ContextLoading,

        /// Context loaded (or skipped).
        ContextComplete,

        /// Computing debt scores.
        ScoringInProgress,

        /// Scores computed.
        ScoringComplete,

        /// Filtering and ranking results.
        FilteringInProgress,

        /// Analysis complete.
        Complete,
    }

    /// Final states - analysis stops here.
    final: [Complete]
}

/// Complete analysis state including phase and accumulated data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisState {
    /// Current phase.
    pub phase: AnalysisPhase,

    /// Configuration for this analysis run.
    pub config: AnalysisConfig,

    /// Accumulated results from completed phases.
    pub results: AnalysisResults,
}

/// Accumulated results from analysis phases.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisResults {
    /// Function metrics from parsing.
    pub metrics: Option<Vec<FunctionMetrics>>,

    /// Call graph from dependency analysis.
    pub call_graph: Option<CallGraph>,

    /// Coverage data if provided.
    pub coverage: Option<CoverageData>,

    /// Purity analysis results.
    pub purity: Option<PurityResults>,

    /// Context data if enabled.
    pub context: Option<ContextData>,

    /// Computed debt scores.
    pub scores: Option<Vec<DebtScore>>,

    /// Final recommendations.
    pub recommendations: Option<Vec<DebtRecommendation>>,
}
```

**Phase 2: Pure Guard Functions**

```rust
// src/analysis/workflow/guards.rs

/// Guard: Can transition from Initialized to CallGraphBuilding?
///
/// Pure function - no side effects.
pub fn can_start_call_graph(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::Initialized)
        && state.results.metrics.is_some()
}

/// Guard: Can transition from CallGraphComplete to CoverageLoading?
///
/// Requires call graph complete and coverage file configured.
pub fn can_start_coverage(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::CallGraphComplete)
        && state.results.call_graph.is_some()
        && state.config.coverage_file.is_some()
}

/// Guard: Can skip coverage phase?
///
/// True if coverage not configured.
pub fn can_skip_coverage(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::CallGraphComplete)
        && state.config.coverage_file.is_none()
}

/// Guard: Can transition to PurityAnalyzing?
///
/// Requires call graph complete and coverage complete (or skipped).
pub fn can_start_purity(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::CoverageComplete)
        && state.results.call_graph.is_some()
}

/// Guard: Can transition to ContextLoading?
///
/// Requires purity complete and context enabled.
pub fn can_start_context(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::PurityComplete)
        && state.results.purity.is_some()
        && state.config.enable_context
}

/// Guard: Can skip context loading?
///
/// True if context not enabled.
pub fn can_skip_context(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::PurityComplete)
        && !state.config.enable_context
}

/// Guard: Can start scoring?
///
/// Requires all dependencies complete.
pub fn can_start_scoring(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::ContextComplete)
        && state.results.call_graph.is_some()
        && state.results.purity.is_some()
}

/// Guard: Can transition to Complete?
pub fn can_complete(state: &AnalysisState) -> bool {
    matches!(state.phase, AnalysisPhase::FilteringInProgress)
        && state.results.recommendations.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_start_call_graph_requires_metrics() {
        let mut state = AnalysisState::new(AnalysisConfig::default());

        // No metrics - can't start
        assert!(!can_start_call_graph(&state));

        // With metrics - can start
        state.results.metrics = Some(vec![]);
        assert!(can_start_call_graph(&state));
    }

    #[test]
    fn test_can_start_coverage_requires_config() {
        let mut state = AnalysisState::new(AnalysisConfig::default());
        state.phase = AnalysisPhase::CallGraphComplete;
        state.results.call_graph = Some(CallGraph::empty());

        // No coverage file configured - can't start
        assert!(!can_start_coverage(&state));
        assert!(can_skip_coverage(&state));

        // With coverage file - can start
        state.config.coverage_file = Some(PathBuf::from("coverage.lcov"));
        assert!(can_start_coverage(&state));
        assert!(!can_skip_coverage(&state));
    }

    #[test]
    fn test_guard_independence() {
        // Guards are pure - same input always produces same output
        let state = AnalysisState::new(AnalysisConfig::default());

        let result1 = can_start_call_graph(&state);
        let result2 = can_start_call_graph(&state);

        assert_eq!(result1, result2);
    }
}
```

**Phase 3: Environment Traits**

```rust
// src/analysis/workflow/env.rs

/// Environment trait for progress reporting.
///
/// Following mindset pattern: pure guards, effectful actions via environment.
pub trait ProgressReporter {
    /// Report that a phase is starting.
    fn phase_starting(&mut self, phase: &str);

    /// Report progress within a phase (0.0 - 1.0).
    fn phase_progress(&mut self, progress: f64);

    /// Report that a phase completed.
    fn phase_complete(&mut self);

    /// Report a warning during analysis.
    fn warn(&mut self, message: &str);
}

/// Environment trait for file system operations.
pub trait FileSystem {
    /// Read file contents.
    fn read_file(&self, path: &Path) -> std::io::Result<Vec<u8>>;

    /// Check if file exists.
    fn file_exists(&self, path: &Path) -> bool;
}

/// Combined environment for analysis workflow.
pub trait AnalysisEnv: ProgressReporter + FileSystem {}

impl<T: ProgressReporter + FileSystem> AnalysisEnv for T {}

/// Real environment implementation.
pub struct RealAnalysisEnv {
    progress: Box<dyn ProgressReporter>,
}

impl RealAnalysisEnv {
    pub fn new(progress: impl ProgressReporter + 'static) -> Self {
        Self {
            progress: Box::new(progress),
        }
    }
}

impl ProgressReporter for RealAnalysisEnv {
    fn phase_starting(&mut self, phase: &str) {
        self.progress.phase_starting(phase);
    }

    fn phase_progress(&mut self, progress: f64) {
        self.progress.phase_progress(progress);
    }

    fn phase_complete(&mut self) {
        self.progress.phase_complete();
    }

    fn warn(&mut self, message: &str) {
        self.progress.warn(message);
    }
}

impl FileSystem for RealAnalysisEnv {
    fn read_file(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

/// Mock environment for testing.
#[cfg(test)]
pub struct MockAnalysisEnv {
    pub phases: Vec<String>,
    pub warnings: Vec<String>,
    pub files: std::collections::HashMap<PathBuf, Vec<u8>>,
}

#[cfg(test)]
impl MockAnalysisEnv {
    pub fn new() -> Self {
        Self {
            phases: vec![],
            warnings: vec![],
            files: std::collections::HashMap::new(),
        }
    }

    pub fn with_file(mut self, path: impl Into<PathBuf>, content: impl AsRef<[u8]>) -> Self {
        self.files.insert(path.into(), content.as_ref().to_vec());
        self
    }
}

#[cfg(test)]
impl ProgressReporter for MockAnalysisEnv {
    fn phase_starting(&mut self, phase: &str) {
        self.phases.push(format!("start:{}", phase));
    }

    fn phase_progress(&mut self, _progress: f64) {}

    fn phase_complete(&mut self) {
        self.phases.push("complete".to_string());
    }

    fn warn(&mut self, message: &str) {
        self.warnings.push(message.to_string());
    }
}

#[cfg(test)]
impl FileSystem for MockAnalysisEnv {
    fn read_file(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        self.files
            .get(path)
            .cloned()
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "file not found",
            ))
    }

    fn file_exists(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }
}
```

**Phase 4: Effectful Actions**

```rust
// src/analysis/workflow/actions.rs

use super::{guards::*, state::*, env::*};
use anyhow::Result;

/// Action: Build call graph.
///
/// Effectful - reports progress via environment.
pub fn build_call_graph<Env: AnalysisEnv>(
    state: &mut AnalysisState,
    env: &mut Env,
) -> Result<()> {
    // Guard check (defense in depth - guards should prevent invalid transitions)
    debug_assert!(can_start_call_graph(state));

    env.phase_starting("Building call graph");
    state.phase = AnalysisPhase::CallGraphBuilding;

    // Pure computation extracted
    let metrics = state.results.metrics.as_ref()
        .expect("guard ensures metrics exist");
    let call_graph = compute_call_graph(metrics);

    state.results.call_graph = Some(call_graph);
    state.phase = AnalysisPhase::CallGraphComplete;

    env.phase_complete();
    Ok(())
}

/// Action: Load coverage data.
pub fn load_coverage<Env: AnalysisEnv>(
    state: &mut AnalysisState,
    env: &mut Env,
) -> Result<()> {
    debug_assert!(can_start_coverage(state));

    env.phase_starting("Loading coverage data");
    state.phase = AnalysisPhase::CoverageLoading;

    let coverage_path = state.config.coverage_file.as_ref()
        .expect("guard ensures coverage_file exists");

    let content = env.read_file(coverage_path)?;
    let coverage = parse_lcov(&content)?;

    state.results.coverage = Some(coverage);
    state.phase = AnalysisPhase::CoverageComplete;

    env.phase_complete();
    Ok(())
}

/// Action: Skip coverage (no file configured).
pub fn skip_coverage(state: &mut AnalysisState) {
    debug_assert!(can_skip_coverage(state));

    state.results.coverage = None;
    state.phase = AnalysisPhase::CoverageComplete;
}

/// Action: Analyze purity.
pub fn analyze_purity<Env: ProgressReporter>(
    state: &mut AnalysisState,
    env: &mut Env,
) -> Result<()> {
    debug_assert!(can_start_purity(state));

    env.phase_starting("Analyzing function purity");
    state.phase = AnalysisPhase::PurityAnalyzing;

    let metrics = state.results.metrics.as_ref().unwrap();
    let call_graph = state.results.call_graph.as_ref().unwrap();

    // Pure computation
    let purity = compute_purity_propagation(metrics, call_graph);

    state.results.purity = Some(purity);
    state.phase = AnalysisPhase::PurityComplete;

    env.phase_complete();
    Ok(())
}

// ... more actions for remaining phases

/// Run the complete analysis workflow.
pub fn run_analysis<Env: AnalysisEnv>(
    mut state: AnalysisState,
    env: &mut Env,
) -> Result<AnalysisState> {
    // Phase: Call Graph
    if can_start_call_graph(&state) {
        build_call_graph(&mut state, env)?;
    }

    // Phase: Coverage (or skip)
    if can_start_coverage(&state) {
        load_coverage(&mut state, env)?;
    } else if can_skip_coverage(&state) {
        skip_coverage(&mut state);
    }

    // Phase: Purity
    if can_start_purity(&state) {
        analyze_purity(&mut state, env)?;
    }

    // Phase: Context (or skip)
    if can_start_context(&state) {
        load_context(&mut state, env)?;
    } else if can_skip_context(&state) {
        skip_context(&mut state);
    }

    // Phase: Scoring
    if can_start_scoring(&state) {
        compute_scores(&mut state, env)?;
    }

    // Phase: Filtering
    if can_start_filtering(&state) {
        filter_and_rank(&mut state, env)?;
    }

    Ok(state)
}
```

**Phase 5: Checkpoint Support**

```rust
// src/analysis/workflow/checkpoint.rs

use super::state::AnalysisState;
use anyhow::Result;
use std::path::Path;

/// Save analysis state to checkpoint file.
pub fn save_checkpoint(state: &AnalysisState, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load analysis state from checkpoint file.
pub fn load_checkpoint(path: &Path) -> Result<AnalysisState> {
    let json = std::fs::read_to_string(path)?;
    let state: AnalysisState = serde_json::from_str(&json)?;
    Ok(state)
}

/// Resume analysis from checkpoint.
pub fn resume_analysis<Env: AnalysisEnv>(
    checkpoint_path: &Path,
    env: &mut Env,
) -> Result<AnalysisState> {
    let state = load_checkpoint(checkpoint_path)?;

    // Log where we're resuming from
    env.phase_starting(&format!("Resuming from {:?}", state.phase));

    run_analysis(state, env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_checkpoint_roundtrip() {
        let state = AnalysisState::new(AnalysisConfig::default());

        let file = NamedTempFile::new().unwrap();
        save_checkpoint(&state, file.path()).unwrap();

        let loaded = load_checkpoint(file.path()).unwrap();
        assert_eq!(state.phase, loaded.phase);
    }

    #[test]
    fn test_resume_from_partial() {
        let mut state = AnalysisState::new(AnalysisConfig::default());
        state.phase = AnalysisPhase::CoverageComplete;
        state.results.call_graph = Some(CallGraph::empty());

        let file = NamedTempFile::new().unwrap();
        save_checkpoint(&state, file.path()).unwrap();

        let mut env = MockAnalysisEnv::new();
        let result = resume_analysis(file.path(), &mut env).unwrap();

        // Should have progressed past coverage
        assert!(matches!(result.phase, AnalysisPhase::Complete | _));
    }
}
```

### File Structure

```
src/analysis/workflow/
├── mod.rs           # Re-exports
├── state.rs         # AnalysisPhase enum and AnalysisState struct
├── guards.rs        # Pure guard functions
├── actions.rs       # Effectful action functions
├── env.rs           # Environment traits
├── checkpoint.rs    # Checkpoint save/load
└── tests.rs         # Integration tests
```

### Architecture Changes

**Before:**
```
perform_unified_analysis_computation()
├─ progress_manager.start("phase 1")
├─ do_phase_1()
├─ progress_manager.complete()
├─ progress_manager.start("phase 2")
├─ ... (260 lines, implicit flow)
```

**After:**
```
AnalysisState { phase: Initialized }
    ↓ can_start_call_graph() guard
AnalysisState { phase: CallGraphBuilding }
    ↓ build_call_graph() action
AnalysisState { phase: CallGraphComplete }
    ↓ can_start_coverage() | can_skip_coverage()
AnalysisState { phase: CoverageComplete }
    ↓ ...
AnalysisState { phase: Complete }
```

## Dependencies

- **Prerequisites**: Spec 201 (config validation provides `AnalysisConfig`)
- **Affected Components**:
  - `src/builders/unified_analysis.rs` - Replace with workflow
  - `src/builders/parallel_unified_analysis.rs` - Update to use state machine
  - `src/main.rs` - Use new workflow API
  - `src/commands/analyze.rs` - Use new workflow API
- **External Dependencies**:
  - `mindset` - State machine library
  - `serde` / `serde_json` - For checkpoint serialization

## Testing Strategy

### Unit Tests (Pure Guards)

```rust
#[cfg(test)]
mod guard_tests {
    use super::*;

    #[test]
    fn test_guards_are_pure() {
        // Pure functions always return same result for same input
        let state = AnalysisState::default();

        let r1 = can_start_call_graph(&state);
        let r2 = can_start_call_graph(&state);

        assert_eq!(r1, r2);
    }

    #[test]
    fn test_phase_dependencies() {
        // Purity requires call graph complete
        let mut state = AnalysisState::default();
        assert!(!can_start_purity(&state));

        state.phase = AnalysisPhase::CoverageComplete;
        state.results.call_graph = Some(CallGraph::empty());
        assert!(can_start_purity(&state));
    }
}
```

### Integration Tests (Full Workflow)

```rust
#[test]
fn test_full_workflow() {
    let config = AnalysisConfig {
        project_path: PathBuf::from("src"),
        coverage_file: None,
        enable_context: false,
        ..Default::default()
    };

    let mut env = MockAnalysisEnv::new();
    let state = AnalysisState::new(config);

    let result = run_analysis(state, &mut env).unwrap();

    assert!(matches!(result.phase, AnalysisPhase::Complete));
    assert!(result.results.recommendations.is_some());
}

#[test]
fn test_workflow_with_coverage() {
    let config = AnalysisConfig {
        project_path: PathBuf::from("src"),
        coverage_file: Some(PathBuf::from("coverage.lcov")),
        ..Default::default()
    };

    let mut env = MockAnalysisEnv::new()
        .with_file("coverage.lcov", SAMPLE_LCOV_DATA);

    let state = AnalysisState::new(config);
    let result = run_analysis(state, &mut env).unwrap();

    assert!(result.results.coverage.is_some());
}
```

## Documentation Requirements

- **Code Documentation**: Each phase, guard, and action documented
- **User Documentation**: Add `--checkpoint` flag documentation
- **Architecture Updates**: Document state machine pattern in ARCHITECTURE.md

## Migration and Compatibility

### Breaking Changes

None - internal refactoring.

### Migration Steps

1. Implement new state machine alongside existing code
2. Add feature flag to switch between old/new
3. Test extensively with both paths
4. Remove old code after verification

## Implementation Notes

### Why mindset?

| Feature | Manual State | mindset |
|---------|-------------|---------|
| State enum | Manual | `state_enum!` macro |
| Guard functions | Ad-hoc | Structured pattern |
| Final states | Manual tracking | Declared in macro |
| Testing | Harder | Guards are pure |
| Environment | Global state | Dependency injection |

### Key Mindset Patterns

1. **Pure Guards, Effectful Actions** - Guards have no side effects
2. **Environment Traits** - I/O isolated to trait implementations
3. **State Machine Enum** - `state_enum!` macro for boilerplate
4. **Checkpoint Serialization** - State is `Serialize + Deserialize`

## References

- **mindset documentation**: State machine patterns
- **Stillwater PHILOSOPHY.md**: Pure guards, effectful actions
- **Current implementation**: `src/builders/unified_analysis.rs`
