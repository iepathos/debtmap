---
number: 151
title: Improve Call Graph Orphaned Node Detection
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-10-24
---

# Specification 151: Improve Call Graph Orphaned Node Detection

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: None

## Context

**Current State**:
- Call graph validation in `src/analyzers/call_graph/validation.rs` detects orphaned nodes
- Orphaned node definition: functions with **no callers AND no callees**
- Health score calculation: 0/100 due to 11,826 orphaned nodes detected
- Current logic (`check_orphaned_nodes` at line 122-140):
  ```rust
  if !has_callers && !has_callees && !is_entry_point {
      report.structural_issues.push(StructuralIssue::OrphanedNode { ... });
  }
  ```

**Problem**:
The orphaned node detection is too strict and generates massive false positives:

1. **Legitimate leaf functions**: Utility functions, getters, constructors that don't call other functions
2. **Trait implementations**: `default()`, `new()`, `clone_box()` often have no callees
3. **Simple property methods**: `Language::extensions()`, `extensions()`, accessor methods
4. **Callback functions**: Functions passed as closures or callbacks (indirect calls)
5. **Public API functions**: Library functions called from external crates
6. **Self-referential calls**: Functions that recursively call themselves

**Impact**:
- Health score drops to 0/100 making validation useless
- 11,826 false positives obscure real structural issues
- Users lose confidence in call graph accuracy
- Dependency score calculations affected for legitimate functions

## Objective

Refine orphaned node detection to distinguish between:
1. **True orphans**: Functions that are truly unused and unreachable
2. **Legitimate leaf nodes**: Functions with callers but no callees (expected)
3. **Expected entry points**: Test functions, main, public APIs
4. **Indirect calls**: Callbacks, trait methods, dynamic dispatch

Reduce false positives by 95% while maintaining detection of actual dead code.

## Requirements

### Functional Requirements

1. **Refine Orphan Definition**:
   - Orphaned = no callers (unreachable), NOT no callees (leaf nodes are fine)
   - Leaf nodes with callers should NOT be flagged as orphaned
   - Split current "OrphanedNode" into "UnreachableFunction" and "IsolatedFunction"

2. **Classify Node Types**:
   - **Unreachable**: No callers, not a known entry point
   - **Leaf**: Has callers, no callees (normal utility function)
   - **Entry Point**: Main, tests, public exports, #[no_mangle] functions
   - **Isolated**: No callers, no callees (true orphan)

3. **Entry Point Detection**:
   - Existing: `main`, `test_*`, `*::test_*`
   - Add: Public functions (`pub fn`)
   - Add: Functions with `#[no_mangle]`, `#[export_name]`
   - Add: Trait method implementations
   - Add: Functions in `lib.rs` with `pub` visibility
   - Add: Benchmark functions (`bench_*`)
   - Add: Example functions in `examples/`

4. **Indirect Call Detection**:
   - Track functions passed as function pointers
   - Track closures that capture and call functions
   - Track trait method calls resolved via dynamic dispatch
   - Mark callback functions (passed to iterators, async, etc.)

5. **Self-Referential Call Detection**:
   - Detect recursive functions (functions that call themselves)
   - Mark as non-isolated if they have self-edges
   - Example: `fn factorial(n: u32) -> u32 { if n == 0 { 1 } else { n * factorial(n-1) } }`

6. **Validation Severity Levels**:
   - **Error**: True unreachable functions (isolated + not entry point)
   - **Warning**: Suspicious patterns (public function with no callers)
   - **Info**: Leaf nodes (for statistics, not health score)

### Non-Functional Requirements

- **Backward Compatibility**: Existing validation API unchanged
- **Performance**: Validation overhead < 50ms for 5000 functions
- **Accuracy**: False positive rate < 5% for orphaned detection
- **Configurability**: Allow users to mark expected orphans via config

## Acceptance Criteria

