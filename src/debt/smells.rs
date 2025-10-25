use crate::core::{DebtItem, DebtType, FunctionMetrics, Priority};
use std::path::{Path, PathBuf};

/// Represents different types of code smells
#[derive(Debug, Clone, PartialEq)]
pub enum SmellType {
    LongParameterList,
    LargeClass,
    LongMethod,
    FeatureEnvy,
    DataClump,
    DeepNesting,
    DuplicateCode,
}

/// A detected code smell with its location and details
#[derive(Debug, Clone)]
pub struct CodeSmell {
    pub smell_type: SmellType,
    pub location: PathBuf,
    pub line: usize,
    pub message: String,
    pub severity: Priority,
}

impl CodeSmell {
    /// Convert a code smell to a debt item
    pub fn to_debt_item(&self) -> DebtItem {
        DebtItem {
            id: format!(
                "smell-{:?}-{}-{}",
                self.smell_type,
                self.location.display(),
                self.line
            ),
            debt_type: DebtType::CodeSmell,
            priority: self.severity,
            file: self.location.clone(),
            line: self.line,
            column: None,
            message: self.message.clone(),
            context: None,
        }
    }
}

/// Detect long parameter lists in functions
pub fn detect_long_parameter_list(func: &FunctionMetrics, param_count: usize) -> Option<CodeSmell> {
    const THRESHOLD: usize = 5;

    if param_count > THRESHOLD {
        Some(CodeSmell {
            smell_type: SmellType::LongParameterList,
            location: func.file.clone(),
            line: func.line,
            message: format!(
                "Function '{}' has {} parameters (threshold: {})",
                func.name, param_count, THRESHOLD
            ),
            severity: if param_count > THRESHOLD * 2 {
                Priority::High
            } else {
                Priority::Medium
            },
        })
    } else {
        None
    }
}

/// Detect large classes/modules based on line count
pub fn detect_large_module(path: &Path, line_count: usize) -> Option<CodeSmell> {
    const THRESHOLD: usize = 300;

    if line_count > THRESHOLD {
        Some(CodeSmell {
            smell_type: SmellType::LargeClass,
            location: path.to_path_buf(),
            line: 1,
            message: format!("Module has {line_count} lines (threshold: {THRESHOLD})"),
            severity: if line_count > THRESHOLD * 2 {
                Priority::High
            } else {
                Priority::Medium
            },
        })
    } else {
        None
    }
}

/// Detect long methods/functions
pub fn detect_long_method(func: &FunctionMetrics) -> Option<CodeSmell> {
    const THRESHOLD: usize = 50;

    if func.length > THRESHOLD {
        Some(CodeSmell {
            smell_type: SmellType::LongMethod,
            location: func.file.clone(),
            line: func.line,
            message: format!(
                "Function '{}' has {} lines (threshold: {})",
                func.name, func.length, THRESHOLD
            ),
            severity: if func.length > THRESHOLD * 2 {
                Priority::High
            } else {
                Priority::Medium
            },
        })
    } else {
        None
    }
}

/// Detect deep nesting in functions
pub fn detect_deep_nesting(func: &FunctionMetrics) -> Option<CodeSmell> {
    const THRESHOLD: u32 = 4;

    if func.nesting > THRESHOLD {
        Some(CodeSmell {
            smell_type: SmellType::DeepNesting,
            location: func.file.clone(),
            line: func.line,
            message: format!(
                "Function '{}' has nesting depth of {} (threshold: {})",
                func.name, func.nesting, THRESHOLD
            ),
            severity: if func.nesting > THRESHOLD * 2 {
                Priority::High
            } else {
                Priority::Medium
            },
        })
    } else {
        None
    }
}

