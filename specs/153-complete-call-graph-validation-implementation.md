---
number: 153
title: Complete Call Graph Validation Implementation
category: optimization
priority: critical
status: draft
dependencies: [151, 152]
created: 2025-10-24
---

# Specification 153: Complete Call Graph Validation Implementation

**Category**: optimization
**Priority**: critical
**Status**: draft
**Dependencies**: Spec 151 (Improve Call Graph Orphaned Node Detection), Spec 152 (Improve Trait Method Call Graph Resolution)

## Context

**Current State**:
Specs 151 and 152 have been **partially implemented** but critical gaps remain:

**What's Working (Spec 151)**:
- ✅ Type distinction: `IsolatedFunction` vs `UnreachableFunction`
- ✅ Basic validation structure in place
- ✅ `ValidationStatistics` data structure exists

**Critical Gaps**:
1. **Health Score Still 0/100** ❌
   - Current: 11,607 isolated/unreachable functions
   - Issue: Health score calculation too harsh (`-10 points per issue`)
   - Target: 70-85/100 health score

2. **Statistics Not Displayed** ❌
   - `ValidationStatistics` structure exists but not shown in output
   - Users can't see: total functions, entry points, leaf count, etc.

3. **Leaf Functions Not Separated** ❌
   - Spec says: Leaf functions (has callers, no callees) should be INFO, not errors
   - Current: May still be in structural issues or not tracked

4. **Entry Point Detection Incomplete** ❌
   - Only 219 functions (1.9%) reclassified from 11,826 to 11,607
   - Missing: `Default::default()`, `new()`, trait impls, public APIs

5. **Trait Resolution Not Active (Spec 152)** ❌
   - Expected 80% reduction in trait impl false positives
   - Current: Minimal improvement suggests trait resolution not working

**Impact**:
- Health score unusable (0/100)
- False positives still overwhelming (11,607 vs target < 500)
- Users can't trust call graph validation
- Dependency scores show "0 callers" for legitimately called functions

## Objective

Complete the implementation of specs 151 and 152 to achieve:
1. Health score 70-85/100 for typical Rust projects
2. Orphaned node false positives < 500 (95% reduction from 11,826)
3. Validation statistics displayed in output
4. Leaf functions properly categorized as INFO
5. Entry point detection working for common patterns
6. Trait method resolution reducing false positives by 80%

## Requirements

### Functional Requirements

1. **Fix Health Score Calculation** (Spec 151):
   ```rust
   // Current (too harsh):
   score = score.saturating_sub(structural_issues.len() as u32 * 10);  // -10 per issue

   // Required (spec 151 formula):
   score = score.saturating_sub(dangling_edge_count * 10);      // Critical
   score = score.saturating_sub(duplicate_count * 5);           // Serious
   score = score.saturating_sub(unreachable_count * 1);         // Moderate
   score = score.saturating_sub((isolated_count as f32 * 0.5) as u32);  // Low
   ```

2. **Display Validation Statistics**:
   - Show in validation report output
   - Include: total functions, entry points, leaf functions, unreachable, isolated
   - Format: Clear, human-readable summary

3. **Implement Leaf Function Tracking**:
   - Detect: `has_callers && !has_callees`
   - Store in: `ValidationInfo::LeafFunction`
   - Not count as structural issue
   - Show count in statistics

4. **Complete Entry Point Detection** (Spec 151):
   - Test and fix existing patterns: main, test functions
   - Add missing: pub functions in lib.rs, benchmarks, examples
   - Add trait implementations: `Default::default()`, `Clone::clone()`, etc.
   - Add constructors: `new()`, `builder()`, `with_*()`, `create()`

5. **Activate Trait Method Resolution** (Spec 152):
   - Integrate `TraitRegistry::detect_common_trait_patterns()` into analysis pipeline
   - Mark trait implementations as entry points
   - Connect trait method calls to implementations
   - Display resolution count in verbose mode

6. **Fix Dependency Score for Callable Functions**:
   - Functions with callers should NOT show "Dependency Score: 5.0 (0 callers)"
   - Integrate call graph data into dependency scoring
   - Show actual caller count in recommendations

### Non-Functional Requirements

- **Backward Compatibility**: Existing validation API unchanged
- **Performance**: < 100ms additional overhead for full implementation
- **Accuracy**: Health score reflects actual call graph quality
- **Usability**: Statistics clearly communicate validation results

