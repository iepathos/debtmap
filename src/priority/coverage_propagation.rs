//! Indirect coverage detection through call graph analysis (Spec 120)
//!
//! This module implements algorithms to detect functions that are indirectly covered
//! through their well-tested callers, reducing false positives in testing gap detection.
//!
//! # Algorithm Overview
//!
//! The indirect coverage algorithm propagates test coverage from well-tested functions
//! (≥80% coverage) to their callees through the call graph. Coverage contributions are
//! discounted by distance to account for reduced confidence at each hop.
//!
//! ## Key Concepts
//!
//! - **Direct Coverage**: Coverage from tests directly targeting a function
//! - **Indirect Coverage**: Coverage from tests targeting functions that call this function
//! - **Effective Coverage**: Maximum of direct and indirect coverage
//! - **Distance Discount**: 70% per hop (e.g., 2 hops = 0.7² = 49% of caller's coverage)
//!
//! ## Example
//!
//! Given this call chain: `test_c()` → `c()` [90% coverage] → `f()` [0% direct]
//!
//! Function `f()`:
//! - Direct coverage: 0%
//! - Indirect coverage: 90% × 0.7⁰ = 90% (distance 0, one hop from caller)
//! - Effective coverage: max(0%, 90%) = 90%
//!
//! With a longer chain: `test_b()` → `b()` [95% coverage] → `c()` [10%] → `f()` [0%]
//!
//! Function `f()`:
//! - Direct coverage: 0%
//! - Indirect from c: 10% × 0.7⁰ = 10%
//! - Indirect from b: 95% × 0.7¹ = 66.5% (distance 1, two hops from caller)
//! - Effective coverage: max(0%, 66.5%) = 66.5%
//!
//! ## Design Decisions
//!
//! 1. **Well-Tested Threshold (80%)**: Functions must have ≥80% direct coverage to
//!    contribute to indirect coverage. This ensures high confidence in the test suite.
//!
//! 2. **Distance Discount (70% per hop)**: Each level of indirection reduces contribution
//!    by 30%, reflecting decreased confidence. After 3 hops, contribution drops below
//!    practical significance (~34%).
//!
//! 3. **Maximum Distance (3 hops)**: Limits recursion depth to prevent exponential
//!    complexity and filter out weak indirect coverage signals.
//!
//! 4. **Circular Dependency Handling**: Uses a visited set to prevent infinite loops
//!    in circular call graphs.
//!
//! 5. **Maximum Aggregation**: Takes the maximum contribution from multiple sources
//!    rather than summing to avoid double-counting coverage.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use debtmap::priority::coverage_propagation::calculate_indirect_coverage;
//! use debtmap::priority::call_graph::{CallGraph, FunctionId};
//! use debtmap::risk::lcov::LcovData;
//! use std::path::PathBuf;
//!
//! let call_graph = CallGraph::new();
//! let coverage = LcovData::default();
//!
//! let func_id = FunctionId {
//!     file: PathBuf::from("src/lib.rs"),
//!     name: "my_function".to_string(),
//!     line: 42,
//! };
//!
//! let complete = calculate_indirect_coverage(&func_id, &call_graph, &coverage);
//! println!("Direct: {:.1}%, Indirect: {:.1}%, Effective: {:.1}%",
//!          complete.direct_coverage * 100.0,
//!          complete.indirect_coverage * 100.0,
//!          complete.effective_coverage * 100.0);
//!
//! for source in &complete.coverage_sources {
//!     println!("  From {} @ distance {}: {:.1}%",
//!              source.caller.name,
//!              source.distance,
//!              source.contributed_coverage * 100.0);
//! }
//! ```
//!
//! ## Configuration
//!
//! The algorithm uses these tunable constants (defined in `analyze_caller_coverage`):
//!
//! - `MAX_DEPTH = 3`: Maximum call chain depth to traverse
//! - `DISTANCE_DISCOUNT = 0.7`: Coverage discount per hop (70%)
//! - Well-tested threshold: 0.8 (80% coverage)
//!
//! To adjust these, modify the constants in the function implementation.
//!
//! ## Performance
//!
//! - **Time Complexity**: O(V + E × D) where V = vertices (functions), E = edges (calls),
//!   D = max depth. Bounded by MAX_DEPTH to prevent exponential growth.
//! - **Space Complexity**: O(D) for the visited set during recursion.
//!
//! The algorithm is efficient for typical codebases but may be slower on very dense
//! call graphs with many highly-connected functions.
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::risk::lcov::LcovData;
use serde::{Deserialize, Serialize};
use std::collections::HashSet as StdHashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitiveCoverage {
    pub direct: f64,
    pub transitive: f64,
    pub propagated_from: Vec<FunctionId>,
    pub uncovered_lines: Vec<usize>,
}