- [ ] Leaf functions (has callers, no callees) NOT flagged as orphaned
- [ ] Entry points detected: main, tests, pub functions, #[no_mangle], trait impls
- [ ] Self-referential functions (recursive) NOT flagged as isolated
- [ ] Indirect calls tracked: function pointers, closures, callbacks
- [ ] Health score calculation separates errors vs warnings vs info
- [ ] False positive count reduced from 11,826 to < 500
- [ ] Health score increases from 0/100 to 80+/100 for typical projects
- [ ] Configuration option to whitelist expected orphans
- [ ] Documentation explains new orphan categories
- [ ] Tests cover edge cases: recursive functions, trait impls, callbacks

## Technical Details

### Implementation Approach

**Phase 1: Refine Data Model**

```rust
// src/analyzers/call_graph/validation.rs

/// Refined structural issues
#[derive(Debug, Clone)]
pub enum StructuralIssue {
    /// Edge references non-existent node
    DanglingEdge {
        caller: FunctionId,
        callee: FunctionId,
    },
    /// Function is unreachable (no callers, not an entry point)
    UnreachableFunction {
        function: FunctionId,
        reason: UnreachableReason,
    },
    /// Function is completely isolated (no callers, no callees, not entry point)
    IsolatedFunction {
        function: FunctionId,
    },
    /// Duplicate nodes
    DuplicateNode {
        function: FunctionId,
        count: usize,
    },
}

#[derive(Debug, Clone)]
pub enum UnreachableReason {
    /// Not called by any function
    NoCallers,
    /// Only called from other unreachable functions
    TransitivelyUnreachable,
}

/// New info-level observations (not errors/warnings)
#[derive(Debug, Clone)]
pub enum ValidationInfo {
    /// Leaf function (has callers, no callees) - this is normal
    LeafFunction {
        function: FunctionId,
        caller_count: usize,
    },
    /// Recursive function (calls itself)
    SelfReferentialFunction {
        function: FunctionId,
    },
}

/// Updated validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub structural_issues: Vec<StructuralIssue>,
    pub warnings: Vec<ValidationWarning>,
    pub info: Vec<ValidationInfo>,  // NEW: informational observations
    pub health_score: u32,
    pub statistics: ValidationStatistics,  // NEW: detailed stats
}

#[derive(Debug, Clone, Default)]
pub struct ValidationStatistics {
    pub total_functions: usize,
    pub entry_points: usize,
    pub leaf_functions: usize,
    pub unreachable_functions: usize,
    pub isolated_functions: usize,
    pub recursive_functions: usize,
}
```

**Phase 2: Entry Point Classification**

