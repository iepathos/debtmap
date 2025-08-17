pub mod call_graph;
pub mod coverage_propagation;
pub mod debt_aggregator;
pub mod external_api_detector;
pub mod formatter;
pub mod formatter_markdown;
pub mod semantic_classifier;
pub mod unified_scorer;

use serde::{Deserialize, Serialize};

pub use call_graph::{CallGraph, FunctionCall};
pub use coverage_propagation::{calculate_transitive_coverage, TransitiveCoverage};
pub use debt_aggregator::{DebtAggregator, FunctionId as AggregatorFunctionId};
pub use formatter::{format_priorities, OutputFormat};
pub use formatter_markdown::format_priorities_markdown;
pub use semantic_classifier::{classify_function_role, FunctionRole};
pub use unified_scorer::{calculate_unified_priority, Location, UnifiedDebtItem, UnifiedScore};

use im::Vector;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAnalysis {
    pub items: Vector<UnifiedDebtItem>,
    pub total_impact: ImpactMetrics,
    pub total_debt_score: f64,
    pub call_graph: CallGraph,
    pub overall_coverage: Option<f64>,
}

// Single function analysis for evidence-based risk calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionAnalysis {
    pub file: PathBuf,
    pub function: String,
    pub line: usize,
    pub function_length: usize,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub nesting_depth: u32,
    pub is_test: bool,
    pub visibility: FunctionVisibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactMetrics {
    pub coverage_improvement: f64,
    pub lines_reduction: u32,
    pub complexity_reduction: f64,
    pub risk_reduction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionableRecommendation {
    pub primary_action: String,
    pub rationale: String,
    pub implementation_steps: Vec<String>,
    pub related_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebtType {
    TestingGap {
        coverage: f64,
        cyclomatic: u32,
        cognitive: u32,
    },
    ComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
    },
    DeadCode {
        visibility: FunctionVisibility,
        cyclomatic: u32,
        cognitive: u32,
        usage_hints: Vec<String>,
    },
    Orchestration {
        delegates_to: Vec<String>,
    },
    Duplication {
        instances: u32,
        total_lines: u32,
    },
    Risk {
        risk_score: f64,
        factors: Vec<String>,
    },
    // Test-specific debt types
    TestComplexityHotspot {
        cyclomatic: u32,
        cognitive: u32,
        threshold: u32,
    },
    TestTodo {
        priority: crate::core::Priority,
        reason: Option<String>,
    },
    TestDuplication {
        instances: u32,
        total_lines: u32,
        similarity: f64,
    },
    ErrorSwallowing {
        pattern: String,
        context: Option<String>,
    },
    // Security debt types
    HardcodedSecrets {
        secret_type: String,
        severity: String,
    },
    WeakCryptography {
        algorithm: String,
        recommendation: String,
    },
    SqlInjectionRisk {
        query_pattern: String,
        risk_level: String,
    },
    UnsafeCode {
        justification: Option<String>,
        safety_concern: String,
    },
    InputValidationGap {
        input_type: String,
        validation_missing: String,
    },
    // Performance debt types
    AllocationInefficiency {
        pattern: String,
        impact: String,
    },
    StringConcatenation {
        loop_type: String,
        iterations: Option<u32>,
    },
    NestedLoops {
        depth: u32,
        complexity_estimate: String,
    },
    BlockingIO {
        operation: String,
        context: String,
    },
    SuboptimalDataStructure {
        current_type: String,
        recommended_type: String,
    },
    // Organization debt types
    GodObject {
        responsibility_count: u32,
        complexity_score: f64,
    },
    FeatureEnvy {
        external_class: String,
        usage_ratio: f64,
    },
    PrimitiveObsession {
        primitive_type: String,
        domain_concept: String,
    },
    MagicValues {
        value: String,
        occurrences: u32,
    },
    // Testing quality debt types
    AssertionComplexity {
        assertion_count: u32,
        complexity_score: f64,
    },
    FlakyTestPattern {
        pattern_type: String,
        reliability_impact: String,
    },
    // Resource management debt types
    AsyncMisuse {
        pattern: String,
        performance_impact: String,
    },
    ResourceLeak {
        resource_type: String,
        cleanup_missing: String,
    },
    CollectionInefficiency {
        collection_type: String,
        inefficiency_type: String,
    },
    // Basic Security and Performance debt types (for core::DebtType integration)
    BasicSecurity {
        vulnerability_type: String,
        severity: String,
        description: String,
    },
    BasicPerformance {
        issue_type: String,
        impact: String,
        description: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FunctionVisibility {
    Private,
    Crate,
    Public,
}

impl UnifiedAnalysis {
    pub fn new(call_graph: CallGraph) -> Self {
        Self {
            items: Vector::new(),
            total_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            total_debt_score: 0.0,
            call_graph,
            overall_coverage: None,
        }
    }

    pub fn add_item(&mut self, item: UnifiedDebtItem) {
        // Get configurable thresholds
        let min_score = crate::config::get_minimum_debt_score();
        let min_cyclomatic = crate::config::get_minimum_cyclomatic_complexity();
        let min_cognitive = crate::config::get_minimum_cognitive_complexity();
        let min_risk = crate::config::get_minimum_risk_score();

        // Filter out items below minimum thresholds
        if item.unified_score.final_score < min_score {
            return;
        }

        // Check risk score threshold for Risk debt types
        if let DebtType::Risk { risk_score, .. } = &item.debt_type {
            if *risk_score < min_risk {
                return;
            }
        }

        // For non-test items, also check complexity thresholds
        // This helps filter out trivial functions that aren't really debt
        if !matches!(
            item.debt_type,
            DebtType::TestComplexityHotspot { .. }
                | DebtType::TestTodo { .. }
                | DebtType::TestDuplication { .. }
                | DebtType::BasicSecurity { .. }
                | DebtType::BasicPerformance { .. }
        ) && item.cyclomatic_complexity <= min_cyclomatic
            && item.cognitive_complexity <= min_cognitive
        {
            // Skip trivial functions unless they have other significant issues
            // (like being completely untested critical paths)
            if item.unified_score.coverage_factor < 8.0 {
                return;
            }
        }

        self.items.push_back(item);
    }

    /// Convert core::DebtItem (Security/Performance) to UnifiedDebtItem for unified analysis
    pub fn add_security_performance_items(
        &mut self,
        debt_items: &[crate::core::DebtItem],
        call_graph: &CallGraph,
    ) {
        for debt_item in debt_items {
            if let Some(unified_item) = self.convert_debt_item_to_unified(debt_item, call_graph) {
                self.add_item(unified_item);
            }
        }
    }

    fn convert_debt_item_to_unified(
        &self,
        debt_item: &crate::core::DebtItem,
        call_graph: &CallGraph,
    ) -> Option<UnifiedDebtItem> {
        use crate::core::DebtType as CoreDebtType;

        // Only process Security and Performance debt items
        match debt_item.debt_type {
            CoreDebtType::Security => self.create_security_unified_item(debt_item, call_graph),
            CoreDebtType::Performance => {
                self.create_performance_unified_item(debt_item, call_graph)
            }
            _ => None,
        }
    }

    fn create_security_unified_item(
        &self,
        debt_item: &crate::core::DebtItem,
        call_graph: &CallGraph,
    ) -> Option<UnifiedDebtItem> {
        // Extract security details from the message
        let (vulnerability_type, severity) = self.parse_security_details(&debt_item.message);

        let debt_type = DebtType::BasicSecurity {
            vulnerability_type: vulnerability_type.clone(),
            severity: severity.clone(),
            description: debt_item.message.clone(),
        };

        // Security issues don't need synthetic function metrics

        // Calculate unified score with security priority boost
        let unified_score = self.calculate_security_score(&severity, &debt_item.priority);

        // Try to find the actual function name at this location
        let function_name = call_graph
            .find_function_at_location(&debt_item.file, debt_item.line)
            .map(|func_id| func_id.name)
            .unwrap_or_else(|| format!("security_issue_at_line_{}", debt_item.line));

        Some(UnifiedDebtItem {
            location: Location {
                file: debt_item.file.clone(),
                function: function_name,
                line: debt_item.line,
            },
            debt_type,
            unified_score,
            function_role: FunctionRole::Unknown,
            recommendation: self.create_security_recommendation(&vulnerability_type, &severity),
            expected_impact: self.create_security_impact(&severity),
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 1,
            cyclomatic_complexity: 1,
            cognitive_complexity: 1,
        })
    }

    fn create_performance_unified_item(
        &self,
        debt_item: &crate::core::DebtItem,
        call_graph: &CallGraph,
    ) -> Option<UnifiedDebtItem> {
        // Extract performance details from the message
        let (issue_type, impact) = self.parse_performance_details(&debt_item.message);

        let debt_type = DebtType::BasicPerformance {
            issue_type: issue_type.clone(),
            impact: impact.clone(),
            description: debt_item.message.clone(),
        };

        // Calculate unified score with performance considerations
        let unified_score = self.calculate_performance_score(&impact, &debt_item.priority);

        // Try to find the actual function name at this location
        let function_name = call_graph
            .find_function_at_location(&debt_item.file, debt_item.line)
            .map(|func_id| func_id.name)
            .unwrap_or_else(|| format!("performance_issue_at_line_{}", debt_item.line));

        Some(UnifiedDebtItem {
            location: Location {
                file: debt_item.file.clone(),
                function: function_name,
                line: debt_item.line,
            },
            debt_type,
            unified_score,
            function_role: FunctionRole::Unknown,
            recommendation: self.create_performance_recommendation(&issue_type, &impact),
            expected_impact: self.create_performance_impact(&impact),
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 1,
            cyclomatic_complexity: 1,
            cognitive_complexity: 1,
        })
    }

    // Helper methods for Security/Performance debt conversion

    /// Classifies a security message into a vulnerability type
    fn classify_vulnerability_type(message: &str) -> &'static str {
        let lower_msg = message.to_lowercase();
        match () {
            _ if lower_msg.contains("unsafe") => "Unsafe Code",
            _ if lower_msg.contains("sql injection") || lower_msg.contains("sql") => {
                "SQL Injection"
            }
            _ if lower_msg.contains("secret")
                || lower_msg.contains("password")
                || lower_msg.contains("api key")
                || lower_msg.contains("private key") =>
            {
                "Hardcoded Secret"
            }
            _ if lower_msg.contains("crypto") || lower_msg.contains("encryption") => {
                "Weak Cryptography"
            }
            _ if lower_msg.contains("validation") || lower_msg.contains("input") => {
                "Input Validation"
            }
            _ => "Security Issue",
        }
    }

    /// Determines the severity level for a given vulnerability type
    fn determine_severity(vulnerability_type: &str) -> &'static str {
        match vulnerability_type {
            "SQL Injection" | "Hardcoded Secret" => "Critical",
            "Unsafe Code" | "Weak Cryptography" => "High",
            _ => "Medium",
        }
    }

    fn parse_security_details(&self, message: &str) -> (String, String) {
        let vulnerability_type = Self::classify_vulnerability_type(message);
        let severity = Self::determine_severity(vulnerability_type);
        (vulnerability_type.to_string(), severity.to_string())
    }

    /// Classifies a performance message into an issue type
    fn classify_performance_issue(message: &str) -> &'static str {
        let lower_msg = message.to_lowercase();
        match () {
            _ if lower_msg.contains("nested loop") => "Nested Loops",
            _ if lower_msg.contains("allocation") || lower_msg.contains("memory") => {
                "Memory Allocation"
            }
            _ if lower_msg.contains("i/o") || lower_msg.contains("blocking") => "Blocking I/O",
            _ if lower_msg.contains("string") && lower_msg.contains("concatenation") => {
                "String Concatenation"
            }
            _ if lower_msg.contains("data structure") || message.contains("Vec::contains") => {
                "Data Structure"
            }
            _ => "Performance Issue",
        }
    }

    /// Determines the impact level for a given performance issue
    fn determine_performance_impact(issue_type: &str) -> &'static str {
        match issue_type {
            "Nested Loops" | "Blocking I/O" => "High",
            _ => "Medium",
        }
    }

    fn parse_performance_details(&self, message: &str) -> (String, String) {
        let issue_type = Self::classify_performance_issue(message);
        let impact = Self::determine_performance_impact(issue_type);
        (issue_type.to_string(), impact.to_string())
    }

    fn calculate_security_score(
        &self,
        severity: &str,
        priority: &crate::core::Priority,
    ) -> UnifiedScore {
        use crate::core::Priority;

        // Security issues get high base scores regardless of function complexity
        let base_score: f64 = match severity {
            "Critical" => 9.5,
            "High" => 8.5,
            "Medium" => 7.0,
            _ => 5.0,
        };

        let priority_boost: f64 = match priority {
            Priority::Critical => 1.0,
            Priority::High => 0.8,
            Priority::Medium => 0.5,
            Priority::Low => 0.2,
        };

        UnifiedScore {
            complexity_factor: 2.0,   // Security issues aren't about complexity
            coverage_factor: 1.0,     // Coverage less relevant for security
            roi_factor: 8.0,          // High ROI to fix security issues
            semantic_factor: 9.0,     // Very important semantically
            dependency_factor: 3.0,   // Variable depending on code location
            security_factor: 10.0,    // Maximum security factor for security issues
            organization_factor: 0.0, // Not an organization issue
            performance_factor: 0.0,  // Not a performance issue
            role_multiplier: 1.5,     // Security issues are always important
            final_score: (base_score + priority_boost).min(10.0_f64),
        }
    }

    fn calculate_performance_score(
        &self,
        impact: &str,
        priority: &crate::core::Priority,
    ) -> UnifiedScore {
        use crate::core::Priority;

        let base_score: f64 = match impact {
            "High" | "Critical" => 7.5,
            "Medium" => 6.0,
            "Low" => 4.5,
            _ => 3.0,
        };

        let priority_boost: f64 = match priority {
            Priority::Critical => 1.0,
            Priority::High => 0.8,
            Priority::Medium => 0.5,
            Priority::Low => 0.2,
        };

        UnifiedScore {
            complexity_factor: 3.0,   // Performance often relates to complexity
            coverage_factor: 2.0,     // Testing helps catch performance regressions
            roi_factor: 6.0,          // Good ROI for performance fixes
            semantic_factor: 5.0,     // Important but not as critical as security
            dependency_factor: 4.0,   // Performance issues can affect many callers
            security_factor: 0.0,     // Not a security issue
            organization_factor: 0.0, // Not an organization issue
            performance_factor: 10.0, // Maximum performance factor for performance issues
            role_multiplier: 1.2,     // Performance issues are important
            final_score: (base_score + priority_boost).min(10.0_f64),
        }
    }

    fn create_security_recommendation(
        &self,
        vulnerability_type: &str,
        severity: &str,
    ) -> ActionableRecommendation {
        let primary_action = format!("Fix {} security vulnerability", vulnerability_type);
        let rationale = format!(
            "Security vulnerability ({}) detected: {}",
            severity, vulnerability_type
        );
        let implementation_steps = vec![
            "Review security vulnerability details".to_string(),
            "Apply security best practices".to_string(),
            "Test fix thoroughly".to_string(),
            "Consider security review".to_string(),
        ];

        ActionableRecommendation {
            primary_action,
            rationale,
            implementation_steps,
            related_items: vec![],
        }
    }

    fn create_performance_recommendation(
        &self,
        issue_type: &str,
        impact: &str,
    ) -> ActionableRecommendation {
        let primary_action = format!("Optimize {} performance issue", issue_type);
        let rationale = format!("Performance issue ({}) detected: {}", impact, issue_type);
        let implementation_steps = vec![
            "Profile performance bottleneck".to_string(),
            "Apply optimization techniques".to_string(),
            "Benchmark improvements".to_string(),
            "Verify no regressions".to_string(),
        ];

        ActionableRecommendation {
            primary_action,
            rationale,
            implementation_steps,
            related_items: vec![],
        }
    }

    fn create_security_impact(&self, severity: &str) -> ImpactMetrics {
        let risk_reduction = match severity {
            "Critical" => 9.0,
            "High" => 7.0,
            "Medium" => 5.0,
            _ => 3.0,
        };

        ImpactMetrics {
            coverage_improvement: 0.0, // Security fixes don't typically improve coverage
            lines_reduction: 3,        // Security fixes usually involve small code changes
            complexity_reduction: 0.0, // Security fixes don't reduce complexity
            risk_reduction,
        }
    }

    fn create_performance_impact(&self, impact: &str) -> ImpactMetrics {
        let (risk_reduction, lines_reduction) = match impact {
            "High" | "Critical" => (6.0, 15),
            "Medium" => (4.0, 8),
            "Low" => (2.0, 3),
            _ => (1.0, 2),
        };

        ImpactMetrics {
            coverage_improvement: 0.0, // Performance fixes don't typically improve coverage
            lines_reduction,
            complexity_reduction: 1.0, // Performance fixes may reduce algorithmic complexity
            risk_reduction,
        }
    }

    pub fn sort_by_priority(&mut self) {
        let mut items_vec: Vec<UnifiedDebtItem> = self.items.iter().cloned().collect();
        items_vec.sort_by(|a, b| {
            b.unified_score
                .final_score
                .partial_cmp(&a.unified_score.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.items = items_vec.into_iter().collect();
    }

    pub fn calculate_total_impact(&mut self) {
        let mut coverage_improvement = 0.0;
        let mut lines_reduction = 0;
        let mut complexity_reduction = 0.0;
        let mut risk_reduction = 0.0;
        let mut _functions_to_test = 0;
        let mut total_debt_score = 0.0;

        for item in &self.items {
            // Sum up all final scores as the total debt score
            total_debt_score += item.unified_score.final_score;

            // Only count functions that actually need testing
            if item.expected_impact.coverage_improvement > 0.0 {
                _functions_to_test += 1;
                // Each function contributes a small amount to overall coverage
                // Estimate based on function count (rough approximation)
                coverage_improvement += item.expected_impact.coverage_improvement / 100.0;
            }
            lines_reduction += item.expected_impact.lines_reduction;
            complexity_reduction += item.expected_impact.complexity_reduction;
            risk_reduction += item.expected_impact.risk_reduction;
        }

        // Coverage improvement is the estimated overall project coverage gain
        // Assuming tested functions represent a portion of the codebase
        coverage_improvement = (coverage_improvement * 5.0).min(100.0); // Scale factor for visibility

        // Total complexity reduction (sum of all reductions)
        let total_complexity_reduction = complexity_reduction;

        self.total_debt_score = total_debt_score;
        self.total_impact = ImpactMetrics {
            coverage_improvement,
            lines_reduction,
            complexity_reduction: total_complexity_reduction,
            risk_reduction,
        };
    }

    pub fn get_top_priorities(&self, n: usize) -> Vector<UnifiedDebtItem> {
        self.items.iter().take(n).cloned().collect()
    }

    pub fn get_bottom_priorities(&self, n: usize) -> Vector<UnifiedDebtItem> {
        let total_items = self.items.len();
        if total_items <= n {
            self.items.clone()
        } else {
            self.items.iter().skip(total_items - n).cloned().collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::CallGraph;

    #[test]
    fn test_classify_vulnerability_type_unsafe_code() {
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("Found unsafe block in function"),
            "Unsafe Code"
        );
    }

    #[test]
    fn test_classify_vulnerability_type_sql_injection() {
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("Potential SQL injection vulnerability"),
            "SQL Injection"
        );
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("Direct sql query construction"),
            "SQL Injection"
        );
    }

    #[test]
    fn test_classify_vulnerability_type_hardcoded_secret() {
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("Found hardcoded secret in code"),
            "Hardcoded Secret"
        );
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("Password stored in plaintext"),
            "Hardcoded Secret"
        );
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("API key exposed in source"),
            "Hardcoded Secret"
        );
    }

    #[test]
    fn test_classify_vulnerability_type_weak_crypto() {
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("Using weak crypto algorithm"),
            "Weak Cryptography"
        );
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("Insecure encryption method"),
            "Weak Cryptography"
        );
    }

    #[test]
    fn test_classify_vulnerability_type_input_validation() {
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("Missing input validation"),
            "Input Validation"
        );
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("No validation on user input"),
            "Input Validation"
        );
    }

    #[test]
    fn test_classify_vulnerability_type_generic() {
        assert_eq!(
            UnifiedAnalysis::classify_vulnerability_type("Some other security concern"),
            "Security Issue"
        );
    }

    #[test]
    fn test_determine_severity_critical() {
        assert_eq!(
            UnifiedAnalysis::determine_severity("SQL Injection"),
            "Critical"
        );
        assert_eq!(
            UnifiedAnalysis::determine_severity("Hardcoded Secret"),
            "Critical"
        );
    }

    #[test]
    fn test_determine_severity_high() {
        assert_eq!(UnifiedAnalysis::determine_severity("Unsafe Code"), "High");
        assert_eq!(
            UnifiedAnalysis::determine_severity("Weak Cryptography"),
            "High"
        );
    }

    #[test]
    fn test_determine_severity_medium() {
        assert_eq!(
            UnifiedAnalysis::determine_severity("Input Validation"),
            "Medium"
        );
        assert_eq!(
            UnifiedAnalysis::determine_severity("Security Issue"),
            "Medium"
        );
        assert_eq!(
            UnifiedAnalysis::determine_severity("Unknown Issue"),
            "Medium"
        );
    }

    #[test]
    fn test_classify_performance_issue_nested_loops() {
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Found nested loop in algorithm"),
            "Nested Loops"
        );
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Nested loop detected"),
            "Nested Loops"
        );
    }

    #[test]
    fn test_classify_performance_issue_memory() {
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Excessive memory allocation"),
            "Memory Allocation"
        );
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Large allocation in hot path"),
            "Memory Allocation"
        );
    }

    #[test]
    fn test_classify_performance_issue_blocking_io() {
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Synchronous I/O in async context"),
            "Blocking I/O"
        );
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Blocking operation detected"),
            "Blocking I/O"
        );
    }

    #[test]
    fn test_classify_performance_issue_string_concat() {
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Inefficient string concatenation in loop"),
            "String Concatenation"
        );
    }

    #[test]
    fn test_classify_performance_issue_data_structure() {
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Using Vec::contains in hot path"),
            "Data Structure"
        );
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Inefficient data structure choice"),
            "Data Structure"
        );
    }

    #[test]
    fn test_classify_performance_issue_generic() {
        assert_eq!(
            UnifiedAnalysis::classify_performance_issue("Some performance concern"),
            "Performance Issue"
        );
    }

    #[test]
    fn test_determine_performance_impact_high() {
        assert_eq!(
            UnifiedAnalysis::determine_performance_impact("Nested Loops"),
            "High"
        );
        assert_eq!(
            UnifiedAnalysis::determine_performance_impact("Blocking I/O"),
            "High"
        );
    }

    #[test]
    fn test_determine_performance_impact_medium() {
        assert_eq!(
            UnifiedAnalysis::determine_performance_impact("Memory Allocation"),
            "Medium"
        );
        assert_eq!(
            UnifiedAnalysis::determine_performance_impact("String Concatenation"),
            "Medium"
        );
        assert_eq!(
            UnifiedAnalysis::determine_performance_impact("Data Structure"),
            "Medium"
        );
        assert_eq!(
            UnifiedAnalysis::determine_performance_impact("Performance Issue"),
            "Medium"
        );
    }

    #[test]
    fn test_parse_security_details_integration() {
        let analysis = UnifiedAnalysis::new(CallGraph::default());

        let (vuln_type, severity) = analysis.parse_security_details("SQL injection found");
        assert_eq!(vuln_type, "SQL Injection");
        assert_eq!(severity, "Critical");

        let (vuln_type, severity) = analysis.parse_security_details("unsafe code block");
        assert_eq!(vuln_type, "Unsafe Code");
        assert_eq!(severity, "High");
    }

    #[test]
    fn test_parse_performance_details_integration() {
        let analysis = UnifiedAnalysis::new(CallGraph::default());

        let (issue_type, impact) = analysis.parse_performance_details("nested loop detected");
        assert_eq!(issue_type, "Nested Loops");
        assert_eq!(impact, "High");

        let (issue_type, impact) = analysis.parse_performance_details("excessive memory usage");
        assert_eq!(issue_type, "Memory Allocation");
        assert_eq!(impact, "Medium");
    }
}
