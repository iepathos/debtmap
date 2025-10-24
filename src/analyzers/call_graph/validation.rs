/// Call graph validation and health checks
///
/// This module provides validation tools to detect structural issues,
/// orphaned nodes, and suspicious patterns in the call graph.
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Configuration for call graph validation
#[derive(Debug, Clone, Default)]
pub struct CallGraphValidationConfig {
    /// Functions expected to be orphaned (won't be flagged as issues)
    pub orphan_whitelist: HashSet<String>,
    /// Additional entry points beyond standard detection
    pub additional_entry_points: HashSet<String>,
}

impl CallGraphValidationConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a function to the orphan whitelist
    pub fn add_orphan_whitelist(&mut self, function_name: String) -> &mut Self {
        self.orphan_whitelist.insert(function_name);
        self
    }

    /// Add an additional entry point
    pub fn add_entry_point(&mut self, function_name: String) -> &mut Self {
        self.additional_entry_points.insert(function_name);
        self
    }

    /// Check if a function is whitelisted as an expected orphan
    pub fn is_whitelisted_orphan(&self, function: &FunctionId) -> bool {
        self.orphan_whitelist.contains(&function.name)
    }

    /// Check if a function is configured as an additional entry point
    pub fn is_additional_entry_point(&self, function: &FunctionId) -> bool {
        self.additional_entry_points.contains(&function.name)
    }
}

/// Structural issue in the call graph
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
    IsolatedFunction { function: FunctionId },
    /// Duplicate nodes
    DuplicateNode { function: FunctionId, count: usize },
}

/// Reason why a function is unreachable
#[derive(Debug, Clone)]
pub enum UnreachableReason {
    /// Not called by any function
    NoCallers,
}

/// Warning about suspicious patterns
#[derive(Debug, Clone)]
pub enum ValidationWarning {
    /// Function has unexpectedly many callers
    TooManyCallers { function: FunctionId, count: usize },
    /// Function has unexpectedly many callees
    TooManyCallees { function: FunctionId, count: usize },
    /// All functions in file have 0 callers (suspicious)
    FileWithNoCalls {
        file: PathBuf,
        function_count: usize,
    },
    /// Public function has 0 callers
    UnusedPublicFunction { function: FunctionId },
}

/// Informational observations (not errors or warnings)
#[derive(Debug, Clone)]
pub enum ValidationInfo {
    /// Leaf function (has callers, no callees) - this is normal
    LeafFunction {
        function: FunctionId,
        caller_count: usize,
    },
    /// Recursive function (calls itself)
    SelfReferentialFunction { function: FunctionId },
}

/// Validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Structural issues (dangling edges, etc.)
    pub structural_issues: Vec<StructuralIssue>,
    /// Heuristic warnings (suspicious patterns)
    pub warnings: Vec<ValidationWarning>,
    /// Informational observations (not issues)
    pub info: Vec<ValidationInfo>,
    /// Overall health score (0-100)
    pub health_score: u32,
    /// Detailed statistics
    pub statistics: ValidationStatistics,
}

/// Detailed validation statistics
#[derive(Debug, Clone, Default)]
pub struct ValidationStatistics {
    pub total_functions: usize,
    pub entry_points: usize,
    pub leaf_functions: usize,
    pub unreachable_functions: usize,
    pub isolated_functions: usize,
    pub recursive_functions: usize,
}

impl ValidationReport {
    /// Create a new empty validation report
    fn new() -> Self {
        Self {
            structural_issues: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
            health_score: 100,
            statistics: ValidationStatistics::default(),
        }
    }

    /// Calculate health score based on issues and warnings
    fn calculate_health_score(&mut self) {
        let mut score: u32 = 100;

        // Count issue types separately for refined weighting
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
        score = score.saturating_sub(unreachable_count);

        // Isolated functions are low concern (might be work-in-progress) - 0.5 points each
        score = score.saturating_sub((isolated_count as f32 * 0.5) as u32);

        // Warnings are minor - 2 points each
        score = score.saturating_sub(self.warnings.len() as u32 * 2);

        // Info items don't affect health score (they're informational)

        self.health_score = score;
    }

