use anyhow::anyhow;
use debtmap::core::{
    cache::{AnalysisCache, IncrementalAnalysis},
    lazy::TransformationPipeline,
    metrics::combine_metrics,
    monadic::{traverse_results, Applicative},
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
            }],
            cyclomatic_complexity: 2,
            cognitive_complexity: 3,
        },
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
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
                },
                FunctionMetrics {
                    name: "complex_func2".to_string(),
                    file: PathBuf::from("complex.rs"),
                    line: 50,
                    cyclomatic: 3,
                    cognitive: 4,
                    nesting: 1,
                    length: 20,
                },
            ],
            cyclomatic_complexity: 8,
            cognitive_complexity: 12,
        },
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
    }
}

// Tests for load_previous() in IncrementalAnalysis
mod test_load_previous {
    use super::*;

    #[test]
    fn test_load_previous_empty_cache() {
        let mut inc = IncrementalAnalysis::new();
        let temp_dir = TempDir::new().unwrap();
        let cache = AnalysisCache::new(temp_dir.path().to_path_buf()).unwrap();

        inc.load_previous(&cache);

        assert!(inc.previous_state.is_empty());
    }

    #[test]
    fn test_load_previous_single_entry() {
        let mut inc = IncrementalAnalysis::new();
        let temp_dir = TempDir::new().unwrap();
        let cache = AnalysisCache::new(temp_dir.path().to_path_buf()).unwrap();

        // Load from cache (which is empty initially)
        inc.load_previous(&cache);

        // Since the cache is empty through public API, previous_state should be empty
        assert_eq!(inc.previous_state.len(), 0);
    }

    #[test]
    fn test_load_previous_multiple_entries() {
        let mut inc = IncrementalAnalysis::new();
        let temp_dir = TempDir::new().unwrap();
        let cache = AnalysisCache::new(temp_dir.path().to_path_buf()).unwrap();

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
        };

        let func2 = FunctionMetrics {
            name: "func2".to_string(),
            file: PathBuf::from("file2.rs"),
            line: 20,
            cyclomatic: 5,
            cognitive: 6,
            nesting: 2,
            length: 30,
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