```rust
impl CallGraphValidator {
    /// Check if a function is an entry point (expected to have no callers)
    fn is_entry_point(function: &FunctionId, call_graph: &CallGraph) -> bool {
        // Existing checks
        if function.name == "main" {
            return true;
        }
        if function.name.starts_with("test_") || function.name.contains("::test_") {
            return true;
        }

        // NEW: Benchmark functions
        if function.name.starts_with("bench_") || function.name.contains("::bench_") {
            return true;
        }

        // NEW: Check file path for examples
        if function.file.to_str().map_or(false, |s| s.contains("/examples/")) {
            return true;
        }

        // NEW: Public functions in lib.rs or main.rs (library APIs)
        let file_name = function.file.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if file_name == "lib.rs" || file_name == "main.rs" {
            // Would need visibility info from AST - for now, heuristic
            // Functions in lib.rs with short names (< 20 chars) likely public
            if function.name.len() < 20 && !function.name.contains("::") {
                return true;
            }
        }

        // NEW: Trait implementations (contains ::)
        if function.name.contains("::") {
            // Check for common trait patterns
            let trait_patterns = ["default", "new", "clone", "from", "into", "display"];
            if trait_patterns.iter().any(|&p| function.name.to_lowercase().contains(p)) {
                return true;
            }
        }

        false
    }

    /// Check if function is self-referential (recursive)
    fn is_self_referential(function: &FunctionId, call_graph: &CallGraph) -> bool {
        let callees = call_graph.get_callees(function);
        callees.iter().any(|callee| callee == function)
    }

    /// Refine orphaned node detection
    fn check_orphaned_nodes(call_graph: &CallGraph, report: &mut ValidationReport) {
        for function in call_graph.get_all_functions() {
            let has_callers = !call_graph.get_callers(function).is_empty();
            let has_callees = !call_graph.get_callees(function).is_empty();
            let is_entry_point = Self::is_entry_point(function, call_graph);
            let is_self_referential = Self::is_self_referential(function, call_graph);

            // Update statistics
            report.statistics.total_functions += 1;
            if is_entry_point {
                report.statistics.entry_points += 1;
            }
            if is_self_referential {
                report.statistics.recursive_functions += 1;
                report.info.push(ValidationInfo::SelfReferentialFunction {
                    function: function.clone(),
                });
            }

            // LEAF FUNCTION: Has callers but no callees (NORMAL - not an issue)
            if has_callers && !has_callees {
                report.statistics.leaf_functions += 1;
                report.info.push(ValidationInfo::LeafFunction {
                    function: function.clone(),
                    caller_count: call_graph.get_callers(function).len(),
                });
                continue;  // NOT an issue
            }

            // SELF-REFERENTIAL: Calls itself (recursive)
            if is_self_referential {
                // Not isolated, even if no other callers/callees
                continue;  // NOT an issue
            }

            // ISOLATED: No callers, no callees (true orphan)
            if !has_callers && !has_callees && !is_entry_point {
                report.statistics.isolated_functions += 1;
                report.structural_issues.push(StructuralIssue::IsolatedFunction {
                    function: function.clone(),
                });
                continue;
            }

            // UNREACHABLE: No callers but has callees (dead code with dependencies)
            if !has_callers && has_callees && !is_entry_point {
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

**Phase 3: Health Score Calculation**

```rust
impl ValidationReport {
    /// Calculate health score with refined weighting
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

        // Dangling edges are critical (graph corruption) - 10 points each
        score = score.saturating_sub(dangling_edge_count * 10);

        // Duplicates are serious (data integrity) - 5 points each
        score = score.saturating_sub(duplicate_count * 5);

        // Unreachable functions are moderate (dead code) - 1 point each
        score = score.saturating_sub(unreachable_count * 1);

        // Isolated functions are low concern (might be work-in-progress) - 0.5 points each
        score = score.saturating_sub((isolated_count as f32 * 0.5) as u32);

        // Warnings are minor - 2 points each (unchanged)
        score = score.saturating_sub(self.warnings.len() as u32 * 2);

        // Info items don't affect health score (they're informational)

        self.health_score = score;
    }
}
```

**Phase 4: Configuration Support**

```rust
// In Config struct
pub struct CallGraphValidationConfig {
    /// Functions to exclude from orphan detection (regex patterns)
    pub orphan_whitelist: Vec<String>,

    /// Entry point patterns beyond defaults
    pub additional_entry_points: Vec<String>,

    /// Whether to report leaf functions in info
    pub report_leaf_functions: bool,
}

impl Default for CallGraphValidationConfig {
    fn default() -> Self {
        Self {
            orphan_whitelist: vec![],
            additional_entry_points: vec![],
            report_leaf_functions: true,
        }
    }
}

