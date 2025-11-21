//! Cluster quality metrics for evaluating clustering results

/// Quality metrics for a cluster
#[derive(Debug, Clone, Copy)]
pub struct ClusterQuality {
    /// Internal coherence: how similar methods within the cluster are (0-1, higher is better)
    pub internal_coherence: f64,
    /// External separation: how distinct the cluster is from others (0-1, higher is better)
    pub external_separation: f64,
    /// Silhouette score: combined quality metric (-1 to 1, >0.4 is good)
    pub silhouette_score: f64,
}

impl ClusterQuality {
    /// Check if this cluster has good quality
    pub fn is_good_quality(&self) -> bool {
        self.silhouette_score > 0.4
    }

    /// Check if this cluster has acceptable quality
    pub fn is_acceptable(&self) -> bool {
        self.internal_coherence > 0.5 && self.external_separation > 0.3
    }

    /// Get a human-readable quality description
    pub fn quality_description(&self) -> &'static str {
        if self.silhouette_score > 0.6 {
            "excellent"
        } else if self.silhouette_score > 0.4 {
            "good"
        } else if self.silhouette_score > 0.2 {
            "fair"
        } else {
            "poor"
        }
    }
}

/// Calculate silhouette score for a set of clusters
///
/// The silhouette score measures how well-separated and cohesive clusters are:
/// - Score near +1: samples are far from neighboring clusters (good)
/// - Score near 0: samples are close to decision boundary
/// - Score near -1: samples might be assigned to wrong cluster (bad)
pub fn calculate_silhouette_score(
    internal_coherences: &[f64],
    external_separations: &[f64],
) -> f64 {
    if internal_coherences.is_empty() {
        return 0.0;
    }

    let total: f64 = internal_coherences
        .iter()
        .zip(external_separations.iter())
        .map(|(coherence, separation)| {
            // Silhouette formula: (b - a) / max(a, b)
            // where a = avg distance within cluster (1 - coherence)
            // and b = avg distance to nearest cluster (1 - separation)
            let a = 1.0 - coherence;
            let b = *separation;

            if a.max(b) == 0.0 {
                0.0
            } else {
                (b - a) / a.max(b)
            }
        })
        .sum();

    total / internal_coherences.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_quality_is_good() {
        let quality = ClusterQuality {
            internal_coherence: 0.7,
            external_separation: 0.8,
            silhouette_score: 0.5,
        };

        assert!(quality.is_good_quality());
        assert!(quality.is_acceptable());
    }

    #[test]
    fn test_cluster_quality_is_poor() {
        let quality = ClusterQuality {
            internal_coherence: 0.3,
            external_separation: 0.2,
            silhouette_score: 0.1,
        };

        assert!(!quality.is_good_quality());
        assert!(!quality.is_acceptable());
    }

    #[test]
    fn test_quality_description() {
        let excellent = ClusterQuality {
            internal_coherence: 0.8,
            external_separation: 0.9,
            silhouette_score: 0.7,
        };
        assert_eq!(excellent.quality_description(), "excellent");

        let good = ClusterQuality {
            internal_coherence: 0.6,
            external_separation: 0.7,
            silhouette_score: 0.5,
        };
        assert_eq!(good.quality_description(), "good");

        let poor = ClusterQuality {
            internal_coherence: 0.2,
            external_separation: 0.3,
            silhouette_score: 0.1,
        };
        assert_eq!(poor.quality_description(), "poor");
    }

    #[test]
    fn test_calculate_silhouette_score() {
        let coherences = vec![0.8, 0.7, 0.75];
        let separations = vec![0.9, 0.85, 0.88];

        let score = calculate_silhouette_score(&coherences, &separations);
        assert!(
            score > 0.0,
            "Silhouette score should be positive for well-separated clusters"
        );
    }

    #[test]
    fn test_calculate_silhouette_score_empty() {
        let score = calculate_silhouette_score(&[], &[]);
        assert_eq!(score, 0.0);
    }
}
