//! Community detection for method clustering.
//!
//! Implements a simplified Louvain-style algorithm to identify communities
//! of methods with high internal cohesion.
//!
//! # Pure Function Properties
//!
//! All functions in this module are pure:
//! - Deterministic output for same input
//! - No side effects (no I/O, no logging)
//! - Thread-safe

use std::collections::HashMap;

use super::super::categorization::infer_cluster_category;
use super::super::types::MethodCluster;
use super::cohesion::calculate_cluster_cohesion;

const MAX_METHODS_FOR_CLUSTERING: usize = 200;
const MAX_ITERATIONS: usize = 10;
const MIN_CLUSTER_SIZE: usize = 3;
const MIN_COHESION_SCORE: f64 = 0.2;

/// Apply community detection algorithm to cluster methods.
///
/// Uses a simplified Louvain-style algorithm to identify communities
/// of methods with high internal cohesion.
///
/// # Algorithm
///
/// 1. Start with each method in its own cluster
/// 2. For each method, try moving it to neighbor clusters
/// 3. Accept moves that increase modularity (cohesion)
/// 4. Repeat until no more improvements
///
/// # Arguments
///
/// * `methods` - Method names to cluster
/// * `adjacency` - Call graph adjacency matrix
///
/// # Returns
///
/// Vector of method clusters with cohesion scores
pub fn apply_community_detection(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    if should_skip_clustering(methods, adjacency) {
        return Vec::new();
    }

    let (clusters, method_to_cluster) = initialize_clusters(methods);
    let refined = iteratively_improve_clusters(clusters, method_to_cluster, adjacency, methods);

    convert_to_method_clusters(refined, adjacency)
}

