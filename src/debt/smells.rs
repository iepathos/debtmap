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

    // Simple heuristic: count method calls on other objects vs self
    for (line_num, line) in content.lines().enumerate() {
        let other_calls = line.matches('.').count() - line.matches("self.").count();
        let self_calls = line.matches("self.").count();

        if other_calls > 3 && other_calls > self_calls * 2 {
            smells.push(CodeSmell {
                smell_type: SmellType::FeatureEnvy,
                location: path.to_path_buf(),
                line: line_num + 1,
                message: format!(
                    "Line has {other_calls} external method calls vs {self_calls} self calls"
                ),
                severity: Priority::Low,
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
}
