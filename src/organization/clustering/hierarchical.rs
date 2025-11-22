//! Hierarchical agglomerative clustering algorithm

use super::quality_metrics::ClusterQuality;
use super::similarity::{CallGraphProvider, ClusteringSimilarityCalculator, FieldAccessProvider};
use super::Method;
use std::collections::HashMap;

/// A cluster of related methods
#[derive(Debug, Clone)]
pub struct Cluster {
    pub methods: Vec<Method>,
    pub coherence: f64,
    pub quality: Option<ClusterQuality>,
}

impl Cluster {
    /// Create a singleton cluster from a single method
    pub fn singleton(method: Method) -> Self {
        Self {
            methods: vec![method],
            coherence: 1.0, // Singleton is perfectly coherent
            quality: None,
        }
    }

    /// Merge this cluster with another
    pub fn merge_with(&mut self, other: Cluster) {
        self.methods.extend(other.methods);
        // Coherence will be recalculated after merge
    }

    /// Split this cluster back into two clusters (for undo)
    pub fn split(self) -> (Cluster, Cluster) {
        let mid = self.methods.len() / 2;
        let (left_methods, right_methods) = self.methods.split_at(mid);

        (
            Cluster {
                methods: left_methods.to_vec(),
                coherence: 0.0, // Will be recalculated
                quality: None,
            },
            Cluster {
                methods: right_methods.to_vec(),
                coherence: 0.0, // Will be recalculated
                quality: None,
            },
        )
    }
}

/// Hierarchical clustering implementation
pub struct HierarchicalClustering<C, F> {
    similarity_calc: ClusteringSimilarityCalculator<C, F>,
    min_similarity_threshold: f64,
    min_coherence: f64,
}

impl<C: CallGraphProvider, F: FieldAccessProvider> HierarchicalClustering<C, F> {
    pub fn new(
        similarity_calc: ClusteringSimilarityCalculator<C, F>,
        min_similarity_threshold: f64,
        min_coherence: f64,
    ) -> Self {
        Self {
            similarity_calc,
            min_similarity_threshold,
            min_coherence,
        }
    }

    /// Cluster methods using hierarchical agglomerative clustering
    pub fn cluster_methods(&self, mut methods: Vec<Method>) -> Vec<Cluster> {
        if methods.is_empty() {
            return vec![];
        }

        // Sort methods by name for deterministic ordering
        methods.sort_by(|a, b| a.name.cmp(&b.name));

        // Start with each method as singleton cluster
        let mut clusters: Vec<Cluster> = methods.into_iter().map(Cluster::singleton).collect();

        // Build similarity matrix (cached for efficiency)
        let similarity_matrix = self.build_similarity_matrix(&clusters);

        // Track failed merge pairs to avoid re-attempting them
        let mut failed_merges: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();

        // Iteratively merge most similar clusters
        loop {
            // Find best merge that hasn't failed before
            let merge_candidate =
                self.find_best_merge_excluding(&clusters, &similarity_matrix, &failed_merges);

            match merge_candidate {
                Some((idx1, idx2, similarity)) if similarity > self.min_similarity_threshold => {
                    // Save original clusters for potential undo
                    let cluster1_original = clusters[idx1].clone();
                    let cluster2_idx = idx2.max(idx1);
                    let cluster2_original = clusters[cluster2_idx].clone();

                    // Merge clusters
                    let cluster2 = clusters.remove(cluster2_idx);
                    let idx1_adjusted = if idx2 < idx1 { idx1 - 1 } else { idx1 };
                    clusters[idx1_adjusted].merge_with(cluster2);

                    // Recompute coherence
                    let merged_coherence = self.calculate_coherence(&clusters[idx1_adjusted]);
                    clusters[idx1_adjusted].coherence = merged_coherence;

                    // Reject if coherence too low
                    if merged_coherence < self.min_coherence {
                        // Undo merge by restoring original clusters
                        clusters.remove(idx1_adjusted);
                        clusters.insert(idx1_adjusted, cluster1_original);
                        clusters.insert(idx2.max(idx1), cluster2_original);

                        // Mark this merge pair as failed to avoid retrying
                        failed_merges.insert((idx1.min(idx2), idx1.max(idx2)));

                        // Continue trying other merges instead of stopping
                        continue;
                    }
                }
                _ => break, // No more valid merges
            }
        }

        // Calculate cluster quality scores
        for i in 0..clusters.len() {
            clusters[i].quality = Some(self.calculate_cluster_quality(&clusters[i], &clusters));
        }

        // Sort by size (largest first) for stable output
        clusters.sort_by_key(|c| std::cmp::Reverse(c.methods.len()));

        clusters
    }

    /// Build pairwise similarity matrix for all clusters
    fn build_similarity_matrix(&self, clusters: &[Cluster]) -> SimilarityMatrix {
        let mut matrix = HashMap::new();

        for i in 0..clusters.len() {
            for j in (i + 1)..clusters.len() {
                let similarity = self.calculate_cluster_similarity(&clusters[i], &clusters[j]);
                matrix.insert((i, j), similarity);
            }
        }

        SimilarityMatrix { matrix }
    }

