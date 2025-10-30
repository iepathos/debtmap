//! Call Graph Pattern Detection for Responsibility Classification
//!
//! This module detects structural patterns in call graphs that indicate
//! specific responsibilities, integrating with I/O detection for comprehensive
//! function classification.

use crate::analysis::graph_metrics::GraphMetrics;
use crate::analysis::io_detection::IoProfile;
use crate::priority::call_graph::{CallGraph, FunctionId};
use serde::{Deserialize, Serialize};

/// Call graph structural patterns
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallGraphPattern {
    /// High outdegree, coordinates multiple operations
    Orchestrator,
    /// Zero outdegree, pure utility function
    LeafNode,
    /// High indegree, frequently called core function
    Hub,
    /// High betweenness, connects different modules
    Bridge,
    /// High clustering, part of tight functional group
    UtilityCluster,
    /// Calls I/O functions, boundary layer
    IoGateway,
}

/// Responsibility classification based on call graph and I/O patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsibilityClassification {
    /// Primary responsibility category
    pub primary: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// Evidence supporting this classification
    pub evidence: Vec<String>,
    /// Detected patterns
    pub patterns: Vec<CallGraphPattern>,
    /// Framework context (if detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework_context: Option<String>,
}

/// Pattern detector for call graphs
pub struct PatternDetector {
    /// Thresholds for pattern detection
    orchestrator_threshold: usize,
    hub_threshold: usize,
    betweenness_threshold: f64,
    clustering_threshold: f64,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternDetector {
    /// Create a new pattern detector with default thresholds
    pub fn new() -> Self {
        Self {
            orchestrator_threshold: 5,
            hub_threshold: 10,
            betweenness_threshold: 0.5,
            clustering_threshold: 0.6,
        }
    }

    /// Create a pattern detector with custom thresholds
    pub fn with_thresholds(
        orchestrator: usize,
        hub: usize,
        betweenness: f64,
        clustering: f64,
    ) -> Self {
        Self {
            orchestrator_threshold: orchestrator,
            hub_threshold: hub,
            betweenness_threshold: betweenness,
            clustering_threshold: clustering,
        }
    }

    /// Detect patterns for a function
    pub fn detect_patterns(
        &self,
        metrics: &GraphMetrics,
        io_profile: Option<&IoProfile>,
        call_graph: &CallGraph,
        function_id: &FunctionId,
    ) -> Vec<CallGraphPattern> {
        let mut patterns = Vec::new();

        // Orchestrator: High outdegree, coordinates operations
        if metrics.outdegree >= self.orchestrator_threshold && metrics.indegree <= 3 {
            patterns.push(CallGraphPattern::Orchestrator);
        }

        // Leaf node: No outgoing calls
        if metrics.outdegree == 0 {
            patterns.push(CallGraphPattern::LeafNode);
        }

        // Hub: Frequently called
        if metrics.indegree >= self.hub_threshold {
            patterns.push(CallGraphPattern::Hub);
        }

        // Bridge: High betweenness centrality
        if metrics.betweenness > self.betweenness_threshold {
            patterns.push(CallGraphPattern::Bridge);
        }

        // Utility cluster: Tight coupling with neighbors
        if metrics.clustering > self.clustering_threshold && metrics.indegree >= 3 {
            patterns.push(CallGraphPattern::UtilityCluster);
        }

        // I/O Gateway: Calls I/O functions or has I/O operations
        if let Some(profile) = io_profile {
            if !profile.is_pure || self.calls_io_functions(call_graph, function_id, profile) {
                patterns.push(CallGraphPattern::IoGateway);
            }
        }

        patterns
    }

    /// Check if function calls other I/O functions
    fn calls_io_functions(
        &self,
        call_graph: &CallGraph,
        function_id: &FunctionId,
        _io_profile: &IoProfile,
    ) -> bool {
        // For now, we check if any callees have I/O in their name
        // This should be enhanced with actual I/O profile propagation
        let callees = call_graph.get_callees(function_id);
        callees.iter().any(|callee| {
            let name = &callee.name;
            name.contains("read")
                || name.contains("write")
                || name.contains("file")
                || name.contains("io")
                || name.contains("fetch")
                || name.contains("request")
        })
    }

    /// Classify responsibility with framework context
    ///
    /// This method checks for framework patterns first, then falls back to
    /// standard classification based on call graph patterns and I/O profile.
    pub fn classify_with_framework(
        &self,
        patterns: &[CallGraphPattern],
        metrics: &GraphMetrics,
        io_profile: Option<&IoProfile>,
        framework_matches: &[crate::analysis::FrameworkMatch],
    ) -> ResponsibilityClassification {
        // Check for framework patterns first (highest priority)
        if let Some(framework_match) = framework_matches.first() {
            return ResponsibilityClassification {
                primary: framework_match.category.clone(),
                confidence: framework_match.confidence,
                evidence: framework_match.evidence.clone(),
                patterns: patterns.to_vec(),
                framework_context: Some(framework_match.framework.clone()),
            };
        }

        // Fall back to standard classification
        self.classify_responsibility(patterns, metrics, io_profile)
    }

    /// Classify Orchestrator pattern - functions that coordinate multiple operations
    ///
    /// Returns classification for functions with high outdegree, indicating they
    /// orchestrate complex workflows by calling many other functions.
    fn classify_orchestrator(metrics: &GraphMetrics) -> (String, f64, Vec<String>) {
        let primary = "Orchestration & Coordination".to_string();
        let confidence = 0.85;
        let evidence = vec![format!(
            "Calls {} functions, orchestrating complex workflow",
            metrics.outdegree
        )];
        (primary, confidence, evidence)
    }

    /// Classify IoGateway pattern - functions that handle I/O operations
    ///
    /// Returns classification for functions that act as gateways to external I/O,
    /// including file, network, and database operations.
    fn classify_io_gateway(io_profile: Option<&IoProfile>) -> (String, f64, Vec<String>) {
        let primary = "I/O & External Communication".to_string();
        let confidence = 0.80;
        let mut evidence = vec!["Acts as gateway to I/O operations".to_string()];

        if let Some(profile) = io_profile {
            if profile.has_file_io() {
                evidence.push("Performs file I/O operations".to_string());
            }
            if profile.has_network_io() {
                evidence.push("Performs network I/O operations".to_string());
            }
            if profile.has_database_io() {
                evidence.push("Performs database operations".to_string());
            }
        }

        (primary, confidence, evidence)
    }

    /// Classify Hub pattern - frequently called core functions
    ///
    /// Returns classification for functions with high indegree, indicating they
    /// are central to the module and called by many other functions.
    fn classify_hub(metrics: &GraphMetrics) -> (String, f64, Vec<String>) {
        let primary = "Core Business Logic".to_string();
        let confidence = 0.75;
        let evidence = vec![format!(
            "Called by {} functions, central to module",
            metrics.indegree
        )];
        (primary, confidence, evidence)
    }

    /// Classify Bridge pattern - functions that connect different modules
    ///
    /// Returns classification for functions with high betweenness centrality,
    /// indicating they bridge different parts of the codebase.
    fn classify_bridge(metrics: &GraphMetrics) -> (String, f64, Vec<String>) {
        let primary = "Module Integration".to_string();
        let confidence = 0.70;
        let evidence = vec![format!(
            "High betweenness centrality ({:.2}), connects modules",
            metrics.betweenness
        )];
        (primary, confidence, evidence)
    }

    /// Classify LeafNode pattern - functions with no outgoing calls
    ///
    /// Returns classification for leaf functions, distinguishing between pure
    /// computation (no side effects) and utility functions (with side effects)
    /// based on the I/O profile.
    fn classify_leaf_node(io_profile: Option<&IoProfile>) -> (String, f64, Vec<String>) {
        if let Some(profile) = io_profile {
            if profile.is_pure {
                let primary = "Pure Computation".to_string();
                let confidence = 0.75;
                let evidence = vec!["Pure function with no side effects or I/O".to_string()];
                (primary, confidence, evidence)
            } else {
                let primary = "Utility & Helper Functions".to_string();
                let confidence = 0.70;
                let evidence = vec!["Leaf function with side effects".to_string()];
                (primary, confidence, evidence)
            }
        } else {
            let primary = "Utility & Helper Functions".to_string();
            let confidence = 0.70;
            let evidence = vec!["Pure function with no external calls".to_string()];
            (primary, confidence, evidence)
        }
    }

    /// Classify UtilityCluster pattern - tightly coupled utility groups
    ///
    /// Returns classification for functions that are part of a tightly-connected
    /// functional group, indicated by high clustering coefficient.
    fn classify_utility_cluster(metrics: &GraphMetrics) -> (String, f64, Vec<String>) {
        let primary = "Domain-Specific Utilities".to_string();
        let confidence = 0.65;
        let evidence = vec![format!(
            "Part of tightly-connected functional group (clustering: {:.2})",
            metrics.clustering
        )];
        (primary, confidence, evidence)
    }

    /// Classify fallback when no specific pattern matches
    ///
    /// Returns classification based on I/O profile when available, or a generic
    /// classification when no strong pattern or I/O profile is detected.
    fn classify_fallback(io_profile: Option<&IoProfile>) -> (String, f64, Vec<String>) {
        if let Some(profile) = io_profile {
            let responsibility = profile.primary_responsibility();
            let primary = responsibility.as_str().to_string();
            let confidence = 0.50;
            let evidence = vec!["Classified based on I/O behavior".to_string()];
            (primary, confidence, evidence)
        } else {
            let primary = "General Logic".to_string();
            let confidence = 0.40;
            let evidence = vec!["No strong call graph pattern detected".to_string()];
            (primary, confidence, evidence)
        }
    }

    /// Classify responsibility based on detected patterns and I/O profile
    ///
    /// This method uses a priority-based strategy to classify functions:
    /// 1. Orchestrator - High outdegree, coordinates operations
    /// 2. IoGateway - Handles I/O operations
    /// 3. Hub - High indegree, core business logic
    /// 4. Bridge - High betweenness, connects modules
    /// 5. LeafNode - No outgoing calls (pure or utility)
    /// 6. UtilityCluster - Part of tightly-coupled group
    /// 7. Fallback - Uses I/O profile or generic classification
    ///
    /// Each classification is delegated to a specialized pure function
    /// for improved maintainability and testability.
    pub fn classify_responsibility(
        &self,
        patterns: &[CallGraphPattern],
        metrics: &GraphMetrics,
        io_profile: Option<&IoProfile>,
    ) -> ResponsibilityClassification {
        let (primary, confidence, evidence) = if patterns.contains(&CallGraphPattern::Orchestrator)
        {
            Self::classify_orchestrator(metrics)
        } else if patterns.contains(&CallGraphPattern::IoGateway) {
            Self::classify_io_gateway(io_profile)
        } else if patterns.contains(&CallGraphPattern::Hub) {
            Self::classify_hub(metrics)
        } else if patterns.contains(&CallGraphPattern::Bridge) {
            Self::classify_bridge(metrics)
        } else if patterns.contains(&CallGraphPattern::LeafNode) {
            Self::classify_leaf_node(io_profile)
        } else if patterns.contains(&CallGraphPattern::UtilityCluster) {
            Self::classify_utility_cluster(metrics)
        } else {
            Self::classify_fallback(io_profile)
        };

        ResponsibilityClassification {
            primary,
            confidence,
            evidence,
            patterns: patterns.to_vec(),
            framework_context: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_metrics(outdegree: usize, indegree: usize) -> GraphMetrics {
        GraphMetrics {
            outdegree,
            indegree,
            depth: 1,
            betweenness: 0.0,
            clustering: 0.0,
        }
    }

    #[test]
    fn test_orchestrator_pattern_detection() {
        let detector = PatternDetector::new();
        let metrics = create_test_metrics(6, 2); // High outdegree, low indegree

        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "orchestrator".to_string(), 1);

        let patterns = detector.detect_patterns(&metrics, None, &call_graph, &func_id);

        assert!(patterns.contains(&CallGraphPattern::Orchestrator));
    }

    #[test]
    fn test_leaf_node_pattern_detection() {
        let detector = PatternDetector::new();
        let metrics = create_test_metrics(0, 5); // No outgoing calls

        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "leaf".to_string(), 1);

        let patterns = detector.detect_patterns(&metrics, None, &call_graph, &func_id);

        assert!(patterns.contains(&CallGraphPattern::LeafNode));
    }

    #[test]
    fn test_hub_pattern_detection() {
        let detector = PatternDetector::new();
        let metrics = create_test_metrics(2, 15); // High indegree

        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "hub".to_string(), 1);

        let patterns = detector.detect_patterns(&metrics, None, &call_graph, &func_id);

        assert!(patterns.contains(&CallGraphPattern::Hub));
    }

