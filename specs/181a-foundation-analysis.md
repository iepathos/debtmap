---
number: 181a
title: "God Object Refactor: Phase 1 - Foundation & Analysis"
category: optimization
priority: high
status: draft
dependencies: []
parent_spec: 181
phase: 1
estimated_time: "1 day"
created: 2025-12-03
updated: 2025-12-03
---

# Specification 181a: God Object Refactor - Phase 1: Foundation & Analysis

**Category**: optimization
**Priority**: high
**Status**: draft
**Parent Spec**: 181 (Refactor God Object Detection Module)
**Phase**: 1 of 9
**Estimated Time**: 1 day
**Dependencies**: None

## Context

This is **Phase 1** of the God Object Detection Module refactoring (spec 181). This phase focuses on analysis and planning before any code changes.

The god object detection system currently has:
- `src/organization/god_object_detector.rs` - 4,362 lines
- `src/organization/god_object_analysis.rs` - 3,304 lines
- `src/organization/god_object_metrics.rs` - 367 lines
- `src/organization/god_object/ast_visitor.rs` - 365 lines (already modularized)
- `src/organization/god_object/metrics.rs` - 349 lines (already modularized)
- `src/organization/god_object/mod.rs` - 15 lines

## Objective

Perform comprehensive analysis and create a detailed refactoring plan before making any code changes. This phase is **read-only** - no code modifications.

## Requirements

### 1. Audit Existing Module Structure

**Files to analyze**:
- `src/organization/god_object/ast_visitor.rs` (365 lines)
- `src/organization/god_object/metrics.rs` (349 lines)
- `src/organization/god_object/mod.rs` (15 lines)

**Questions to answer**:
- What functionality do these modules provide?
- Are they following Stillwater principles (pure core, imperative shell)?
- Do they need refactoring or can they be preserved?
- What's the current dependency structure?

**Output**: Section in `REFACTORING_PLAN.md` documenting existing module structure

### 2. Map Public API Exports

**Files to analyze**:
- `src/organization/god_object/mod.rs` (current exports, lines 30-57 per spec)
- All 6 test files in `tests/god_object_*.rs`

**Questions to answer**:
- What types/functions are currently exported?
- What do the tests import and use?
- Which APIs are critical for backward compatibility?
- Are there any deprecated exports?

**Output**: Complete list of public API exports that must be preserved

### 3. Create Function Classification Map

**Files to analyze**:
- `src/organization/god_object_detector.rs` (4,362 lines)
- `src/organization/god_object_analysis.rs` (3,304 lines)

**For each function, document**:
1. Function name and signature
2. Classification: Pure Computation | I/O Operation | Orchestration | Mixed (needs splitting)
3. Dependencies (what other functions/types it uses)
4. Target module assignment (types, thresholds, predicates, scoring, classifier, recommender, detector)
5. Line count and complexity estimate

**Output**: Complete function classification table in `REFACTORING_PLAN.md`

### 4. Identify Pure vs Impure Code

**Analysis criteria**:
- **Pure**: Takes inputs, returns outputs, no side effects, deterministic
- **Impure**: Performs I/O, mutates state, non-deterministic

**For each file**:
- Identify purely functional sections
- Identify I/O operations (AST traversal, file operations, etc.)
- Identify mixed functions that need splitting

**Output**: Purity analysis section with specific line ranges

### 5. Group Functions by Responsibility

**Target modules** (from spec 181):
- `types.rs` - All data structures (~200 lines target)
- `thresholds.rs` - Constants and configuration (~100 lines)
- `predicates.rs` - Pure detection predicates (~150 lines)
- `scoring.rs` - Pure scoring algorithms (~200 lines)
- `classifier.rs` - Pure classification logic (~200 lines)
- `recommender.rs` - Pure recommendation generation (~250 lines)
- `detector.rs` - Orchestration layer (~250 lines)

**For each target module**:
- List functions that should be moved there
- Estimate line count
- Document dependencies on other modules
- Identify any circular dependency risks

**Output**: Module assignment table with size estimates

### 6. Write Benchmarks for Critical Paths

**Critical paths to benchmark**:
- God object score calculation
- Classification determination
- Method grouping by responsibility
- Module split recommendation generation
- Full analysis pipeline (orchestration)

**Deliverable**:
- Create `benches/god_object_bench.rs`
- Benchmark baseline performance before refactoring
- Document baseline numbers in `REFACTORING_PLAN.md`

### 7. Create Dependency Graph

**Analysis**:
- Map current dependencies between functions
- Identify circular dependencies
- Design acyclic dependency graph for new modules
- Verify dependency direction: types → utils → domain → orchestration

**Output**: Dependency graph diagram in `REFACTORING_PLAN.md`

## Acceptance Criteria

### Documentation Created
- [ ] `REFACTORING_PLAN.md` exists with all required sections
- [ ] Existing module structure documented
- [ ] Complete public API export list
- [ ] Function classification map (all functions categorized)
- [ ] Purity analysis (pure vs impure code identified)
- [ ] Module assignment table (function → target module mapping)
- [ ] Dependency graph (current and proposed)
- [ ] Benchmark baseline numbers documented

### Benchmarks Created
- [ ] `benches/god_object_bench.rs` exists
- [ ] Benchmarks for critical paths defined
- [ ] Baseline performance numbers captured
- [ ] Benchmark output saved to `REFACTORING_PLAN.md`

