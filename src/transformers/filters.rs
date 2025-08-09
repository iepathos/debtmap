use crate::core::{DebtType, FileMetrics, FunctionMetrics, Language, Priority};

fn calculate_total_complexity(functions: &[FunctionMetrics]) -> (u32, u32) {
    functions.iter().fold((0, 0), |(cyc, cog), f| {
        (cyc + f.cyclomatic, cog + f.cognitive)
    })
}

#[derive(Default)]
pub struct FilterConfig {
    pub min_complexity: Option<u32>,
    pub max_complexity: Option<u32>,
    pub languages: Option<Vec<Language>>,
    pub file_patterns: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
    pub min_priority: Option<Priority>,
    pub debt_types: Option<Vec<DebtType>>,
}

impl FilterConfig {
    pub fn apply(&self, metrics: FileMetrics) -> FileMetrics {
        let mut result = metrics;

        if let Some(min) = self.min_complexity {
            result = filter_by_min_complexity(result, min);
        }

        if let Some(max) = self.max_complexity {
            result = filter_by_max_complexity(result, max);
        }

        if let Some(ref langs) = self.languages {
            result = filter_by_language(result, langs.clone());
        }

        if let Some(ref patterns) = self.file_patterns {
            result = filter_by_file_pattern(result, patterns.clone());
        }

        if let Some(ref patterns) = self.exclude_patterns {
            result = exclude_by_pattern(result, patterns.clone());
        }

        if let Some(min_prio) = self.min_priority {
            result = filter_by_min_priority(result, min_prio);
        }

        if let Some(ref types) = self.debt_types {
            result = filter_by_debt_types(result, types.clone());
        }

        result
    }
}

pub fn filter_by_min_complexity(metrics: FileMetrics, threshold: u32) -> FileMetrics {
    let filtered_functions: Vec<_> = metrics
        .complexity
        .functions
        .into_iter()
        .filter(|f| f.cyclomatic >= threshold || f.cognitive >= threshold)
        .collect();

    let (cyclomatic, cognitive) = calculate_total_complexity(&filtered_functions);

    FileMetrics {
        complexity: crate::core::ComplexityMetrics {
            functions: filtered_functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
        },
        ..metrics
    }
}

pub fn filter_by_max_complexity(metrics: FileMetrics, threshold: u32) -> FileMetrics {
    let filtered_functions: Vec<_> = metrics
        .complexity
        .functions
        .into_iter()
        .filter(|f| f.cyclomatic <= threshold && f.cognitive <= threshold)
        .collect();

    let (cyclomatic, cognitive) = calculate_total_complexity(&filtered_functions);

    FileMetrics {
        complexity: crate::core::ComplexityMetrics {
            functions: filtered_functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
        },
        ..metrics
    }
}

pub fn filter_by_language(metrics: FileMetrics, languages: Vec<Language>) -> FileMetrics {
    if languages.contains(&metrics.language) {
        metrics
    } else {
        FileMetrics {
            complexity: crate::core::ComplexityMetrics {
                functions: Vec::new(),
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
            },
            debt_items: Vec::new(),
            ..metrics
        }
    }
}

pub fn filter_by_file_pattern(metrics: FileMetrics, patterns: Vec<String>) -> FileMetrics {
    let path_str = metrics.path.to_string_lossy();
    let matches = patterns.iter().any(|pattern| {
        glob::Pattern::new(pattern)
            .map(|p| p.matches(&path_str))
            .unwrap_or(false)
    });

    if matches {
        metrics
    } else {
        FileMetrics {
            complexity: crate::core::ComplexityMetrics {
                functions: Vec::new(),
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
            },
            debt_items: Vec::new(),
            ..metrics
        }
    }
}

pub fn exclude_by_pattern(metrics: FileMetrics, patterns: Vec<String>) -> FileMetrics {
    let path_str = metrics.path.to_string_lossy();
    let matches = patterns.iter().any(|pattern| {
        glob::Pattern::new(pattern)
            .map(|p| p.matches(&path_str))
            .unwrap_or(false)
    });

    if !matches {
        metrics
    } else {
        FileMetrics {
            complexity: crate::core::ComplexityMetrics {
                functions: Vec::new(),
                cyclomatic_complexity: 0,
                cognitive_complexity: 0,
            },
            debt_items: Vec::new(),
            ..metrics
        }
    }
}

pub fn filter_by_min_priority(metrics: FileMetrics, min_priority: Priority) -> FileMetrics {
    FileMetrics {
        debt_items: metrics
            .debt_items
            .into_iter()
            .filter(|item| item.priority >= min_priority)
            .collect(),
        ..metrics
    }
}

pub fn filter_by_debt_types(metrics: FileMetrics, types: Vec<DebtType>) -> FileMetrics {
    FileMetrics {
        debt_items: metrics
            .debt_items
            .into_iter()
            .filter(|item| types.contains(&item.debt_type))
            .collect(),
        ..metrics
    }
}

pub fn compose_filters(
    filters: Vec<Box<dyn Fn(FileMetrics) -> FileMetrics>>,
) -> Box<dyn Fn(FileMetrics) -> FileMetrics> {
    Box::new(move |metrics| filters.iter().fold(metrics, |acc, filter| filter(acc)))
}
