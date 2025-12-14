---
number: 195
title: Integrate Layering Score into God Object Detection
category: optimization
priority: medium
status: draft
dependencies: []
created: 2025-12-13
---

# Specification 195: Integrate Layering Score into God Object Detection

**Category**: optimization
**Priority**: medium
**Status**: draft
**Dependencies**: None

## Context

Debtmap currently contains a standalone Dependency Structure Matrix (DSM) module (`src/analysis/dsm.rs`, ~780 lines) that provides:

1. **Propagation Cost** - Average modules reachable via BFS
2. **Layering Score** - Ratio of forward vs backward dependencies (unique signal)
3. **Density** - Dependency density metric (trivial)
4. **Cycle Detection** - Using Kosaraju's SCC algorithm (duplicates `organization/cycle_detector.rs`)
5. **Matrix Visualization** - TUI view and `--format dsm` output

### Problems with Current DSM Implementation

1. **Not Integrated**: DSM is completely standalone - not used by god object detection, scoring, or recommendations
2. **Undiscoverable**: The 'm' key for TUI DSM view isn't shown in footer or help screen
3. **Duplicates Functionality**: Cycle detection already exists in `organization/cycle_detector.rs`
4. **0% Coverage**: Key function `from_file_dependencies` has 0% test coverage
5. **Disconnected from Mission**: DSM visualization doesn't help "find where bugs hide"

### Why Layering Score Matters

Layering score is the **only unique signal** DSM provides that relates to god object detection:

- God objects typically **break layering** by depending on everything and having everything depend on them
- A module with many backward dependencies (lower layering score) indicates architectural problems
- When splitting a god object, layering should **improve** (fewer backward deps)
- Layering impact can **prioritize** which god objects to fix first

```
Perfect layering (score = 1.0):        Broken layering (score < 1.0):
     A  B  C                                A  B  C
  A  .  .  .  (no backward deps)         A  .  X  .  (A depends on B - backward!)
  B  X  .  .  (B depends on A)           B  X  .  .
  C  X  X  .  (C depends on A, B)        C  X  X  .
```

## Objective

Extract the layering score calculation from DSM and integrate it into god object detection and recommendations, then remove all other DSM code. This:

1. **Adds value** by using layering as a prioritization signal for god objects
2. **Reduces complexity** by removing ~1000 lines of disconnected code
3. **Improves focus** by eliminating undiscoverable features

## Requirements

### Functional Requirements

1. **Extract Layering Score Calculation**
   - Create `src/organization/layering.rs` with minimal layering score computation (~50 lines)
   - Compute layering at **module level** (e.g., `src/analysis`, `src/priority`, not file level)
   - Return score from 0.0 (all cycles) to 1.0 (perfect layers)

2. **Integrate into God Object Scoring**
   - Add `layering_impact: f64` field to god object scoring
   - Apply **15-20% priority boost** for god objects that break layering (cause ≥3 backward deps)
   - Use as modifier, not primary signal: `final_score = base_score * (1.0 + layering_penalty)`

3. **Show in Split Recommendations**
   - Display current module layering score when relevant
   - Show estimated improvement after proposed splits
   - Highlight number of backward dependencies caused by god object

4. **Remove DSM Code**
   - Delete `src/analysis/dsm.rs` (~780 lines)
   - Delete `src/tui/results/dsm_view.rs`
   - Delete `src/io/writers/dsm.rs`
   - Delete `src/output/dsm.rs`
   - Remove `OutputFormat::Dsm` variant from CLI
   - Remove DSM-related TUI navigation code (`ViewMode::Dsm`, 'm' key handler)
   - Remove DSM-related tests

### Non-Functional Requirements

1. **Performance**: Layering calculation should add negligible overhead (<10ms for typical codebase)
2. **Test Coverage**: New `layering.rs` module must have >80% coverage
3. **Documentation**: Update ARCHITECTURE.md to reflect removal of DSM

## Acceptance Criteria