## Acceptance Criteria

### Health Score (Priority 1)
- [ ] Health score calculation uses weighted formula from spec 151
- [ ] Health score for debtmap's own codebase: 70-85/100
- [ ] Dangling edges: -10 points each
- [ ] Duplicates: -5 points each
- [ ] Unreachable functions: -1 point each
- [ ] Isolated functions: -0.5 points each
- [ ] Info items (leaf functions): 0 points

### Statistics Display (Priority 1)
- [ ] `ValidationStatistics` populated during validation
- [ ] Statistics displayed after "Call Graph Validation Report" header
- [ ] Shows: total_functions, entry_points, leaf_functions, unreachable, isolated, recursive
- [ ] Format is clear and human-readable

### Leaf Function Handling (Priority 1)
- [ ] Leaf functions detected: `has_callers && !has_callees`
- [ ] Added to `ValidationInfo::LeafFunction` (not structural issues)
- [ ] Count shown in statistics
- [ ] Not penalized in health score

### Entry Point Detection (Priority 2)
- [ ] Main functions excluded from orphan detection
- [ ] Test functions (test_*, #[test]) excluded
- [ ] Benchmark functions (bench_*, #[bench]) excluded
- [ ] Functions in examples/ directory excluded
- [ ] Trait implementations (Default, Clone, etc.) excluded
- [ ] Constructor patterns (new(), builder()) excluded
- [ ] Public functions in lib.rs excluded (heuristic)

### Trait Resolution (Priority 2)
- [ ] `TraitRegistry::detect_common_trait_patterns()` called in analysis pipeline
- [ ] Common traits resolved: Default, Clone, From, Into, new()
- [ ] Trait implementations marked as entry points
- [ ] Resolution count displayed in verbose mode: "🔗 Resolved X trait method calls"
- [ ] False positives reduced by at least 50% (target: 80%)

### Integration (Priority 3)
- [ ] Dependency scoring uses call graph caller count
- [ ] Functions with callers show actual count in recommendations
- [ ] Verbose output explains entry point classifications
- [ ] Tests verify health score improvements

## Technical Details

### Implementation Approach

**Phase 1: Fix Health Score Calculation (Quick Win)**

Location: `src/analyzers/call_graph/validation.rs`

```rust
impl ValidationReport {
    /// Calculate health score with refined weighting (spec 151)
    fn calculate_health_score(&mut self) {
        let mut score: u32 = 100;

        // Count issue types separately
        let mut unreachable_count = 0;
        let mut isolated_count = 0;
        let mut dangling_edge_count = 0;
        let mut duplicate_count = 0;

        for issue in &self.structural_issues {
            match issue {
                StructuralIssue::UnreachableFunction { .. } => unreachable_count += 1,
                StructuralIssue::IsolatedFunction { .. } => isolated_count += 1,
                StructuralIssue::DanglingEdge { .. } => dangling_edge_count += 1,
                StructuralIssue::DuplicateNode { .. } => duplicate_count += 1,
            }
        }

        // Apply weighted penalties (spec 151 formula)
        score = score.saturating_sub(dangling_edge_count * 10);      // Critical
        score = score.saturating_sub(duplicate_count * 5);           // Serious
        score = score.saturating_sub(unreachable_count * 1);         // Moderate
        score = score.saturating_sub((isolated_count as f32 * 0.5) as u32);  // Low
        score = score.saturating_sub(self.warnings.len() as u32 * 2);  // Minor

        // Info items don't affect score

        self.health_score = score;
    }
}
```

**Phase 2: Display Statistics**

Location: `src/commands/analyze.rs` (in `handle_call_graph_diagnostics()`)

```rust
fn format_validation_report(report: &ValidationReport) -> String {
    let mut output = String::new();

    output.push_str("=== Call Graph Validation Report ===\n");
    output.push_str(&format!("Health Score: {}/100\n", report.health_score));

    // NEW: Display statistics
    output.push_str("\n📊 Statistics:\n");
    output.push_str(&format!("  Total Functions: {}\n", report.statistics.total_functions));
    output.push_str(&format!("  Entry Points: {}\n", report.statistics.entry_points));
    output.push_str(&format!("  Leaf Functions: {} (has callers, no callees)\n",
                            report.statistics.leaf_functions));
    output.push_str(&format!("  Unreachable: {} (no callers, has callees)\n",
                            report.statistics.unreachable_functions));
    output.push_str(&format!("  Isolated: {} (no callers, no callees)\n",
                            report.statistics.isolated_functions));

    if report.statistics.recursive_functions > 0 {
        output.push_str(&format!("  Recursive: {}\n", report.statistics.recursive_functions));
    }

    output.push_str(&format!("\nStructural Issues: {}\n", report.structural_issues.len()));
    output.push_str(&format!("Warnings: {}\n", report.warnings.len()));

    // ... rest of output

    output
}
```

**Phase 3: Implement Leaf Function Detection**

Location: `src/analyzers/call_graph/validation.rs` (in `check_orphaned_nodes()`)

```rust
impl CallGraphValidator {
    fn check_orphaned_nodes(call_graph: &CallGraph, report: &mut ValidationReport) {
        for function in call_graph.get_all_functions() {
            let has_callers = !call_graph.get_callers(function).is_empty();
            let has_callees = !call_graph.get_callees(function).is_empty();
            let is_entry_point = Self::is_entry_point(function, call_graph);
            let is_self_referential = Self::is_self_referential(function, call_graph);

            report.statistics.total_functions += 1;

            // LEAF FUNCTION: Has callers but no callees (NORMAL - not an issue)
            if has_callers && !has_callees {
                report.statistics.leaf_functions += 1;
                report.info.push(ValidationInfo::LeafFunction {
                    function: function.clone(),
                    caller_count: call_graph.get_callers(function).len(),
                });
                continue;  // NOT a structural issue
            }

            // Entry points
            if is_entry_point {
                report.statistics.entry_points += 1;
                continue;  // NOT an issue
            }

            // Self-referential
            if is_self_referential {
                report.statistics.recursive_functions += 1;
                report.info.push(ValidationInfo::SelfReferentialFunction {
                    function: function.clone(),
                });
                continue;  // NOT an issue
            }

            // ISOLATED: No callers, no callees (true orphan)
            if !has_callers && !has_callees {
                report.statistics.isolated_functions += 1;
                report.structural_issues.push(StructuralIssue::IsolatedFunction {
                    function: function.clone(),
                });
                continue;
            }

            // UNREACHABLE: No callers but has callees (dead code)
            if !has_callers && has_callees {
                report.statistics.unreachable_functions += 1;
                report.structural_issues.push(StructuralIssue::UnreachableFunction {
                    function: function.clone(),
                    reason: UnreachableReason::NoCallers,
                });
            }
        }
    }
}
```

**Phase 4: Enhance Entry Point Detection**

Location: `src/analyzers/call_graph/validation.rs`

```rust
impl CallGraphValidator {
    /// Check if a function is an entry point (spec 151)
    fn is_entry_point(function: &FunctionId, call_graph: &CallGraph) -> bool {
        // Main function
        if function.name == "main" {
            return true;
        }

        // Test functions
        if function.name.starts_with("test_")
            || function.name.contains("::test_")
            || function.name.starts_with("#[test]") {
            return true;
        }

        // Benchmark functions
        if function.name.starts_with("bench_")
            || function.name.contains("::bench_")
            || function.name.starts_with("#[bench]") {
            return true;
        }

        // Functions in examples/ directory
        if function.file.to_str().map_or(false, |s| s.contains("/examples/")) {
            return true;
        }

        // Functions in lib.rs (library entry points)
        let file_name = function.file.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if file_name == "lib.rs" {
            // Heuristic: short names without :: are likely public exports
            if !function.name.contains("::") && function.name.len() < 30 {
                return true;
            }
        }

        // Trait implementations - common patterns
        if function.name.contains("::") {
            let trait_methods = [
                "default", "new", "clone", "clone_box", "clone_from",
                "from", "into", "fmt", "display", "debug",
                "drop", "deref", "deref_mut", "hash", "eq",
                "builder", "create", "with_", "try_from", "try_into"
            ];

            let name_lower = function.name.to_lowercase();
            if trait_methods.iter().any(|&method| name_lower.contains(method)) {
                return true;
            }
        }

        // Constructor patterns (without ::)
        if function.name == "new"
            || function.name == "builder"
            || function.name == "create"
            || function.name.starts_with("with_") {
            return true;
        }

        false
    }
}
```

**Phase 5: Activate Trait Resolution (Spec 152)**

Location: `src/commands/analyze.rs` (in `handle_analyze()`)

```rust
async fn handle_analyze(args: AnalyzeArgs) -> Result<()> {
    // ... existing analysis code ...

    // Build call graph
    let mut call_graph = build_call_graph(&analysis_results)?;

    // NEW: Trait method resolution (spec 152)
    let trait_registry = build_trait_registry(&analysis_results)?;

    // Detect and mark trait implementations as entry points
    trait_registry.detect_common_trait_patterns(&mut call_graph);

    // Resolve trait method calls to implementations
    let resolved_count = trait_registry.resolve_trait_method_calls(&mut call_graph);

    if args.verbose_call_graph || args.verbose >= 1 {
        eprintln!("🔗 Resolved {} trait method calls", resolved_count);
        eprintln!("🎯 Marked {} trait implementations as callable",
                 trait_registry.get_implementation_count());
    }

    // Validate call graph
    if args.validate_call_graph {
        let report = CallGraphValidator::validate(&call_graph);
        print_validation_report(&report, args.call_graph_stats);
    }

    // ... rest of analysis ...
}
```

**Phase 6: Fix Dependency Scoring**

Location: `src/priority/scoring/*.rs` (wherever dependency score is calculated)

```rust
fn calculate_dependency_score(
    function: &FunctionId,
    call_graph: &CallGraph,
    max_score: f64
) -> f64 {
    let caller_count = call_graph.get_callers(function).len();

    if caller_count == 0 {
        // Check if it's an entry point or trait impl
        let role = call_graph.get_function_role(function);
        match role {
            FunctionRole::EntryPoint { .. }
            | FunctionRole::TraitEntryPoint { .. }
            | FunctionRole::Constructor => {
                // Entry points expected to have no direct callers
                return max_score * 0.5;  // Medium score, not penalty
            }
            _ => {
                // Truly uncalled
                return max_score;  // High score = high priority
            }
        }
    }

    // Has callers - use inverse relationship
    // More callers = higher impact = lower score (already well-tested)
    let score = max_score / (1.0 + (caller_count as f64).ln());
    score.max(0.0).min(max_score)
}
```

### Architecture Changes

**Data Flow**:
```
Analysis Pipeline
    ↓
Build Call Graph
    ↓
Trait Registry Construction  ← NEW
    ↓
Trait Pattern Detection      ← NEW
    ↓
Trait Call Resolution        ← NEW
    ↓
Call Graph Validation
    ↓
Statistics Display           ← NEW (enhanced)
    ↓
Dependency Scoring           ← FIXED
```

**Modified Files**:
1. `src/analyzers/call_graph/validation.rs` - Health score formula, leaf detection, entry points
2. `src/commands/analyze.rs` - Statistics display, trait resolution integration
3. `src/priority/scoring/*.rs` - Dependency scoring with call graph data
4. `src/analysis/call_graph/trait_registry.rs` - Hook up to analysis pipeline

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_score_with_weighted_penalties() {
        let mut report = ValidationReport::new();

        // Add 100 isolated functions (should be 0.5 points each = 50 total)
        for i in 0..100 {
            report.structural_issues.push(StructuralIssue::IsolatedFunction {
                function: FunctionId::new("test.rs", &format!("fn_{}", i), i),
            });
        }

        report.calculate_health_score();

        // Should be 100 - 50 = 50
        assert!(report.health_score >= 40 && report.health_score <= 60,
                "Expected health score ~50, got {}", report.health_score);
    }

    #[test]
    fn test_leaf_functions_not_penalized() {
        let mut call_graph = CallGraph::new();
        let leaf = FunctionId::new("test.rs", "utility_fn", 10);
        let caller = FunctionId::new("test.rs", "main", 5);

        call_graph.add_function(leaf.clone());
        call_graph.add_function(caller.clone());
        call_graph.add_call(caller, leaf.clone());

        let report = CallGraphValidator::validate(&call_graph);

        // Leaf should be in info, not structural issues
        assert!(!report.structural_issues.iter().any(|i|
            matches!(i, StructuralIssue::IsolatedFunction { .. })
        ));
        assert!(report.info.iter().any(|i|
            matches!(i, ValidationInfo::LeafFunction { .. })
        ));

        // Health score should be high (no real issues)
        assert!(report.health_score >= 95);
    }

    #[test]
    fn test_trait_impl_excluded_from_orphans() {
        let mut call_graph = CallGraph::new();
        let default_impl = FunctionId::new("src/config.rs", "Config::default", 42);

        call_graph.add_function(default_impl.clone());
        // No calls added (implementation detail)

        // Mark as trait entry point
        call_graph.mark_as_trait_entry_point(default_impl.clone(), "Default::default");

        let report = CallGraphValidator::validate(&call_graph);

        // Should NOT be in structural issues
        assert!(!report.structural_issues.iter().any(|i|
            matches!(i, StructuralIssue::IsolatedFunction { function } if function == &default_impl)
        ));

        // Statistics should count it as entry point
        assert_eq!(report.statistics.entry_points, 1);
    }

    #[test]
    fn test_statistics_populated() {
        let mut call_graph = CallGraph::new();

        // Add various function types
        let main_fn = FunctionId::new("main.rs", "main", 1);
        let leaf_fn = FunctionId::new("util.rs", "helper", 10);
        let caller_fn = FunctionId::new("util.rs", "processor", 5);
        let isolated_fn = FunctionId::new("old.rs", "unused", 100);

        call_graph.add_function(main_fn.clone());
        call_graph.add_function(leaf_fn.clone());
        call_graph.add_function(caller_fn.clone());
        call_graph.add_function(isolated_fn.clone());

        call_graph.add_call(main_fn.clone(), caller_fn.clone());
        call_graph.add_call(caller_fn, leaf_fn);

        let report = CallGraphValidator::validate(&call_graph);

        assert_eq!(report.statistics.total_functions, 4);
        assert_eq!(report.statistics.entry_points, 1);  // main
        assert_eq!(report.statistics.leaf_functions, 1);  // helper
        assert_eq!(report.statistics.isolated_functions, 1);  // unused
    }

    #[test]
    fn test_real_project_health_score() {
        // Integration test on debtmap's own codebase
        let analysis = run_analysis_on_debtmap_project();
        let call_graph = build_call_graph(&analysis).unwrap();

        // Apply trait resolution
        let trait_registry = build_trait_registry(&analysis).unwrap();
        trait_registry.detect_common_trait_patterns(&mut call_graph);

        let report = CallGraphValidator::validate(&call_graph);

        // After full implementation, health score should be 70-85
        assert!(
            report.health_score >= 70 && report.health_score <= 85,
            "Health score should be 70-85, got {}. Issues: {} isolated, {} unreachable",
            report.health_score,
            report.statistics.isolated_functions,
            report.statistics.unreachable_functions
        );

        // Isolated functions should be < 500 (95% reduction from 11,826)
        assert!(
            report.statistics.isolated_functions < 500,
            "Expected < 500 isolated functions, got {}",
            report.statistics.isolated_functions
        );
    }
}
```

## Dependencies

- **Prerequisites**: Spec 151 (partial), Spec 152 (partial)
- **Affected Components**:
  - `src/analyzers/call_graph/validation.rs` - Health score, leaf detection, entry points
  - `src/commands/analyze.rs` - Statistics display, trait resolution integration
  - `src/analysis/call_graph/trait_registry.rs` - Pipeline integration
  - `src/priority/scoring/*.rs` - Dependency scoring
- **External Dependencies**: None

## Documentation Requirements

### Code Documentation

- Document health score calculation formula with examples
- Explain entry point detection heuristics and limitations
- Document trait resolution integration points
- Add doctests for validation scenarios

### User Documentation

```markdown
## Call Graph Validation

### Health Score Interpretation

**Scoring Formula**:
- Dangling edges: -10 points (critical - graph corruption)
- Duplicate nodes: -5 points (serious - data integrity)
- Unreachable functions: -1 point (moderate - dead code)
- Isolated functions: -0.5 points (low - might be WIP)
- Warnings: -2 points (minor issues)

**Health Ranges**:
- 90-100: Excellent - minimal issues
- 70-89: Good - some cleanup needed
- 50-69: Fair - significant dead code
- 0-49: Poor - major structural problems

### Statistics Explained

```
📊 Statistics:
  Total Functions: 4,991        # All functions in codebase
  Entry Points: 1,234           # Main, tests, pub APIs, traits
  Leaf Functions: 2,156         # Has callers, no callees (normal)
  Unreachable: 123              # No callers, has callees (dead code)
  Isolated: 89                  # No callers, no callees (orphans)
  Recursive: 45                 # Self-referential functions
```

**Leaf Functions**: Normal utility functions, getters, constructors - NOT problematic

**Unreachable Functions**: Dead code that depends on other code - should be removed

**Isolated Functions**: True orphans - completely disconnected code

### Improving Health Score

1. **Remove isolated functions**: Truly unused code
2. **Remove unreachable functions**: Dead code with dependencies
3. **Fix dangling edges**: Broken references (critical)
4. **Address warnings**: High fan-in/fan-out, suspicious patterns

### Trait Resolution

Automatically detects and excludes from orphan detection:
- `Default::default()` implementations
- `Clone::clone()`, `Clone::clone_box()`
- `From::from()`, `Into::into()`
- Constructor patterns: `new()`, `builder()`, `with_*()`

Enable verbose mode to see resolution details:
```bash
debtmap analyze . --validate-call-graph -v
🔗 Resolved 89 trait method calls
🎯 Marked 67 trait implementations as callable
```
```

## Implementation Notes

### Phase Order (by Priority)

1. **Phase 1** (Quick Win): Fix health score calculation
   - 1 hour, immediate health score improvement
   - File: `validation.rs::calculate_health_score()`

2. **Phase 2** (High Value): Display statistics
   - 2 hours, provides visibility into validation
   - File: `analyze.rs::format_validation_report()`

3. **Phase 3** (Core Fix): Leaf function detection
   - 3 hours, eliminates major false positive category
   - File: `validation.rs::check_orphaned_nodes()`

4. **Phase 4** (Major Impact): Entry point detection
   - 4 hours, reduces false positives significantly
   - File: `validation.rs::is_entry_point()`

5. **Phase 5** (Spec 152): Trait resolution integration
   - 5 hours, integrates trait registry into pipeline
   - File: `analyze.rs::handle_analyze()`

6. **Phase 6** (Polish): Fix dependency scoring
   - 2 hours, improves recommendation accuracy
   - Files: `priority/scoring/*.rs`

**Total Estimated Effort**: 17 hours (2-3 days)

### Testing Approach

1. **Unit tests**: Each phase independently tested
2. **Integration test**: Full pipeline on debtmap codebase
3. **Regression test**: Health score >= 70 after all phases
4. **Validation**: Orphan count < 500 after completion

### Rollout Strategy

1. Deploy Phase 1-2 first (statistics display)
2. Monitor health score improvements
3. Deploy Phase 3-4 (leaf + entry point)
4. Validate 50%+ reduction in false positives
5. Deploy Phase 5-6 (trait resolution + scoring)
6. Final validation: Health score 70-85/100

## Migration and Compatibility

### Backward Compatibility

- All changes are improvements to existing validation
- No breaking API changes
- Existing validation still works, just better

### Performance Impact

- Health score calculation: +5ms (same complexity, different weights)
- Statistics display: +10ms (formatting only)
- Leaf detection: +20ms (one extra pass, O(n))
- Entry point detection: +30ms (string checks, O(n))
- Trait resolution: +50ms (already implemented in spec 152)

**Total**: ~115ms additional overhead (acceptable for < 150ms target)

## Success Metrics

| Metric | Before | Target | Measure |
|--------|--------|--------|---------|
| Health Score | 0/100 | 70-85/100 | Validation report |
| Orphaned Nodes | 11,826 | < 500 | Statistics count |
| False Positive Rate | 98% | < 5% | Manual review |
| Leaf Functions Tracked | 0 | ~2,000 | Statistics count |
| Entry Points Detected | ~100 | ~1,200 | Statistics count |
| Trait Impls Resolved | 0 | ~500 | Resolution count |

**Success Criteria**:
- Health score >= 70/100 on debtmap codebase
- < 500 isolated/unreachable functions total
- Statistics displayed in validation output
- No regressions in existing functionality

## Open Questions

1. **Entry Point Heuristic Accuracy**: How to reliably detect public functions without AST visibility?
   - Current: File-based + name pattern heuristics
   - Future: Integrate with parser visibility tracking

2. **Trait Resolution Completeness**: Are all common trait patterns covered?
   - Current: Default, Clone, From, Into, new()
   - Missing: Display, Debug, Drop, Deref, custom traits?

3. **Health Score Calibration**: Are the weights (10, 5, 1, 0.5) optimal?
   - Test on multiple projects
   - Adjust based on user feedback

4. **Performance at Scale**: How does this perform on 50K+ function codebases?
   - May need caching or incremental analysis
   - Profile on large projects
