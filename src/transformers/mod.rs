use crate::core::{ComplexityMetrics, FileMetrics, FunctionMetrics};

fn calculate_total_complexity(functions: &[FunctionMetrics]) -> (u32, u32) {
    functions.iter().fold((0, 0), |(cyc, cog), f| {
        (cyc + f.cyclomatic, cog + f.cognitive)
    })
}

pub mod filters;

pub type Transformer<T> = Box<dyn Fn(T) -> T>;

pub fn compose_transformers<T: 'static>(transformers: Vec<Transformer<T>>) -> Transformer<T> {
    Box::new(move |input| transformers.iter().fold(input, |acc, f| f(acc)))
}

pub fn transform_metrics<F>(metrics: FileMetrics, f: F) -> FileMetrics
where
    F: Fn(FileMetrics) -> FileMetrics,
{
    f(metrics)
}

pub fn map_functions<F>(metrics: FileMetrics, f: F) -> FileMetrics
where
    F: Fn(FunctionMetrics) -> FunctionMetrics,
{
    let functions: Vec<_> = metrics.complexity.functions.into_iter().map(f).collect();
    let (cyclomatic, cognitive) = calculate_total_complexity(&functions);

    FileMetrics {
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
        },
        ..metrics
    }
}

pub fn filter_functions<F>(metrics: FileMetrics, predicate: F) -> FileMetrics
where
    F: Fn(&FunctionMetrics) -> bool,
{
    let functions: Vec<_> = metrics
        .complexity
        .functions
        .into_iter()
        .filter(|f| predicate(f))
        .collect();
    let (cyclomatic, cognitive) = calculate_total_complexity(&functions);

    FileMetrics {
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
        },
        ..metrics
    }
}

pub fn sort_functions_by_complexity(metrics: FileMetrics) -> FileMetrics {
    let mut functions = metrics.complexity.functions;
    functions.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic));
    let (cyclomatic, cognitive) = calculate_total_complexity(&functions);

    FileMetrics {
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
        },
        ..metrics
    }
}

pub fn limit_results(metrics: FileMetrics, limit: usize) -> FileMetrics {
    let functions: Vec<_> = metrics
        .complexity
        .functions
        .into_iter()
        .take(limit)
        .collect();
    let (cyclomatic, cognitive) = calculate_total_complexity(&functions);

    FileMetrics {
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
        },
        debt_items: metrics.debt_items.into_iter().take(limit).collect(),
        ..metrics
    }
}

pub fn combine_file_metrics(metrics: Vec<FileMetrics>) -> FileMetrics {
    metrics.into_iter().fold(
        FileMetrics {
            path: std::path::PathBuf::new(),
            language: crate::core::Language::Unknown,
            complexity: ComplexityMetrics {
                functions: Vec::new(),
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
            },
            debt_items: Vec::new(),
            dependencies: Vec::new(),
            duplications: Vec::new(),
        },
        |mut acc, m| {
            acc.complexity
                .functions
                .extend(m.complexity.functions.clone());
            acc.complexity.cyclomatic_complexity += m.complexity.cyclomatic_complexity;
            acc.complexity.cognitive_complexity += m.complexity.cognitive_complexity;
            acc.debt_items.extend(m.debt_items);
            acc.dependencies.extend(m.dependencies);
            acc.duplications.extend(m.duplications);
            acc
        },
    )
}

pub fn enrich_with_context(metrics: FileMetrics) -> FileMetrics {
    FileMetrics {
        debt_items: metrics
            .debt_items
            .into_iter()
            .map(|mut item| {
                if item.context.is_none() {
                    item.context = Some(format!("Found in {}", item.file.display()));
                }
                item
            })
            .collect(),
        ..metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
        }
    }

    #[test]
    fn test_calculate_total_complexity_empty() {
        let functions = vec![];
        let (cyc, cog) = calculate_total_complexity(&functions);
        assert_eq!(cyc, 0);
        assert_eq!(cog, 0);
    }

    #[test]
    fn test_calculate_total_complexity_single_function() {
        let functions = vec![create_test_function("test_func", 5, 3)];
        let (cyc, cog) = calculate_total_complexity(&functions);
        assert_eq!(cyc, 5);
        assert_eq!(cog, 3);
    }

    #[test]
    fn test_calculate_total_complexity_multiple_functions() {
        let functions = vec![
            create_test_function("func1", 5, 3),
            create_test_function("func2", 10, 7),
            create_test_function("func3", 2, 1),
        ];
        let (cyc, cog) = calculate_total_complexity(&functions);
        assert_eq!(cyc, 17);
        assert_eq!(cog, 11);
    }

    #[test]
    fn test_calculate_total_complexity_zero_values() {
        let functions = vec![
            create_test_function("func1", 0, 0),
            create_test_function("func2", 0, 0),
        ];
        let (cyc, cog) = calculate_total_complexity(&functions);
        assert_eq!(cyc, 0);
        assert_eq!(cog, 0);
    }

    #[test]
    fn test_calculate_total_complexity_large_values() {
        let functions = vec![
            create_test_function("complex_func1", 100, 80),
            create_test_function("complex_func2", 200, 150),
            create_test_function("complex_func3", 50, 40),
        ];
        let (cyc, cog) = calculate_total_complexity(&functions);
        assert_eq!(cyc, 350);
        assert_eq!(cog, 270);
    }

    #[test]
    fn test_calculate_total_complexity_mixed_values() {
        let functions = vec![
            create_test_function("simple", 1, 1),
            create_test_function("medium", 10, 8),
            create_test_function("complex", 25, 20),
            create_test_function("zero", 0, 0),
        ];
        let (cyc, cog) = calculate_total_complexity(&functions);
        assert_eq!(cyc, 36);
        assert_eq!(cog, 29);
    }
}