// Usage in validation
impl CallGraphValidator {
    fn is_whitelisted(function: &FunctionId, config: &CallGraphValidationConfig) -> bool {
        use regex::Regex;

        for pattern in &config.orphan_whitelist {
            if let Ok(re) = Regex::new(pattern) {
                let full_name = format!("{}:{}", function.file.display(), function.name);
                if re.is_match(&full_name) {
                    return true;
                }
            }
        }

        false
    }
}
```

### Architecture Changes

1. **Data Model**:
   - Split `OrphanedNode` into `UnreachableFunction` and `IsolatedFunction`
   - Add `ValidationInfo` enum for non-issue observations
   - Add `ValidationStatistics` for detailed breakdown

2. **Validation Logic**:
   - Refine `check_orphaned_nodes()` to classify node types
   - Add `is_entry_point()` with comprehensive checks
   - Add `is_self_referential()` for recursive detection
   - Add `is_whitelisted()` for config-based exclusions

3. **Health Score**:
   - Weight structural issues by severity
   - Exclude info-level observations from score
   - Provide detailed breakdown in verbose output

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaf_function_not_orphaned() {
        let mut call_graph = CallGraph::new();
        let leaf = FunctionId::new("test.rs", "utility_fn", 10);
        let caller = FunctionId::new("test.rs", "main_fn", 5);

        call_graph.add_function(leaf.clone());
        call_graph.add_function(caller.clone());
        call_graph.add_call(caller, leaf.clone());

        let report = CallGraphValidator::validate(&call_graph);

        // Leaf function should NOT be in structural_issues
        assert!(!report.structural_issues.iter().any(|issue| matches!(issue, StructuralIssue::IsolatedFunction { .. })));

        // Should be in info as leaf
        assert!(report.info.iter().any(|info| matches!(info, ValidationInfo::LeafFunction { .. })));
    }

    #[test]
    fn test_self_referential_not_isolated() {
        let mut call_graph = CallGraph::new();
        let recursive = FunctionId::new("test.rs", "factorial", 10);

        call_graph.add_function(recursive.clone());
        call_graph.add_call(recursive.clone(), recursive.clone());  // Self-call

        let report = CallGraphValidator::validate(&call_graph);

        // Should NOT be marked as isolated
        assert!(!report.structural_issues.iter().any(|issue| matches!(issue, StructuralIssue::IsolatedFunction { .. })));

        // Should be in info as self-referential
        assert!(report.info.iter().any(|info| matches!(info, ValidationInfo::SelfReferentialFunction { .. })));
    }

    #[test]
    fn test_entry_point_detection() {
        let test_cases = vec![
            ("src/main.rs", "main"),
            ("src/lib.rs", "test_my_function"),
            ("examples/demo.rs", "demo_main"),
            ("benches/my_bench.rs", "bench_performance"),
            ("src/traits.rs", "Default::default"),
            ("src/types.rs", "MyType::new"),
        ];

        let call_graph = CallGraph::new();

        for (file, name) in test_cases {
            let func = FunctionId::new(file, name, 1);
            assert!(CallGraphValidator::is_entry_point(&func, &call_graph),
                    "Expected {} to be entry point", name);
        }
    }

    #[test]
    fn test_isolated_function_detected() {
        let mut call_graph = CallGraph::new();
        let isolated = FunctionId::new("test.rs", "unused_fn", 10);

        call_graph.add_function(isolated.clone());
        // No calls added

        let report = CallGraphValidator::validate(&call_graph);

        // Should be marked as isolated
        assert!(report.structural_issues.iter().any(|issue|
            matches!(issue, StructuralIssue::IsolatedFunction { function } if function == &isolated)
        ));
    }

    #[test]
    fn test_health_score_improved() {
        let mut call_graph = CallGraph::new();

        // Add many leaf functions (should NOT hurt score)
        for i in 0..1000 {
            let leaf = FunctionId::new("test.rs", &format!("leaf_{}", i), i * 10);
            let caller = FunctionId::new("test.rs", "main", 1);
            call_graph.add_function(leaf.clone());
            call_graph.add_function(caller.clone());
            call_graph.add_call(caller, leaf);
        }

        let report = CallGraphValidator::validate(&call_graph);

        // Health score should be high (no real issues)
        assert!(report.health_score >= 80, "Health score should be 80+ for leaf functions, got {}", report.health_score);
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**:
  - `src/analyzers/call_graph/validation.rs` - Core validation logic
  - `src/priority/call_graph/mod.rs` - CallGraph trait methods
  - `src/commands/analyze.rs` - Validation reporting
- **External Dependencies**: None

## Documentation Requirements

### Code Documentation

- Document new `StructuralIssue` variants with examples
- Explain entry point detection heuristics
- Document health score calculation formula
- Add doctests for validation scenarios

### User Documentation

```markdown
## Call Graph Validation

