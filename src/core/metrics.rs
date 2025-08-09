use crate::core::{ComplexityMetrics, FunctionMetrics};
use std::path::PathBuf;

pub fn calculate_average_complexity(metrics: &[FunctionMetrics]) -> f64 {
    if metrics.is_empty() {
        return 0.0;
    }

    let total: u32 = metrics.iter().map(|m| m.cyclomatic).sum();
    total as f64 / metrics.len() as f64
}

pub fn find_max_complexity(metrics: &[FunctionMetrics]) -> u32 {
    metrics.iter().map(|m| m.cyclomatic).max().unwrap_or(0)
}

pub fn count_high_complexity(metrics: &[FunctionMetrics], threshold: u32) -> usize {
    metrics.iter().filter(|m| m.is_complex(threshold)).count()
}

pub fn combine_metrics(left: ComplexityMetrics, right: ComplexityMetrics) -> ComplexityMetrics {
    let functions = [left.functions, right.functions].concat();
    let (cyclomatic, cognitive) = functions.iter().fold((0, 0), |(cyc, cog), f| {
        (cyc + f.cyclomatic, cog + f.cognitive)
    });

    ComplexityMetrics {
        functions,
        cyclomatic_complexity: cyclomatic,
        cognitive_complexity: cognitive,
    }
}

pub fn filter_metrics<F>(metrics: ComplexityMetrics, predicate: F) -> ComplexityMetrics
where
    F: Fn(&FunctionMetrics) -> bool,
{
    let functions: Vec<_> = metrics.functions.into_iter().filter(predicate).collect();
    let (cyclomatic, cognitive) = functions.iter().fold((0, 0), |(cyc, cog), f| {
        (cyc + f.cyclomatic, cog + f.cognitive)
    });

    ComplexityMetrics {
        functions,
        cyclomatic_complexity: cyclomatic,
        cognitive_complexity: cognitive,
    }
}

pub fn sort_by_complexity(mut metrics: Vec<FunctionMetrics>) -> Vec<FunctionMetrics> {
    metrics.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic));
    metrics
}

pub fn group_by_file(
    metrics: Vec<FunctionMetrics>,
) -> std::collections::HashMap<PathBuf, Vec<FunctionMetrics>> {
    use std::collections::HashMap;

    metrics.into_iter().fold(HashMap::new(), |mut acc, metric| {
        acc.entry(metric.file.clone()).or_default().push(metric);
        acc
    })
}

pub fn calculate_nesting_penalty(nesting: u32) -> u32 {
    match nesting {
        0..=2 => 0,
        3..=4 => 1,
        5..=6 => 2,
        _ => 3,
    }
}

pub fn calculate_length_penalty(length: usize) -> u32 {
    match length {
        0..=20 => 0,
        21..=50 => 1,
        51..=100 => 2,
        _ => 3,
    }
}