/// Check if clustering should be skipped.
fn should_skip_clustering(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> bool {
    adjacency.is_empty() || methods.len() > MAX_METHODS_FOR_CLUSTERING
}

/// Initialize each method in its own cluster.
fn initialize_clusters(
    methods: &[String],
) -> (HashMap<usize, Vec<String>>, HashMap<String, usize>) {
    let clusters = methods
        .iter()
        .enumerate()
        .map(|(i, m)| (i, vec![m.clone()]))
        .collect();

    let method_to_cluster = methods
        .iter()
        .enumerate()
        .map(|(i, m)| (m.clone(), i))
        .collect();

    (clusters, method_to_cluster)
}

/// Iteratively improve clustering by moving methods to better clusters.
fn iteratively_improve_clusters(
    mut clusters: HashMap<usize, Vec<String>>,
    mut method_to_cluster: HashMap<String, usize>,
    adjacency: &HashMap<(String, String), usize>,
    methods: &[String],
) -> (HashMap<usize, Vec<String>>, HashMap<String, usize>) {
    let mut improved = true;
    let mut iterations = 0;

    while improved && iterations < MAX_ITERATIONS {
        improved = false;
        iterations += 1;

        for method in methods {
            if let Some(improvement) = try_improve_method_placement(
                method,
                &clusters,
                &method_to_cluster,
                adjacency,
                methods,
            ) {
                apply_method_move(method, improvement, &mut clusters, &mut method_to_cluster);
                improved = true;
            }
        }

        clusters.retain(|_, methods| !methods.is_empty());
    }

    (clusters, method_to_cluster)
}

/// Try to find a better cluster for a method.
fn try_improve_method_placement(
    method: &str,
    clusters: &HashMap<usize, Vec<String>>,
    method_to_cluster: &HashMap<String, usize>,
    adjacency: &HashMap<(String, String), usize>,
    all_methods: &[String],
) -> Option<usize> {
    let current_cluster = *method_to_cluster.get(method)?;
    let current_modularity =
        calculate_method_modularity(method, &clusters[&current_cluster], adjacency, all_methods);

    let (best_cluster, _) = find_best_cluster(
        method,
        current_cluster,
        current_modularity,
        clusters,
        adjacency,
        all_methods,
    );

    if best_cluster != current_cluster {
        Some(best_cluster)
    } else {
        None
    }
}

/// Find the best cluster for a method based on modularity.
fn find_best_cluster(
    method: &str,
    current_cluster: usize,
    current_modularity: f64,
    clusters: &HashMap<usize, Vec<String>>,
    adjacency: &HashMap<(String, String), usize>,
    all_methods: &[String],
) -> (usize, f64) {
    let mut best_cluster = current_cluster;
    let mut best_modularity = current_modularity;

    for (cluster_id, cluster_methods) in clusters {
        if *cluster_id == current_cluster {
            continue;
        }

        let modularity =
            calculate_method_modularity(method, cluster_methods, adjacency, all_methods);

        if modularity > best_modularity {
            best_modularity = modularity;
            best_cluster = *cluster_id;
        }
    }

    (best_cluster, best_modularity)
}

/// Apply a method move from one cluster to another.
fn apply_method_move(
    method: &str,
    target_cluster: usize,
    clusters: &mut HashMap<usize, Vec<String>>,
    method_to_cluster: &mut HashMap<String, usize>,
) {
    let current_cluster = method_to_cluster[method];

    // Remove from current cluster
    if let Some(cluster) = clusters.get_mut(&current_cluster) {
        cluster.retain(|m| m != method);
    }

    // Add to target cluster
    clusters
        .entry(target_cluster)
        .or_default()
        .push(method.to_string());
    method_to_cluster.insert(method.to_string(), target_cluster);
}

/// Convert raw clusters to MethodCluster structs.
fn convert_to_method_clusters(
    (clusters, method_to_cluster): (HashMap<usize, Vec<String>>, HashMap<String, usize>),
    adjacency: &HashMap<(String, String), usize>,
) -> Vec<MethodCluster> {
    clusters
        .into_values()
        .filter(|methods| methods.len() >= MIN_CLUSTER_SIZE)
        .filter_map(|methods| build_method_cluster(methods, &method_to_cluster, adjacency))
        .filter(|cluster| cluster.cohesion_score > MIN_COHESION_SCORE)
        .collect()
}

/// Build a single MethodCluster from a list of methods.
fn build_method_cluster(
    methods: Vec<String>,
    method_to_cluster: &HashMap<String, usize>,
    adjacency: &HashMap<(String, String), usize>,
) -> Option<MethodCluster> {
    let (internal_calls, external_calls) =
        calculate_cluster_cohesion(&methods, adjacency, method_to_cluster);

    let mut cluster = MethodCluster {
        category: infer_cluster_category(&methods),
        methods,
        fields_accessed: vec![],
        internal_calls,
        external_calls,
        cohesion_score: 0.0,
    };

    cluster.calculate_cohesion();
    Some(cluster)
}

/// Calculate modularity score for a method in a cluster.
///
/// Modularity measures how well a method fits in a cluster based on
/// the ratio of internal to total connections.
pub fn calculate_method_modularity(
    method: &str,
    cluster: &[String],
    adjacency: &HashMap<(String, String), usize>,
    all_methods: &[String],
) -> f64 {
    if cluster.is_empty() {
        return 0.0;
    }

    let internal = count_internal_connections(method, cluster, adjacency);
    let external = count_external_connections(method, cluster, adjacency, all_methods);

    let total = internal + external;
    if total == 0 {
        0.0
    } else {
        internal as f64 / total as f64
    }
}

/// Count connections from method to other methods in the same cluster.
fn count_internal_connections(
    method: &str,
    cluster: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> usize {
    cluster
        .iter()
        .filter(|m| *m != method)
        .map(|cluster_method| get_bidirectional_count(method, cluster_method, adjacency))
        .sum()
}

/// Count connections from method to methods outside the cluster.
fn count_external_connections(
    method: &str,
    cluster: &[String],
    adjacency: &HashMap<(String, String), usize>,
    all_methods: &[String],
) -> usize {
    all_methods
        .iter()
        .filter(|m| !cluster.contains(m) && *m != method)
        .map(|other_method| get_bidirectional_count(method, other_method, adjacency))
        .sum()
}

/// Get bidirectional call count between two methods.
fn get_bidirectional_count(
    method_a: &str,
    method_b: &str,
    adjacency: &HashMap<(String, String), usize>,
) -> usize {
    let forward = adjacency
        .get(&(method_a.to_string(), method_b.to_string()))
        .unwrap_or(&0);
    let backward = adjacency
        .get(&(method_b.to_string(), method_a.to_string()))
        .unwrap_or(&0);
    forward + backward
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_empty_adjacency() {
        assert!(should_skip_clustering(&["a".into()], &HashMap::new()));
    }

    #[test]
    fn test_should_skip_too_many_methods() {
        let methods: Vec<String> = (0..250).map(|i| format!("m{}", i)).collect();
        let adjacency = HashMap::from([((String::from("m0"), String::from("m1")), 1)]);
        assert!(should_skip_clustering(&methods, &adjacency));
    }

    #[test]
    fn test_initialize_clusters_creates_one_per_method() {
        let methods = vec!["a".into(), "b".into(), "c".into()];
        let (clusters, method_map) = initialize_clusters(&methods);

        assert_eq!(clusters.len(), 3);
        assert_eq!(method_map.len(), 3);
    }

    #[test]
    fn test_modularity_with_empty_cluster() {
        let modularity = calculate_method_modularity("a", &[], &HashMap::new(), &[]);
        assert_eq!(modularity, 0.0);
    }

    #[test]
    fn test_bidirectional_count() {
        let adjacency =
            HashMap::from([(("a".into(), "b".into()), 2), (("b".into(), "a".into()), 3)]);
        assert_eq!(get_bidirectional_count("a", "b", &adjacency), 5);
    }
}