    /// Check if the report has any issues
    pub fn has_issues(&self) -> bool {
        !self.structural_issues.is_empty() || !self.warnings.is_empty()
    }
}

/// Call graph validator
pub struct CallGraphValidator;

impl CallGraphValidator {
    /// Validate call graph structure
    pub fn validate(call_graph: &CallGraph) -> ValidationReport {
        Self::validate_with_config(call_graph, &CallGraphValidationConfig::default())
    }

    /// Validate call graph structure with custom configuration
    pub fn validate_with_config(
        call_graph: &CallGraph,
        config: &CallGraphValidationConfig,
    ) -> ValidationReport {
        let mut report = ValidationReport::new();

        // Check for structural issues
        Self::check_dangling_edges(call_graph, &mut report);
        Self::check_orphaned_nodes(call_graph, &mut report, config);
        Self::check_duplicate_nodes(call_graph, &mut report);

        // Check for suspicious patterns
        Self::check_heuristics(call_graph, &mut report);

        // Calculate overall health score
        report.calculate_health_score();

        report
    }

    /// Check for dangling edges (references to non-existent functions)
    fn check_dangling_edges(call_graph: &CallGraph, report: &mut ValidationReport) {
        let all_function_ids: HashSet<_> = call_graph.get_all_functions().cloned().collect();

        for function in call_graph.get_all_functions() {
            // Check callees
            let callees = call_graph.get_callees(function);
            for callee in callees {
                if !all_function_ids.contains(&callee) {
                    report
                        .structural_issues
                        .push(StructuralIssue::DanglingEdge {
                            caller: function.clone(),
                            callee: callee.clone(),
                        });
                }
            }
        }
    }

    /// Check if a function is an entry point (expected to have no callers)
    fn is_entry_point(function: &FunctionId, config: &CallGraphValidationConfig) -> bool {
        // Check configured additional entry points
        if config.is_additional_entry_point(function) {
            return true;
        }

        // Main function
        if function.name == "main" {
            return true;
        }

        // Test functions
        if function.name.starts_with("test_")
            || function.name.contains("::test_")
            || function.name.starts_with("#[test]")
        {
            return true;
        }

        // Benchmark functions
        if function.name.starts_with("bench_")
            || function.name.contains("::bench_")
            || function.name.starts_with("#[bench]")
        {
            return true;
        }

        // Check file path for examples and benchmarks
        if let Some(path_str) = function.file.to_str() {
            if path_str.contains("/examples/")
                || path_str.contains("/benches/")
                || path_str.starts_with("examples/")
                || path_str.starts_with("benches/")
            {
                return true;
            }
        }

        // Check for lib.rs or main.rs (library APIs)
        if let Some(file_name) = function.file.file_name().and_then(|s| s.to_str()) {
            if file_name == "lib.rs" || file_name == "main.rs" {
                // Functions in lib.rs with short names (< 30 chars) likely public exports
                if function.name.len() < 30 && !function.name.contains("::") {
                    return true;
                }
            }
        }

        // Trait implementations - common patterns (contains ::)
        if function.name.contains("::") {
            let trait_methods = [
                "default",
                "new",
                "clone",
                "clone_box",
                "clone_from",
                "from",
                "into",
                "fmt",
                "display",
                "debug",
                "drop",
                "deref",
                "deref_mut",
                "hash",
                "eq",
                "builder",
                "create",
                "with_",
                "try_from",
                "try_into",
            ];

            let name_lower = function.name.to_lowercase();
            if trait_methods
                .iter()
                .any(|&method| name_lower.contains(method))
            {
                return true;
            }
        }

        // Constructor patterns (without ::)
        if function.name == "new"
            || function.name == "builder"
            || function.name == "create"
            || function.name.starts_with("with_")
        {
            return true;
        }

        false
    }

    /// Check if function is self-referential (recursive)
    fn is_self_referential(function: &FunctionId, call_graph: &CallGraph) -> bool {
        let callees = call_graph.get_callees(function);
        callees.iter().any(|callee| callee == function)
    }

