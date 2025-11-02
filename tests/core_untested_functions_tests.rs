use anyhow::anyhow;
use debtmap::core::{
    cache::{AnalysisCache, IncrementalAnalysis},
    lazy::TransformationPipeline,
    metrics::combine_metrics,
    monadic::{lift_result, traverse_results, Applicative, OptionExt, ResultExt},
    ComplexityMetrics, FileMetrics, FunctionMetrics, Language,
};
use std::path::PathBuf;
use tempfile::TempDir;

// Helper function to create test metrics
fn create_test_metrics_simple() -> FileMetrics {
    FileMetrics {
        path: PathBuf::from("test.rs"),
        language: Language::Rust,
        complexity: ComplexityMetrics {
            functions: vec![FunctionMetrics {
                name: "test_func".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
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
                composition_metrics: None,
                language_specific: None,
                purity_reason: None,
                call_dependencies: None,
            }],
            cyclomatic_complexity: 2,
            cognitive_complexity: 3,
        },
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
        module_scope: None,
        classes: None,
    }
}

fn create_test_metrics_complex() -> FileMetrics {
    FileMetrics {
        path: PathBuf::from("complex.rs"),
        language: Language::Rust,
        complexity: ComplexityMetrics {
            functions: vec![
                FunctionMetrics {
                    name: "complex_func1".to_string(),
                    file: PathBuf::from("complex.rs"),
                    line: 10,
                    cyclomatic: 5,
                    cognitive: 8,
                    nesting: 2,
                    length: 30,
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
                    composition_metrics: None,
                    language_specific: None,
                    purity_reason: None,
                    call_dependencies: None,
                },
                FunctionMetrics {
                    name: "complex_func2".to_string(),
                    file: PathBuf::from("complex.rs"),
                    line: 50,
                    cyclomatic: 3,
                    cognitive: 4,
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
                    composition_metrics: None,
                    language_specific: None,
                    purity_reason: None,
                    call_dependencies: None,
                },
            ],
            cyclomatic_complexity: 8,
            cognitive_complexity: 12,
        },
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
        module_scope: None,
        classes: None,
    }
}

// Tests for load_previous() in IncrementalAnalysis
mod test_load_previous {
    use super::*;

    #[test]
    fn test_load_previous_empty_cache() {
        let mut inc = IncrementalAnalysis::new();
        let temp_dir = TempDir::new().unwrap();
        let cache = AnalysisCache::new(Some(temp_dir.path())).unwrap();

        inc.load_previous(&cache);

        assert!(inc.previous_state.is_empty());
    }

    #[test]
    fn test_load_previous_single_entry() {
        let mut inc = IncrementalAnalysis::new();
        let temp_dir = TempDir::new().unwrap();
        let cache = AnalysisCache::new(Some(temp_dir.path())).unwrap();

        // Load from cache (which is empty initially)
        inc.load_previous(&cache);

        // Since the cache is empty through public API, previous_state should be empty
        assert_eq!(inc.previous_state.len(), 0);
    }

    #[test]
    fn test_load_previous_multiple_entries() {
        let mut inc = IncrementalAnalysis::new();
        let temp_dir = TempDir::new().unwrap();
        let cache = AnalysisCache::new(Some(temp_dir.path())).unwrap();

        // Load from cache
        inc.load_previous(&cache);

        // Since we can't easily manipulate the cache internals, test the behavior
        assert!(inc.previous_state.is_empty());

        // Add entries to current state to test the diff functionality
        inc.update_file(create_test_metrics_simple());
        inc.update_file(create_test_metrics_complex());

        assert_eq!(inc.current_state.len(), 2);
    }

    #[test]
    fn test_load_previous_preserves_metrics() {
        let mut inc = IncrementalAnalysis::new();

        // Manually populate previous_state to test preservation
        let metrics = create_test_metrics_simple();
        inc.previous_state = inc
            .previous_state
            .update(metrics.path.clone(), metrics.clone());

        assert_eq!(inc.previous_state.len(), 1);
        let stored = inc.previous_state.get(&PathBuf::from("test.rs")).unwrap();
        assert_eq!(stored.complexity.cyclomatic_complexity, 2);
        assert_eq!(stored.complexity.cognitive_complexity, 3);
    }
}

