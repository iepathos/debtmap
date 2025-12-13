---
number: 269
title: Refactor Analyze Command God Object
category: optimization
priority: high
status: draft
dependencies: [187]
created: 2025-12-13
---

# Specification 269: Refactor Analyze Command God Object

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: Spec 187 (Extract Pure Functions pattern)

## Context

The file `src/commands/analyze.rs` has been identified as a **critical technical debt item** with a score of 100.0 (the maximum severity). This file violates the Stillwater philosophy of "Pure Core, Imperative Shell" by mixing I/O operations with business logic throughout.

### Current State

| Metric | Value | Guideline | Status |
|--------|-------|-----------|--------|
| Lines of Code | 601 | < 200 | ❌ 3x over |
| Functions | 7 | Small, focused | ❌ God functions |
| Responsibilities | 3 | 1 per module | ❌ Mixed |
| Max Nesting | 3 | 2 | ❌ Deep |
| Coverage | 57.4% | > 85% | ❌ Low |
| Cyclomatic (accum.) | 79 | Low | ❌ High |
| Cognitive (accum.) | 90 | Low | ❌ High |

### God Functions Identified

1. **`handle_analyze`** (lines 78-236, 158 lines)
   - Environment configuration
   - Progress tracking setup
   - Analysis invocation
   - Category filtering (pure logic)
   - TUI decision logic (pure logic)
   - Output routing

2. **`analyze_project`** (lines 277-409, 132 lines)
   - Progress tracking interleaved
   - File discovery
   - Parallel configuration
   - Metric extraction
   - Report building

3. **`handle_call_graph_diagnostics`** (lines 463-601, 137 lines)
   - Validation logic
   - Debug formatting
   - Statistics calculation
   - Output generation

### Stillwater Violations

```
Current: Everything Mixed
┌─────────────────────────────────────┐
│  I/O + Logic + Progress + Output    │
│  (handle_analyze: 158 lines)        │
│  Can't test, can't reuse            │
└─────────────────────────────────────┘

Target: Pure Core, Imperative Shell
┌─────────────────────────────────────┐
│  Shell: config.rs (I/O, env vars)   │
├─────────────────────────────────────┤
│  Core: pipeline.rs (pure logic)     │
├─────────────────────────────────────┤
│  Shell: orchestrator.rs (thin I/O)  │
├─────────────────────────────────────┤
│  Shell: diagnostics.rs (output I/O) │
└─────────────────────────────────────┘
```

## Objective

Refactor `src/commands/analyze.rs` into 4 focused modules following the Stillwater "Pure Core, Imperative Shell" pattern:

1. **config.rs** - Configuration structures and environment setup
2. **pipeline.rs** - Pure transformation and filtering logic
3. **orchestrator.rs** - Thin I/O orchestration
4. **diagnostics.rs** - Debug output and validation display

Each module under 200 lines, each function under 20 lines, with clear separation between pure logic and I/O operations.

## Requirements

### Functional Requirements

#### FR1: Module Split

Create directory `src/commands/analyze/` with:

```
src/commands/analyze/
├── mod.rs           # Public API, re-exports
├── config.rs        # Configuration (< 150 lines)
├── pipeline.rs      # Pure transformations (< 150 lines)
├── orchestrator.rs  # I/O orchestration (< 200 lines)
└── diagnostics.rs   # Debug/validation output (< 150 lines)
```

#### FR2: Config Module (`config.rs`)

Move to this module:
- `AnalyzeConfig` struct (lines 16-76)
- `configure_output` function
- `set_threshold_preset` function
- New: `setup_environment` function (extracted from `handle_analyze`)
- New: `setup_progress_manager` function (extracted from `handle_analyze`)

```rust
// config.rs - Shell (I/O for environment setup)
pub struct AnalyzeConfig { ... }

/// Configure environment based on config (I/O)
pub fn setup_environment(config: &AnalyzeConfig) {
    configure_output(config);
    set_threshold_preset(config.threshold_preset);
    setup_max_files(config.max_files);
    setup_min_score(config.min_score);
    setup_jobs(config.jobs);
    setup_functional_analysis(config);
}

/// Initialize progress manager (I/O)
pub fn setup_progress_manager(verbosity: u8) -> Option<ProgressManager> { ... }
```

#### FR3: Pipeline Module (`pipeline.rs`)

Extract pure logic:
- Category filtering logic
- File context adjustments
- Empty results checking
- TUI decision logic