### Orphaned Node Detection

Debtmap distinguishes between different types of unreachable code:

**Isolated Functions** (Error):
- No callers AND no callees
- Not an entry point (main, test, pub fn)
- Truly unused code that can be removed

**Unreachable Functions** (Error):
- No callers but HAS callees
- Dead code that depends on other functions
- Should be removed or connected to call graph

**Leaf Functions** (Info):
- HAS callers but no callees
- Normal utility functions, getters, constructors
- NOT considered problematic

**Entry Points** (Expected):
- Functions with no callers by design
- Includes: main, tests, benchmarks, pub APIs, trait impls
- Automatically excluded from orphan detection

### Health Score Calculation

- **Dangling edges**: -10 points each (critical)
- **Duplicate nodes**: -5 points each (serious)
- **Unreachable functions**: -1 point each (moderate)
- **Isolated functions**: -0.5 points each (low concern)
- **Warnings**: -2 points each (minor)
- **Info items**: No impact (informational only)

### Configuration

Whitelist expected orphans in `.debtmap.toml`:

```toml
[call_graph_validation]
# Regex patterns for functions to exclude from orphan detection
orphan_whitelist = [
    ".*::generated_.*",  # Generated code
    "test_helpers::.*",   # Test utilities
]

# Additional entry point patterns
additional_entry_points = [
    "custom_main_.*",
    ".*::plugin_init",
]
```
```

## Implementation Notes

### Entry Point Heuristics

1. **File-based detection**: Check file path for `examples/`, `benches/`, `tests/`
2. **Name patterns**: `main`, `test_*`, `bench_*`, `*_test`, `*_bench`
3. **Trait implementations**: Functions containing `::` with trait method names
4. **Visibility**: Would ideally use AST visibility info, but requires parser integration

### Self-Referential Detection

- Simple check: function's callees include itself
- Handles direct recursion (mutual recursion is more complex)
- Future enhancement: Detect strongly connected components for mutual recursion

### Performance Considerations

- Entry point check: O(1) per function (simple string checks)
- Self-referential check: O(callees) per function
- Overall: O(n) where n = number of functions (same as before)

## Migration and Compatibility

### Backward Compatibility

- Existing `ValidationReport` API unchanged for consumers
- New fields (`info`, `statistics`) are additions (non-breaking)
- Health score calculation improves (always beneficial)

### Migration Path

No migration needed - improvements are automatic when validation runs.

### Configuration Migration

New config section is optional with sensible defaults.

## Success Metrics

- **False positive reduction**: From 11,826 to < 500 orphaned nodes
- **Health score improvement**: From 0/100 to 80+/100 for typical projects
- **Accuracy**: < 5% false positive rate on real codebases
- **Performance**: Validation completes in < 50ms for 5000 functions
- **User feedback**: Zero bug reports on false orphan detection

## Open Questions

1. **Public API detection**: How to reliably detect public functions without full AST visibility info?
   - Current approach uses heuristics (file name, function name length)
   - Better approach would integrate with parser visibility tracking

2. **Callback detection**: How to track indirect calls through function pointers and closures?
   - Requires data flow analysis to track function references
   - May need spec 152 (trait method resolution) to be complete first

3. **Whitelist granularity**: Should config support file-level, module-level, or only function-level whitelisting?
   - Current spec uses regex on full function path (flexible but complex)
   - Alternative: Simple glob patterns like `src/generated/**`
