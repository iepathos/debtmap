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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FunctionMetrics;

    fn create_test_function(name: &str, cyclomatic: u32, cognitive: u32) -> FunctionMetrics {
        FunctionMetrics {
            name: name.to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic,
            cognitive,
            nesting: 0,
            length: 10,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            detected_patterns: None,
        }
    }

    #[test]
    fn test_filter_metrics_keeps_matching_functions() {
        let metrics = ComplexityMetrics {
            functions: vec![
                create_test_function("simple", 1, 1),
                create_test_function("complex", 10, 15),
                create_test_function("medium", 5, 7),
            ],
            cyclomatic_complexity: 16,
            cognitive_complexity: 23,
        };

        let filtered = filter_metrics(metrics, |f| f.cyclomatic > 4);

        assert_eq!(filtered.functions.len(), 2);
        assert_eq!(filtered.functions[0].name, "complex");
        assert_eq!(filtered.functions[1].name, "medium");
        assert_eq!(filtered.cyclomatic_complexity, 15);
        assert_eq!(filtered.cognitive_complexity, 22);
    }

    #[test]
    fn test_filter_metrics_empty_result() {
        let metrics = ComplexityMetrics {
            functions: vec![
                create_test_function("simple1", 1, 1),
                create_test_function("simple2", 2, 2),
            ],
            cyclomatic_complexity: 3,
            cognitive_complexity: 3,
        };

        let filtered = filter_metrics(metrics, |f| f.cyclomatic > 10);

        assert_eq!(filtered.functions.len(), 0);
        assert_eq!(filtered.cyclomatic_complexity, 0);
        assert_eq!(filtered.cognitive_complexity, 0);
    }

    #[test]
    fn test_filter_metrics_all_match() {
        let metrics = ComplexityMetrics {
            functions: vec![
                create_test_function("func1", 10, 12),
                create_test_function("func2", 15, 18),
            ],
            cyclomatic_complexity: 25,
            cognitive_complexity: 30,
        };

        let filtered = filter_metrics(metrics.clone(), |_| true);

        assert_eq!(filtered.functions.len(), 2);
        assert_eq!(filtered.cyclomatic_complexity, 25);
        assert_eq!(filtered.cognitive_complexity, 30);
    }

    #[test]
    fn test_filter_metrics_by_cognitive_complexity() {
        let metrics = ComplexityMetrics {
            functions: vec![
                create_test_function("low_cognitive", 5, 3),
                create_test_function("high_cognitive", 3, 20),
                create_test_function("medium_cognitive", 4, 10),
            ],
            cyclomatic_complexity: 12,
            cognitive_complexity: 33,
        };

        let filtered = filter_metrics(metrics, |f| f.cognitive > 9);

        assert_eq!(filtered.functions.len(), 2);
        assert!(filtered
            .functions
            .iter()
            .any(|f| f.name == "high_cognitive"));
        assert!(filtered
            .functions
            .iter()
            .any(|f| f.name == "medium_cognitive"));
    }

    #[test]
    fn test_filter_metrics_complex_predicate() {
        let metrics = ComplexityMetrics {
            functions: vec![
                create_test_function("func1", 5, 10),
                create_test_function("func2", 10, 5),
                create_test_function("func3", 8, 8),
            ],
            cyclomatic_complexity: 23,
            cognitive_complexity: 23,
        };

        let filtered = filter_metrics(metrics, |f| f.cyclomatic > 7 && f.cognitive < 9);

        assert_eq!(filtered.functions.len(), 2);
        assert!(filtered.functions.iter().any(|f| f.name == "func2"));
        assert!(filtered.functions.iter().any(|f| f.name == "func3"));
    }
}