- [ ] New `src/organization/layering.rs` module exists with `compute_layering_score()` function
- [ ] Layering is computed at module level, not file level
- [ ] God object scoring includes layering impact as 15-20% modifier
- [ ] God objects causing ≥3 backward deps receive layering penalty
- [ ] Split recommendations show layering improvement when applicable
- [ ] All DSM-related files deleted (dsm.rs, dsm_view.rs, writers/dsm.rs, output/dsm.rs)
- [ ] `OutputFormat::Dsm` removed from CLI args
- [ ] `ViewMode::Dsm` and 'm' key handler removed from TUI
- [ ] `cargo build` succeeds with no DSM references
- [ ] `cargo test` passes
- [ ] New layering module has >80% test coverage

## Technical Details

### Implementation Approach

#### Phase 1: Extract Layering Score

Create minimal layering calculation:

```rust
// src/organization/layering.rs

use std::collections::{HashMap, HashSet};

/// Dependency between two modules
pub struct ModuleDependency {
    pub from_module: String,
    pub to_module: String,
}

/// Result of layering analysis
pub struct LayeringAnalysis {
    /// Score from 0.0 (all backward deps) to 1.0 (perfect layers)
    pub score: f64,
    /// Number of backward dependencies (cycles)
    pub backward_dep_count: usize,
    /// Modules causing the most backward deps
    pub problematic_modules: Vec<(String, usize)>,
}

/// Compute layering score from module dependencies.
///
/// A well-layered architecture has dependencies flowing one direction.
/// Backward dependencies (lower-level depending on higher-level) indicate
/// architectural problems.
///
/// # Arguments
/// * `dependencies` - List of (from_module, to_module) pairs
///
/// # Returns
/// LayeringAnalysis with score and backward dependency details
pub fn compute_layering_score(dependencies: &[ModuleDependency]) -> LayeringAnalysis {
    // 1. Build dependency graph
    // 2. Compute topological ordering (or best-effort if cycles)
    // 3. Count deps in lower triangle (forward) vs upper triangle (backward)
    // 4. Return score = forward_deps / total_deps
}

/// Extract module name from file path
/// e.g., "src/analysis/dsm.rs" -> "analysis"
pub fn path_to_module(path: &str) -> String {
    // Implementation
}
```

#### Phase 2: Integrate into God Object Scoring

Modify `src/organization/god_object/scoring.rs`:

```rust
pub struct GodObjectScore {
    // Existing fields...
    pub responsibility_count: usize,
    pub method_count: usize,
    pub coupling_score: f64,

    // New field
    pub layering_impact: f64,  // 0.0 to 1.0, higher = more backward deps caused
}

impl GodObjectScore {
    pub fn compute_final_score(&self) -> f64 {
        let base = self.compute_base_score();
        let layering_penalty = if self.backward_deps_caused >= 3 {
            0.15  // 15% boost for architectural damage
        } else {
            0.0
        };
        base * (1.0 + layering_penalty)
    }
}
```

#### Phase 3: Update Recommendations

In split recommendations output:

```
Split Recommendation: src/organization/god_object_detector.rs
├─ Current module layering: 0.72 (28% backward deps)
├─ Backward deps caused: 5 (to: cli, priority, risk, tui, output)
├─ After split estimate: 0.85 (+0.13 improvement)
├─ Responsibilities to extract:
│  ├─ detection_logic → god_object/detection.rs
│  └─ recommendation_gen → god_object/recommendations.rs
└─ Architectural impact: Removes 3 backward dependencies
```

#### Phase 4: Remove DSM Code

Files to delete:
- `src/analysis/dsm.rs`
- `src/tui/results/dsm_view.rs`
- `src/io/writers/dsm.rs`
- `src/output/dsm.rs`

Code to modify:
- `src/cli/args.rs`: Remove `OutputFormat::Dsm` variant
- `src/tui/results/mod.rs`: Remove `dsm_view` module
- `src/tui/results/view_mode.rs`: Remove `ViewMode::Dsm`
- `src/tui/results/navigation.rs`: Remove 'm' key handler and DSM navigation
- `src/tui/results/nav_state.rs`: Remove DSM-related transitions and state
- `src/tui/results/app.rs`: Remove DSM rendering case
- `src/output/mod.rs`: Remove DSM output handling
- `src/io/writers/mod.rs`: Remove DSM writer
- `src/analysis/mod.rs`: Remove `dsm` module declaration

### Architecture Changes

```
Before:
src/analysis/dsm.rs (780 lines, standalone)
src/tui/results/dsm_view.rs (standalone visualization)
src/io/writers/dsm.rs (output format)
src/output/dsm.rs (output functions)

After:
src/organization/layering.rs (~50 lines, integrated)
  └─ Used by: god_object/scoring.rs, god_object/recommender.rs
```

