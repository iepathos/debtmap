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

    /// Classify responsibility based on detected patterns and I/O profile
    pub fn classify_responsibility(
        &self,
        patterns: &[CallGraphPattern],
        metrics: &GraphMetrics,
        io_profile: Option<&IoProfile>,
    ) -> ResponsibilityClassification {
        let mut evidence = Vec::new();
        let primary: String;
        let confidence: f64;

        // Priority order: Most specific patterns first
        if patterns.contains(&CallGraphPattern::Orchestrator) {
            primary = "Orchestration & Coordination".to_string();
            confidence = 0.85;
            evidence.push(format!(
                "Calls {} functions, orchestrating complex workflow",
                metrics.outdegree
            ));
        } else if patterns.contains(&CallGraphPattern::IoGateway) {
            primary = "I/O & External Communication".to_string();
            confidence = 0.80;
            evidence.push("Acts as gateway to I/O operations".to_string());

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
        } else if patterns.contains(&CallGraphPattern::Hub) {
            primary = "Core Business Logic".to_string();
            confidence = 0.75;
            evidence.push(format!(
                "Called by {} functions, central to module",
                metrics.indegree
            ));
        } else if patterns.contains(&CallGraphPattern::Bridge) {
            primary = "Module Integration".to_string();
            confidence = 0.70;
            evidence.push(format!(
                "High betweenness centrality ({:.2}), connects modules",
                metrics.betweenness
            ));
        } else if patterns.contains(&CallGraphPattern::LeafNode) {
            // Check I/O profile to distinguish between pure computation and utilities
            if let Some(profile) = io_profile {
                if profile.is_pure {
                    primary = "Pure Computation".to_string();
                    confidence = 0.75;
                    evidence.push("Pure function with no side effects or I/O".to_string());
                } else {
                    primary = "Utility & Helper Functions".to_string();
                    confidence = 0.70;
                    evidence.push("Leaf function with side effects".to_string());
                }
            } else {
                primary = "Utility & Helper Functions".to_string();
                confidence = 0.70;
                evidence.push("Pure function with no external calls".to_string());
            }
        } else if patterns.contains(&CallGraphPattern::UtilityCluster) {
            primary = "Domain-Specific Utilities".to_string();
            confidence = 0.65;
            evidence.push(format!(
                "Part of tightly-connected functional group (clustering: {:.2})",
                metrics.clustering
            ));
        } else {
            // Default fallback based on I/O profile
            if let Some(profile) = io_profile {
                let responsibility = profile.primary_responsibility();
                primary = responsibility.as_str().to_string();
                confidence = 0.50;
                evidence.push("Classified based on I/O behavior".to_string());
            } else {
                primary = "General Logic".to_string();
                confidence = 0.40;
                evidence.push("No strong call graph pattern detected".to_string());
            }
        }

        ResponsibilityClassification {
            primary,
            confidence,
            evidence,
            patterns: patterns.to_vec(),
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
}
