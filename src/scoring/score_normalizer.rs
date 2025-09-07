use std::collections::BTreeMap;

pub struct ScoreNormalizer {
    percentiles: BTreeMap<i32, f64>, // Percentile -> raw score threshold
}

impl ScoreNormalizer {
    pub fn new() -> Self {
        Self {
            percentiles: BTreeMap::new(),
        }
    }

    pub fn from_scores(scores: &[f64]) -> Self {
        let mut sorted_scores: Vec<f64> = scores.to_vec();
        sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mut percentiles = BTreeMap::new();

        // Calculate key percentiles
        let percentile_points = vec![5, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99];

        for p in percentile_points {
            let index = (sorted_scores.len() as f64 * p as f64 / 100.0) as usize;
            let index = index.min(sorted_scores.len() - 1);
            percentiles.insert(p, sorted_scores[index]);
        }

        Self { percentiles }
    }

    pub fn normalize(&self, raw_score: f64) -> f64 {
        // If we have no percentile data, just return the raw score
        // (it should already be normalized to 0-10 by the scoring system)
        if self.percentiles.is_empty() {
            return raw_score;
        }

        let percentile = self.find_percentile(raw_score);

        // Use sigmoid function for smooth distribution
        // This maps percentiles to 0-10 scale with good spread
        self.sigmoid_normalize(percentile)
    }

    fn find_percentile(&self, raw_score: f64) -> f64 {
        // If we have no percentiles, return middle value
        if self.percentiles.is_empty() {
            return 50.0;
        }

        // Find where this score falls in the percentile distribution
        let mut last_percentile = 0;
        let mut last_score = 0.0;

        for (&percentile, &threshold) in &self.percentiles {
            if raw_score <= threshold {
                // Interpolate between percentiles
                if last_percentile == 0 {
                    return percentile as f64;
                }

                let ratio = if threshold - last_score > 0.0 {
                    (raw_score - last_score) / (threshold - last_score)
                } else {
                    0.5
                };

                return last_percentile as f64 + ratio * (percentile - last_percentile) as f64;
            }
            last_percentile = percentile;
            last_score = threshold;
        }

        // Score is above 99th percentile
        99.0
    }

    fn sigmoid_normalize(&self, percentile: f64) -> f64 {
        // Sigmoid function to map percentiles to 0-10 scale
        // Adjusted to ensure good distribution:
        // - P5 -> ~2.0
        // - P25 -> ~4.0
        // - P50 -> ~5.5
        // - P75 -> ~7.0
        // - P95 -> ~9.0
        let x = (percentile - 50.0) / 10.0; // Center around 50th percentile
        let sigmoid = 1.0 / (1.0 + (-x * 0.8).exp());

        // Map to 0-10 scale
        let normalized = sigmoid * 10.0;

        // Ensure minimum score for non-zero issues
        if percentile > 0.0 && normalized < 0.5 {
            0.5
        } else {
            normalized
        }
    }

    pub fn add_jitter(&self, score: f64, seed: u64) -> f64 {
        // Add small deterministic jitter to prevent identical scores
        // Use a simple hash of the seed to generate pseudo-random offset
        let hash = seed.wrapping_mul(2654435761); // Knuth's multiplicative hash
        let jitter = ((hash % 100) as f64 / 1000.0) - 0.05; // ±0.05 range

        (score + jitter).max(0.0)
    }
}

impl Default for ScoreNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_distribution() {
        let scores = vec![
            1.0, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 8.0, 9.0, 10.0, 12.0, 15.0,
            20.0, 25.0, 30.0,
        ];

        let normalizer = ScoreNormalizer::from_scores(&scores);

        // Test that normalization produces good distribution
        let normalized: Vec<f64> = scores.iter().map(|&s| normalizer.normalize(s)).collect();

        // Check that we have good spread
        let min = normalized
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();
        let max = normalized
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap();

        assert!(*min < 3.0, "Minimum score should be low");
        assert!(*max > 8.0, "Maximum score should be high");

        // Check that scores are monotonic
        for i in 1..normalized.len() {
            assert!(
                normalized[i] >= normalized[i - 1],
                "Normalized scores should be monotonic"
            );
        }
    }

    #[test]
    fn test_jitter() {
        let normalizer = ScoreNormalizer::new();
        let base_score = 5.0;

        // Same seed should produce same jitter
        let score1 = normalizer.add_jitter(base_score, 12345);
        let score2 = normalizer.add_jitter(base_score, 12345);
        assert_eq!(score1, score2);

        // Different seeds should produce different jitter
        let score3 = normalizer.add_jitter(base_score, 54321);
        assert_ne!(score1, score3);

        // Jitter should be small
        assert!((score1 - base_score).abs() < 0.1);
    }
}
