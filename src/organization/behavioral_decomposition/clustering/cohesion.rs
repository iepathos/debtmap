//! Cohesion calculation utilities for clustering.
//!
//! Provides pure functions for calculating cluster cohesion metrics.

use std::collections::HashMap;

/// Calculate cohesion metrics for a cluster.
///
/// Returns (internal_calls, external_calls) tuple.
///
/// # Arguments
///
/// * `cluster` - Methods in the cluster
/// * `adjacency` - Call graph adjacency matrix
/// * `method_to_cluster` - Mapping from method name to cluster ID
pub fn calculate_cluster_cohesion(
    cluster: &[String],
    adjacency: &HashMap<(String, String), usize>,
    method_to_cluster: &HashMap<String, usize>,
) -> (usize, usize) {
    let cluster_id = cluster
        .first()
        .and_then(|m| method_to_cluster.get(m).copied());

    let mut internal = 0;
    let mut external = 0;

    for method in cluster {
        let (method_internal, method_external) =
            count_method_calls(method, adjacency, method_to_cluster, cluster_id);
        internal += method_internal;
        external += method_external;
    }

    (internal, external)
}

/// Count internal and external calls for a single method.
fn count_method_calls(
    method: &str,
    adjacency: &HashMap<(String, String), usize>,
    method_to_cluster: &HashMap<String, usize>,
    cluster_id: Option<usize>,
) -> (usize, usize) {
    let mut internal = 0;
    let mut external = 0;

    for ((from, to), &count) in adjacency {
        if from != method {
            continue;
        }

        let to_cluster = method_to_cluster.get(to);
        if to_cluster == cluster_id.as_ref() {
            internal += count;
        } else {
            external += count;
        }
    }

    (internal, external)
}

/// Calculate cohesion for a standalone cluster without cluster mapping.
///
/// Creates a temporary method-to-cluster mapping and calculates cohesion.
pub fn calculate_standalone_cohesion(
    methods: &[String],
    adjacency: &HashMap<(String, String), usize>,
) -> (usize, usize) {
    let method_to_cluster: HashMap<String, usize> =
        methods.iter().map(|m| (m.clone(), 0)).collect();

    calculate_cluster_cohesion(methods, adjacency, &method_to_cluster)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_cluster_cohesion() {
        let (internal, external) =
            calculate_cluster_cohesion(&[], &HashMap::new(), &HashMap::new());
        assert_eq!(internal, 0);
        assert_eq!(external, 0);
    }

    #[test]
    fn test_cluster_with_internal_calls() {
        let methods = vec!["a".into(), "b".into()];
        let adjacency = HashMap::from([(("a".into(), "b".into()), 3)]);
        let method_to_cluster: HashMap<String, usize> =
            vec![("a".into(), 0), ("b".into(), 0)].into_iter().collect();

        let (internal, external) =
            calculate_cluster_cohesion(&methods, &adjacency, &method_to_cluster);
        assert_eq!(internal, 3);
        assert_eq!(external, 0);
    }

    #[test]
    fn test_cluster_with_external_calls() {
        let methods = vec!["a".into()];
        let adjacency = HashMap::from([(("a".into(), "external".into()), 2)]);
        let method_to_cluster: HashMap<String, usize> =
            vec![("a".into(), 0), ("external".into(), 1)]
                .into_iter()
                .collect();

        let (internal, external) =
            calculate_cluster_cohesion(&methods, &adjacency, &method_to_cluster);
        assert_eq!(internal, 0);
        assert_eq!(external, 2);
    }
}