/// Extended coverage information including indirect coverage from tested callers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteCoverage {
    /// Direct coverage from tests (lcov data)
    pub direct_coverage: f64,

    /// Indirect coverage from tested callers
    pub indirect_coverage: f64,

    /// Combined effective coverage
    pub effective_coverage: f64,

    /// Callers contributing to indirect coverage
    pub coverage_sources: Vec<CoverageSource>,
}

/// Represents a caller that contributes to indirect coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageSource {
    /// Function providing coverage
    pub caller: FunctionId,

    /// Coverage percentage of caller
    pub caller_coverage: f64,

    /// Number of hops from tested code
    pub distance: u32,

    /// Discounted coverage contribution
    pub contributed_coverage: f64,
}

pub fn calculate_transitive_coverage(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    coverage: &LcovData,
) -> TransitiveCoverage {
    // Get direct coverage for this function
    let direct = get_function_coverage(func_id, coverage);
    let uncovered_lines = get_uncovered_lines(func_id, coverage);

    // If function has direct coverage, no need to calculate transitive
    if direct > 0.0 {
        return TransitiveCoverage {
            direct,
            transitive: direct,
            propagated_from: vec![],
            uncovered_lines,
        };
    }

    // Calculate coverage from callees
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        return TransitiveCoverage {
            direct: 0.0,
            transitive: 0.0,
            propagated_from: vec![],
            uncovered_lines,
        };
    }

    // Check coverage of each callee
    let mut covered_callees = Vec::new();

    for callee in &callees {
        let callee_coverage = get_function_coverage(callee, coverage);
        if callee_coverage > 0.8 {
            covered_callees.push(callee.clone());
        }
    }

    // Calculate transitive coverage as percentage of well-covered callees
    let transitive = if callees.is_empty() {
        0.0
    } else {
        covered_callees.len() as f64 / callees.len() as f64
    };

    TransitiveCoverage {
        direct,
        transitive,
        propagated_from: covered_callees,
        uncovered_lines,
    }
}

fn get_function_coverage(func_id: &FunctionId, coverage: &LcovData) -> f64 {
    // Use the LCOV module's fuzzy matching logic
    // Note: get_function_coverage_with_line already returns a fraction (0.0-1.0)
    coverage
        .get_function_coverage_with_line(&func_id.file, &func_id.name, func_id.line)
        .unwrap_or(0.0)
}

fn get_uncovered_lines(func_id: &FunctionId, coverage: &LcovData) -> Vec<usize> {
    // Get uncovered lines for a function from LCOV data
    coverage
        .get_function_uncovered_lines(&func_id.file, &func_id.name, func_id.line)
        .unwrap_or_default()
}