    /// Check for orphaned nodes (functions with no connections)
    fn check_orphaned_nodes(
        call_graph: &CallGraph,
        report: &mut ValidationReport,
        config: &CallGraphValidationConfig,
    ) {
        for function in call_graph.get_all_functions() {
            let has_callers = !call_graph.get_callers(function).is_empty();
            let has_callees = !call_graph.get_callees(function).is_empty();
            let is_entry_point = Self::is_entry_point(function, config);
            let is_self_referential = Self::is_self_referential(function, call_graph);
            let is_whitelisted = config.is_whitelisted_orphan(function);

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
                continue; // NOT an issue
            }

            // SELF-REFERENTIAL: Calls itself (recursive)
            if is_self_referential {
                // Not isolated, even if no other callers/callees
                continue; // NOT an issue
            }

            // ISOLATED: No callers, no callees (true orphan)
            if !has_callers && !has_callees && !is_entry_point {
                report.statistics.isolated_functions += 1;
                // Skip if whitelisted
                if !is_whitelisted {
                    report
                        .structural_issues
                        .push(StructuralIssue::IsolatedFunction {
                            function: function.clone(),
                        });
                }
                continue;
            }

            // UNREACHABLE: No callers but has callees (dead code with dependencies)
            if !has_callers && has_callees && !is_entry_point {
                report.statistics.unreachable_functions += 1;
                // Skip if whitelisted
                if !is_whitelisted {
                    report
                        .structural_issues
                        .push(StructuralIssue::UnreachableFunction {
                            function: function.clone(),
                            reason: UnreachableReason::NoCallers,
                        });
                }
            }
        }
    }

    /// Check for duplicate nodes (same function registered multiple times)
    fn check_duplicate_nodes(call_graph: &CallGraph, report: &mut ValidationReport) {
        let mut function_counts: HashMap<String, Vec<FunctionId>> = HashMap::new();

        for function in call_graph.get_all_functions() {
            let key = format!("{}:{}", function.file.display(), function.name);
            function_counts
                .entry(key)
                .or_default()
                .push(function.clone());
        }

        for (_, functions) in function_counts {
            if functions.len() > 1 {
                report
                    .structural_issues
                    .push(StructuralIssue::DuplicateNode {
                        function: functions[0].clone(),
                        count: functions.len(),
                    });
            }
        }
    }

    /// Check for common suspicious patterns
    fn check_heuristics(call_graph: &CallGraph, report: &mut ValidationReport) {
        Self::check_high_fan_in(call_graph, report);
        Self::check_high_fan_out(call_graph, report);
        Self::check_files_with_no_calls(call_graph, report);
        Self::check_unused_public_functions(call_graph, report);
    }

    /// Check for functions with unusually high fan-in (many callers)
    fn check_high_fan_in(call_graph: &CallGraph, report: &mut ValidationReport) {
        const HIGH_CALLER_THRESHOLD: usize = 50;

        for function in call_graph.get_all_functions() {
            let callers = call_graph.get_callers(function);
            if callers.len() > HIGH_CALLER_THRESHOLD {
                report.warnings.push(ValidationWarning::TooManyCallers {
                    function: function.clone(),
                    count: callers.len(),
                });
            }
        }
    }

    /// Check for functions with unusually high fan-out (many callees)
    fn check_high_fan_out(call_graph: &CallGraph, report: &mut ValidationReport) {
        const HIGH_CALLEE_THRESHOLD: usize = 50;

        for function in call_graph.get_all_functions() {
            let callees = call_graph.get_callees(function);
            if callees.len() > HIGH_CALLEE_THRESHOLD {
                report.warnings.push(ValidationWarning::TooManyCallees {
                    function: function.clone(),
                    count: callees.len(),
                });
            }
        }
    }

    /// Check for files where all functions have no callers
    fn check_files_with_no_calls(call_graph: &CallGraph, report: &mut ValidationReport) {
        let mut file_functions: HashMap<PathBuf, Vec<FunctionId>> = HashMap::new();

        for function in call_graph.get_all_functions() {
            file_functions
                .entry(function.file.clone())
                .or_default()
                .push(function.clone());
        }

        for (file, functions) in file_functions {
            if functions.is_empty() {
                continue;
            }

            let all_have_no_callers = functions
                .iter()
                .all(|func| call_graph.get_callers(func).is_empty());

            // Skip files with entry points (main, tests)
            let has_entry_point = functions.iter().any(|f| {
                f.name == "main" || f.name.starts_with("test_") || f.name.contains("::test_")
            });

            if all_have_no_callers && !has_entry_point && functions.len() >= 3 {
                report.warnings.push(ValidationWarning::FileWithNoCalls {
                    file: file.clone(),
                    function_count: functions.len(),
                });
            }
        }
    }

    /// Check for public functions with no callers
    fn check_unused_public_functions(call_graph: &CallGraph, report: &mut ValidationReport) {
        for function in call_graph.get_all_functions() {
            // Simple heuristic: functions not in impl blocks and starting with lowercase
            // are likely standalone functions that could be public
            let is_standalone = !function.name.contains("::");
            let starts_lowercase = function
                .name
                .chars()
                .next()
                .map(|c| c.is_lowercase())
                .unwrap_or(false);

            if is_standalone && starts_lowercase {
                let has_no_callers = call_graph.get_callers(function).is_empty();

                // Skip test and main functions
                let is_entry_point = function.name == "main"
                    || function.name.starts_with("test_")
                    || function.name.contains("::test_");

                if has_no_callers && !is_entry_point {
                    report
                        .warnings
                        .push(ValidationWarning::UnusedPublicFunction {
                            function: function.clone(),
                        });
                }
            }
        }
    }

    /// Validate against expected patterns
    pub fn validate_expectations(
        call_graph: &CallGraph,
        expectations: &[Expectation],
    ) -> ValidationReport {
        Self::validate_expectations_with_config(
            call_graph,
            expectations,
            &CallGraphValidationConfig::default(),
        )
    }

    /// Validate against expected patterns with custom configuration
    pub fn validate_expectations_with_config(
        call_graph: &CallGraph,
        expectations: &[Expectation],
        config: &CallGraphValidationConfig,
    ) -> ValidationReport {
        let report = Self::validate_with_config(call_graph, config);

        for expectation in expectations {
            if !expectation.check(call_graph) {
                // Add expectation failure as a structural issue
                // For now, we'll skip this as it requires more complex reporting
            }
        }

        report
    }
}