    /// Calculate similarity between two clusters (average linkage)
    fn calculate_cluster_similarity(&self, cluster1: &Cluster, cluster2: &Cluster) -> f64 {
        let mut total_similarity = 0.0;
        let mut count = 0;

        for m1 in &cluster1.methods {
            for m2 in &cluster2.methods {
                total_similarity += self.similarity_calc.calculate_similarity(m1, m2);
                count += 1;
            }
        }

        if count == 0 {
            0.0
        } else {
            total_similarity / count as f64
        }
    }

    /// Find the best pair of clusters to merge, excluding failed merge attempts
    fn find_best_merge_excluding(
        &self,
        clusters: &[Cluster],
        similarity_matrix: &SimilarityMatrix,
        failed_merges: &std::collections::HashSet<(usize, usize)>,
    ) -> Option<(usize, usize, f64)> {
        let mut best_merge: Option<(usize, usize, f64)> = None;

        for i in 0..clusters.len() {
            for j in (i + 1)..clusters.len() {
                // Skip if this merge pair has failed before
                if failed_merges.contains(&(i, j)) {
                    continue;
                }

                let similarity = similarity_matrix.get(i, j);

                if similarity > best_merge.map(|(_, _, sim)| sim).unwrap_or(0.0) {
                    best_merge = Some((i, j, similarity));
                }
            }
        }

        best_merge
    }

    /// Calculate internal coherence of a cluster
    fn calculate_coherence(&self, cluster: &Cluster) -> f64 {
        if cluster.methods.len() < 2 {
            return 1.0; // Singleton is perfectly coherent
        }

        // Average pairwise similarity within cluster
        let mut total_similarity = 0.0;
        let mut count = 0;

        for i in 0..cluster.methods.len() {
            for j in (i + 1)..cluster.methods.len() {
                total_similarity += self
                    .similarity_calc
                    .calculate_similarity(&cluster.methods[i], &cluster.methods[j]);
                count += 1;
            }
        }

        if count == 0 {
            1.0
        } else {
            total_similarity / count as f64
        }
    }

    /// Calculate quality metrics for a cluster
    fn calculate_cluster_quality(
        &self,
        cluster: &Cluster,
        all_clusters: &[Cluster],
    ) -> ClusterQuality {
        let internal_coherence = cluster.coherence;

        // External separation: average similarity to OTHER clusters
        let mut external_sim = 0.0;
        let mut count = 0;

        for other in all_clusters {
            if std::ptr::eq(cluster, other) {
                continue;
            }

            for m1 in &cluster.methods {
                for m2 in &other.methods {
                    external_sim += self.similarity_calc.calculate_similarity(m1, m2);
                    count += 1;
                }
            }
        }

        let external_separation = if count == 0 {
            1.0
        } else {
            1.0 - (external_sim / count as f64)
        };

        // Silhouette score: (separation - (1-coherence)) normalized
        let silhouette_score = if internal_coherence + external_separation == 0.0 {
            0.0
        } else {
            (external_separation - (1.0 - internal_coherence))
                / (external_separation.max(1.0 - internal_coherence))
        };

        ClusterQuality {
            internal_coherence,
            external_separation,
            silhouette_score,
        }
    }
}

/// Cached similarity matrix for cluster pairs
struct SimilarityMatrix {
    matrix: HashMap<(usize, usize), f64>,
}

impl SimilarityMatrix {
    fn get(&self, i: usize, j: usize) -> f64 {
        let key = if i < j { (i, j) } else { (j, i) };
        *self.matrix.get(&key).unwrap_or(&0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::clustering::similarity::{
        CallGraphProvider, ClusteringSimilarityCalculator, FieldAccessProvider,
    };
    use crate::organization::clustering::Visibility;
    use std::collections::HashSet;

    struct MockCallGraph;
    impl CallGraphProvider for MockCallGraph {
        fn call_count(&self, _from: &str, _to: &str) -> usize {
            0
        }
        fn callees(&self, _method: &str) -> HashSet<String> {
            HashSet::new()
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
    fn test_singleton_cluster() {
        let method = create_method("test");
        let cluster = Cluster::singleton(method);

        assert_eq!(cluster.methods.len(), 1);
        assert_eq!(cluster.coherence, 1.0);
    }

    #[test]
    fn test_deterministic_clustering() {
        let methods = vec![
            create_method("method_c"),
            create_method("method_a"),
            create_method("method_b"),
        ];

        let call_graph = MockCallGraph;
        let field_tracker = MockFieldAccess;
        let similarity_calc = ClusteringSimilarityCalculator::new(call_graph, field_tracker);
        let clusterer = HierarchicalClustering::new(similarity_calc, 0.3, 0.5);

        let clusters1 = clusterer.cluster_methods(methods.clone());
        let clusters2 = clusterer.cluster_methods(methods);

        // Same input should produce same number of clusters
        assert_eq!(clusters1.len(), clusters2.len());
    }
}
