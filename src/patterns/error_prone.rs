use std::path::Path;

/// Checks for error-prone patterns in code
///
/// This function analyzes code for common error-prone patterns that could
/// lead to bugs or maintenance issues.
pub fn check_error_prone_patterns(content: &str, file_path: &Path) -> Vec<ErrorPronePattern> {
    let mut patterns = Vec::new();

    // Check for unwrap() calls that could panic
    patterns.extend(detect_unsafe_unwraps(content, file_path));

    // Check for expect() with generic messages
    patterns.extend(detect_generic_expects(content, file_path));

    // Check for ignored Results
    patterns.extend(detect_ignored_results(content, file_path));

    // Check for panic! in production code
    patterns.extend(detect_panics(content, file_path));

    patterns
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorPronePattern {
    pub pattern_type: PatternType,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    UnsafeUnwrap,
    GenericExpect,
    IgnoredResult,
    PanicInCode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    High,
    Medium,
    Low,
}

fn detect_unsafe_unwraps(content: &str, _file_path: &Path) -> Vec<ErrorPronePattern> {
    content
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            line.find(".unwrap()").map(|col| ErrorPronePattern {
                pattern_type: PatternType::UnsafeUnwrap,
                line: line_num + 1,
                column: col + 1,
                message: "Consider using ? operator or proper error handling instead of unwrap()"
                    .to_string(),
                severity: Severity::High,
            })
        })
        .collect()
}

fn detect_generic_expects(content: &str, _file_path: &Path) -> Vec<ErrorPronePattern> {
    content
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            if let Some(col) = line.find(".expect(") {
                // Check if the expect message is too generic
                let msg_start = col + 8;
                if let Some(msg_content) = line.get(msg_start..) {
                    if msg_content.starts_with("\"failed\"")
                        || msg_content.starts_with("\"error\"")
                        || msg_content.starts_with("\"should work\"")
                    {
                        return Some(ErrorPronePattern {
                            pattern_type: PatternType::GenericExpect,
                            line: line_num + 1,
                            column: col + 1,
                            message: "Expect message should be more descriptive".to_string(),
                            severity: Severity::Medium,
                        });
                    }
                }
            }
            None
        })
        .collect()
}

fn detect_ignored_results(content: &str, _file_path: &Path) -> Vec<ErrorPronePattern> {
    content
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            // Look for function calls that return Result but aren't handled
            if line.contains("Result<")
                && !line.contains("->")
                && !line.contains("fn ")
                && !line.contains("?")
                && !line.contains(".unwrap")
                && !line.contains(".expect")
            {
                line.find("Result<").map(|col| ErrorPronePattern {
                    pattern_type: PatternType::IgnoredResult,
                    line: line_num + 1,
                    column: col + 1,
                    message: "Result type may be ignored without proper handling".to_string(),
                    severity: Severity::Medium,
                })
            } else {
                None
            }
        })
        .collect()
}

fn detect_panics(content: &str, _file_path: &Path) -> Vec<ErrorPronePattern> {
    content
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            if let Some(col) = line.find("panic!(") {
                // Skip test code
                if !content
                    .lines()
                    .take(line_num)
                    .any(|l| l.contains("#[test]") || l.contains("#[cfg(test)]"))
                {
                    return Some(ErrorPronePattern {
                        pattern_type: PatternType::PanicInCode,
                        line: line_num + 1,
                        column: col + 1,
                        message: "Avoid panic! in production code; return Result instead"
                            .to_string(),
                        severity: Severity::High,
                    });
                }
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_unsafe_unwraps() {
        let content = r#"
fn example() {
    let value = Some(5).unwrap();
    let result = get_result().unwrap();
}
"#;
        let patterns = detect_unsafe_unwraps(content, Path::new("test.rs"));
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].pattern_type, PatternType::UnsafeUnwrap);
        assert_eq!(patterns[0].line, 3);
        assert_eq!(patterns[1].line, 4);
    }

    #[test]
    fn test_detect_generic_expects() {
        let content = r#"
fn example() {
    let value = Some(5).expect("failed");
    let result = get_result().expect("should have specific context");
    let good = something().expect("failed to parse configuration file");
}
"#;
        let patterns = detect_generic_expects(content, Path::new("test.rs"));
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::GenericExpect);
        assert_eq!(patterns[0].line, 3);
    }

    #[test]
    fn test_detect_ignored_results() {
        let content = r#"
fn example() {
    let result: Result<i32, Error> = Ok(5);
    process_data(); // Returns Result but ignored
}

fn process_data() -> Result<(), Error> {
    Ok(())
}
"#;
        let patterns = detect_ignored_results(content, Path::new("test.rs"));
        // Should detect the Result type declaration that might be ignored
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_detect_panics() {
        let content = r#"
fn production_code() {
    if condition {
        panic!("This should not happen");
    }
}

#[test]
fn test_code() {
    panic!("This is OK in tests");
}
"#;
        let patterns = detect_panics(content, Path::new("test.rs"));
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern_type, PatternType::PanicInCode);
        assert_eq!(patterns[0].line, 4);
    }

    #[test]
    fn test_check_error_prone_patterns_integration() {
        let content = r#"
fn risky_function() {
    let value = Some(42).unwrap();
    let data = get_data().expect("error");
    panic!("Something went wrong");
}

fn safe_function() -> Result<(), Error> {
    let value = Some(42)?;
    let data = get_data().expect("Failed to get data from database connection");
    Ok(())
}
"#;
        let file_path = PathBuf::from("test.rs");
        let patterns = check_error_prone_patterns(content, &file_path);

        // Should find: 1 unwrap, 1 generic expect, 1 panic
        assert!(patterns.len() >= 3);

        // Check we have each type of pattern
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::UnsafeUnwrap));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::GenericExpect));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::PanicInCode));
    }

    #[test]
    fn test_severity_levels() {
        let content = r#"
fn example() {
    something.unwrap(); // High severity
    other.expect("failed"); // Medium severity  
}
"#;
        let patterns = check_error_prone_patterns(content, Path::new("test.rs"));

        let unwrap_pattern = patterns
            .iter()
            .find(|p| p.pattern_type == PatternType::UnsafeUnwrap)
            .expect("Should find unwrap pattern");
        assert_eq!(unwrap_pattern.severity, Severity::High);

        let expect_pattern = patterns
            .iter()
            .find(|p| p.pattern_type == PatternType::GenericExpect)
            .expect("Should find expect pattern");
        assert_eq!(expect_pattern.severity, Severity::Medium);
    }

    #[test]
    fn test_line_and_column_accuracy() {
        let content = "    let x = foo.unwrap();";
        let patterns = detect_unsafe_unwraps(content, Path::new("test.rs"));

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].line, 1);
        assert_eq!(patterns[0].column, 16); // Position of .unwrap() (0-indexed + 1)
    }

    #[test]
    fn test_empty_content() {
        let patterns = check_error_prone_patterns("", Path::new("test.rs"));
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_no_error_prone_patterns() {
        let content = r#"
fn safe_function() -> Result<i32, Error> {
    let value = get_value()?;
    match process(value) {
        Ok(result) => Ok(result * 2),
        Err(e) => {
            log::error!("Processing failed: {}", e);
            Err(e)
        }
    }
}
"#;
        let patterns = check_error_prone_patterns(content, Path::new("test.rs"));
        assert!(patterns.is_empty());
    }
}
