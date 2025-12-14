//! Production-ready clustering pipeline.
//!
//! Provides the main clustering entry point with:
//! - Test method filtering
//! - Oversized cluster subdivision
//! - Tiny cluster merging
//! - Rust-specific pattern application
//! - Warnings as data (no side effects)

use std::collections::{HashMap, HashSet};

use super::super::categorization::is_test_method;
use super::super::types::{BehaviorCategory, MethodCluster};
use super::cohesion::calculate_standalone_cohesion;
use super::hybrid::apply_hybrid_clustering;
use super::refinement::{
    apply_rust_patterns, merge_duplicate_categories, merge_tiny_clusters,
    subdivide_oversized_clusters,
};

/// Warning generated during clustering process.
#[derive(Debug, Clone, PartialEq)]
pub enum ClusteringWarning {
    /// Some methods could not be assigned to cohesive clusters.
    UnclusteredMethods { count: usize, sample: Vec<String> },
    /// Cluster was force-merged due to size constraints.
    ForceMerged {
        from_category: String,
        into_category: String,
        method_count: usize,
    },
}

/// Result of clustering operation with metadata.
#[derive(Debug)]
pub struct ClusteringResult {
    /// The resulting method clusters.
    pub clusters: Vec<MethodCluster>,
    /// Warnings generated during clustering.
    pub warnings: Vec<ClusteringWarning>,
}

impl ClusteringResult {
    /// Create an empty clustering result.
    pub fn empty() -> Self {
        Self {
            clusters: vec![],
            warnings: vec![],
        }
    }