/// Analyze a function for all code smells
pub fn analyze_function_smells(func: &FunctionMetrics, param_count: usize) -> Vec<CodeSmell> {
    let mut smells = Vec::new();

    if let Some(smell) = detect_long_parameter_list(func, param_count) {
        smells.push(smell);
    }

    if let Some(smell) = detect_long_method(func) {
        smells.push(smell);
    }

    if let Some(smell) = detect_deep_nesting(func) {
        smells.push(smell);
    }

    smells
}

/// Analyze a file for module-level code smells
pub fn analyze_module_smells(path: &Path, line_count: usize) -> Vec<CodeSmell> {
    let mut smells = Vec::new();

    if let Some(smell) = detect_large_module(path, line_count) {
        smells.push(smell);
    }

    smells
}

/// Detect feature envy - methods that use other class data more than their own
/// This is a simplified version that looks for method calls on other objects
pub fn detect_feature_envy(content: &str, path: &Path) -> Vec<CodeSmell> {
    let mut smells = Vec::new();
    let mut object_calls: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut self_calls = 0;

    // Count method calls per object
    for line in content.lines() {
        // Count self calls
        self_calls += line.matches("self.").count();

        // Look for pattern: identifier.method_call
        // Simple regex-like pattern matching without regex
        let trimmed = line.trim();
        if let Some(dot_pos) = trimmed.find('.') {
            if dot_pos > 0 {
                let before_dot = &trimmed[..dot_pos];
                let object_name = before_dot.split_whitespace().last().unwrap_or("");

                // Skip if it's self or if it doesn't look like an identifier
                if !object_name.is_empty()
                    && object_name != "self"
                    && object_name
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_alphabetic() || c == '_')
                    && !object_name.contains('(')
                    && !object_name.contains('"')
                    && !object_name.contains('\'')
                {
                    *object_calls.entry(object_name.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    // Check if any object is used more than self
    for (object, count) in object_calls {
        if count >= 3 && count > self_calls {
            smells.push(CodeSmell {
                smell_type: SmellType::FeatureEnvy,
                location: path.to_path_buf(),
                line: 1, // We don't track specific lines in this simple implementation
                message: format!(
                    "Possible feature envy: {} calls to '{}' vs {} self calls",
                    count, object, self_calls
                ),
                severity: if count > 5 {
                    Priority::Medium
                } else {
                    Priority::Low
                },
            });
        }
    }

    smells
}

/// Detect data clumps - groups of parameters that often appear together
pub fn detect_data_clumps(functions: &[FunctionMetrics]) -> Vec<CodeSmell> {
    let mut smells = Vec::new();

    // This is a simplified implementation
    // In a real implementation, we'd analyze actual parameter names and types
    for i in 0..functions.len() {
        for j in i + 1..functions.len() {
            // If two functions are in the same file and have similar high parameter counts,
            // they might have data clumps
            if functions[i].file == functions[j].file {
                // This is a placeholder - real implementation would compare actual parameters
                if functions[i].length > 30 && functions[j].length > 30 {
                    smells.push(CodeSmell {
                        smell_type: SmellType::DataClump,
                        location: functions[i].file.clone(),
                        line: functions[i].line,
                        message: format!(
                            "Functions '{}' and '{}' may share data clumps",
                            functions[i].name, functions[j].name
                        ),
                        severity: Priority::Low,
                    });
                    break; // Only report once per function
                }
            }
        }
    }

    smells
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FunctionMetrics;
    use std::path::PathBuf;

    #[test]
    fn test_detect_data_clumps_empty_functions() {
        let functions = vec![];
        let smells = detect_data_clumps(&functions);
        assert_eq!(
            smells.len(),
            0,
            "No smells should be detected for empty input"
        );
    }

    #[test]
    fn test_detect_data_clumps_single_function() {
        let functions = vec![FunctionMetrics {
            name: "large_function".to_string(),
            file: PathBuf::from("src/lib.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 10,
            nesting: 2,
            length: 35,
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
        }];
        let smells = detect_data_clumps(&functions);
        assert_eq!(
            smells.len(),
            0,
            "Single function cannot have data clumps with itself"
        );
    }

    #[test]
    fn test_detect_data_clumps_different_files() {
        let functions = vec![
            FunctionMetrics {
                name: "function_a".to_string(),
                file: PathBuf::from("src/module_a.rs"),
                line: 10,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 2,
                length: 35,
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
                name: "function_b".to_string(),
                file: PathBuf::from("src/module_b.rs"),
                line: 20,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 2,
                length: 35,
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
        let smells = detect_data_clumps(&functions);
        assert_eq!(
            smells.len(),
            0,
            "Functions in different files should not be reported as data clumps"
        );
    }

    #[test]
    fn test_detect_data_clumps_same_file_large_functions() {
        let functions = vec![
            FunctionMetrics {
                name: "process_user_data".to_string(),
                file: PathBuf::from("src/user_handler.rs"),
                line: 10,
                cyclomatic: 8,
                cognitive: 15,
                nesting: 3,
                length: 40,
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
                name: "validate_user_data".to_string(),
                file: PathBuf::from("src/user_handler.rs"),
                line: 60,
                cyclomatic: 6,
                cognitive: 12,
                nesting: 2,
                length: 35,
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
        let smells = detect_data_clumps(&functions);
        assert_eq!(
            smells.len(),
            1,
            "Should detect data clump for large functions in same file"
        );

        let smell = &smells[0];
        assert_eq!(smell.smell_type, SmellType::DataClump);
        assert_eq!(smell.location, PathBuf::from("src/user_handler.rs"));
        assert_eq!(smell.line, 10);
        assert!(smell.message.contains("process_user_data"));
        assert!(smell.message.contains("validate_user_data"));
        assert_eq!(smell.severity, Priority::Low);
    }

    #[test]
    fn test_detect_data_clumps_multiple_clumps() {
        let functions = vec![
            FunctionMetrics {
                name: "func_a".to_string(),
                file: PathBuf::from("src/module.rs"),
                line: 10,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 2,
                length: 35,
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
                name: "func_b".to_string(),
                file: PathBuf::from("src/module.rs"),
                line: 50,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 2,
                length: 32,
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
                name: "func_c".to_string(),
                file: PathBuf::from("src/module.rs"),
                line: 90,
                cyclomatic: 5,
                cognitive: 10,
                nesting: 2,
                length: 31,
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
                name: "small_func".to_string(),
                file: PathBuf::from("src/module.rs"),
                line: 130,
                cyclomatic: 2,
                cognitive: 3,
                nesting: 1,
                length: 10,
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
        let smells = detect_data_clumps(&functions);

        // Should detect clumps between func_a & func_b, func_a & func_c
        // But due to break after first match per function, we get 2 smells (one for func_a, one for func_b)
        assert_eq!(smells.len(), 2, "Should detect multiple data clumps");

        // First smell should be between func_a and func_b
        assert_eq!(smells[0].line, 10);
        assert!(smells[0].message.contains("func_a"));
        assert!(smells[0].message.contains("func_b"));

        // Second smell should be between func_b and func_c
        assert_eq!(smells[1].line, 50);
        assert!(smells[1].message.contains("func_b"));
        assert!(smells[1].message.contains("func_c"));
    }

    #[test]
    fn test_detect_long_parameter_list() {
        let func = FunctionMetrics {
            name: "test_func".to_string(),
            file: PathBuf::from("src/test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 10,
            nesting: 2,
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
        };

        // Test with parameter count below threshold
        let smell = detect_long_parameter_list(&func, 3);
        assert!(smell.is_none(), "Should not detect smell for 3 parameters");

        // Test with parameter count at threshold
        let smell = detect_long_parameter_list(&func, 5);
        assert!(smell.is_none(), "Should not detect smell at threshold");

        // Test with parameter count above threshold
        let smell = detect_long_parameter_list(&func, 6);
        assert!(smell.is_some(), "Should detect smell for 6 parameters");
        let smell = smell.unwrap();
        assert_eq!(smell.smell_type, SmellType::LongParameterList);
        assert_eq!(smell.severity, Priority::Medium);
        assert!(smell.message.contains("6 parameters"));

        // Test with parameter count way above threshold (high severity)
        let smell = detect_long_parameter_list(&func, 12);
        assert!(smell.is_some(), "Should detect smell for 12 parameters");
        let smell = smell.unwrap();
        assert_eq!(smell.severity, Priority::High);
    }

    #[test]
    fn test_detect_large_module() {
        let path = PathBuf::from("src/large_module.rs");

        // Test with line count below threshold
        let smell = detect_large_module(&path, 250);
        assert!(smell.is_none(), "Should not detect smell for 250 lines");

        // Test with line count at threshold
        let smell = detect_large_module(&path, 300);
        assert!(smell.is_none(), "Should not detect smell at threshold");

        // Test with line count above threshold
        let smell = detect_large_module(&path, 350);
        assert!(smell.is_some(), "Should detect smell for 350 lines");
        let smell = smell.unwrap();
        assert_eq!(smell.smell_type, SmellType::LargeClass);
        assert_eq!(smell.severity, Priority::Medium);
        assert!(smell.message.contains("350 lines"));

        // Test with line count way above threshold (high severity)
        let smell = detect_large_module(&path, 700);
        assert!(smell.is_some(), "Should detect smell for 700 lines");
        let smell = smell.unwrap();
        assert_eq!(smell.severity, Priority::High);
    }

    #[test]
    fn test_detect_long_method() {
        let func = FunctionMetrics {
            name: "long_func".to_string(),
            file: PathBuf::from("src/test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 10,
            nesting: 2,
            length: 40,
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
        };

        // Test with length below threshold
        let smell = detect_long_method(&func);
        assert!(smell.is_none(), "Should not detect smell for 40 lines");

        // Test with length above threshold
        let mut long_func = func.clone();
        long_func.length = 60;
        let smell = detect_long_method(&long_func);
        assert!(smell.is_some(), "Should detect smell for 60 lines");
        let smell = smell.unwrap();
        assert_eq!(smell.smell_type, SmellType::LongMethod);
        assert_eq!(smell.severity, Priority::Medium);
        assert!(smell.message.contains("60 lines"));

        // Test with length way above threshold (high severity)
        long_func.length = 120;
        let smell = detect_long_method(&long_func);
        assert!(smell.is_some(), "Should detect smell for 120 lines");
        let smell = smell.unwrap();
        assert_eq!(smell.severity, Priority::High);
    }

    #[test]
    fn test_detect_deep_nesting() {
        let func = FunctionMetrics {
            name: "nested_func".to_string(),
            file: PathBuf::from("src/test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 10,
            nesting: 3,
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
        };

        // Test with nesting below threshold
        let smell = detect_deep_nesting(&func);
        assert!(
            smell.is_none(),
            "Should not detect smell for nesting depth 3"
        );

        // Test with nesting at threshold
        let mut nested_func = func.clone();
        nested_func.nesting = 4;
        let smell = detect_deep_nesting(&nested_func);
        assert!(smell.is_none(), "Should not detect smell at threshold");

        // Test with nesting above threshold
        nested_func.nesting = 5;
        let smell = detect_deep_nesting(&nested_func);
        assert!(smell.is_some(), "Should detect smell for nesting depth 5");
        let smell = smell.unwrap();
        assert_eq!(smell.smell_type, SmellType::DeepNesting);
        assert_eq!(smell.severity, Priority::Medium);
        assert!(smell.message.contains("nesting depth of 5"));

        // Test with nesting way above threshold (high severity)
        nested_func.nesting = 10;
        let smell = detect_deep_nesting(&nested_func);
        assert!(smell.is_some(), "Should detect smell for nesting depth 10");
        let smell = smell.unwrap();
        assert_eq!(smell.severity, Priority::High);
    }

    #[test]
    fn test_analyze_function_smells() {
        let func = FunctionMetrics {
            name: "complex_func".to_string(),
            file: PathBuf::from("src/test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 10,
            nesting: 5,
            length: 60,
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
        };

        // Test function with multiple smells
        let smells = analyze_function_smells(&func, 7);
        assert_eq!(smells.len(), 3, "Should detect 3 smells");

        // Verify each smell type is detected
        let smell_types: Vec<SmellType> = smells.iter().map(|s| s.smell_type.clone()).collect();
        assert!(smell_types.contains(&SmellType::LongParameterList));
        assert!(smell_types.contains(&SmellType::LongMethod));
        assert!(smell_types.contains(&SmellType::DeepNesting));

        // Test function with no smells
        let clean_func = FunctionMetrics {
            name: "clean_func".to_string(),
            file: PathBuf::from("src/test.rs"),
            line: 10,
            cyclomatic: 3,
            cognitive: 5,
            nesting: 2,
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
        };

        let smells = analyze_function_smells(&clean_func, 3);
        assert_eq!(smells.len(), 0, "Clean function should have no smells");
    }

    #[test]
    fn test_analyze_module_smells() {
        let path = PathBuf::from("src/module.rs");

        // Test small module with no smells
        let smells = analyze_module_smells(&path, 200);
        assert_eq!(smells.len(), 0, "Small module should have no smells");

        // Test large module
        let smells = analyze_module_smells(&path, 400);
        assert_eq!(smells.len(), 1, "Large module should have 1 smell");
        assert_eq!(smells[0].smell_type, SmellType::LargeClass);

        // Test edge case with exactly threshold
        let smells = analyze_module_smells(&path, 300);
        assert_eq!(smells.len(), 0, "Module at threshold should have no smells");
    }

    #[test]
    fn test_detect_feature_envy() {
        let path = PathBuf::from("src/test.rs");

        // Test with no feature envy
        let content = r#"
            fn process_data(&self) {
                self.validate();
                self.transform();
                self.save();
            }
        "#;
        let smells = detect_feature_envy(content, &path);
        assert_eq!(smells.len(), 0, "Should not detect feature envy");

        // Test with feature envy pattern
        let content = r#"
            fn process_order(&self, customer: &Customer) {
                customer.validate_address();
                customer.check_credit();
                customer.update_status();
                customer.send_notification();
                customer.log_activity();
                self.save();
            }
        "#;
        let smells = detect_feature_envy(content, &path);
        assert!(!smells.is_empty(), "Should detect feature envy");
        assert_eq!(smells[0].smell_type, SmellType::FeatureEnvy);
        assert!(smells[0].message.contains("customer"));

        // Test with multiple objects
        let content = r#"
            fn coordinate(&self, order: &Order, payment: &Payment) {
                order.validate();
                order.calculate_total();
                order.apply_discount();
                payment.process();
                payment.verify();
                payment.record();
            }
        "#;
        let smells = detect_feature_envy(content, &path);
        assert_eq!(
            smells.len(),
            2,
            "Should detect feature envy for both objects"
        );
    }

    #[test]
    fn test_code_smell_to_debt_item() {
        let smell = CodeSmell {
            smell_type: SmellType::LongMethod,
            location: PathBuf::from("src/test.rs"),
            line: 42,
            message: "Test message".to_string(),
            severity: Priority::High,
        };

        let debt_item = smell.to_debt_item();
        assert_eq!(debt_item.debt_type, DebtType::CodeSmell);
        assert_eq!(debt_item.file, PathBuf::from("src/test.rs"));
        assert_eq!(debt_item.line, 42);
        assert_eq!(debt_item.message, "Test message");
        assert_eq!(debt_item.priority, Priority::High);
    }
}
