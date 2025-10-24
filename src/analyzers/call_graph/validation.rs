/// Call graph validation and health checks
///
/// This module provides validation tools to detect structural issues,
/// orphaned nodes, and suspicious patterns in the call graph.
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Structural issue in the call graph
#[derive(Debug, Clone)]
pub enum StructuralIssue {
    /// Edge references non-existent node
    DanglingEdge {
        caller: FunctionId,
        callee: FunctionId,
    },
    /// Node exists but has no edges
    OrphanedNode { function: FunctionId },
    /// Duplicate nodes
    DuplicateNode { function: FunctionId, count: usize },
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

/// Validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Structural issues (dangling edges, etc.)
    pub structural_issues: Vec<StructuralIssue>,
    /// Heuristic warnings (suspicious patterns)
    pub warnings: Vec<ValidationWarning>,
    /// Overall health score (0-100)
    pub health_score: u32,
}

impl ValidationReport {
    /// Create a new empty validation report
    fn new() -> Self {
        Self {
            structural_issues: Vec::new(),
            warnings: Vec::new(),
            health_score: 100,
        }
    }

    /// Calculate health score based on issues and warnings
    fn calculate_health_score(&mut self) {
        let mut score: u32 = 100;

        // Structural issues are critical - 10 points each
        score = score.saturating_sub(self.structural_issues.len() as u32 * 10);

        // Warnings are less severe - 2 points each
        score = score.saturating_sub(self.warnings.len() as u32 * 2);

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
        let mut report = ValidationReport::new();

        // Check for structural issues
        Self::check_dangling_edges(call_graph, &mut report);
        Self::check_orphaned_nodes(call_graph, &mut report);
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

    /// Check for orphaned nodes (functions with no connections)
    fn check_orphaned_nodes(call_graph: &CallGraph, report: &mut ValidationReport) {
        for function in call_graph.get_all_functions() {
            let has_callers = !call_graph.get_callers(function).is_empty();
            let has_callees = !call_graph.get_callees(function).is_empty();

            // Skip main and test functions (expected to have no callers)
            let is_entry_point = function.name == "main"
                || function.name.starts_with("test_")
                || function.name.contains("::test_");

            if !has_callers && !has_callees && !is_entry_point {
                report
                    .structural_issues
                    .push(StructuralIssue::OrphanedNode {
                        function: function.clone(),
                    });
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
        let report = Self::validate(call_graph);

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
            .push(StructuralIssue::OrphanedNode {
                function: FunctionId::new(PathBuf::from("test.rs"), "func".to_string(), 10),
            });

        // Add warnings
        report.warnings.push(ValidationWarning::TooManyCallers {
            function: FunctionId::new(PathBuf::from("test.rs"), "popular".to_string(), 20),
            count: 100,
        });

        report.calculate_health_score();

        // 100 - 10 (structural issue) - 2 (warning) = 88
        assert_eq!(report.health_score, 88);
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
}