### Quality Checks
- [ ] No code modifications made (read-only phase)
- [ ] All analysis based on actual code (not assumptions)
- [ ] Line counts and estimates verified
- [ ] No circular dependencies in proposed structure
- [ ] Size estimates for all new modules < 300 lines

### Validation
- [ ] All 6 test files identified and analyzed
- [ ] All public exports from mod.rs documented
- [ ] Every function in god_object_detector.rs classified
- [ ] Every function in god_object_analysis.rs classified
- [ ] Acyclic dependency graph verified

## Deliverables

### 1. REFACTORING_PLAN.md

Must include:
```markdown
# God Object Refactoring Plan

## Executive Summary
- Current state (line counts, structure)
- Target state (new module structure)
- Risk assessment
- Estimated effort per phase

## Existing Module Analysis
- ast_visitor.rs analysis
- metrics.rs analysis
- mod.rs current exports

## Public API Inventory
- All exported types
- All exported functions
- Test dependencies
- Backward compatibility requirements

## Function Classification Map
| Function | Source File | Type | Line Count | Complexity | Target Module | Notes |
|----------|-------------|------|------------|------------|---------------|-------|
| calculate_god_object_score | god_object_analysis.rs | Pure | 25 | Low | scoring.rs | Deterministic math |
| ... | ... | ... | ... | ... | ... | ... |

## Purity Analysis
- Pure functions (list with line ranges)
- I/O operations (list with line ranges)
- Mixed functions requiring splitting (list with plan)

## Module Assignment Table
| Target Module | Functions | Est. Lines | Dependencies | Risk |
|---------------|-----------|------------|--------------|------|
| types.rs | [list] | ~200 | None | Low |
| scoring.rs | [list] | ~200 | types.rs | Low |
| ... | ... | ... | ... | ... |

## Dependency Graph
### Current Dependencies
[Diagram or text representation]

### Proposed Dependencies
types.rs (foundation)
  ↑
  ├── thresholds.rs
  ├── predicates.rs
  ├── scoring.rs
  etc.

## Benchmark Baselines
- calculate_god_object_score: X ns
- determine_confidence: Y ns
- full_analysis_pipeline: Z ns

## Risk Assessment
- High risk areas
- Circular dependency concerns
- Performance concerns
- Test compatibility concerns

## Phase 2 Readiness Checklist
- [ ] All functions classified
- [ ] Module boundaries clear
- [ ] No circular dependencies
- [ ] Benchmarks established
```

### 2. benches/god_object_bench.rs

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use debtmap::organization::god_object_analysis::*;

fn bench_god_object_scoring(c: &mut Criterion) {
    let thresholds = GodObjectThresholds::default();

    c.bench_function("calculate_god_object_score", |b| {
        b.iter(|| {
            calculate_god_object_score(
                black_box(25),
                black_box(6),
                black_box(&thresholds)
            )
        })
    });

    // Additional benchmarks for other critical paths
}

criterion_group!(benches, bench_god_object_scoring);
criterion_main!(benches);
```

## Testing Strategy

### No Code Changes
- This phase is read-only analysis
- No tests to run (except benchmarks for baseline)
- All existing tests should continue to pass (no changes made)

### Validation
- Run benchmarks: `cargo bench --bench god_object_bench`
- Verify no code modifications: `git status` (should be clean except for docs/benches)
- Document all findings

## Implementation Notes

### Tools to Use
- `cargo tree` - Analyze current dependencies
- `tokei` or `cloc` - Verify line counts
- `cargo expand` - Understand macro expansions if needed
- `rg` (ripgrep) - Search for function usage patterns
- `cargo clippy` - Identify complexity issues

### Analysis Approach
1. Start with existing modularized code (ast_visitor.rs, metrics.rs)
2. Map all public exports from mod.rs
3. Read test files to understand API usage
4. Systematically go through god_object_detector.rs (top to bottom)
5. Systematically go through god_object_analysis.rs (top to bottom)
6. Build function classification table as you go
7. Create dependency graph
8. Write benchmarks
9. Compile all findings into REFACTORING_PLAN.md

### Red Flags to Watch For
- Functions > 100 lines (may need splitting before moving)
- Circular dependencies between proposed modules
- Heavy coupling between pure and impure code
- Performance-critical paths (need benchmarking)
- Test dependencies on internal implementation details

## Success Metrics

### Quantitative
- ✅ 100% of functions classified (0 unknowns)
- ✅ All public API exports documented
- ✅ All 6 test files analyzed
- ✅ Baseline benchmarks established
- ✅ Dependency graph shows no cycles

### Qualitative
- ✅ Clear understanding of pure vs impure code
- ✅ Confidence in module boundaries
- ✅ No surprises or unknowns remaining
- ✅ Ready to start Phase 2 (Extract Types)

## Next Phase

**Phase 2** (Spec 181b): Extract Types & Thresholds
- Create `types.rs` with all data structures
- Create `thresholds.rs` with constants
- Update existing files to use new modules
- Commit and verify tests pass

## References

- Parent spec: `specs/181-split-god-object-detector-module.md`
- Stillwater Philosophy: `../stillwater/PHILOSOPHY.md`
- Project Guidelines: `CLAUDE.md`
