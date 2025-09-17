use crate::core::{DebtItem, DebtType, Priority};
use crate::debt::suppression::SuppressionContext;
use std::path::Path;
use syn::visit::Visit;
use syn::{File, ItemFn, ReturnType, Type};

/// Pure function to check for Box<dyn Error> pattern
fn detect_box_dyn_error(type_str: &str) -> Option<(PropagationQuality, &'static str)> {
    if is_box_dyn_error_pattern(type_str) {
        Some((
            PropagationQuality::BoxDynError,
            "Using Box<dyn Error> loses type information",
        ))
    } else {
        None
    }
}

/// Pure function to check for String error type
fn detect_string_error_type(type_str: &str) -> Option<(PropagationQuality, &'static str)> {
    if is_result_with_string_error(type_str) {
        Some((
            PropagationQuality::TypeErasure,
            "Using String as error type loses structure",
        ))
    } else {
        None
    }
}

/// Pure function to check for anyhow without context
fn detect_anyhow_no_context(type_str: &str) -> Option<(PropagationQuality, &'static str)> {
    if is_anyhow_error_type(type_str) {
        Some((
            PropagationQuality::PassthroughNoContext,
            "Consider adding context to anyhow errors",
        ))
    } else {
        None
    }
}

/// Pure predicate for Box<dyn Error> pattern
fn is_box_dyn_error_pattern(type_str: &str) -> bool {
    type_str.contains("Box")
        && type_str.contains("dyn")
        && (type_str.contains("Error") || type_str.contains("std::error::Error"))
}

/// Pure predicate for Result with String error
fn is_result_with_string_error(type_str: &str) -> bool {
    type_str.contains("Result") && (type_str.contains(", String") || type_str.contains(",String"))
}

/// Pure predicate for anyhow error types
fn is_anyhow_error_type(type_str: &str) -> bool {
    type_str.contains("anyhow::Error") || type_str.contains("anyhow :: Error")
}

/// Error type check definition
type ErrorTypeCheck = fn(&str) -> Option<(PropagationQuality, &'static str)>;

/// Array of error type checks using functional composition
const ERROR_TYPE_CHECKS: &[ErrorTypeCheck] = &[
    detect_box_dyn_error,
    detect_string_error_type,
    detect_anyhow_no_context,
];

pub struct ErrorPropagationAnalyzer<'a> {
    items: Vec<DebtItem>,
    current_file: &'a Path,
    suppression: Option<&'a SuppressionContext>,
    in_test_function: bool,
}

impl<'a> ErrorPropagationAnalyzer<'a> {
    pub fn new(file_path: &'a Path, suppression: Option<&'a SuppressionContext>) -> Self {
        Self {
            items: Vec::new(),
            current_file: file_path,
            suppression,
            in_test_function: false,
        }
    }

