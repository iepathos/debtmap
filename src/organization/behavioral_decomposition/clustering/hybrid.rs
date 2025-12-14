//! Hybrid clustering: name-based categorization refined by community detection.
//!
//! Combines the best of both strategies:
//! - Initial clustering by behavioral categories (name-based)
//! - Refinement using call-graph community detection for large clusters
//!
//! # Benefits
//!
//! - Works for files with sparse call graphs (utility modules)
//! - Finds natural cohesion boundaries within behavioral categories
//! - More accurate than either approach alone

use std::collections::{HashMap, HashSet};

use super::super::categorization::cluster_methods_by_behavior;
use super::super::types::{BehaviorCategory, MethodCluster};
use super::cohesion::calculate_standalone_cohesion;
use super::community::apply_community_detection;

/// Apply hybrid clustering: name-based categorization refined by community detection.
///
/// # Arguments
///
/// * `methods` - Method names to cluster
/// * `adjacency` - Call graph adjacency matrix
///
/// # Returns
///
/// Vector of refined method clusters
pub fn apply_hybrid_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    let name_clusters = cluster_methods_by_behavior(methods);

    if name_clusters.is_empty() {
        return apply_community_detection(methods, adjacency);
    }

    name_clusters
        .into_iter()
        .flat_map(|(category, cluster_methods)| {
            refine_category_cluster(category, cluster_methods, adjacency)
        })
        .collect()
}

/// Refine a behavioral category cluster.
fn refine_category_cluster(
    category: BehaviorCategory,
    methods: Vec<String>,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    if methods.len() <= 5 {
        return vec![create_small_cluster(category, methods, adjacency)];
    }

    refine_large_cluster(category, methods, adjacency)
}

/// Create a cluster for small method groups (<=5 methods).
fn create_small_cluster(
    category: BehaviorCategory,
    methods: Vec<String>,
    adjacency: &HashMap<(String, String), usize>,
) -> MethodCluster {
    let (internal_calls, external_calls) = calculate_standalone_cohesion(&methods, adjacency);

    let mut cluster = MethodCluster {
        category,
        methods,
        fields_accessed: vec![],
        internal_calls,
        external_calls,
        cohesion_score: 0.0,
    };

    cluster.calculate_cohesion();
    cluster
}

/// Refine a large cluster using community detection.
fn refine_large_cluster(
    category: BehaviorCategory,
    methods: Vec<String>,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    let sub_clusters = apply_community_detection(&methods, adjacency);

    if sub_clusters.is_empty() {
        return vec![create_small_cluster(category, methods, adjacency)];
    }

    let refined = refine_subclusters(&sub_clusters, &category);
    let lost_methods = find_lost_methods(&sub_clusters, &methods);

    recover_lost_methods(refined, lost_methods, category, adjacency)
}

/// Refine subclusters by preserving behavioral category where appropriate.
fn refine_subclusters(
    sub_clusters: &[MethodCluster],
    original_category: &BehaviorCategory,
) -> Vec<MethodCluster> {
    sub_clusters
        .iter()
        .map(|subcluster| refine_single_subcluster(subcluster, original_category))
        .collect()
}

/// Refine a single subcluster's category if needed.
fn refine_single_subcluster(
    subcluster: &MethodCluster,
    original_category: &BehaviorCategory,
) -> MethodCluster {
    let mut refined = subcluster.clone();

    // If the subcluster's inferred category is generic (Domain),
    // prefer the original behavioral category
    if matches!(refined.category, BehaviorCategory::Domain(_)) {
        refined.category = original_category.clone();
    }

    refined
}

/// Find methods that were lost during community detection.
fn find_lost_methods(sub_clusters: &[MethodCluster], all_methods: &[String]) -> Vec<String> {
    let clustered: HashSet<&String> = sub_clusters.iter().flat_map(|c| &c.methods).collect();

    all_methods
        .iter()
        .filter(|m| !clustered.contains(m))
        .cloned()
        .collect()
}

/// Recover lost methods into a recovery cluster.
fn recover_lost_methods(
    mut refined: Vec<MethodCluster>,
    lost_methods: Vec<String>,
    category: BehaviorCategory,
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    if lost_methods.is_empty() {
        return refined;
    }

    let recovery_cluster = create_small_cluster(category, lost_methods, adjacency);
    refined.push(recovery_cluster);
    refined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_cluster_no_refinement() {
        let methods = vec!["parse_a".into(), "parse_b".into()];
        let clusters = apply_hybrid_clustering(&methods, &HashMap::new());

        // Small clusters (<=5) should not be split further
        assert!(!clusters.is_empty());
    }

    #[test]
    fn test_find_lost_methods_none_lost() {
        let sub_clusters = vec![MethodCluster {
            category: BehaviorCategory::Parsing,
            methods: vec!["a".into(), "b".into()],
            fields_accessed: vec![],
            internal_calls: 0,
            external_calls: 0,
            cohesion_score: 0.0,
        }];
        let all_methods = vec!["a".into(), "b".into()];

        let lost = find_lost_methods(&sub_clusters, &all_methods);
        assert!(lost.is_empty());
    }

    #[test]
    fn test_find_lost_methods_some_lost() {
        let sub_clusters = vec![MethodCluster {
            category: BehaviorCategory::Parsing,
            methods: vec!["a".into()],
            fields_accessed: vec![],
            internal_calls: 0,
            external_calls: 0,
            cohesion_score: 0.0,
        }];
        let all_methods = vec!["a".into(), "b".into(), "c".into()];

        let lost = find_lost_methods(&sub_clusters, &all_methods);
        assert_eq!(lost.len(), 2);
        assert!(lost.contains(&"b".into()));
        assert!(lost.contains(&"c".into()));
    }
}
