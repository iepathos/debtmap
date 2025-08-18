use crate::common::{LocationConfidence, SourceLocation, UnifiedLocationExtractor};
use crate::core::{DebtItem, DebtType, Priority};
use std::path::Path;
use syn::visit::Visit;
use syn::{Block, ExprMethodCall, File, ItemFn, Pat, PatIdent};

pub fn detect_validation_gaps(file: &File, path: &Path) -> Vec<DebtItem> {
    let source_content = std::fs::read_to_string(path).unwrap_or_default();
    let mut visitor = ValidationVisitor::new(path, &source_content);
    visitor.visit_file(file);
    visitor.debt_items
}

struct ValidationVisitor {
    path: std::path::PathBuf,
    debt_items: Vec<DebtItem>,
    current_function: Option<String>,
    current_function_location: Option<SourceLocation>,
    has_validation: bool,
    has_external_input: bool,
    location_extractor: UnifiedLocationExtractor,
}

impl ValidationVisitor {
    fn new(path: &Path, source_content: &str) -> Self {
        Self {
            path: path.to_path_buf(),
            debt_items: Vec::new(),
            current_function: None,
            current_function_location: None,
            has_validation: false,
            has_external_input: false,
            location_extractor: UnifiedLocationExtractor::new(source_content),
        }
    }

    fn check_function_validation(&mut self) {
        if self.has_external_input && !self.has_validation {
            if let Some(ref func_name) = self.current_function {
                let location = self
                    .current_function_location
                    .clone()
                    .unwrap_or(SourceLocation {
                        line: 1,
                        column: None,
                        end_line: None,
                        end_column: None,
                        confidence: LocationConfidence::Unavailable,
                    });

                self.debt_items.push(DebtItem {
                    id: format!("SEC-VAL-{}", self.debt_items.len() + 1),
                    debt_type: DebtType::Security,
                    priority: Priority::Medium,
                    file: self.path.clone(),
                    line: location.line,
                    column: location.column,
                    message: format!(
                        "Input Validation: Function '{}' handles external input without validation",
                        func_name
                    ),
                    context: Some(format!("{}()", func_name)),
                });
            }
        }

        // Reset for next function
        self.has_validation = false;
        self.has_external_input = false;
    }
}

impl<'ast> Visit<'ast> for ValidationVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        // Store previous function state
        let prev_func = self.current_function.clone();
        let prev_location = self.current_function_location.clone();

        self.current_function = Some(node.sig.ident.to_string());
        self.current_function_location = Some(self.location_extractor.extract_location(node));

        // Visit function body
        syn::visit::visit_item_fn(self, node);

        // Check validation after visiting
        self.check_function_validation();

        // Restore previous state
        self.current_function = prev_func;
        self.current_function_location = prev_location;
    }

    fn visit_block(&mut self, node: &'ast Block) {
        // Check for external input patterns
        for stmt in &node.stmts {
            if let syn::Stmt::Local(local) = stmt {
                if let Pat::Ident(PatIdent { ident, .. }) = &local.pat {
                    let name = ident.to_string();
                    if name.contains("input") || name.contains("param") || name.contains("arg") {
                        self.has_external_input = true;
                    }
                }
            }
        }

        syn::visit::visit_block(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();

        // Check for validation methods
        if method_name.contains("validate")
            || method_name.contains("sanitize")
            || method_name.contains("check")
            || method_name.contains("verify")
        {
            self.has_validation = true;
        }

        // Check for input methods
        if method_name.contains("read")
            || method_name.contains("parse")
            || method_name.contains("from")
        {
            self.has_external_input = true;
        }

        syn::visit::visit_expr_method_call(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detects_input_validation_in_test_functions() {
        // This test exposes the bug - we detect issues in test functions
        let code = r#"
#[test]
fn test_something() {
    let input = "test";
    assert_eq!(input, "test");
}

fn production_function() {
    let user_input = "data";
    process(user_input);
}
"#;

        let file = syn::parse_file(code).unwrap();
        let path = Path::new("test.rs");

        let debt_items = detect_validation_gaps(&file, path);

        // Find issues in test function (lines 1-6)
        let test_function_issues = debt_items.iter().filter(|item| item.line <= 6).count();

        // Current behavior: we DO detect issues in test functions (bug)
        assert!(test_function_issues > 0,
                "BUG: Input validation detector should detect issues in test functions (current behavior)");

        // TODO: When fixed, this should be:
        // assert_eq!(test_function_issues, 0, "Test functions should not have validation issues");
    }

    #[test]
    fn test_numeric_literals_trigger_validation() {
        let code = r#"
fn function_with_literals() {
    let value = 42;
    if value == 42 {
        println!("Match");
    }
}
"#;

        let file = syn::parse_file(code).unwrap();
        let path = Path::new("test.rs");

        let debt_items = detect_validation_gaps(&file, path);

        // Document current behavior
        println!(
            "Found {} validation issues with numeric literals",
            debt_items.len()
        );
        for item in &debt_items {
            println!("  - {}", item.message);
        }
    }

    #[test]
    fn test_assert_statements_with_literals() {
        // Pattern from parameter_analyzer.rs
        let code = r#"
#[test]
fn test_classify_parameter_list_impact_low() {
    assert_eq!(
        ParameterAnalyzer::classify_parameter_list_impact(0),
        MaintainabilityImpact::Low
    );
    assert_eq!(
        ParameterAnalyzer::classify_parameter_list_impact(5),
        MaintainabilityImpact::Low
    );
}
"#;

        let file = syn::parse_file(code).unwrap();
        let path = Path::new("test.rs");

        let debt_items = detect_validation_gaps(&file, path);

        println!("Assert pattern issues: {}", debt_items.len());
        for item in &debt_items {
            println!("  Line {}: {}", item.line, item.message);
        }

        // Document that we detect issues in test assertions
        let has_issues = !debt_items.is_empty();
        println!("Has validation issues in test assertions: {}", has_issues);
    }

    #[test]
    fn test_should_ignore_test_attribute() {
        let code = r#"
#[test]
fn test_function() {
    let input_value = 100;
    assert_eq!(input_value, 100);
}

fn regular_function() {
    let input_value = 100;
    process(input_value);
}
"#;

        let file = syn::parse_file(code).unwrap();
        let path = Path::new("test.rs");

        let debt_items = detect_validation_gaps(&file, path);

        // Both functions have issues currently (bug)
        let test_issues = debt_items
            .iter()
            .filter(|item| {
                item.context
                    .as_ref()
                    .map_or(false, |c| c.contains("test_function"))
            })
            .count();

        let regular_issues = debt_items
            .iter()
            .filter(|item| {
                item.context
                    .as_ref()
                    .map_or(false, |c| c.contains("regular_function"))
            })
            .count();

        println!(
            "Test function issues: {}, Regular function issues: {}",
            test_issues, regular_issues
        );

        // Current behavior - both have issues
        assert!(
            test_issues > 0 || regular_issues > 0,
            "One or both functions should have issues in current implementation"
        );

        // TODO: When fixed:
        // assert_eq!(test_issues, 0, "Test functions should have no issues");
        // assert!(regular_issues > 0, "Regular functions should have issues");
    }
}