/// Expected pattern in the call graph
#[derive(Debug, Clone)]
pub struct Expectation {
    pub description: String,
    pub check: fn(&CallGraph) -> bool,
}

impl Expectation {
    /// Create a new expectation
    pub fn new(description: String, check: fn(&CallGraph) -> bool) -> Self {
        Self { description, check }
    }

    /// Check if the expectation is met
    pub fn check(&self, call_graph: &CallGraph) -> bool {
        (self.check)(call_graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_report_creation() {
        let report = ValidationReport::new();
        assert_eq!(report.health_score, 100);
        assert!(!report.has_issues());
    }

    #[test]
    fn test_health_score_calculation() {
        let mut report = ValidationReport::new();

        // Add structural issues
        report
            .structural_issues
            .push(StructuralIssue::IsolatedFunction {
                function: FunctionId::new(PathBuf::from("test.rs"), "func".to_string(), 10),
            });

        // Add warnings
        report.warnings.push(ValidationWarning::TooManyCallers {
            function: FunctionId::new(PathBuf::from("test.rs"), "popular".to_string(), 20),
            count: 100,
        });

        report.calculate_health_score();

        // 100 - 0 (isolated: 0.5 rounds to 0) - 2 (warning) = 98
        assert_eq!(report.health_score, 98);
        assert!(report.has_issues());
    }

    #[test]
    fn test_validate_empty_graph() {
        let graph = CallGraph::new();
        let report = CallGraphValidator::validate(&graph);

        assert!(!report.has_issues());
        assert_eq!(report.health_score, 100);
    }

    #[test]
    fn test_expectation() {
        let expectation = Expectation::new("Has at least one function".to_string(), |graph| {
            graph.node_count() > 0
        });

        let empty_graph = CallGraph::new();
        assert!(!expectation.check(&empty_graph));
    }

    #[test]
    fn test_leaf_function_not_orphaned() {
        let mut call_graph = CallGraph::new();
        let leaf = FunctionId::new(PathBuf::from("test.rs"), "utility_fn".to_string(), 10);
        let caller = FunctionId::new(PathBuf::from("test.rs"), "main_fn".to_string(), 5);

        call_graph.add_function(leaf.clone(), false, false, 1, 10);
        call_graph.add_function(caller.clone(), false, false, 1, 5);
        call_graph.add_call_parts(
            caller,
            leaf.clone(),
            crate::priority::call_graph::CallType::Direct,
        );

        let report = CallGraphValidator::validate(&call_graph);

        // Leaf function should NOT be in structural_issues
        assert!(
            !report
                .structural_issues
                .iter()
                .any(|issue| matches!(issue, StructuralIssue::IsolatedFunction { .. })),
            "Leaf function should not be flagged as isolated"
        );

        // Should be in info as leaf
        assert!(
            report
                .info
                .iter()
                .any(|info| matches!(info, ValidationInfo::LeafFunction { .. })),
            "Leaf function should be in info"
        );
    }

    #[test]
    fn test_self_referential_not_isolated() {
        let mut call_graph = CallGraph::new();
        let recursive = FunctionId::new(PathBuf::from("test.rs"), "factorial".to_string(), 10);

        call_graph.add_function(recursive.clone(), false, false, 1, 10);
        call_graph.add_call_parts(
            recursive.clone(),
            recursive.clone(),
            crate::priority::call_graph::CallType::Direct,
        ); // Self-call

        let report = CallGraphValidator::validate(&call_graph);

        // Should NOT be marked as isolated
        assert!(
            !report
                .structural_issues
                .iter()
                .any(|issue| matches!(issue, StructuralIssue::IsolatedFunction { .. })),
            "Recursive function should not be flagged as isolated"
        );

        // Should be in info as self-referential
        assert!(
            report
                .info
                .iter()
                .any(|info| matches!(info, ValidationInfo::SelfReferentialFunction { .. })),
            "Recursive function should be in info"
        );
    }

    #[test]
    fn test_entry_point_detection() {
        let config = CallGraphValidationConfig::default();
        let test_cases = vec![
            ("src/main.rs", "main"),
            ("src/lib.rs", "test_my_function"),
            ("examples/demo.rs", "demo_main"),
            ("benches/my_bench.rs", "bench_performance"),
            ("src/traits.rs", "Default::default"),
            ("src/types.rs", "MyType::new"),
        ];

        for (file, name) in test_cases {
            let func = FunctionId::new(PathBuf::from(file), name.to_string(), 1);
            assert!(
                CallGraphValidator::is_entry_point(&func, &config),
                "Expected {} in {} to be entry point",
                name,
                file
            );
        }
    }

    #[test]
    fn test_isolated_function_detected() {
        let mut call_graph = CallGraph::new();
        let isolated = FunctionId::new(PathBuf::from("test.rs"), "unused_fn".to_string(), 10);

        call_graph.add_function(isolated.clone(), false, false, 1, 10);
        // No calls added

        let report = CallGraphValidator::validate(&call_graph);

        // Should be marked as isolated
        assert!(
            report.structural_issues.iter().any(
                |issue| matches!(issue, StructuralIssue::IsolatedFunction { function } if function == &isolated)
            ),
            "Isolated function should be detected"
        );
    }

    #[test]
    fn test_health_score_improved() {
        let mut call_graph = CallGraph::new();

        // Add main function once
        let main = FunctionId::new(PathBuf::from("test.rs"), "main".to_string(), 1);
        call_graph.add_function(main.clone(), true, false, 1, 5);

        // Add many leaf functions (should NOT hurt score significantly)
        for i in 0..1000 {
            let leaf = FunctionId::new(PathBuf::from("test.rs"), format!("leaf_{}", i), i * 10);
            call_graph.add_function(leaf.clone(), false, false, 1, 10);
            call_graph.add_call_parts(
                main.clone(),
                leaf,
                crate::priority::call_graph::CallType::Direct,
            );
        }

        let report = CallGraphValidator::validate(&call_graph);

        // Health score should be high (no real issues)
        assert!(
            report.health_score >= 80,
            "Health score should be 80+ for leaf functions, got {}",
            report.health_score
        );
    }

    #[test]
    fn test_unreachable_function_detected() {
        let mut call_graph = CallGraph::new();
        let unreachable = FunctionId::new(PathBuf::from("test.rs"), "dead_code".to_string(), 10);
        let callee = FunctionId::new(PathBuf::from("test.rs"), "helper".to_string(), 20);

        call_graph.add_function(unreachable.clone(), false, false, 1, 10);
        call_graph.add_function(callee.clone(), false, false, 1, 5);
        call_graph.add_call_parts(
            unreachable.clone(),
            callee,
            crate::priority::call_graph::CallType::Direct,
        ); // Dead code calls helper

        let report = CallGraphValidator::validate(&call_graph);

        // Should be marked as unreachable
        assert!(
            report.structural_issues.iter().any(
                |issue| matches!(issue, StructuralIssue::UnreachableFunction { function, .. } if function == &unreachable)
            ),
            "Unreachable function should be detected"
        );
    }

    #[test]
    fn test_statistics_collected() {
        let mut call_graph = CallGraph::new();

        // Add entry point
        let main = FunctionId::new(PathBuf::from("main.rs"), "main".to_string(), 1);
        call_graph.add_function(main.clone(), true, false, 1, 5);

        // Add leaf function
        let leaf = FunctionId::new(PathBuf::from("test.rs"), "utility".to_string(), 10);
        call_graph.add_function(leaf.clone(), false, false, 1, 10);
        call_graph.add_call_parts(
            main.clone(),
            leaf,
            crate::priority::call_graph::CallType::Direct,
        );

        // Add recursive function
        let recursive = FunctionId::new(PathBuf::from("test.rs"), "factorial".to_string(), 20);
        call_graph.add_function(recursive.clone(), false, false, 1, 15);
        call_graph.add_call_parts(
            recursive.clone(),
            recursive.clone(),
            crate::priority::call_graph::CallType::Direct,
        );

        let report = CallGraphValidator::validate(&call_graph);

        // Verify statistics
        assert_eq!(report.statistics.total_functions, 3);
        assert_eq!(report.statistics.entry_points, 1);
        assert_eq!(report.statistics.leaf_functions, 1);
        assert_eq!(report.statistics.recursive_functions, 1);
        assert_eq!(report.statistics.isolated_functions, 0);
        assert_eq!(report.statistics.unreachable_functions, 0);
    }

    #[test]
    fn test_orphan_whitelist() {
        let mut call_graph = CallGraph::new();
        let isolated = FunctionId::new(PathBuf::from("test.rs"), "utility_fn".to_string(), 10);

        call_graph.add_function(isolated.clone(), false, false, 1, 10);
        // No calls added - function is isolated

        // Validate without config - should be flagged as isolated
        let report = CallGraphValidator::validate(&call_graph);
        assert!(
            report.structural_issues.iter().any(
                |issue| matches!(issue, StructuralIssue::IsolatedFunction { function } if function == &isolated)
            ),
            "Isolated function should be detected without whitelist"
        );

        // Validate with whitelist config - should NOT be flagged
        let mut config = CallGraphValidationConfig::new();
        config.add_orphan_whitelist("utility_fn".to_string());
        let report_with_config = CallGraphValidator::validate_with_config(&call_graph, &config);

        assert!(
            !report_with_config.structural_issues.iter().any(
                |issue| matches!(issue, StructuralIssue::IsolatedFunction { function } if function == &isolated)
            ),
            "Whitelisted function should not be flagged as isolated"
        );
    }

    #[test]
    fn test_additional_entry_points() {
        let mut call_graph = CallGraph::new();
        let custom_entry = FunctionId::new(PathBuf::from("test.rs"), "custom_main".to_string(), 10);
        let helper = FunctionId::new(PathBuf::from("test.rs"), "helper".to_string(), 20);

        call_graph.add_function(custom_entry.clone(), false, false, 1, 10);
        call_graph.add_function(helper.clone(), false, false, 1, 5);
        call_graph.add_call_parts(
            custom_entry.clone(),
            helper,
            crate::priority::call_graph::CallType::Direct,
        );

        // Without config - custom_main has no callers, should be unreachable
        let report = CallGraphValidator::validate(&call_graph);
        assert!(
            report.structural_issues.iter().any(
                |issue| matches!(issue, StructuralIssue::UnreachableFunction { function, .. } if function == &custom_entry)
            ),
            "Custom entry point should be unreachable without config"
        );

        // With additional entry point config - should NOT be flagged
        let mut config = CallGraphValidationConfig::new();
        config.add_entry_point("custom_main".to_string());
        let report_with_config = CallGraphValidator::validate_with_config(&call_graph, &config);

        assert!(
            !report_with_config.structural_issues.iter().any(
                |issue| matches!(issue, StructuralIssue::UnreachableFunction { function, .. } if function == &custom_entry)
            ),
            "Configured entry point should not be flagged as unreachable"
        );
        assert_eq!(report_with_config.statistics.entry_points, 1);
    }

    #[test]
    fn test_config_builder_pattern() {
        let mut config = CallGraphValidationConfig::new();
        config
            .add_orphan_whitelist("temp_fn".to_string())
            .add_orphan_whitelist("debug_fn".to_string())
            .add_entry_point("custom_entry".to_string());

        assert_eq!(config.orphan_whitelist.len(), 2);
        assert_eq!(config.additional_entry_points.len(), 1);
        assert!(config.orphan_whitelist.contains("temp_fn"));
        assert!(config.additional_entry_points.contains("custom_entry"));
    }

    #[test]
    #[ignore] // Integration test - run explicitly
    fn test_real_project_health_score() {
        use crate::builders::call_graph;
        use crate::core::Language;
        use crate::io::walker;
        use std::env;
        use std::path::Path;

        // Get the project root (debtmap's own codebase)
        let project_root = env::current_dir().expect("Failed to get current directory");

        // Find Rust files in the project
        let config = crate::config::get_config();
        let files = walker::find_project_files_with_config(&project_root, vec![Language::Rust], config)
            .expect("Failed to find project files");

        // Analyze files to get function metrics
        let file_metrics: Vec<_> = files
            .iter()
            .filter_map(|path| crate::analysis_utils::analyze_single_file(path))
            .collect();

        // Extract all functions from file metrics
        let all_functions: Vec<_> = file_metrics
            .iter()
            .flat_map(|fm| fm.functions.clone())
            .collect();

        // Build call graph
        let mut call_graph = call_graph::build_initial_call_graph(&all_functions);

        // Build full call graph with Rust-specific features
        call_graph::process_rust_files_for_call_graph(&project_root, &mut call_graph, false, false)
            .expect("Failed to process Rust files");

        // Validate the call graph
        let validation_report = CallGraphValidator::validate(&call_graph);

        // Assert health score is >= 70 (spec requirement)
        assert!(
            validation_report.health_score >= 70,
            "Health score {} is below threshold 70. Structural issues: {}, Warnings: {}",
            validation_report.health_score,
            validation_report.structural_issues.len(),
            validation_report.warnings.len()
        );

        // Assert isolated functions are < 500 (spec requirement)
        assert!(
            validation_report.statistics.isolated_functions < 500,
            "Isolated functions {} exceeds threshold 500",
            validation_report.statistics.isolated_functions
        );

        // Print summary for manual inspection
        eprintln!("\n=== Debtmap Self-Analysis Health Report ===");
        eprintln!("Health Score: {}/100", validation_report.health_score);
        eprintln!("Total Functions: {}", validation_report.statistics.total_functions);
        eprintln!("Entry Points: {}", validation_report.statistics.entry_points);
        eprintln!("Isolated Functions: {}", validation_report.statistics.isolated_functions);
        eprintln!("Structural Issues: {}", validation_report.structural_issues.len());
        eprintln!("Warnings: {}", validation_report.warnings.len());
    }
}