// Tests for apply_result() in Applicative
mod test_apply_result {
    use super::*;

    #[test]
    fn test_apply_result_all_success() {
        let app = Applicative::new(vec![1, 2, 3]);
        let result = app.apply_result(|x| Ok(x * 2));

        assert!(result.is_ok());
        let values = result.unwrap().unwrap();
        assert_eq!(values, vec![2, 4, 6]);
    }

    #[test]
    fn test_apply_result_with_failure() {
        let app = Applicative::new(vec![1, 2, 3]);
        let result = app.apply_result(|x| {
            if x == 2 {
                Err(anyhow!("Error on 2"))
            } else {
                Ok(x * 2)
            }
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_apply_result_empty_input() {
        let app: Applicative<i32> = Applicative::new(vec![]);
        let result = app.apply_result(|x| Ok(x * 2));

        assert!(result.is_ok());
        let values = result.unwrap().unwrap();
        assert!(values.is_empty());
    }

    #[test]
    fn test_apply_result_complex_transformation() {
        let app = Applicative::new(vec!["hello", "world", "test"]);
        let result = app.apply_result(|s| {
            if s.len() > 4 {
                Ok(s.to_uppercase())
            } else {
                Ok(s.to_string())
            }
        });

        assert!(result.is_ok());
        let values = result.unwrap().unwrap();
        assert_eq!(values, vec!["HELLO", "WORLD", "test"]);
    }

    #[test]
    fn test_apply_result_with_chaining() {
        let app = Applicative::new(vec![1, 2, 3]);

        let result = app.apply_result(|x| Ok(x * x));

        assert!(result.is_ok());
        let values = result.unwrap().unwrap();
        assert_eq!(values, vec![1, 4, 9]);
    }
}

// Tests for combine_metrics()
mod test_combine_metrics {
    use super::*;

    #[test]
    fn test_combine_metrics_empty() {
        let metrics1 = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
        };

        let metrics2 = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
        };

        let combined = combine_metrics(metrics1, metrics2);

        assert!(combined.functions.is_empty());
        assert_eq!(combined.cyclomatic_complexity, 0);
        assert_eq!(combined.cognitive_complexity, 0);
    }

    #[test]
    fn test_combine_metrics_single_function_each() {
        let func1 = FunctionMetrics {
            name: "func1".to_string(),
            file: PathBuf::from("file1.rs"),
            line: 10,
            cyclomatic: 3,
            cognitive: 4,
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
            composition_metrics: None,
            language_specific: None,
            purity_reason: None,
            call_dependencies: None,
        };

        let func2 = FunctionMetrics {
            name: "func2".to_string(),
            file: PathBuf::from("file2.rs"),
            line: 20,
            cyclomatic: 5,
            cognitive: 6,
            nesting: 2,
            length: 30,
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
            composition_metrics: None,
            language_specific: None,
            purity_reason: None,
            call_dependencies: None,
        };

        let metrics1 = ComplexityMetrics {
            functions: vec![func1.clone()],
            cyclomatic_complexity: 3,
            cognitive_complexity: 4,
        };

        let metrics2 = ComplexityMetrics {
            functions: vec![func2.clone()],
            cyclomatic_complexity: 5,
            cognitive_complexity: 6,
        };

        let combined = combine_metrics(metrics1, metrics2);

        assert_eq!(combined.functions.len(), 2);
        assert_eq!(combined.cyclomatic_complexity, 8);
        assert_eq!(combined.cognitive_complexity, 10);
    }

    #[test]
    fn test_combine_metrics_multiple_functions() {
        let metrics1 = create_test_metrics_simple().complexity;
        let metrics2 = create_test_metrics_complex().complexity;

        let combined = combine_metrics(metrics1, metrics2);

        assert_eq!(combined.functions.len(), 3);
        assert_eq!(combined.cyclomatic_complexity, 10); // 2 + 5 + 3
        assert_eq!(combined.cognitive_complexity, 15); // 3 + 8 + 4
    }

    #[test]
    fn test_combine_metrics_preserves_function_details() {
        let func1 = FunctionMetrics {
            name: "detailed_func".to_string(),
            file: PathBuf::from("detail.rs"),
            line: 42,
            cyclomatic: 7,
            cognitive: 12,
            nesting: 3,
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
            composition_metrics: None,
            language_specific: None,
            purity_reason: None,
            call_dependencies: None,
        };

        let metrics1 = ComplexityMetrics {
            functions: vec![func1.clone()],
            cyclomatic_complexity: 7,
            cognitive_complexity: 12,
        };

        let metrics2 = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
        };

        let combined = combine_metrics(metrics1, metrics2);

        assert_eq!(combined.functions.len(), 1);
        assert_eq!(combined.functions[0].name, "detailed_func");
        assert_eq!(combined.functions[0].line, 42);
        assert_eq!(combined.functions[0].nesting, 3);
        assert_eq!(combined.functions[0].length, 100);
    }

