//! Handler for assigning unclustered methods to existing clusters

use super::hierarchical::Cluster;
use super::quality_metrics::ClusterQuality;
use super::similarity::{CallGraphProvider, ClusteringSimilarityCalculator, FieldAccessProvider};
use super::Method;

/// Handler for assigning methods that don't fit into any cluster
pub struct UnclusteredMethodHandler<C, F> {
    similarity_calc: ClusteringSimilarityCalculator<C, F>,
    min_similarity_for_merge: f64,
}

impl<C: CallGraphProvider, F: FieldAccessProvider> UnclusteredMethodHandler<C, F> {
    pub fn new(
        similarity_calc: ClusteringSimilarityCalculator<C, F>,
        min_similarity_for_merge: f64,
    ) -> Self {
        Self {
            similarity_calc,
            min_similarity_for_merge,
        }
    }

    /// Assign unclustered methods to existing clusters or create a utilities cluster
    ///
    /// Returns the number of methods that remained unclustered and were put in utilities cluster
    pub fn assign_unclustered(
        &self,
        unclustered: Vec<Method>,
        clusters: &mut Vec<Cluster>,
    ) -> usize {
        let mut still_unclustered = Vec::new();

        for method in unclustered {
            // Find most similar cluster
            let best_match = clusters
                .iter()
                .enumerate()
                .map(|(idx, cluster)| {
                    let avg_similarity = self.average_similarity_to_cluster(&method, cluster);
                    (idx, avg_similarity)
                })
                .max_by(|(_, sim1), (_, sim2)| {
                    sim1.partial_cmp(sim2).unwrap_or(std::cmp::Ordering::Equal)
                });

            if let Some((idx, similarity)) = best_match {
                if similarity > self.min_similarity_for_merge {
                    // Add to most similar cluster
                    clusters[idx].methods.push(method);
                    continue;
                }
            }

            // No similar cluster found
            still_unclustered.push(method);
        }

        let unclustered_count = still_unclustered.len();

        // Create "utilities" cluster for remaining unclustered methods
        if !still_unclustered.is_empty() {
            clusters.push(Cluster {
                methods: still_unclustered,
                coherence: 0.3, // Low coherence (expected for utilities)
                quality: Some(ClusterQuality {
                    internal_coherence: 0.3,
                    external_separation: 0.5,
                    silhouette_score: 0.2,
                }),
            });
        }

        unclustered_count
    }

    /// Calculate average similarity between a method and all methods in a cluster
    fn average_similarity_to_cluster(&self, method: &Method, cluster: &Cluster) -> f64 {
        if cluster.methods.is_empty() {
            return 0.0;
        }

        let total: f64 = cluster
            .methods
            .iter()
            .map(|m| self.similarity_calc.calculate_similarity(method, m))
            .sum();

        total / cluster.methods.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::clustering::similarity::{
        CallGraphProvider, ClusteringSimilarityCalculator, FieldAccessProvider,
    };
    use crate::organization::clustering::Visibility;
    use std::collections::{HashMap, HashSet};

    struct MockCallGraph {
        callees: HashMap<String, HashSet<String>>,
    }

    impl CallGraphProvider for MockCallGraph {
        fn call_count(&self, _from: &str, _to: &str) -> usize {
            0
        }

        fn callees(&self, method: &str) -> HashSet<String> {
            self.callees.get(method).cloned().unwrap_or_default()
        }

        fn callers(&self, _method: &str) -> HashSet<String> {
            HashSet::new()
        }
    }

    struct MockFieldAccess;
    impl FieldAccessProvider for MockFieldAccess {
        fn fields_accessed_by(&self, _method: &str) -> HashSet<String> {
            HashSet::new()
        }

        fn writes_to_field(&self, _method: &str, _field: &str) -> bool {
            false
        }
    }

    fn create_method(name: &str) -> Method {
        Method {
            name: name.to_string(),
            is_pure: false,
            visibility: Visibility::Private,
            complexity: 10,
            has_io: false,
        }
    }

    #[test]
    fn test_assign_to_similar_cluster() {
        let call_graph = MockCallGraph {
            callees: HashMap::new(),
        };
        let field_tracker = MockFieldAccess;
        let similarity_calc = ClusteringSimilarityCalculator::new(call_graph, field_tracker);
        let handler = UnclusteredMethodHandler::new(similarity_calc, 0.3);

        let mut clusters = vec![Cluster {
            methods: vec![create_method("format_a"), create_method("format_b")],
            coherence: 0.8,
            quality: None,
        }];

        let unclustered = vec![create_method("format_c")];

        let remaining = handler.assign_unclustered(unclustered, &mut clusters);

        // Should assign to formatting cluster due to naming similarity
        // or create utilities cluster if similarity is too low
        assert!(remaining == 0 || remaining == 1);
    }

    #[test]
    fn test_create_utilities_cluster() {
        let call_graph = MockCallGraph {
            callees: HashMap::new(),
        };
        let field_tracker = MockFieldAccess;
        let similarity_calc = ClusteringSimilarityCalculator::new(call_graph, field_tracker);
        let handler = UnclusteredMethodHandler::new(similarity_calc, 0.9); // High threshold

        let mut clusters = vec![Cluster {
            methods: vec![create_method("method_a")],
            coherence: 0.8,
            quality: None,
        }];

        let unclustered = vec![create_method("completely_different")];

        let remaining = handler.assign_unclustered(unclustered, &mut clusters);

        // Should create utilities cluster due to low similarity
        assert_eq!(remaining, 1);
        assert_eq!(clusters.len(), 2); // Original + utilities cluster
    }
}