    pub fn detect(mut self, file: &File) -> Vec<DebtItem> {
        self.visit_file(file);
        self.items
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn add_debt_item(&mut self, line: usize, quality: PropagationQuality, context: &str) {
        // Check if this item is suppressed
        if let Some(checker) = self.suppression {
            if checker.is_suppressed(line, &DebtType::ErrorSwallowing) {
                return;
            }
        }

        let priority = self.determine_priority(&quality);
        let message = format!("{}: {}", quality.description(), quality.remediation());

        self.items.push(DebtItem {
            id: format!("error-propagation-{}-{}", self.current_file.display(), line),
            debt_type: DebtType::ErrorSwallowing,
            priority,
            file: self.current_file.to_path_buf(),
            line,
            column: None,
            message,
            context: Some(context.to_string()),
        });
    }

    fn determine_priority(&self, quality: &PropagationQuality) -> Priority {
        // Lower priority for test code
        if self.in_test_function {
            return Priority::Low;
        }

        match quality {
            PropagationQuality::BoxDynError => Priority::Medium,
            PropagationQuality::OverlyBroadConversion => Priority::Low,
            PropagationQuality::TypeErasure => Priority::Medium,
            PropagationQuality::PassthroughNoContext => Priority::Low,
        }
    }

    fn analyze_return_type(&mut self, return_type: &ReturnType, line: usize) {
        if let ReturnType::Type(_, ty) = return_type {
            self.check_error_type(ty, line);
        }
    }

    fn check_error_type(&mut self, ty: &Type, line: usize) {
        let type_str = quote::quote!(#ty).to_string();

        // Apply all error type checks using functional style
        ERROR_TYPE_CHECKS
            .iter()
            .filter_map(|check| check(&type_str))
            .for_each(|(quality, context)| {
                self.add_debt_item(line, quality, context);
            });
    }
}

impl<'a> Visit<'_> for ErrorPropagationAnalyzer<'a> {
    fn visit_item_fn(&mut self, node: &ItemFn) {
        let was_in_test = self.in_test_function;
        self.in_test_function = node
            .attrs
            .iter()
            .any(|attr| attr.path().get_ident().map(|i| i.to_string()).as_deref() == Some("test"));

        // Analyze the function's return type
        let line = self.get_line_number(node.sig.fn_token.span);
        self.analyze_return_type(&node.sig.output, line);

        syn::visit::visit_item_fn(self, node);
        self.in_test_function = was_in_test;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropagationQuality {
    BoxDynError,
    OverlyBroadConversion,
    TypeErasure,
    PassthroughNoContext,
}

impl PropagationQuality {
    /// Pure method to get base priority for this quality type
    #[allow(dead_code)]
    fn base_priority(self) -> Priority {
        match self {
            Self::BoxDynError => Priority::Medium,
            Self::OverlyBroadConversion => Priority::Low,
            Self::TypeErasure => Priority::Medium,
            Self::PassthroughNoContext => Priority::Low,
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::BoxDynError => "Box<dyn Error> type erasure",
            Self::OverlyBroadConversion => "Overly broad error conversion",
            Self::TypeErasure => "Error type erasure",
            Self::PassthroughNoContext => "Error passthrough without context",
        }
    }

    fn remediation(&self) -> &'static str {
        match self {
            Self::BoxDynError => "Use specific error types or error enums",
            Self::OverlyBroadConversion => "Use more specific error conversions",
            Self::TypeErasure => "Preserve error type information with structured errors",
            Self::PassthroughNoContext => "Add context when propagating errors",
        }
    }
}

pub fn analyze_error_propagation(
    file: &File,
    file_path: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem> {
    let analyzer = ErrorPropagationAnalyzer::new(file_path, suppression);
    analyzer.detect(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn test_box_dyn_error() {
        let code = r#"
            fn example() -> Result<i32, Box<dyn std::error::Error>> {
                Ok(42)
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_propagation(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("Box<dyn Error>"));
    }

    #[test]
    fn test_string_error_type() {
        let code = r#"
            fn example() -> Result<i32, String> {
                Ok(42)
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_propagation(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("type erasure"));
    }

    #[test]
    fn test_anyhow_error() {
        let code = r#"
            fn example() -> Result<i32, anyhow::Error> {
                Ok(42)
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_propagation(&file, Path::new("test.rs"), None);

        assert!(!items.is_empty());
        assert!(items[0].message.contains("context"));
    }

    #[test]
    fn test_specific_error_type() {
        let code = r#"
            #[derive(Debug)]
            enum MyError {
                IoError(std::io::Error),
                ParseError(String),
            }
            
            fn example() -> Result<i32, MyError> {
                Ok(42)
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_propagation(&file, Path::new("test.rs"), None);

        // Should not flag specific error types
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn test_no_issues_in_tests() {
        let code = r#"
            #[test]
            fn test_example() -> Result<(), Box<dyn std::error::Error>> {
                Ok(())
            }
        "#;

        let file = parse_str::<File>(code).expect("Failed to parse test code");
        let items = analyze_error_propagation(&file, Path::new("test.rs"), None);

        // Should detect but with low priority
        assert!(!items.is_empty());
        assert_eq!(items[0].priority, Priority::Low);
    }
}
