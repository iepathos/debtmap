#[cfg(test)]
mod tests {
    use super::super::rust_patterns::*;
    use crate::extraction_patterns::{
        AccumulationOp, AnalysisContext, Expression, ExtractablePattern, GuardCheck,
        MatchedPattern, PatternMatcher, ReturnType,
    };
    use syn::{parse_str, File};

    fn create_test_context() -> AnalysisContext {
        AnalysisContext {
            function_name: "test_func".to_string(),
            file_path: "test.rs".to_string(),
            language: "rust".to_string(),
            complexity_before: 10,
            has_side_effects: false,
            data_dependencies: vec![],
        }
    }

    #[test]
    fn test_detect_accumulation_loop() {
        let code = r#"
            fn sum_values(items: &[i32]) -> i32 {
                let mut total = 0;
                for item in items {
                    total += item;
                }
                total
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse code");
        // Use the source-aware matcher
        let matcher = RustPatternMatcher::with_source_context(code, 1);
        let context = create_test_context();
        let patterns = matcher.match_patterns(&file, &context);

        assert!(!patterns.is_empty(), "Should detect accumulation pattern");

        let pattern = &patterns[0].pattern;
        match pattern {
            ExtractablePattern::AccumulationLoop {
                iterator_binding,
                operation,
                start_line,
                end_line,
                ..
            } => {
                assert_eq!(iterator_binding, "item");
                assert!(matches!(operation, AccumulationOp::Sum));
                assert!(end_line > start_line, "Should have valid line range");
            }
            _ => panic!("Expected AccumulationLoop pattern"),
        }
    }

    #[test]
    fn test_detect_guard_chain() {
        let code = r#"
            fn validate_input(value: i32) -> Result<i32, String> {
                if value < 0 {
                    return Err("Value must be non-negative".to_string());
                }
                if value > 100 {
                    return Err("Value too large".to_string());
                }
                if value % 2 != 0 {
                    return Err("Value must be even".to_string());
                }
                Ok(value)
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse code");
        let matcher = RustPatternMatcher::with_source_context(code, 1);
        let context = create_test_context();
        let patterns = matcher.match_patterns(&file, &context);

        assert!(!patterns.is_empty(), "Should detect guard chain pattern");

        let pattern = &patterns[0].pattern;
        match pattern {
            ExtractablePattern::GuardChainSequence {
                checks,
                early_return,
                ..
            } => {
                assert!(checks.len() >= 2, "Should have at least 2 guard checks");
                assert!(early_return.is_early_return, "Should be early returns");
            }
            _ => panic!("Expected GuardChainSequence pattern"),
        }
    }

    #[test]
    fn test_detect_transformation_pipeline() {
        let code = r#"
            fn process_data(items: Vec<String>) -> Vec<i32> {
                items
                    .iter()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.len() as i32)
                    .filter(|&n| n > 0)
                    .collect()
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse code");
        let matcher = RustPatternMatcher::with_source_context(code, 1);
        let context = create_test_context();
        let patterns = matcher.match_patterns(&file, &context);

        let has_pipeline = patterns.iter().any(|p| {
            matches!(
                &p.pattern,
                ExtractablePattern::TransformationPipeline { .. }
            )
        });
        assert!(has_pipeline, "Should detect transformation pipeline");
    }

    #[test]
    fn test_generate_extraction_suggestion() {
        let pattern = MatchedPattern {
            pattern: ExtractablePattern::AccumulationLoop {
                iterator_binding: "item".to_string(),
                accumulator: "sum".to_string(),
                operation: AccumulationOp::Sum,
                filter: None,
                transform: None,
                start_line: 10,
                end_line: 15,
            },
            confidence: 0.85,
            context: create_test_context(),
        };

        let matcher = RustPatternMatcher::new();
        let suggestion = matcher.generate_extraction(&pattern);

        assert_eq!(suggestion.start_line, 10);
        assert_eq!(suggestion.end_line, 15);
        assert!(
            suggestion.suggested_name.contains("sum")
                || suggestion.suggested_name.contains("accumulate")
        );
        assert!(suggestion.confidence > 0.0);
        assert!(
            suggestion.complexity_reduction.predicted_cyclomatic
                < suggestion.complexity_reduction.current_cyclomatic
        );
    }

    #[test]
    fn test_confidence_scoring() {
        let pattern = MatchedPattern {
            pattern: ExtractablePattern::GuardChainSequence {
                checks: vec![
                    GuardCheck {
                        condition: "value < 0".to_string(),
                        return_value: Some("Error".to_string()),
                        line: 5,
                    },
                    GuardCheck {
                        condition: "value > 100".to_string(),
                        return_value: Some("Error".to_string()),
                        line: 7,
                    },
                ],
                early_return: ReturnType {
                    type_name: "Result<()>".to_string(),
                    is_early_return: true,
                },
                start_line: 5,
                end_line: 10,
            },
            confidence: 0.0,
            context: create_test_context(),
        };

        let matcher = RustPatternMatcher::new();
        let confidence = matcher.score_confidence(&pattern, &pattern.context);

        assert!(confidence > 0.5, "Guard chains should have good confidence");
        assert!(confidence <= 1.0, "Confidence should not exceed 1.0");
    }

    #[test]
    fn test_complex_function_extraction() {
        let code = r#"
            fn complex_calculation(data: Vec<i32>) -> Result<i32, String> {
                // Validation guards
                if data.is_empty() {
                    return Err("Empty data".to_string());
                }
                if data.len() > 1000 {
                    return Err("Too much data".to_string());
                }
                
                // Accumulation loop
                let mut sum = 0;
                for value in &data {
                    if *value > 0 {
                        sum += value;
                    }
                }
                
                // Transformation pipeline
                let processed = data
                    .iter()
                    .filter(|&&x| x > 0)
                    .map(|&x| x * 2)
                    .collect::<Vec<_>>();
                
                Ok(sum + processed.len() as i32)
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse code");
        let matcher = RustPatternMatcher::with_source_context(code, 1);
        let context = AnalysisContext {
            function_name: "complex_calculation".to_string(),
            file_path: "test.rs".to_string(),
            language: "rust".to_string(),
            complexity_before: 15,
            has_side_effects: false,
            data_dependencies: vec!["data".to_string()],
        };

        let patterns = matcher.match_patterns(&file, &context);

        assert!(patterns.len() >= 2, "Should detect multiple patterns");

        // Check for different pattern types
        let has_guards = patterns
            .iter()
            .any(|p| matches!(&p.pattern, ExtractablePattern::GuardChainSequence { .. }));
        let has_accumulation = patterns
            .iter()
            .any(|p| matches!(&p.pattern, ExtractablePattern::AccumulationLoop { .. }));

        assert!(
            has_guards || has_accumulation,
            "Should detect at least guards or accumulation"
        );

        // Test extraction suggestions
        for (i, pattern) in patterns.iter().enumerate() {
            println!("Pattern {}: {:?}", i, pattern.pattern);
            let suggestion = matcher.generate_extraction(pattern);
            println!(
                "  Start: {}, End: {}",
                suggestion.start_line, suggestion.end_line
            );
            assert!(suggestion.start_line > 0, "Should have valid start line");
            assert!(
                suggestion.end_line > suggestion.start_line,
                "Should have valid line range"
            );
            assert!(
                !suggestion.suggested_name.is_empty(),
                "Should have suggested name"
            );
            assert!(
                suggestion
                    .complexity_reduction
                    .extracted_function_complexity
                    > 0,
                "Should calculate extracted complexity"
            );
        }
    }

    #[test]
    fn test_line_number_preservation() {
        // This test verifies that line numbers are properly tracked through extraction
        let pattern = ExtractablePattern::AccumulationLoop {
            iterator_binding: "item".to_string(),
            accumulator: "result".to_string(),
            operation: AccumulationOp::Collection,
            filter: Some(Box::new(Expression {
                code: "*item > 0".to_string(),
                variables: vec!["item".to_string()],
            })),
            transform: Some(Box::new(Expression {
                code: "item * 2".to_string(),
                variables: vec!["item".to_string()],
            })),
            start_line: 42,
            end_line: 52,
        };

        let matched = MatchedPattern {
            pattern: pattern.clone(),
            confidence: 0.9,
            context: create_test_context(),
        };

        let matcher = RustPatternMatcher::new();
        let suggestion = matcher.generate_extraction(&matched);

        assert_eq!(suggestion.start_line, 42, "Start line should be preserved");
        assert_eq!(suggestion.end_line, 52, "End line should be preserved");

        // Verify the pattern type is preserved in suggestion
        match suggestion.pattern_type {
            ExtractablePattern::AccumulationLoop {
                start_line,
                end_line,
                ..
            } => {
                assert_eq!(start_line, 42);
                assert_eq!(end_line, 52);
            }
            _ => panic!("Pattern type should be preserved"),
        }
    }

    #[test]
    fn test_empty_function_no_patterns() {
        let code = r#"
            fn simple_function() -> i32 {
                42
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse code");
        let matcher = RustPatternMatcher::with_source_context(code, 1);
        let context = create_test_context();
        let patterns = matcher.match_patterns(&file, &context);

        assert!(
            patterns.is_empty(),
            "Simple function should have no patterns"
        );
    }

    #[test]
    fn test_pattern_with_side_effects() {
        let code = r#"
            fn process_with_io(items: Vec<String>) -> std::io::Result<()> {
                for item in items {
                    println!("{}", item);  // Side effect
                    std::fs::write("output.txt", item)?;  // I/O
                }
                Ok(())
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse code");
        let matcher = RustPatternMatcher::with_source_context(code, 1);
        let mut context = create_test_context();
        context.has_side_effects = true;

        let patterns = matcher.match_patterns(&file, &context);

        // Should still detect patterns even with side effects
        // The confidence might be lower though
        for pattern in patterns {
            let confidence = matcher.score_confidence(&pattern, &context);
            // Side effects should reduce confidence but not eliminate detection
            assert!(confidence > 0.0, "Should still have some confidence");
        }
    }
}