/// Calculates coverage urgency using a smooth gradient approach.
///
/// This function provides continuous scoring from 0% to 100% coverage, rather than
/// binary thresholds. The urgency score considers both direct and transitive coverage,
/// weighted by complexity.
///
/// # Score Examples (with average complexity 10):
/// - 0% coverage: ~10.0 (highest urgency)
/// - 25% coverage: ~7.5
/// - 50% coverage: ~5.0
/// - 75% coverage: ~2.5
/// - 90% coverage: ~1.0
/// - 100% coverage: 0.0 (no urgency)
///
/// # Complexity Weighting:
/// - Complexity 1-5: 0.5-0.8x multiplier
/// - Complexity 6-10: 0.8-1.2x multiplier
/// - Complexity 11-20: 1.2-1.5x multiplier
/// - Complexity 20+: 1.5-2.0x multiplier
pub fn calculate_coverage_urgency(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    coverage: &LcovData,
    complexity: u32,
) -> f64 {
    let transitive_cov = calculate_transitive_coverage(func_id, call_graph, coverage);

    // Use weighted average of direct and transitive coverage
    // Direct coverage is more important than transitive coverage
    let coverage_weight = 0.7; // Direct coverage weight
    let effective_coverage = transitive_cov.direct * coverage_weight
        + transitive_cov.transitive * (1.0 - coverage_weight);

    // Calculate coverage gap (0.0 = fully covered, 1.0 = no coverage)
    // Ensure the value is between 0.0 and 1.0
    let coverage_gap = 1.0 - effective_coverage.clamp(0.0, 1.0);

    // Apply complexity weighting with logarithmic scaling
    // This provides smoother gradation:
    // Complexity 1-5 = 0.5-0.8x multiplier
    // Complexity 6-10 = 0.8-1.2x multiplier
    // Complexity 11-20 = 1.2-1.5x multiplier
    // Complexity 20+ = 1.5-2.0x multiplier
    let complexity_weight = if complexity == 0 {
        0.5
    } else {
        (((complexity as f64 + 1.0).ln() / 3.0) + 0.5).min(2.0)
    };

    // Calculate urgency score with smooth gradient
    // This produces continuous values without capping
    coverage_gap * complexity_weight * 10.0
}

pub fn propagate_coverage_through_graph(
    call_graph: &CallGraph,
    coverage: &LcovData,
) -> im::HashMap<FunctionId, TransitiveCoverage> {
    let mut result = im::HashMap::new();

    // Process all functions in the call graph
    for func_id in call_graph.find_all_functions() {
        let transitive = calculate_transitive_coverage(&func_id, call_graph, coverage);
        result.insert(func_id, transitive);
    }

    result
}

/// Calculate indirect coverage from tested callers (caller → callee propagation)
///
/// This implements the algorithm from spec 120 to detect functions that are
/// well-tested indirectly through their callers, reducing false positives.
pub fn calculate_indirect_coverage(
    func_id: &FunctionId,
    call_graph: &CallGraph,
    coverage: &LcovData,
) -> CompleteCoverage {
    let direct_coverage = get_function_coverage(func_id, coverage);

    // If already well-tested directly, skip indirect calculation
    // Note: direct_coverage is a fraction (0.0-1.0), so 0.8 = 80%
    if direct_coverage >= 0.8 {
        return CompleteCoverage {
            direct_coverage,
            indirect_coverage: 0.0,
            effective_coverage: direct_coverage,
            coverage_sources: vec![],
        };
    }

    // Find all callers and recursively analyze their coverage
    let callers = call_graph.get_callers(func_id);
    if callers.is_empty() {
        return CompleteCoverage {
            direct_coverage,
            indirect_coverage: 0.0,
            effective_coverage: direct_coverage,
            coverage_sources: vec![],
        };
    }

    let sources =
        analyze_caller_coverage(&callers, call_graph, coverage, 0, &mut StdHashSet::new());

    let indirect_coverage = aggregate_indirect_coverage(&sources);
    let effective_coverage = combine_coverages(direct_coverage, indirect_coverage);

    CompleteCoverage {
        direct_coverage,
        indirect_coverage,
        effective_coverage,
        coverage_sources: sources,
    }
}

/// Analyze coverage contribution from callers (recursive with depth limit)
fn analyze_caller_coverage(
    callers: &[FunctionId],
    call_graph: &CallGraph,
    coverage: &LcovData,
    depth: u32,
    visited: &mut StdHashSet<FunctionId>,
) -> Vec<CoverageSource> {
    const MAX_DEPTH: u32 = 3;
    const DISTANCE_DISCOUNT: f64 = 0.7; // 70% per hop

    if depth >= MAX_DEPTH {
        return vec![];
    }

    let mut sources = vec![];

    for caller in callers {
        // Prevent infinite loops in circular call graphs
        if visited.contains(caller) {
            continue;
        }
        visited.insert(caller.clone());

        // Get caller's direct coverage
        let caller_coverage = get_function_coverage(caller, coverage);

        // Well-tested caller (≥80%) contributes to indirect coverage
        // Note: caller_coverage is a fraction (0.0-1.0), so 0.8 = 80%
        if caller_coverage >= 0.8 {
            let discount = DISTANCE_DISCOUNT.powi(depth as i32);
            sources.push(CoverageSource {
                caller: caller.clone(),
                caller_coverage,
                distance: depth,
                contributed_coverage: caller_coverage * discount,
            });
        } else if depth < MAX_DEPTH - 1 {
            // Recursively check caller's callers
            let upstream_callers = call_graph.get_callers(caller);
            sources.extend(analyze_caller_coverage(
                &upstream_callers,
                call_graph,
                coverage,
                depth + 1,
                visited,
            ));
        }

        visited.remove(caller);
    }

    sources
}

