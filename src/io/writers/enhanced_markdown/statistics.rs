use crate::core::{AnalysisResults, FunctionMetrics};

pub struct DistributionStats {
    pub mean: f64,
    pub median: u32,
    pub std_dev: f64,
    pub min: u32,
    pub max: u32,
    pub quartiles: (u32, u32, u32),
}

pub struct CouplingMetrics {
    pub afferent: usize,
    pub efferent: usize,
    pub instability: f64,
}

/// Calculate standard deviation from a set of values
pub fn calculate_std_dev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

    variance.sqrt()
}

/// Calculate average complexity across all functions
pub fn calculate_average_complexity(results: &AnalysisResults) -> f64 {
    if results.complexity.metrics.is_empty() {
        return 0.0;
    }

    let total: u32 = results
        .complexity
        .metrics
        .iter()
        .map(|m| m.cyclomatic)
        .sum();
    total as f64 / results.complexity.metrics.len() as f64
}

/// Calculate complexity distribution percentages
pub fn calculate_complexity_distribution(results: &AnalysisResults) -> Vec<(&'static str, f64)> {
    let total = results.complexity.metrics.len() as f64;
    if total == 0.0 {
        return vec![
            ("Low (0-5)", 0.0),
            ("Medium (6-10)", 0.0),
            ("High (11-20)", 0.0),
            ("Critical (20+)", 0.0),
        ];
    }

    let low = results
        .complexity
        .metrics
        .iter()
        .filter(|m| m.cyclomatic <= 5)
        .count() as f64;
    let medium = results
        .complexity
        .metrics
        .iter()
        .filter(|m| m.cyclomatic > 5 && m.cyclomatic <= 10)
        .count() as f64;
    let high = results
        .complexity
        .metrics
        .iter()
        .filter(|m| m.cyclomatic > 10 && m.cyclomatic <= 20)
        .count() as f64;
    let critical = results
        .complexity
        .metrics
        .iter()
        .filter(|m| m.cyclomatic > 20)
        .count() as f64;

    vec![
        ("Low (0-5)", (low / total) * 100.0),
        ("Medium (6-10)", (medium / total) * 100.0),
        ("High (11-20)", (high / total) * 100.0),
        ("Critical (20+)", (critical / total) * 100.0),
    ]
}

/// Calculate percentiles for complexity metrics
pub fn calculate_percentiles(metrics: &[FunctionMetrics]) -> Vec<(u32, u32, u32)> {
    if metrics.is_empty() {
        return vec![(0, 0, 0)];
    }

    let mut complexities: Vec<u32> = metrics.iter().map(|m| m.cyclomatic).collect();
    complexities.sort_unstable();

    let p25 = complexities[complexities.len() / 4];
    let p50 = complexities[complexities.len() / 2];
    let p75 = complexities[complexities.len() * 3 / 4];

    vec![(p25, p50, p75)]
}

/// Calculate comprehensive distribution statistics
pub fn calculate_distribution_stats(metrics: &[FunctionMetrics]) -> DistributionStats {
    if metrics.is_empty() {
        return DistributionStats {
            mean: 0.0,
            median: 0,
            std_dev: 0.0,
            min: 0,
            max: 0,
            quartiles: (0, 0, 0),
        };
    }

    let mut complexities: Vec<u32> = metrics.iter().map(|m| m.cyclomatic).collect();
    complexities.sort_unstable();

    let mean = complexities.iter().sum::<u32>() as f64 / complexities.len() as f64;
    let median = complexities[complexities.len() / 2];
    let values: Vec<f64> = complexities.iter().map(|&c| c as f64).collect();
    let std_dev = calculate_std_dev(&values);

    let quartiles = (
        complexities[complexities.len() / 4],
        median,
        complexities[complexities.len() * 3 / 4],
    );

    DistributionStats {
        mean,
        median,
        std_dev,
        min: *complexities.first().unwrap(),
        max: *complexities.last().unwrap(),
        quartiles,
    }
}

/// Calculate coupling metrics for dependencies
pub fn calculate_coupling_metrics(afferent: usize, efferent: usize) -> CouplingMetrics {
    let instability = if afferent + efferent == 0 {
        0.0
    } else {
        efferent as f64 / (afferent + efferent) as f64
    };

    CouplingMetrics {
        afferent,
        efferent,
        instability,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_std_dev() {
        let values = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let std_dev = calculate_std_dev(&values);
        assert!((std_dev - 2.83).abs() < 0.01);
    }
}