```rust
// pipeline.rs - Pure Core (no I/O)

/// Filters analysis by categories (pure)
pub fn filter_by_categories(
    analysis: UnifiedAnalysis,
    categories: &[String],
) -> UnifiedAnalysis { ... }

/// Checks if results are empty and returns message (pure)
pub fn check_empty_results(
    items: &[DebtItem],
    file_items: &[FileDebtItem],
    min_score_env: Option<&str>,
) -> Option<EmptyResultsMessage> { ... }

/// Determines if TUI should be used (pure)
pub fn should_use_tui(
    no_tui: bool,
    format: OutputFormat,
    output_file: Option<&PathBuf>,
    is_terminal: bool,
    is_ci: bool,
) -> bool { ... }

/// Applies file context adjustments (pure)
pub fn apply_file_context(
    analysis: &mut UnifiedAnalysis,
    file_contexts: &FileContexts,
) { ... }
```

#### FR4: Orchestrator Module (`orchestrator.rs`)

Thin wrapper composing I/O and pure functions:

```rust
// orchestrator.rs - Shell (composes I/O + pure)

/// Main entry point - orchestrates analysis (thin wrapper)
pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    // Setup (I/O)
    config::setup_environment(&config);
    let _progress = config::setup_progress_manager(config.verbosity);

    // Analyze (I/O)
    let results = analyze_project::run(&config)?;
    let unified = build_unified_analysis(&config, &results)?;

    // Transform (pure)
    let filtered = pipeline::filter_by_categories(unified, &config.filter_categories);
    pipeline::apply_file_context(&mut filtered, &results.file_contexts);

    // Diagnostics (I/O)
    if config.needs_diagnostics() {
        diagnostics::handle_call_graph(&filtered, &config)?;
    }

    // Output (I/O)
    output_results(filtered, &config, &results)
}
```

#### FR5: Diagnostics Module (`diagnostics.rs`)

Extract diagnostic output:

```rust
// diagnostics.rs - Shell (output formatting)

/// Handle call graph debug output (I/O)
pub fn handle_call_graph(
    analysis: &UnifiedAnalysis,
    config: &AnalyzeConfig,
) -> Result<()> { ... }

/// Format and print validation report (I/O)
fn print_validation_report(report: &ValidationReport, verbosity: u8) { ... }

/// Format and print debug report (I/O)
fn print_debug_report(debugger: &CallGraphDebugger) -> Result<()> { ... }

/// Format and print statistics (I/O)
fn print_statistics(call_graph: &CallGraph) { ... }
```

#### FR6: Preserve API Compatibility

The public API must remain unchanged:
- `handle_analyze(config: AnalyzeConfig) -> Result<()>`
- `analyze_project(...)` function signature preserved
- All CLI arguments continue to work identically

### Non-Functional Requirements

#### NFR1: Function Size
- All functions under 20 lines (target 5-10)
- Clear single responsibility per function

#### NFR2: Testability
- Pure functions testable without mocks
- Test coverage target: 80%+ for pipeline.rs
- Unit tests for all pure functions

#### NFR3: Performance
- No performance regression
- Same or better parallel processing
- Memory usage unchanged

#### NFR4: Code Quality
- No clippy warnings
- Proper documentation for public functions
- Consistent naming conventions

## Acceptance Criteria

### Core Split (Must Have)

- [ ] `src/commands/analyze.rs` split into `src/commands/analyze/` directory
- [ ] `config.rs` contains all configuration logic (< 150 lines)
- [ ] `pipeline.rs` contains all pure transformation logic (< 150 lines)
- [ ] `orchestrator.rs` contains thin I/O orchestration (< 200 lines)
- [ ] `diagnostics.rs` contains debug output logic (< 150 lines)
- [ ] `mod.rs` re-exports public API unchanged

### Function Quality (Must Have)

- [ ] All functions under 20 lines
- [ ] No function has cyclomatic complexity > 5
- [ ] Pure functions clearly marked with no I/O
- [ ] I/O functions thin and composing pure functions

### Testing (Must Have)

- [ ] All existing tests pass
- [ ] New unit tests for pure functions in `pipeline.rs`
- [ ] Test coverage > 75% for `pipeline.rs`
- [ ] Integration test for `handle_analyze` end-to-end

### Code Quality (Must Have)

- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` applied
- [ ] Documentation for all public functions
- [ ] No performance regression (benchmark)

### Verification (Should Have)

- [ ] Self-analysis: debtmap no longer reports this as critical debt
- [ ] Reduced LOC by clear separation (600 → ~550 total across modules)
- [ ] Code review confirms improved readability

## Technical Details

### Implementation Approach

#### Phase 1: Create Directory Structure

```bash
mkdir -p src/commands/analyze
touch src/commands/analyze/{mod.rs,config.rs,pipeline.rs,orchestrator.rs,diagnostics.rs}
```

#### Phase 2: Extract Config Module

Move from `analyze.rs`:
- Lines 16-76: `AnalyzeConfig` struct
- Lines 255-275: `configure_output`, `set_threshold_preset`
- Extract environment setup from `handle_analyze` (lines 83-120)

**Before:**
```rust
// analyze.rs:78-120 (mixed concerns)
pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    configure_output(&config);
    set_threshold_preset(config.threshold_preset);

    let quiet = std::env::var("DEBTMAP_QUIET").is_ok();
    let progress_config = ProgressConfig::from_env(quiet, config.verbosity);
    ProgressManager::init_global(progress_config);

    if let Some(max_files) = config.max_files {
        std::env::set_var("DEBTMAP_MAX_FILES", max_files.to_string());
    }
    // ... 15 more lines of env setup
```

**After:**
```rust
// config.rs
pub fn setup_environment(config: &AnalyzeConfig) {
    configure_output(config);
    set_threshold_preset(config.threshold_preset);
    setup_env_vars(config);
}

fn setup_env_vars(config: &AnalyzeConfig) {
    if let Some(max) = config.max_files {
        std::env::set_var("DEBTMAP_MAX_FILES", max.to_string());
    }
    // Each env var in focused helper
}

// orchestrator.rs
pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    config::setup_environment(&config);
    // ...
}
```

#### Phase 3: Extract Pipeline Module

Extract pure functions:

**Before (mixed in handle_analyze):**
```rust
// analyze.rs:168-181 (inline, mixed with I/O)
let filtered_analysis = if let Some(ref filter_cats) = config.filter_categories {
    let categories: Vec<crate::priority::DebtCategory> = filter_cats
        .iter()
        .filter_map(|s| crate::priority::DebtCategory::from_string(s))
        .collect();

    if !categories.is_empty() {
        unified_analysis.filter_by_categories(&categories)
    } else {
        unified_analysis
    }
} else {
    unified_analysis
};
```

**After:**
```rust
// pipeline.rs - Pure function
pub fn filter_by_categories(
    analysis: UnifiedAnalysis,
    filter_categories: Option<&[String]>,
) -> UnifiedAnalysis {
    let categories = parse_categories(filter_categories);
    if categories.is_empty() {
        return analysis;
    }
    analysis.filter_by_categories(&categories)
}

fn parse_categories(filter_cats: Option<&[String]>) -> Vec<DebtCategory> {
    filter_cats
        .map(|cats| cats.iter().filter_map(|s| DebtCategory::from_string(s)).collect())
        .unwrap_or_default()
}
```

**TUI Decision (pure):**
```rust
// pipeline.rs
pub fn should_use_tui(
    no_tui: bool,
    format: OutputFormat,
    output_file: &Option<PathBuf>,
    is_terminal: bool,
    is_ci: bool,
) -> bool {
    !no_tui
        && matches!(format, OutputFormat::Terminal)
        && output_file.is_none()
        && is_terminal
        && !is_ci
}
```

#### Phase 4: Extract Diagnostics Module

Move `handle_call_graph_diagnostics` (lines 463-601) and split:

```rust
// diagnostics.rs

pub fn handle_call_graph(
    analysis: &UnifiedAnalysis,
    config: &AnalyzeConfig,
) -> Result<()> {
    if config.validate_call_graph {
        let report = validate_call_graph(&analysis.call_graph);
        print_validation_report(&report, config.verbosity);
    }

    if config.debug_call_graph {
        let debugger = build_debugger(config);
        print_debug_report(&debugger)?;
    }

    if config.call_graph_stats_only {
        print_statistics(&analysis.call_graph);
    }

    print_coverage_diagnostics_if_enabled();
    Ok(())
}

// Split into focused functions (each < 20 lines)
fn print_validation_report(report: &ValidationReport, verbosity: u8) { ... }
fn print_debug_report(debugger: &CallGraphDebugger) -> Result<()> { ... }
fn print_statistics(call_graph: &CallGraph) { ... }
fn print_coverage_diagnostics_if_enabled() { ... }
```

#### Phase 5: Create Orchestrator

Compose everything in thin orchestrator:

```rust
// orchestrator.rs

