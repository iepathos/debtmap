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
        // Build a pipeline of filters based on configuration
        let filters = self.build_filter_pipeline();

        // Apply all filters in sequence
        filters.into_iter().fold(metrics, |acc, filter| filter(acc))
    }

    fn build_filter_pipeline(&self) -> Vec<Box<dyn FnOnce(FileMetrics) -> FileMetrics>> {
        let mut filters: Vec<Box<dyn FnOnce(FileMetrics) -> FileMetrics>> = Vec::new();

        if let Some(min) = self.min_complexity {
            filters.push(Box::new(move |m| filter_by_min_complexity(m, min)));
        }

        if let Some(max) = self.max_complexity {
            filters.push(Box::new(move |m| filter_by_max_complexity(m, max)));
        }

        if let Some(ref langs) = self.languages {
            let langs = langs.clone();
            filters.push(Box::new(move |m| filter_by_language(m, langs)));
        }

        if let Some(ref patterns) = self.file_patterns {
            let patterns = patterns.clone();
            filters.push(Box::new(move |m| filter_by_file_pattern(m, patterns)));
        }

        if let Some(ref patterns) = self.exclude_patterns {
            let patterns = patterns.clone();
            filters.push(Box::new(move |m| exclude_by_pattern(m, patterns)));
        }

        if let Some(min_prio) = self.min_priority {
            filters.push(Box::new(move |m| filter_by_min_priority(m, min_prio)));
        }

        if let Some(ref types) = self.debt_types {
            let types = types.clone();
            filters.push(Box::new(move |m| filter_by_debt_types(m, types)));
        }

        filters
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ComplexityMetrics, DebtItem};
    use std::path::PathBuf;

    fn create_test_metrics() -> FileMetrics {
        FileMetrics {
            path: PathBuf::from("test.rs"),
            language: Language::Rust,
            complexity: ComplexityMetrics {
                functions: vec![
                    FunctionMetrics {
                        name: "low_complexity".to_string(),
                        file: PathBuf::from("test.rs"),
                        line: 10,
                        cyclomatic: 2,
                        cognitive: 3,
                        nesting: 1,
                        length: 15,
                        is_test: false,
                        visibility: None,
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                        detected_patterns: None,
                        upstream_callers: None,
                        downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
                    },
                    FunctionMetrics {
                        name: "high_complexity".to_string(),
                        file: PathBuf::from("test.rs"),
                        line: 30,
                        cyclomatic: 15,
                        cognitive: 20,
                        nesting: 4,
                        length: 100,
                        is_test: false,
                        visibility: None,
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                        detected_patterns: None,
                        upstream_callers: None,
                        downstream_callees: None,
                    mapping_pattern_result: None,
            adjusted_complexity: None,
        },
                ],
                cyclomatic_complexity: 17,
                cognitive_complexity: 23,
            },
            debt_items: vec![
                DebtItem {
                    id: "debt1".to_string(),
                    debt_type: DebtType::Todo,
                    priority: Priority::Low,
                    file: PathBuf::from("test.rs"),
                    line: 5,
                    column: None,
                    message: "TODO: Fix this".to_string(),
                    context: None,
                },
                DebtItem {
                    id: "debt2".to_string(),
                    debt_type: DebtType::Fixme,
                    priority: Priority::High,
                    file: PathBuf::from("test.rs"),
                    line: 50,
                    column: None,
                    message: "FIXME: Critical issue".to_string(),
                    context: None,
                },
            ],
            dependencies: Vec::new(),
            duplications: Vec::new(),
            module_scope: None,
            classes: None,
        }
    }

    #[test]
    fn test_apply_no_filters() {
        let config = FilterConfig::default();
        let metrics = create_test_metrics();
        let original_functions = metrics.complexity.functions.len();
        let original_debt = metrics.debt_items.len();

        let result = config.apply(metrics);

        assert_eq!(result.complexity.functions.len(), original_functions);
        assert_eq!(result.debt_items.len(), original_debt);
    }

    #[test]
    fn test_apply_min_complexity_filter() {
        let config = FilterConfig {
            min_complexity: Some(10),
            ..Default::default()
        };

        let metrics = create_test_metrics();
        let result = config.apply(metrics);

        assert_eq!(result.complexity.functions.len(), 1);
        assert_eq!(result.complexity.functions[0].name, "high_complexity");
        assert_eq!(result.complexity.cyclomatic_complexity, 15);
        assert_eq!(result.complexity.cognitive_complexity, 20);
    }

    #[test]
    fn test_apply_max_complexity_filter() {
        let config = FilterConfig {
            max_complexity: Some(5),
            ..Default::default()
        };

        let metrics = create_test_metrics();
        let result = config.apply(metrics);

        assert_eq!(result.complexity.functions.len(), 1);
        assert_eq!(result.complexity.functions[0].name, "low_complexity");
        assert_eq!(result.complexity.cyclomatic_complexity, 2);
        assert_eq!(result.complexity.cognitive_complexity, 3);
    }

    #[test]
    fn test_apply_language_filter() {
        let config = FilterConfig {
            languages: Some(vec![Language::Python]),
            ..Default::default()
        };

        let metrics = create_test_metrics();
        let result = config.apply(metrics);

        assert_eq!(result.complexity.functions.len(), 0);
        assert_eq!(result.debt_items.len(), 0);
    }

    #[test]
    fn test_apply_file_pattern_filter() {
        let config = FilterConfig {
            file_patterns: Some(vec!["*.rs".to_string()]),
            ..Default::default()
        };

        let metrics = create_test_metrics();
        let result = config.apply(metrics);

        assert_eq!(result.complexity.functions.len(), 2);
        assert_eq!(result.debt_items.len(), 2);
    }

    #[test]
    fn test_apply_min_priority_filter() {
        let config = FilterConfig {
            min_priority: Some(Priority::High),
            ..Default::default()
        };

        let metrics = create_test_metrics();
        let result = config.apply(metrics);

        assert_eq!(result.debt_items.len(), 1);
        assert_eq!(result.debt_items[0].priority, Priority::High);
    }

    #[test]
    fn test_apply_debt_types_filter() {
        let config = FilterConfig {
            debt_types: Some(vec![DebtType::Fixme]),
            ..Default::default()
        };

        let metrics = create_test_metrics();
        let result = config.apply(metrics);

        assert_eq!(result.debt_items.len(), 1);
        assert_eq!(result.debt_items[0].debt_type, DebtType::Fixme);
    }

    #[test]
    fn test_apply_combined_filters() {
        let config = FilterConfig {
            min_complexity: Some(10),
            min_priority: Some(Priority::High),
            debt_types: Some(vec![DebtType::Fixme]),
            ..Default::default()
        };

        let metrics = create_test_metrics();
        let result = config.apply(metrics);

        assert_eq!(result.complexity.functions.len(), 1);
        assert_eq!(result.complexity.functions[0].name, "high_complexity");
        assert_eq!(result.debt_items.len(), 1);
        assert_eq!(result.debt_items[0].debt_type, DebtType::Fixme);
        assert_eq!(result.debt_items[0].priority, Priority::High);
    }

    #[test]
    fn test_calculate_total_complexity() {
        let functions = vec![
            FunctionMetrics {
                name: "func1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 10,
                cyclomatic: 5,
                cognitive: 7,
                nesting: 1,
                length: 20,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
        },
            FunctionMetrics {
                name: "func2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 40,
                cyclomatic: 3,
                cognitive: 4,
                nesting: 2,
                length: 15,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                is_pure: None,
                purity_confidence: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
        },
        ];

        let (total_cyc, total_cog) = calculate_total_complexity(&functions);
        assert_eq!(total_cyc, 8);
        assert_eq!(total_cog, 11);
    }

    #[test]
    fn test_calculate_total_complexity_empty() {
        let functions = vec![];
        let (total_cyc, total_cog) = calculate_total_complexity(&functions);
        assert_eq!(total_cyc, 0);
        assert_eq!(total_cog, 0);
    }

    #[test]
    fn test_exclude_by_pattern() {
        let metrics = create_test_metrics();

        // Test exclusion pattern that matches
        let result = exclude_by_pattern(metrics.clone(), vec!["*.rs".to_string()]);
        assert_eq!(result.complexity.functions.len(), 0);
        assert_eq!(result.debt_items.len(), 0);

        // Test exclusion pattern that doesn't match
        let result = exclude_by_pattern(metrics.clone(), vec!["*.py".to_string()]);
        assert_eq!(result.complexity.functions.len(), 2);
        assert_eq!(result.debt_items.len(), 2);
    }

    #[test]
    fn test_exclude_by_pattern_multiple() {
        let metrics = create_test_metrics();

        // Test multiple exclusion patterns
        let result = exclude_by_pattern(
            metrics.clone(),
            vec!["*.py".to_string(), "*.js".to_string()],
        );
        assert_eq!(result.complexity.functions.len(), 2);
        assert_eq!(result.debt_items.len(), 2);

        // Test when one pattern matches
        let result = exclude_by_pattern(metrics, vec!["*.py".to_string(), "test.*".to_string()]);
        assert_eq!(result.complexity.functions.len(), 0);
        assert_eq!(result.debt_items.len(), 0);
    }

    #[test]
    fn test_compose_filters() {
        let metrics = create_test_metrics();

        // Create individual filter functions
        let min_complexity_filter = Box::new(|m: FileMetrics| filter_by_min_complexity(m, 10));

        let max_complexity_filter = Box::new(|m: FileMetrics| filter_by_max_complexity(m, 20));

        // Compose the filters
        let composed = compose_filters(vec![min_complexity_filter, max_complexity_filter]);
        let result = composed(metrics);

        // Should only have the high_complexity function (cyclomatic=15, cognitive=20)
        // which passes min_complexity >= 10 and max_complexity <= 20
        assert_eq!(result.complexity.functions.len(), 1);
        assert_eq!(result.complexity.functions[0].name, "high_complexity");
    }

    #[test]
    fn test_compose_filters_empty() {
        let metrics = create_test_metrics();
        let original_functions = metrics.complexity.functions.len();

        // Compose with no filters should return unchanged metrics
        let composed = compose_filters(vec![]);
        let result = composed(metrics);

        assert_eq!(result.complexity.functions.len(), original_functions);
    }

    #[test]
    fn test_compose_filters_order() {
        let metrics = create_test_metrics();

        // Test that filter order matters
        let filter1 = Box::new(|m: FileMetrics| filter_by_min_complexity(m, 10));

        let filter2 = Box::new(|m: FileMetrics| filter_by_min_priority(m, Priority::High));

        let composed = compose_filters(vec![filter1, filter2]);
        let result = composed(metrics);

        // First filter reduces to high_complexity function
        // Second filter reduces debt_items to only High priority
        assert_eq!(result.complexity.functions.len(), 1);
        assert_eq!(result.debt_items.len(), 1);
        assert_eq!(result.debt_items[0].priority, Priority::High);
    }

    #[test]
    fn test_apply_exclude_patterns() {
        let config = FilterConfig {
            exclude_patterns: Some(vec!["*test*".to_string()]),
            ..Default::default()
        };

        let metrics = create_test_metrics();
        let result = config.apply(metrics);

        // test.rs should be excluded
        assert_eq!(result.complexity.functions.len(), 0);
        assert_eq!(result.debt_items.len(), 0);
    }

    #[test]
    fn test_filter_by_file_pattern_no_match() {
        let metrics = create_test_metrics();
        let result = filter_by_file_pattern(metrics, vec!["*.py".to_string()]);

        assert_eq!(result.complexity.functions.len(), 0);
        assert_eq!(result.debt_items.len(), 0);
    }

    #[test]
    fn test_filter_by_file_pattern_invalid_pattern() {
        let metrics = create_test_metrics();
        // Invalid glob pattern should not match
        let result = filter_by_file_pattern(metrics, vec!["[".to_string()]);

        assert_eq!(result.complexity.functions.len(), 0);
        assert_eq!(result.debt_items.len(), 0);
    }

    #[test]
    fn test_exclude_by_pattern_invalid_pattern() {
        let metrics = create_test_metrics();
        // Invalid glob pattern should not exclude (returns original)
        let result = exclude_by_pattern(metrics.clone(), vec!["[".to_string()]);

        assert_eq!(result.complexity.functions.len(), 2);
        assert_eq!(result.debt_items.len(), 2);
    }
}