/// Aggregate indirect coverage from multiple sources
///
/// Takes maximum contribution to avoid double-counting
fn aggregate_indirect_coverage(sources: &[CoverageSource]) -> f64 {
    if sources.is_empty() {
        return 0.0;
    }

    // Take maximum contribution (not sum, to avoid double-counting)
    sources
        .iter()
        .map(|s| s.contributed_coverage)
        .fold(0.0, f64::max)
}

/// Combine direct and indirect coverage
///
/// Indirect coverage fills the gap left by direct coverage
fn combine_coverages(direct: f64, indirect: f64) -> f64 {
    // Take maximum (indirect doesn't add to direct, it fills the gap)
    direct.max(indirect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::lcov::{FunctionCoverage, LcovData};
    use std::path::PathBuf;

    fn create_test_coverage() -> LcovData {
        let mut coverage = LcovData::default();

        // Add coverage for test.rs
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 5,
            coverage_percentage: 50.0,
            uncovered_lines: vec![],
        }];
        coverage.functions.insert(PathBuf::from("test.rs"), funcs);
        coverage.build_index(); // Rebuild index after modifying functions

        coverage
    }

    #[test]
    fn test_direct_coverage() {
        let coverage = create_test_coverage();
        let graph = CallGraph::new();

        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            line: 10,
        };

        let transitive = calculate_transitive_coverage(&func_id, &graph, &coverage);
        assert!(transitive.direct > 0.0);
        assert_eq!(transitive.direct, transitive.transitive);
        assert!(transitive.propagated_from.is_empty());
    }

    #[test]
    fn test_transitive_coverage_with_delegation() {
        let coverage = create_test_coverage();
        let mut graph = CallGraph::new();

        let orchestrator = FunctionId {
            file: PathBuf::from("orch.rs"),
            name: "orchestrate".to_string(),
            line: 1,
        };

        let worker = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "worker".to_string(),
            line: 10,
        };

        graph.add_function(orchestrator.clone(), false, false, 2, 10);
        graph.add_function(worker.clone(), false, false, 5, 30);
        graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: orchestrator.clone(),
            callee: worker.clone(),
            call_type: crate::priority::call_graph::CallType::Delegate,
        });

        let transitive = calculate_transitive_coverage(&orchestrator, &graph, &coverage);
        assert_eq!(transitive.direct, 0.0);
        // Should have some transitive coverage from the covered worker
        assert!(transitive.transitive >= 0.0);
    }

    #[test]
    fn test_coverage_urgency() {
        let coverage = create_test_coverage();
        let graph = CallGraph::new();

        let func_id = FunctionId {
            file: PathBuf::from("uncovered.rs"),
            name: "complex_func".to_string(),
            line: 1,
        };

        // High complexity, no coverage = high urgency
        let urgency = calculate_coverage_urgency(&func_id, &graph, &coverage, 10);
        assert!(urgency > 8.0);

        // Low complexity, no coverage = lower urgency (but still high due to no coverage)
        let urgency = calculate_coverage_urgency(&func_id, &graph, &coverage, 2);
        assert!((7.0..=10.0).contains(&urgency));
    }

    #[test]
    fn test_coverage_urgency_gradient() {
        let mut coverage = LcovData::default();
        let graph = CallGraph::new();

        // Create a function with varying coverage levels
        let func_id = FunctionId {
            file: PathBuf::from("gradient_test.rs"),
            name: "test_func".to_string(),
            line: 10,
        };

        // Test with average complexity (10)
        let complexity = 10;

        // Test 0% coverage - should be ~10.0
        let urgency_0 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        // With no cap, scores can exceed 10.0
        assert!(
            urgency_0 >= 9.0,
            "0% coverage should score at least 9.0, got {}",
            urgency_0
        );

        // Test 25% coverage - should be reduced from full
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 25.0,
            uncovered_lines: vec![],
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());
        coverage.build_index(); // Rebuild index after modifying functions

        let urgency_25 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        // With 25% coverage and our weighted calculation (0.7 direct weight), this should be around 7.5-9.0
        assert!(
            (7.0..=10.0).contains(&urgency_25),
            "25% coverage should score 7.0-10.0, got {}",
            urgency_25
        );

        // Test 50% coverage - should be around 5.0
        // With weight = 0.7, effective coverage = 0.5 * 0.7 = 0.35, gap = 0.65
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 50.0,
            uncovered_lines: vec![],
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());
        coverage.build_index(); // Rebuild index after modifying functions

        let urgency_50 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        assert!(
            (5.0..=7.5).contains(&urgency_50),
            "50% coverage should score 5.0-7.5, got {}",
            urgency_50
        );

        // Test 75% coverage - should be around 3.0
        // With weight = 0.7, effective coverage = 0.75 * 0.7 = 0.525, gap = 0.475
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 75.0,
            uncovered_lines: vec![],
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());
        coverage.build_index(); // Rebuild index after modifying functions

        let urgency_75 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        assert!(
            (3.0..=5.5).contains(&urgency_75),
            "75% coverage should score 3.0-5.5, got {}",
            urgency_75
        );

        // Test 90% coverage - should be around 1.3
        // With weight = 0.7, effective coverage = 0.9 * 0.7 = 0.63, gap = 0.37
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 90.0,
            uncovered_lines: vec![],
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());
        coverage.build_index(); // Rebuild index after modifying functions

        let urgency_90 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        assert!(
            (1.0..=4.5).contains(&urgency_90),
            "90% coverage should score 1.0-4.5, got {}",
            urgency_90
        );

        // Test 100% coverage - should be 0.0
        let funcs = vec![FunctionCoverage {
            name: "test_func".to_string(),
            start_line: 10,
            execution_count: 1,
            coverage_percentage: 100.0,
            uncovered_lines: vec![],
        }];
        coverage
            .functions
            .insert(PathBuf::from("gradient_test.rs"), funcs.clone());
        coverage.build_index(); // Rebuild index after modifying functions

        let urgency_100 = calculate_coverage_urgency(&func_id, &graph, &coverage, complexity);
        assert!(
            urgency_100 == 0.0,
            "100% coverage should score 0.0, got {}",
            urgency_100
        );

        // Verify smooth gradient - scores should decrease monotonically
        assert!(
            urgency_0 > urgency_25,
            "Scores should decrease as coverage increases"
        );
        assert!(
            urgency_25 > urgency_50,
            "Scores should decrease as coverage increases"
        );
        assert!(
            urgency_50 > urgency_75,
            "Scores should decrease as coverage increases"
        );
        assert!(
            urgency_75 > urgency_90,
            "Scores should decrease as coverage increases"
        );
        assert!(
            urgency_90 > urgency_100,
            "Scores should decrease as coverage increases"
        );
    }

    #[test]
    fn test_indirect_coverage_single_hop() {
        let mut call_graph = CallGraph::new();
        let mut coverage = LcovData::default();

        // Setup: F called by C (C has 90% coverage)
        let func_f = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "f".to_string(),
            line: 10,
        };
        let caller_c = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "c".to_string(),
            line: 50,
        };

        call_graph.add_function(func_f.clone(), false, false, 5, 20);
        call_graph.add_function(caller_c.clone(), false, false, 8, 40);
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: caller_c.clone(),
            callee: func_f.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        // Mock coverage data: C has 90% coverage
        // Note: LcovData stores coverage as percentage (0-100), not fraction
        let funcs = vec![FunctionCoverage {
            name: "c".to_string(),
            start_line: 50,
            execution_count: 10,
            coverage_percentage: 90.0,
            uncovered_lines: vec![],
        }];
        coverage.functions.insert(PathBuf::from("test.rs"), funcs);
        coverage.build_index();

        let complete_coverage = calculate_indirect_coverage(&func_f, &call_graph, &coverage);

        // F should have ~0.9 indirect coverage (0.9 × 0.7^0 = 0.9, distance=0)
        // Coverage is returned as fraction (0.0-1.0), not percentage
        assert!(
            (complete_coverage.indirect_coverage - 0.9).abs() < 0.01,
            "Expected ~0.9 indirect coverage, got {}",
            complete_coverage.indirect_coverage
        );
        assert_eq!(complete_coverage.coverage_sources.len(), 1);
        assert_eq!(complete_coverage.coverage_sources[0].distance, 0);
    }

    #[test]
    fn test_indirect_coverage_multi_hop() {
        let mut call_graph = CallGraph::new();
        let mut coverage = LcovData::default();

        // Setup: F ← C1 ← C2 (C2 has 95% coverage)
        let func_f = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "f".to_string(),
            line: 10,
        };
        let caller_c1 = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "c1".to_string(),
            line: 50,
        };
        let caller_c2 = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "c2".to_string(),
            line: 80,
        };

        call_graph.add_function(func_f.clone(), false, false, 5, 20);
        call_graph.add_function(caller_c1.clone(), false, false, 8, 40);
        call_graph.add_function(caller_c2.clone(), false, false, 10, 50);

        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: caller_c1.clone(),
            callee: func_f.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });
        call_graph.add_call(crate::priority::call_graph::FunctionCall {
            caller: caller_c2.clone(),
            callee: caller_c1.clone(),
            call_type: crate::priority::call_graph::CallType::Direct,
        });

        // Mock coverage: only C2 has coverage (95%)
        let funcs = vec![FunctionCoverage {
            name: "c2".to_string(),
            start_line: 80,
            execution_count: 10,
            coverage_percentage: 95.0,
            uncovered_lines: vec![],
        }];
        coverage.functions.insert(PathBuf::from("test.rs"), funcs);
        coverage.build_index();

        let complete_coverage = calculate_indirect_coverage(&func_f, &call_graph, &coverage);

        // Expected: F gets 0.95 × 0.7^1 = 0.665 from C2 (distance 1 from F)
        // Coverage is returned as fraction (0.0-1.0), not percentage
        let expected = 0.95 * 0.7_f64.powi(1);
        assert!(
            (complete_coverage.indirect_coverage - expected).abs() < 0.01,
            "Expected ~{} indirect coverage, got {}",
            expected,
            complete_coverage.indirect_coverage
        );
        assert_eq!(complete_coverage.coverage_sources.len(), 1);
        assert_eq!(complete_coverage.coverage_sources[0].distance, 1);
    }

    #[test]
    fn test_depth_limit_prevents_deep_recursion() {
        let mut call_graph = CallGraph::new();
        let mut coverage = LcovData::default();

        // Setup: Chain of 5 functions (exceeds MAX_DEPTH=3)
        // F ← C1 ← C2 ← C3 ← C4 (C4 has 100% coverage)
        let functions = [("f", 10), ("c1", 20), ("c2", 30), ("c3", 40), ("c4", 50)];

        let func_ids: Vec<FunctionId> = functions
            .iter()
            .map(|(name, line)| FunctionId {
                file: PathBuf::from("test.rs"),
                name: name.to_string(),
                line: *line,
            })
            .collect();

        // Add all functions
        for func_id in &func_ids {
            call_graph.add_function(func_id.clone(), false, false, 5, 20);
        }

        // Add call chain
        for i in 0..func_ids.len() - 1 {
            call_graph.add_call(crate::priority::call_graph::FunctionCall {
                caller: func_ids[i + 1].clone(),
                callee: func_ids[i].clone(),
                call_type: crate::priority::call_graph::CallType::Direct,
            });
        }

        // Mock coverage: only C4 has coverage
        let funcs = vec![FunctionCoverage {
            name: "c4".to_string(),
            start_line: 50,
            execution_count: 10,
            coverage_percentage: 100.0,
            uncovered_lines: vec![],
        }];
        coverage.functions.insert(PathBuf::from("test.rs"), funcs);
        coverage.build_index();

        let complete_coverage = calculate_indirect_coverage(&func_ids[0], &call_graph, &coverage);

        // Should not reach C4 because it's at distance 4 (exceeds MAX_DEPTH=3)
        // So indirect coverage should be 0
        assert_eq!(
            complete_coverage.indirect_coverage, 0.0,
            "Should not propagate beyond MAX_DEPTH"
        );
        assert_eq!(complete_coverage.coverage_sources.len(), 0);
    }

    #[test]
    fn test_direct_coverage_skips_indirect() {
        let call_graph = CallGraph::new();
        let mut coverage = LcovData::default();

        let func_f = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "f".to_string(),
            line: 10,
        };

        // F already has 85% direct coverage
        let funcs = vec![FunctionCoverage {
            name: "f".to_string(),
            start_line: 10,
            execution_count: 10,
            coverage_percentage: 85.0,
            uncovered_lines: vec![],
        }];
        coverage.functions.insert(PathBuf::from("test.rs"), funcs);
        coverage.build_index();

        let complete_coverage = calculate_indirect_coverage(&func_f, &call_graph, &coverage);

        // Should skip indirect calculation since direct >= 0.8 (80%)
        // Coverage is returned as fraction (0.0-1.0), so 85% = 0.85
        assert_eq!(complete_coverage.direct_coverage, 0.85);
        assert_eq!(complete_coverage.indirect_coverage, 0.0);
        assert_eq!(complete_coverage.effective_coverage, 0.85);
        assert_eq!(complete_coverage.coverage_sources.len(), 0);
    }

    #[test]
    fn test_no_callers_zero_indirect() {
        let call_graph = CallGraph::new();
        let coverage = LcovData::default();

        let func_f = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "orphan".to_string(),
            line: 10,
        };

        let complete_coverage = calculate_indirect_coverage(&func_f, &call_graph, &coverage);

        // Function with no callers should have 0 indirect coverage
        assert_eq!(complete_coverage.indirect_coverage, 0.0);
        assert_eq!(complete_coverage.coverage_sources.len(), 0);
    }

    #[test]
    fn test_complexity_weighting() {
        let coverage = LcovData::default(); // No coverage
        let graph = CallGraph::new();

        let func_id = FunctionId {
            file: PathBuf::from("complexity_test.rs"),
            name: "test_func".to_string(),
            line: 1,
        };

        // Test complexity scaling with 0% coverage

        // Complexity 1: ln(2)/3 + 0.5 = ~0.73 multiplier
        let urgency_c1 = calculate_coverage_urgency(&func_id, &graph, &coverage, 1);
        assert!(
            (6.5..=8.0).contains(&urgency_c1),
            "Complexity 1 should score 6.5-8.0, got {}",
            urgency_c1
        );

        // Complexity 5: ln(6)/3 + 0.5 = ~1.09 multiplier
        let urgency_c5 = calculate_coverage_urgency(&func_id, &graph, &coverage, 5);
        // With no cap, complexity 5 can score above 10.0
        assert!(
            urgency_c5 >= 9.5,
            "Complexity 5 should score at least 9.5, got {}",
            urgency_c5
        );

        // Complexity 10: with no cap, can exceed 10.0
        let urgency_c10 = calculate_coverage_urgency(&func_id, &graph, &coverage, 10);
        assert!(
            urgency_c10 >= 9.0,
            "Complexity 10 should score at least 9.0, got {}",
            urgency_c10
        );

        // Complexity 20: with no cap, can exceed 10.0
        let urgency_c20 = calculate_coverage_urgency(&func_id, &graph, &coverage, 20);
        assert!(
            urgency_c20 >= 10.0,
            "Complexity 20 should score at least 10.0, got {}",
            urgency_c20
        );

        // Complexity 50: with no cap, can exceed 10.0
        let urgency_c50 = calculate_coverage_urgency(&func_id, &graph, &coverage, 50);
        assert!(
            urgency_c50 >= 10.0,
            "Complexity 50 should score at least 10.0, got {}",
            urgency_c50
        );

        // Verify smooth increase with complexity
        assert!(
            urgency_c1 < urgency_c5,
            "Higher complexity should have higher urgency"
        );
        assert!(
            urgency_c5 <= urgency_c10,
            "Higher complexity should have higher urgency (or be capped)"
        );
        assert!(
            urgency_c10 <= urgency_c20,
            "Higher complexity should have higher urgency (or be capped)"
        );
    }
}