pub fn handle_analyze(config: AnalyzeConfig) -> Result<()> {
    // Setup phase (I/O)
    config::setup_environment(&config);
    let _progress = config::setup_progress_manager(config.verbosity);

    // Analysis phase (I/O + pure)
    let results = run_analysis(&config)?;
    let mut unified = build_unified_analysis(&config, &results)?;

    // Transform phase (pure)
    unified = pipeline::filter_by_categories(unified, config.filter_categories.as_deref());
    pipeline::apply_file_context(&mut unified, &results.file_contexts);

    // Diagnostics phase (I/O)
    run_diagnostics_if_needed(&unified, &config)?;

    // Check results (pure + I/O)
    handle_empty_results(&unified)?;

    // Cleanup TUI (I/O)
    cleanup_progress();

    // Output phase (I/O)
    output_results(unified, &config, &results)
}

fn run_analysis(config: &AnalyzeConfig) -> Result<AnalysisResults> { ... }
fn build_unified_analysis(...) -> Result<UnifiedAnalysis> { ... }
fn run_diagnostics_if_needed(...) -> Result<()> { ... }
fn handle_empty_results(analysis: &UnifiedAnalysis) -> Result<()> { ... }
fn cleanup_progress() { ... }
fn output_results(...) -> Result<()> { ... }
```

#### Phase 6: Update mod.rs

```rust
// src/commands/analyze/mod.rs

mod config;
mod diagnostics;
mod orchestrator;
mod pipeline;

// Re-export public API (unchanged signatures)
pub use config::AnalyzeConfig;
pub use orchestrator::{analyze_project, handle_analyze};
```

### File Organization

```
src/commands/analyze/
├── mod.rs           (~30 lines)  - Re-exports, public API
├── config.rs        (~120 lines) - AnalyzeConfig, env setup
├── pipeline.rs      (~100 lines) - Pure transformations
├── orchestrator.rs  (~180 lines) - I/O orchestration, analyze_project
└── diagnostics.rs   (~120 lines) - Debug output, validation display
                     ─────────────
                     ~550 lines total (down from 601)
```

### Migration Steps

1. Create directory and module files
2. Move `AnalyzeConfig` to `config.rs`
3. Extract environment setup functions
4. Extract pure pipeline functions
5. Extract diagnostics functions
6. Refactor `handle_analyze` to thin orchestrator
7. Update `mod.rs` with re-exports
8. Update import in `src/commands/mod.rs`
9. Run tests and fix any issues
10. Run benchmarks to verify no regression

## Dependencies

- **Prerequisites**: Spec 187 (Extract Pure Functions pattern established)
- **Affected Components**:
  - `src/commands/analyze.rs` → `src/commands/analyze/` directory
  - `src/commands/mod.rs` (update import)
- **External Dependencies**: None

## Testing Strategy

### Unit Tests

```rust
// tests for pipeline.rs
#[cfg(test)]
mod pipeline_tests {
    use super::*;

    #[test]
    fn filter_by_categories_empty_returns_unchanged() {
        let analysis = create_test_analysis();
        let result = filter_by_categories(analysis.clone(), None);
        assert_eq!(result.items.len(), analysis.items.len());
    }

    #[test]
    fn filter_by_categories_filters_correctly() {
        let analysis = create_test_analysis_with_categories();
        let result = filter_by_categories(
            analysis,
            Some(&["complexity".to_string()]),
        );
        assert!(result.items.iter().all(|i|
            i.category == DebtCategory::Complexity
        ));
    }

    #[test]
    fn should_use_tui_disabled_when_no_tui_flag() {
        assert!(!should_use_tui(true, OutputFormat::Terminal, &None, true, false));
    }

    #[test]
    fn should_use_tui_disabled_for_json_format() {
        assert!(!should_use_tui(false, OutputFormat::Json, &None, true, false));
    }

    #[test]
    fn should_use_tui_disabled_in_ci() {
        assert!(!should_use_tui(false, OutputFormat::Terminal, &None, true, true));
    }

    #[test]
    fn should_use_tui_enabled_when_all_conditions_met() {
        assert!(should_use_tui(false, OutputFormat::Terminal, &None, true, false));
    }
}
```

### Integration Tests

```rust
// tests/integration/analyze_command.rs
#[test]
fn test_analyze_command_produces_same_output() {
    let config = create_test_config();
    let result = handle_analyze(config);
    assert!(result.is_ok());
    // Verify output matches expected format
}