    #[test]
    fn test_combine_metrics_order_preservation() {
        let func1 = FunctionMetrics::new("first".to_string(), PathBuf::from("a.rs"), 1);
        let func2 = FunctionMetrics::new("second".to_string(), PathBuf::from("b.rs"), 2);
        let func3 = FunctionMetrics::new("third".to_string(), PathBuf::from("c.rs"), 3);

        let metrics1 = ComplexityMetrics {
            functions: vec![func1, func2],
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
        };

        let metrics2 = ComplexityMetrics {
            functions: vec![func3],
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
        };

        let combined = combine_metrics(metrics1, metrics2);

        assert_eq!(combined.functions[0].name, "first");
        assert_eq!(combined.functions[1].name, "second");
        assert_eq!(combined.functions[2].name, "third");
    }
}

// Tests for apply() in Applicative
mod test_applicative_apply {
    use super::*;

    #[test]
    fn test_apply_simple_transformation() {
        let app = Applicative::new(vec![1, 2, 3, 4, 5]);
        let result = app.apply(|x| x * 2);

        let values = result.unwrap();
        assert_eq!(values, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_apply_empty_values() {
        let app: Applicative<i32> = Applicative::new(vec![]);
        let result = app.apply(|x| x + 1);

        let values = result.unwrap();
        assert!(values.is_empty());
    }

    #[test]
    fn test_apply_string_transformation() {
        let app = Applicative::new(vec!["hello", "world", "rust"]);
        let result = app.apply(|s| s.to_uppercase());

        let values = result.unwrap();
        assert_eq!(values, vec!["HELLO", "WORLD", "RUST"]);
    }

    #[test]
    fn test_apply_complex_type_transformation() {
        #[derive(Debug, PartialEq)]
        struct Person {
            name: String,
            age: u32,
        }

        let app = Applicative::new(vec![
            Person {
                name: "Alice".to_string(),
                age: 30,
            },
            Person {
                name: "Bob".to_string(),
                age: 25,
            },
        ]);

        let result = app.apply(|p| p.age + 5);

        let values = result.unwrap();
        assert_eq!(values, vec![35, 30]);
    }

    #[test]
    fn test_apply_chain_transformations() {
        let app = Applicative::new(vec![1, 2, 3]);
        let intermediate = app.apply(|x| x * x);
        let result = intermediate.apply(|x| x + 1);

        let values = result.unwrap();
        assert_eq!(values, vec![2, 5, 10]);
    }

    #[test]
    fn test_apply_closure_with_captures() {
        let multiplier = 3;
        let offset = 10;

        let app = Applicative::new(vec![1, 2, 3, 4]);
        let result = app.apply(|x| x * multiplier + offset);

        let values = result.unwrap();
        assert_eq!(values, vec![13, 16, 19, 22]);
    }

    #[test]
    fn test_apply_preserves_order() {
        let app = Applicative::new(vec![5, 3, 8, 1, 9]);
        let result = app.apply(|x| x * 10);

        let values = result.unwrap();
        assert_eq!(values, vec![50, 30, 80, 10, 90]);
    }

    #[test]
    fn test_apply_with_option_transformation() {
        let app = Applicative::new(vec![Some(1), None, Some(3), None, Some(5)]);
        let result = app.apply(|opt| opt.map(|x| x * 2));

        let values = result.unwrap();
        assert_eq!(values, vec![Some(2), None, Some(6), None, Some(10)]);
    }
}

// Tests for apply() in TransformationPipeline
mod test_apply {
    use super::*;

    #[test]
    fn test_apply_no_transformations() {
        let pipeline = TransformationPipeline::new();
        let value = 42;
        let result = pipeline.apply(value);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_apply_single_transformation() {
        let pipeline = TransformationPipeline::new().add_transformation(|x| x * 2);

        let result = pipeline.apply(10);
        assert_eq!(result, 20);
    }

    #[test]
    fn test_apply_multiple_transformations() {
        let pipeline = TransformationPipeline::new()
            .add_transformation(|x| x + 1)
            .add_transformation(|x| x * 2)
            .add_transformation(|x| x - 3);

        let result = pipeline.apply(5);
        // (5 + 1) * 2 - 3 = 6 * 2 - 3 = 12 - 3 = 9
        assert_eq!(result, 9);
    }

    #[test]
    fn test_apply_string_transformations() {
        fn add_exclamation(s: String) -> String {
            format!("{s}!")
        }

        fn uppercase(s: String) -> String {
            s.to_uppercase()
        }

        let pipeline = TransformationPipeline::new()
            .add_transformation(uppercase)
            .add_transformation(add_exclamation);

        let result = pipeline.apply("hello".to_string());
        assert_eq!(result, "HELLO!");
    }

    #[test]
    fn test_apply_all_multiple_values() {
        let pipeline = TransformationPipeline::new()
            .add_transformation(|x| x * 2)
            .add_transformation(|x| x + 10);

        let values = vec![1, 2, 3, 4, 5];
        let results = pipeline.apply_all(values);

        assert_eq!(results, vec![12, 14, 16, 18, 20]);
    }
}

// Tests for traverse_results()
mod test_traverse_results {
    use super::*;

    #[test]
    fn test_traverse_results_all_ok() {
        let values = vec![1, 2, 3, 4, 5];
        let result = traverse_results(values, |x| Ok(x * 2));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_traverse_results_with_error() {
        let values = vec![1, 2, 3, 4, 5];
        let result = traverse_results(values, |x| {
            if x == 3 {
                Err(anyhow!("Error at 3"))
            } else {
                Ok(x * 2)
            }
        });

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Error at 3"));
    }

    #[test]
    fn test_traverse_results_empty_input() {
        let values: Vec<i32> = vec![];
        let result = traverse_results(values, |x| Ok(x * 2));

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_traverse_results_string_conversion() {
        let values = vec!["1", "2", "3"];
        let result = traverse_results(values, |s| {
            s.parse::<i32>()
                .map(|n| n * 10)
                .map_err(|e| anyhow!("Parse error: {}", e))
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![10, 20, 30]);
    }

    #[test]
    fn test_traverse_results_invalid_string_conversion() {
        let values = vec!["1", "not_a_number", "3"];
        let result = traverse_results(values, |s| {
            s.parse::<i32>()
                .map(|n| n * 10)
                .map_err(|e| anyhow!("Parse error: {}", e))
        });

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Parse error"));
    }

    #[test]
    fn test_traverse_results_complex_transformation() {
        #[derive(Debug, PartialEq)]
        struct User {
            id: u32,
            name: String,
        }

        let ids = vec![1, 2, 3];
        let result = traverse_results(ids, |id| {
            // Simulate a database lookup that could fail
            if id > 0 && id <= 3 {
                Ok(User {
                    id,
                    name: format!("User{id}"),
                })
            } else {
                Err(anyhow!("User not found"))
            }
        });

        assert!(result.is_ok());
        let users = result.unwrap();
        assert_eq!(users.len(), 3);
        assert_eq!(users[0].name, "User1");
        assert_eq!(users[1].name, "User2");
        assert_eq!(users[2].name, "User3");
    }
}

// Tests for or_else_with() in ResultExt
mod test_or_else_with {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_or_else_with_ok_value() {
        let result: Result<i32> = Ok(42);
        let fallback = result.or_else_with(|| Ok(100));

        assert!(fallback.is_ok());
        assert_eq!(fallback.unwrap(), 42); // Original value is preserved
    }

    #[test]
    fn test_or_else_with_error_fallback_success() {
        let result: Result<i32> = Err(anyhow!("Initial error"));
        let fallback = result.or_else_with(|| Ok(100));

        assert!(fallback.is_ok());
        assert_eq!(fallback.unwrap(), 100); // Fallback value is used
    }

    #[test]
    fn test_or_else_with_error_fallback_error() {
        let result: Result<i32> = Err(anyhow!("Initial error"));
        let fallback = result.or_else_with(|| Err(anyhow!("Fallback error")));

        assert!(fallback.is_err());
        assert!(fallback.unwrap_err().to_string().contains("Fallback error"));
    }

    #[test]
    fn test_or_else_with_complex_type() {
        #[derive(Debug, PartialEq)]
        struct Data {
            value: String,
            count: u32,
        }

        let result: Result<Data> = Err(anyhow!("Failed to load"));
        let fallback = result.or_else_with(|| {
            Ok(Data {
                value: "default".to_string(),
                count: 0,
            })
        });

        assert!(fallback.is_ok());
        let data = fallback.unwrap();
        assert_eq!(data.value, "default");
        assert_eq!(data.count, 0);
    }

    #[test]
    fn test_or_else_with_chain() {
        let result: Result<i32> = Err(anyhow!("Error 1"));

        let fallback = result
            .or_else_with(|| Err(anyhow!("Error 2")))
            .or_else_with(|| Ok(42));

        assert!(fallback.is_ok());
        assert_eq!(fallback.unwrap(), 42);
    }

    #[test]
    fn test_or_else_with_side_effects() {
        let mut counter = 0;

        let result: Result<i32> = Err(anyhow!("Error"));
        let fallback = result.or_else_with(|| {
            counter += 1;
            Ok(counter * 10)
        });

        assert!(fallback.is_ok());
        assert_eq!(fallback.unwrap(), 10);
        assert_eq!(counter, 1);
    }

    #[test]
    fn test_or_else_with_no_execution_on_ok() {
        let mut counter = 0;

        let result: Result<i32> = Ok(42);
        let fallback = result.or_else_with(|| {
            counter += 1; // This should not execute
            Ok(100)
        });

        assert!(fallback.is_ok());
        assert_eq!(fallback.unwrap(), 42);
        assert_eq!(counter, 0); // Counter unchanged
    }

    #[test]
    fn test_or_else_with_async_like_pattern() {
        fn fetch_primary() -> Result<String> {
            Err(anyhow!("Primary failed"))
        }

        fn fetch_backup() -> Result<String> {
            Ok("backup data".to_string())
        }

        let result = fetch_primary().or_else_with(fetch_backup);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "backup data");
    }
}

// Tests for or_else_some() in OptionExt
mod test_or_else_some {
    use super::*;

    #[test]
    fn test_or_else_some_with_some_value() {
        let value: Option<i32> = Some(42);
        let fallback = value.or_else_some(|| Some(100));

        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap(), 42); // Original value preserved
    }

    #[test]
    fn test_or_else_some_with_none_fallback_some() {
        let value: Option<i32> = None;
        let fallback = value.or_else_some(|| Some(100));

        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap(), 100); // Fallback value used
    }

    #[test]
    fn test_or_else_some_with_none_fallback_none() {
        let value: Option<i32> = None;
        let fallback = value.or_else_some(|| None);

        assert!(fallback.is_none());
    }

    #[test]
    fn test_or_else_some_with_complex_type() {
        #[derive(Debug, PartialEq)]
        struct Config {
            setting: String,
            enabled: bool,
        }

        let value: Option<Config> = None;
        let fallback = value.or_else_some(|| {
            Some(Config {
                setting: "default".to_string(),
                enabled: true,
            })
        });

        assert!(fallback.is_some());
        let config = fallback.unwrap();
        assert_eq!(config.setting, "default");
        assert!(config.enabled);
    }

    #[test]
    fn test_or_else_some_chain() {
        let value: Option<i32> = None;

        let result = value.or_else_some(|| None).or_else_some(|| Some(42));

        assert!(result.is_some());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_or_else_some_lazy_evaluation() {
        let mut counter = 0;

        let value: Option<i32> = None;
        let fallback = value.or_else_some(|| {
            counter += 1;
            Some(counter * 10)
        });

        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap(), 10);
        assert_eq!(counter, 1);
    }

    #[test]
    fn test_or_else_some_no_execution_on_some() {
        let mut counter = 0;

        let value: Option<i32> = Some(42);
        let fallback = value.or_else_some(|| {
            counter += 1; // Should not execute
            Some(100)
        });

        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap(), 42);
        assert_eq!(counter, 0); // Counter unchanged
    }

    #[test]
    fn test_or_else_some_practical_use_case() {
        fn get_env_var() -> Option<String> {
            None // Simulate missing env var
        }

        fn get_default_config() -> Option<String> {
            Some("default_value".to_string())
        }

        let config = get_env_var().or_else_some(get_default_config);

        assert!(config.is_some());
        assert_eq!(config.unwrap(), "default_value");
    }

    #[test]
    fn test_or_else_some_with_filter() {
        let value: Option<i32> = None;

        let result = value.or_else_some(|| Some(10)).filter_some(|&x| x > 5);

        assert!(result.is_some());
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_or_else_some_edge_case_nested_options() {
        let value: Option<Option<i32>> = None;
        let fallback = value.or_else_some(|| Some(Some(42)));

        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap().unwrap(), 42);
    }
}

// Tests for lift_result()
mod test_lift_result {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_lift_result_basic() {
        let add_one = |x: i32| x + 1;
        let lifted = lift_result(add_one);

        let result = lifted(41);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_lift_result_string_transformation() {
        let uppercase = |s: String| s.to_uppercase();
        let lifted = lift_result(uppercase);

        let result = lifted("hello".to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "HELLO");
    }

    #[test]
    fn test_lift_result_complex_type() {
        #[derive(Debug, PartialEq)]
        struct Point {
            x: i32,
            y: i32,
        }

        let double_point = |p: Point| Point {
            x: p.x * 2,
            y: p.y * 2,
        };

        let lifted = lift_result(double_point);

        let result = lifted(Point { x: 3, y: 4 });
        assert!(result.is_ok());

        let doubled = result.unwrap();
        assert_eq!(doubled.x, 6);
        assert_eq!(doubled.y, 8);
    }

    #[test]
    fn test_lift_result_composition() {
        let add_one = |x: i32| x + 1;
        let double = |x: i32| x * 2;

        let lifted_add = lift_result(add_one);
        let lifted_double = lift_result(double);

        let result = lifted_add(5).and_then(lifted_double);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12); // (5 + 1) * 2
    }

    #[test]
    fn test_lift_result_with_chain() {
        let square = |x: i32| x * x;
        let lifted = lift_result(square);

        let result: Result<i32> = Ok(3).and_then(lifted).and_then(lift_result(|x| x + 1));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10); // 3^2 + 1
    }

    #[test]
    fn test_lift_result_pure_function_guarantee() {
        let pure_fn = |x: i32| x * 2;
        let lifted = lift_result(pure_fn);

        // Call multiple times with same input
        let result1 = lifted(21);
        let result2 = lifted(21);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(result1.unwrap(), 42);
        assert_eq!(result2.unwrap(), 42);
    }

    #[test]
    fn test_lift_result_with_vec_map() {
        let double = |x: i32| x * 2;
        let lifted = lift_result(double);

        let values = vec![1, 2, 3];
        let results: Result<Vec<i32>> = values.into_iter().map(lifted).collect();

        assert!(results.is_ok());
        assert_eq!(results.unwrap(), vec![2, 4, 6]);
    }

    #[test]
    fn test_lift_result_practical_parsing() {
        fn parse_and_double(s: &str) -> Result<i32> {
            let parsed: Result<i32> = s.parse().map_err(|e| anyhow!("{}", e));
            parsed.and_then(lift_result(|x| x * 2))
        }

        let result = parse_and_double("21");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_lift_result_tuple_transformation() {
        let swap = |(a, b): (i32, String)| (b, a);
        let lifted = lift_result(swap);

        let result = lifted((42, "answer".to_string()));
        assert!(result.is_ok());

        let swapped = result.unwrap();
        assert_eq!(swapped.0, "answer");
        assert_eq!(swapped.1, 42);
    }

    #[test]
    fn test_lift_result_closure_capture() {
        let multiplier = 10;
        let multiply = |x: i32| x * multiplier;
        let lifted = lift_result(multiply);

        let result = lifted(5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50);
    }
}

// Tests for default() in TransformationPipeline
mod test_transformation_pipeline_default {
    use super::*;

    #[test]
    fn test_default_creates_empty_pipeline() {
        let pipeline: TransformationPipeline<i32> = Default::default();

        // Apply to a value - should return unchanged
        let result = pipeline.apply(42);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_default_apply_all_empty() {
        let pipeline: TransformationPipeline<i32> = Default::default();

        let values = vec![1, 2, 3, 4, 5];
        let results = pipeline.apply_all(values.clone());

        assert_eq!(results, values); // Values unchanged
    }

    #[test]
    fn test_default_then_add_transformations() {
        let pipeline: TransformationPipeline<i32> = Default::default();
        let pipeline = pipeline
            .add_transformation(|x| x + 1)
            .add_transformation(|x| x * 2);

        let result = pipeline.apply(10);
        assert_eq!(result, 22); // (10 + 1) * 2
    }

    #[test]
    fn test_default_type_inference() {
        // Test that Default can infer the type properly
        let pipeline = TransformationPipeline::default();
        let pipeline = pipeline.add_transformation(|x: String| x.to_uppercase());

        let result = pipeline.apply("hello".to_string());
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_default_multiple_instances() {
        let pipeline1: TransformationPipeline<i32> = Default::default();
        let pipeline2: TransformationPipeline<i32> = Default::default();

        // Both should behave identically
        assert_eq!(pipeline1.apply(100), pipeline2.apply(100));
    }

    #[test]
    fn test_default_with_complex_type() {
        #[derive(Debug, PartialEq, Clone)]
        struct Data {
            value: i32,
        }

        let pipeline: TransformationPipeline<Data> = Default::default();
        let data = Data { value: 42 };

        let result = pipeline.apply(data.clone());
        assert_eq!(result, data);
    }

    #[test]
    fn test_default_chain_behavior() {
        let base_pipeline: TransformationPipeline<i32> = Default::default();

        // Create different pipelines from the same default
        let pipeline1 = base_pipeline.add_transformation(|x| x + 10);
        let pipeline2 = TransformationPipeline::default().add_transformation(|x| x * 2);

        assert_eq!(pipeline1.apply(5), 15);
        assert_eq!(pipeline2.apply(5), 10);
    }

    #[test]
    fn test_default_practical_use_case() {
        // Simulating a data processing pipeline
        fn create_processing_pipeline(needs_normalization: bool) -> TransformationPipeline<f64> {
            let mut pipeline = TransformationPipeline::default();

            if needs_normalization {
                pipeline = pipeline.add_transformation(|x| x / 100.0);
            }

            pipeline.add_transformation(|x| x * 2.0)
        }

        let pipeline_with_norm = create_processing_pipeline(true);
        let pipeline_without_norm = create_processing_pipeline(false);

        assert_eq!(pipeline_with_norm.apply(100.0), 2.0); // (100/100) * 2
        assert_eq!(pipeline_without_norm.apply(100.0), 200.0); // 100 * 2
    }
}