### Data Structures

```rust
// New in src/organization/layering.rs
pub struct ModuleDependency {
    pub from_module: String,
    pub to_module: String,
}

pub struct LayeringAnalysis {
    pub score: f64,
    pub backward_dep_count: usize,
    pub problematic_modules: Vec<(String, usize)>,
}

// Modified in src/organization/god_object/types.rs
pub struct GodObjectAnalysis {
    // Existing fields...
    pub layering_impact: Option<LayeringImpact>,
}

pub struct LayeringImpact {
    pub backward_deps_caused: usize,
    pub affected_modules: Vec<String>,
    pub estimated_improvement: f64,
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/organization/god_object/` - scoring and recommendations
  - `src/cli/args.rs` - OutputFormat enum
  - `src/tui/results/` - view modes and navigation
  - `src/output/` - output format handling
  - `src/io/writers/` - output writers

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_layering_returns_1() {
        let deps = vec![
            ModuleDependency { from: "b".into(), to: "a".into() },
            ModuleDependency { from: "c".into(), to: "a".into() },
            ModuleDependency { from: "c".into(), to: "b".into() },
        ];
        let result = compute_layering_score(&deps);
        assert_eq!(result.score, 1.0);
        assert_eq!(result.backward_dep_count, 0);
    }

    #[test]
    fn all_backward_deps_returns_0() {
        let deps = vec![
            ModuleDependency { from: "a".into(), to: "b".into() },
            ModuleDependency { from: "a".into(), to: "c".into() },
            ModuleDependency { from: "b".into(), to: "c".into() },
        ];
        let result = compute_layering_score(&deps);
        assert!(result.score < 0.5);
        assert!(result.backward_dep_count > 0);
    }

    #[test]
    fn path_to_module_extracts_correctly() {
        assert_eq!(path_to_module("src/analysis/dsm.rs"), "analysis");
        assert_eq!(path_to_module("src/cli/args.rs"), "cli");
        assert_eq!(path_to_module("src/main.rs"), "root");
    }
}
```

### Integration Tests

- Verify god object detection still works after DSM removal
- Verify layering score appears in god object recommendations
- Verify `--format dsm` produces helpful error message (removed format)

### Regression Tests

- All existing god object tests must pass
- TUI must function without DSM view
- All output formats except DSM must work

## Documentation Requirements

- **Code Documentation**: Document `layering.rs` with examples
- **ARCHITECTURE.md**: Update to remove DSM section, add layering integration note
- **README.md**: No changes needed (DSM not documented there)

## Implementation Notes

### Layering Score Calculation Algorithm

1. **Build module dependency graph** from file dependencies
2. **Attempt topological sort** - if successful, modules are already well-ordered
3. **If cycles exist**, use heuristic ordering (by out-degree or alphabetical)
4. **Count dependencies**:
   - Lower triangle (row > col in ordered matrix) = forward deps (good)
   - Upper triangle (row < col) = backward deps (bad)
5. **Score = lower_triangle_deps / total_deps**

### Why Module Level, Not File Level

- Intra-module deps are expected (`analysis/dsm.rs` → `analysis/call_graph.rs`)
- Cross-module backward deps are the architectural problem
- Module boundaries are where layering violations matter
- Reduces noise from legitimate sibling file dependencies

### Threshold Rationale

- **0.75 overall**: Achievable for most codebases, flags significant issues
- **≥3 backward deps**: One or two can happen; three+ indicates a hub file
- **15% boost**: Noticeable but not dominant; layering is supporting signal

## Migration and Compatibility

### Breaking Changes

- `--format dsm` will error with "DSM format removed, layering integrated into recommendations"
- TUI 'm' key will do nothing (or show message if pressed)

### Migration Path

Users who relied on DSM output:
1. View layering information in god object recommendations
2. Use `--format json` and parse layering data programmatically
3. For full DSM visualization, consider external tools (Lattix, Structure101)

## Estimated Complexity

- **Lines removed**: ~1000 (DSM code)
- **Lines added**: ~150 (layering.rs + integration)
- **Net reduction**: ~850 lines
- **Risk**: Low - DSM is standalone, removal is clean