#[test]
fn test_analyze_with_category_filter() {
    let mut config = create_test_config();
    config.filter_categories = Some(vec!["complexity".to_string()]);
    let result = handle_analyze(config);
    assert!(result.is_ok());
}
```

### Performance Benchmarks

```rust
// benches/analyze.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_handle_analyze(c: &mut Criterion) {
    let config = create_bench_config();
    c.bench_function("handle_analyze", |b| {
        b.iter(|| handle_analyze(config.clone()))
    });
}

fn bench_filter_by_categories(c: &mut Criterion) {
    let analysis = create_large_analysis();
    let categories = vec!["complexity".to_string()];
    c.bench_function("filter_by_categories", |b| {
        b.iter(|| filter_by_categories(analysis.clone(), Some(&categories)))
    });
}
```

## Documentation Requirements

### Code Documentation

All public functions documented with:
- Purpose description
- Whether pure or I/O
- Parameters and return values
- Example usage where appropriate

```rust
/// Filters unified analysis by debt categories.
///
/// This is a **pure function** - no I/O, deterministic, easily testable.
///
/// # Arguments
/// * `analysis` - The unified analysis to filter
/// * `filter_categories` - Optional list of category names to include
///
/// # Returns
/// Filtered analysis containing only items in specified categories
///
/// # Examples
/// ```
/// let filtered = filter_by_categories(analysis, Some(&["complexity"]));
/// ```
pub fn filter_by_categories(...) { ... }
```

### Architecture Updates

Add to `ARCHITECTURE.md`:

```markdown
## Command Module Pattern

The `analyze` command demonstrates the "Pure Core, Imperative Shell" pattern:

```
src/commands/analyze/
├── config.rs        # Shell: Environment & config setup
├── pipeline.rs      # Core: Pure transformations (testable)
├── orchestrator.rs  # Shell: Thin I/O composition
└── diagnostics.rs   # Shell: Output formatting
```

### Module Responsibilities

- **config.rs**: Configuration structures, environment variable setup
- **pipeline.rs**: Pure functions for filtering, transforming, deciding
- **orchestrator.rs**: Composes I/O + pure functions, thin entry point
- **diagnostics.rs**: Debug output, validation display, statistics
```

## Implementation Notes

### Order of Extraction

1. Start with `config.rs` (most independent)
2. Then `pipeline.rs` (pure functions, easy to test)
3. Then `diagnostics.rs` (output functions)
4. Finally `orchestrator.rs` (ties everything together)

### Gotchas

- **Progress Manager**: Has global state, handle carefully
- **Environment Variables**: Currently scattered, consolidate in config.rs
- **TUI Cleanup**: Must happen before output, timing sensitive

### Testing During Refactor

After each module extraction:
1. `cargo check` - Ensure compilation
2. `cargo test` - Ensure tests pass
3. `cargo clippy` - Ensure no new warnings
4. Manual test of `debtmap analyze .` - Verify behavior

## Migration and Compatibility

### Breaking Changes

**None** - This is a pure internal refactoring. The public API remains identical:
- `handle_analyze(config: AnalyzeConfig) -> Result<()>`
- `analyze_project(...)` signature unchanged
- All CLI arguments work identically

### Import Updates

Only `src/commands/mod.rs` needs update:

```rust
// Before
pub mod analyze;

// After
pub mod analyze;  // Now points to analyze/mod.rs
```

## Success Metrics

After implementation:

| Metric | Before | After | Target |
|--------|--------|-------|--------|
| Total LOC | 601 | ~550 | < 600 |
| Largest file | 601 | ~180 | < 200 |
| Largest function | 158 | < 20 | < 20 |
| Test coverage | 57.4% | > 75% | > 75% |
| Debt score | 100.0 | < 30 | < 50 |
| Responsibilities per file | 3 | 1 | 1 |

## Follow-up Work

After this spec:
- Apply same pattern to other command modules
- Consider extracting `analyze_project` to separate module
- Document the pattern for future command implementations

## References

- **Stillwater PHILOSOPHY.md** - Pure Core, Imperative Shell pattern
- **Spec 187** - Extract Pure Functions pattern
- **CLAUDE.md** - Function design guidelines (max 20 lines)
- **Debtmap self-analysis** - Critical debt item identification