    /// Check if any warnings were generated.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Apply production-ready clustering with test filtering and size balancing.
///
/// This is a refinement pipeline on top of hybrid clustering that:
/// 1. Filters out test methods (should stay in #[cfg(test)])
/// 2. Subdivides oversized Domain clusters using secondary heuristics
/// 3. Merges tiny clusters (<3 methods) into related ones
/// 4. Applies Rust-specific patterns (I/O vs Pure vs Query)
///
/// # Arguments
///
/// * `methods` - Method names to cluster
/// * `adjacency` - Call graph adjacency matrix
///
/// # Returns
///
/// ClusteringResult with clusters and any warnings generated
pub fn apply_production_ready_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> ClusteringResult {
    let production_methods = filter_test_methods(methods);

    if production_methods.is_empty() {
        return ClusteringResult::empty();
    }

    let clusters = apply_hybrid_clustering(&production_methods, adjacency);
    let clusters = subdivide_oversized_clusters(clusters, adjacency);
    let clusters = merge_tiny_clusters(clusters);
    let clusters = apply_rust_patterns(clusters);
    let clusters = merge_duplicate_categories(clusters);

    ensure_all_methods_with_warnings(clusters, &production_methods, adjacency)
}

/// Filter out test methods from the method list.
fn filter_test_methods(methods: &[String]) -> Vec<String> {
    methods
        .iter()
        .filter(|m| !is_test_method(m))
        .cloned()
        .collect()
}

/// Ensure all methods are clustered, generating warnings for any that weren't.
fn ensure_all_methods_with_warnings(
    mut clusters: Vec<MethodCluster>,
    all_methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> ClusteringResult {
    let missing = find_missing_methods(&clusters, all_methods);
    let warnings = generate_missing_warnings(&missing);

    if !missing.is_empty() {
        clusters = recover_missing_methods(clusters, missing, adjacency);
    }

    ClusteringResult { clusters, warnings }
}

/// Find methods that are not in any cluster.
fn find_missing_methods(clusters: &[MethodCluster], all_methods: &[String]) -> Vec<String> {
    let clustered: HashSet<&String> = clusters.iter().flat_map(|c| &c.methods).collect();

    all_methods
        .iter()
        .filter(|m| !clustered.contains(m))
        .cloned()
        .collect()
}

/// Generate warnings for missing methods.
fn generate_missing_warnings(missing: &[String]) -> Vec<ClusteringWarning> {
    if missing.is_empty() {
        return vec![];
    }

    vec![ClusteringWarning::UnclusteredMethods {
        count: missing.len(),
        sample: missing.iter().take(5).cloned().collect(),
    }]
}

/// Recover missing methods into appropriate clusters.
fn recover_missing_methods(
    mut clusters: Vec<MethodCluster>,
    missing: Vec<String>,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    if let Some(utilities) = find_or_create_utilities_cluster(&mut clusters, &missing, adjacency) {
        if !clusters
            .iter()
            .any(|c| matches!(&c.category, BehaviorCategory::Domain(name) if name == "Utilities"))
        {
            clusters.push(utilities);
        }
    } else {
        merge_into_largest_cluster(&mut clusters, missing);
    }

    clusters
}

/// Find existing utilities cluster or create a new one.
fn find_or_create_utilities_cluster(
    clusters: &mut [MethodCluster],
    missing: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Option<MethodCluster> {
    // Try to find existing Utilities cluster
    if let Some(utilities) = clusters
        .iter_mut()
        .find(|c| matches!(&c.category, BehaviorCategory::Domain(name) if name == "Utilities"))
    {
        utilities.methods.extend(missing.iter().cloned());
        return None;
    }

    // Create new Utilities cluster if enough methods
    if missing.len() >= 3 {
        let (internal_calls, external_calls) = calculate_standalone_cohesion(missing, adjacency);

        let mut cluster = MethodCluster {
            category: BehaviorCategory::Domain("Utilities".to_string()),
            methods: missing.to_vec(),
            fields_accessed: vec![],
            internal_calls,
            external_calls,
            cohesion_score: 0.0,
        };
        cluster.calculate_cohesion();
        return Some(cluster);
    }

    None
}

/// Merge methods into the largest existing cluster.
fn merge_into_largest_cluster(clusters: &mut [MethodCluster], methods: Vec<String>) {
    if let Some(largest) = clusters.iter_mut().max_by_key(|c| c.methods.len()) {
        largest.methods.extend(methods);
    } else if !methods.is_empty() {
        // Edge case: no clusters exist, create utilities
        // This shouldn't happen in practice
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clustering_result_empty() {
        let result = ClusteringResult::empty();
        assert!(result.clusters.is_empty());
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_filter_test_methods() {
        let methods = vec![
            "parse_data".into(),
            "test_parse_data".into(),
            "mock_parser".into(),
            "validate".into(),
        ];

        let filtered = filter_test_methods(&methods);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"parse_data".into()));
        assert!(filtered.contains(&"validate".into()));
    }

    #[test]
    fn test_find_missing_methods_none() {
        let clusters = vec![MethodCluster {
            category: BehaviorCategory::Parsing,
            methods: vec!["a".into(), "b".into()],
            fields_accessed: vec![],
            internal_calls: 0,
            external_calls: 0,
            cohesion_score: 0.0,
        }];
        let all = vec!["a".into(), "b".into()];

        let missing = find_missing_methods(&clusters, &all);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_find_missing_methods_some() {
        let clusters = vec![MethodCluster {
            category: BehaviorCategory::Parsing,
            methods: vec!["a".into()],
            fields_accessed: vec![],
            internal_calls: 0,
            external_calls: 0,
            cohesion_score: 0.0,
        }];
        let all = vec!["a".into(), "b".into(), "c".into()];

        let missing = find_missing_methods(&clusters, &all);
        assert_eq!(missing.len(), 2);
    }

    #[test]
    fn test_generate_warnings_for_missing() {
        let missing = vec!["a".into(), "b".into()];
        let warnings = generate_missing_warnings(&missing);

        assert_eq!(warnings.len(), 1);
        match &warnings[0] {
            ClusteringWarning::UnclusteredMethods { count, sample } => {
                assert_eq!(*count, 2);
                assert_eq!(sample.len(), 2);
            }
            _ => panic!("Expected UnclusteredMethods warning"),
        }
    }

    #[test]
    fn test_generate_warnings_empty() {
        let warnings = generate_missing_warnings(&[]);
        assert!(warnings.is_empty());
    }
}