    #[test]
    fn test_bridge_pattern_detection() {
        let detector = PatternDetector::new();
        let mut metrics = create_test_metrics(3, 3);
        metrics.betweenness = 0.6; // High betweenness

        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "bridge".to_string(), 1);

        let patterns = detector.detect_patterns(&metrics, None, &call_graph, &func_id);

        assert!(patterns.contains(&CallGraphPattern::Bridge));
    }

    #[test]
    fn test_utility_cluster_pattern_detection() {
        let detector = PatternDetector::new();
        let mut metrics = create_test_metrics(2, 5);
        metrics.clustering = 0.7; // High clustering

        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "utility".to_string(), 1);

        let patterns = detector.detect_patterns(&metrics, None, &call_graph, &func_id);

        assert!(patterns.contains(&CallGraphPattern::UtilityCluster));
    }

    #[test]
    fn test_io_gateway_pattern_detection() {
        let detector = PatternDetector::new();
        let metrics = create_test_metrics(2, 2);

        let mut io_profile = IoProfile::new();
        io_profile
            .file_operations
            .push(crate::analysis::io_detection::IoOperation::FileRead { path_expr: None });
        io_profile.is_pure = false;

        let call_graph = CallGraph::new();
        let func_id = FunctionId::new(PathBuf::from("test.rs"), "read_file".to_string(), 1);

        let patterns = detector.detect_patterns(&metrics, Some(&io_profile), &call_graph, &func_id);

        assert!(patterns.contains(&CallGraphPattern::IoGateway));
    }

    #[test]
    fn test_orchestrator_responsibility_classification() {
        let detector = PatternDetector::new();
        let metrics = create_test_metrics(6, 2);
        let patterns = vec![CallGraphPattern::Orchestrator];

        let classification = detector.classify_responsibility(&patterns, &metrics, None);

        assert_eq!(classification.primary, "Orchestration & Coordination");
        assert!(classification.confidence > 0.8);
        assert!(!classification.evidence.is_empty());
    }

    #[test]
    fn test_hub_responsibility_classification() {
        let detector = PatternDetector::new();
        let metrics = create_test_metrics(2, 15);
        let patterns = vec![CallGraphPattern::Hub];

        let classification = detector.classify_responsibility(&patterns, &metrics, None);

        assert_eq!(classification.primary, "Core Business Logic");
        assert!(classification.confidence > 0.7);
    }

    #[test]
    fn test_pure_leaf_responsibility_classification() {
        let detector = PatternDetector::new();
        let metrics = create_test_metrics(0, 5);
        let patterns = vec![CallGraphPattern::LeafNode];

        let mut io_profile = IoProfile::new();
        io_profile.is_pure = true;

        let classification =
            detector.classify_responsibility(&patterns, &metrics, Some(&io_profile));

        assert_eq!(classification.primary, "Pure Computation");
        assert!(classification.confidence > 0.7);
    }

    // Unit tests for extracted classification functions

    #[test]
    fn test_classify_orchestrator() {
        let metrics = create_test_metrics(6, 2);
        let (primary, confidence, evidence) = PatternDetector::classify_orchestrator(&metrics);

        assert_eq!(primary, "Orchestration & Coordination");
        assert_eq!(confidence, 0.85);
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].contains("6 functions"));
    }

    #[test]
    fn test_classify_io_gateway_with_profile() {
        let mut io_profile = IoProfile::new();
        io_profile
            .file_operations
            .push(crate::analysis::io_detection::IoOperation::FileRead { path_expr: None });
        io_profile.is_pure = false;

        let (primary, confidence, evidence) =
            PatternDetector::classify_io_gateway(Some(&io_profile));

        assert_eq!(primary, "I/O & External Communication");
        assert_eq!(confidence, 0.80);
        assert!(evidence.len() >= 2); // Base + file operation
        assert!(evidence.iter().any(|e| e.contains("file I/O")));
    }

    #[test]
    fn test_classify_io_gateway_without_profile() {
        let (primary, confidence, evidence) = PatternDetector::classify_io_gateway(None);

        assert_eq!(primary, "I/O & External Communication");
        assert_eq!(confidence, 0.80);
        assert_eq!(evidence.len(), 1);
    }

    #[test]
    fn test_classify_hub() {
        let metrics = create_test_metrics(2, 15);
        let (primary, confidence, evidence) = PatternDetector::classify_hub(&metrics);

        assert_eq!(primary, "Core Business Logic");
        assert_eq!(confidence, 0.75);
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].contains("15 functions"));
    }

    #[test]
    fn test_classify_bridge() {
        let mut metrics = create_test_metrics(3, 3);
        metrics.betweenness = 0.6;

        let (primary, confidence, evidence) = PatternDetector::classify_bridge(&metrics);

        assert_eq!(primary, "Module Integration");
        assert_eq!(confidence, 0.70);
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].contains("0.60"));
    }

    #[test]
    fn test_classify_leaf_node_pure() {
        let mut io_profile = IoProfile::new();
        io_profile.is_pure = true;

        let (primary, confidence, evidence) =
            PatternDetector::classify_leaf_node(Some(&io_profile));

        assert_eq!(primary, "Pure Computation");
        assert_eq!(confidence, 0.75);
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].contains("no side effects"));
    }

    #[test]
    fn test_classify_leaf_node_impure() {
        let mut io_profile = IoProfile::new();
        io_profile.is_pure = false;

        let (primary, confidence, evidence) =
            PatternDetector::classify_leaf_node(Some(&io_profile));

        assert_eq!(primary, "Utility & Helper Functions");
        assert_eq!(confidence, 0.70);
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].contains("side effects"));
    }

    #[test]
    fn test_classify_leaf_node_no_profile() {
        let (primary, confidence, evidence) = PatternDetector::classify_leaf_node(None);

        assert_eq!(primary, "Utility & Helper Functions");
        assert_eq!(confidence, 0.70);
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].contains("no external calls"));
    }

    #[test]
    fn test_classify_utility_cluster() {
        let mut metrics = create_test_metrics(2, 5);
        metrics.clustering = 0.7;

        let (primary, confidence, evidence) = PatternDetector::classify_utility_cluster(&metrics);

        assert_eq!(primary, "Domain-Specific Utilities");
        assert_eq!(confidence, 0.65);
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].contains("0.70"));
    }

    #[test]
    fn test_classify_fallback_with_io_profile() {
        let io_profile = IoProfile::new();
        let (_primary, confidence, evidence) =
            PatternDetector::classify_fallback(Some(&io_profile));

        assert_eq!(confidence, 0.50);
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].contains("I/O behavior"));
    }

    #[test]
    fn test_classify_fallback_without_profile() {
        let (primary, confidence, evidence) = PatternDetector::classify_fallback(None);

        assert_eq!(primary, "General Logic");
        assert_eq!(confidence, 0.40);
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].contains("No strong call graph pattern"));
    }
}
