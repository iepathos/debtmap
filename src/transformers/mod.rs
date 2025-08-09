use crate::core::{ComplexityMetrics, FileMetrics, FunctionMetrics};

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
    FileMetrics {
        complexity: ComplexityMetrics {
            functions: metrics.complexity.functions.into_iter().map(f).collect(),
        },
        ..metrics
    }
}

pub fn filter_functions<F>(metrics: FileMetrics, predicate: F) -> FileMetrics
where
    F: Fn(&FunctionMetrics) -> bool,
{
    FileMetrics {
        complexity: ComplexityMetrics {
            functions: metrics
                .complexity
                .functions
                .into_iter()
                .filter(|f| predicate(f))
                .collect(),
        },
        ..metrics
    }
}

pub fn sort_functions_by_complexity(metrics: FileMetrics) -> FileMetrics {
    let mut functions = metrics.complexity.functions;
    functions.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic));

    FileMetrics {
        complexity: ComplexityMetrics { functions },
        ..metrics
    }
}

pub fn limit_results(metrics: FileMetrics, limit: usize) -> FileMetrics {
    FileMetrics {
        complexity: ComplexityMetrics {
            functions: metrics
                .complexity
                .functions
                .into_iter()
                .take(limit)
                .collect(),
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
            },
            debt_items: Vec::new(),
            dependencies: Vec::new(),
            duplications: Vec::new(),
        },
        |mut acc, m| {
            acc.complexity.functions.extend(m.complexity.functions);
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
