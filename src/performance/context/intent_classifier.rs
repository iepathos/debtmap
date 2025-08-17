use super::FunctionIntent;
use crate::priority::call_graph::CallGraph;
use syn::{visit::Visit, Expr, ItemFn};

pub struct IntentClassifier {
    setup_patterns: Vec<String>,
    teardown_patterns: Vec<String>,
    business_logic_indicators: Vec<String>,
    io_wrapper_patterns: Vec<String>,
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentClassifier {
    pub fn new() -> Self {
        Self {
            setup_patterns: vec![
                "setup".to_string(),
                "before".to_string(),
                "init".to_string(),
                "initialize".to_string(),
                "create".to_string(),
                "prepare".to_string(),
                "arrange".to_string(),
                "given".to_string(),
                "fixture".to_string(),
                "mock".to_string(),
                "stub".to_string(),
                "build".to_string(),
                "construct".to_string(),
                "configure".to_string(),
            ],
            teardown_patterns: vec![
                "teardown".to_string(),
                "cleanup".to_string(),
                "clean".to_string(),
                "after".to_string(),
                "destroy".to_string(),
                "dispose".to_string(),
                "finalize".to_string(),
                "reset".to_string(),
                "clear".to_string(),
                "remove".to_string(),
                "delete".to_string(),
                "drop".to_string(),
            ],
            business_logic_indicators: vec![
                "process".to_string(),
                "calculate".to_string(),
                "compute".to_string(),
                "analyze".to_string(),
                "transform".to_string(),
                "convert".to_string(),
                "execute".to_string(),
                "handle".to_string(),
            ],
            io_wrapper_patterns: vec![
                "read".to_string(),
                "write".to_string(),
                "load".to_string(),
                "save".to_string(),
                "fetch".to_string(),
                "store".to_string(),
                "get".to_string(),
                "put".to_string(),
            ],
        }
    }

    pub fn classify_function_intent(
        &self,
        function: &ItemFn,
        _call_graph: Option<&CallGraph>,
    ) -> FunctionIntent {
        let function_name = function.sig.ident.to_string().to_lowercase();

        // Check setup/teardown patterns
        if self.is_setup_function(&function_name) {
            return FunctionIntent::Setup;
        }

        if self.is_teardown_function(&function_name) {
            return FunctionIntent::Teardown;
        }

        // Analyze function body and call patterns
        let body_analysis = self.analyze_function_body(function);

        // Check for validation patterns
        if self.is_validation_function(&function_name, &body_analysis) {
            return FunctionIntent::Validation;
        }

        // Check for I/O wrapper patterns
        if self.is_io_wrapper(&function_name, &body_analysis) {
            return FunctionIntent::IOWrapper;
        }

        // Check for data transformation patterns
        if self.is_data_transformation(&function_name, &body_analysis) {
            return FunctionIntent::DataTransformation;
        }

        // Check for error handling patterns
        if self.is_error_handling(&function_name, &body_analysis) {
            return FunctionIntent::ErrorHandling;
        }

        // Check for configuration patterns
        if self.is_configuration(&function_name) {
            return FunctionIntent::Configuration;
        }

        // Check for business logic indicators
        if self.is_business_logic(&function_name) {
            return FunctionIntent::BusinessLogic;
        }

        // Default to unknown for ambiguous functions
        FunctionIntent::Unknown
    }

    fn is_setup_function(&self, name: &str) -> bool {
        let setup_keywords = [
            "setup",
            "set_up",
            "before",
            "init",
            "initialize",
            "create",
            "prepare",
            "arrange",
            "given",
            "fixture",
            "mock",
            "stub",
            "build",
            "construct",
            "configure",
        ];

        setup_keywords.iter().any(|keyword| {
            name.contains(keyword)
                || (name.starts_with("test_") && name.contains(keyword))
                || name.starts_with(&format!("{}_", keyword))
                || name.ends_with(&format!("_{}", keyword))
        })
    }

    fn is_teardown_function(&self, name: &str) -> bool {
        let teardown_keywords = [
            "teardown",
            "tear_down",
            "cleanup",
            "clean",
            "after",
            "destroy",
            "dispose",
            "finalize",
            "reset",
            "clear",
            "remove",
            "delete",
            "drop",
        ];

        teardown_keywords
            .iter()
            .any(|keyword| name.contains(keyword))
    }

    fn is_validation_function(&self, name: &str, body_analysis: &BodyAnalysis) -> bool {
        let validation_keywords = [
            "validate", "verify", "check", "assert", "ensure", "is_valid",
        ];

        validation_keywords
            .iter()
            .any(|keyword| name.contains(keyword))
            || body_analysis.has_validation_pattern
    }

    fn is_io_wrapper(&self, name: &str, body_analysis: &BodyAnalysis) -> bool {
        // Function primarily delegates to I/O operations
        let io_keywords = [
            "read", "write", "load", "save", "fetch", "store", "get", "put", "download", "upload",
        ];

        let has_io_name = io_keywords.iter().any(|keyword| name.contains(keyword));
        let io_ratio =
            body_analysis.io_call_count as f64 / (body_analysis.total_call_count.max(1) as f64);

        has_io_name || (io_ratio > 0.7 && body_analysis.complexity < 3.0)
    }

    fn is_data_transformation(&self, name: &str, body_analysis: &BodyAnalysis) -> bool {
        let transform_keywords = [
            "transform",
            "convert",
            "map",
            "filter",
            "reduce",
            "parse",
            "serialize",
            "deserialize",
            "encode",
            "decode",
        ];

        transform_keywords
            .iter()
            .any(|keyword| name.contains(keyword))
            || body_analysis.has_transformation_pattern
    }

    fn is_error_handling(&self, name: &str, body_analysis: &BodyAnalysis) -> bool {
        let error_keywords = [
            "handle_error",
            "on_error",
            "catch",
            "recover",
            "fallback",
            "retry",
        ];

        error_keywords.iter().any(|keyword| name.contains(keyword))
            || body_analysis.has_error_handling_pattern
    }

    fn is_configuration(&self, name: &str) -> bool {
        let config_keywords = [
            "config",
            "configure",
            "setup",
            "init",
            "options",
            "settings",
        ];

        config_keywords.iter().any(|keyword| name.contains(keyword))
    }

    fn is_business_logic(&self, name: &str) -> bool {
        let business_keywords = [
            "process",
            "calculate",
            "compute",
            "analyze",
            "execute",
            "handle",
            "perform",
            "run",
            "apply",
        ];

        business_keywords
            .iter()
            .any(|keyword| name.contains(keyword))
    }

    fn analyze_function_body(&self, function: &ItemFn) -> BodyAnalysis {
        let mut analyzer = BodyAnalyzer::new();
        analyzer.visit_item_fn(function);
        analyzer.analysis
    }
}

#[derive(Debug, Default)]
struct BodyAnalysis {
    io_call_count: usize,
    total_call_count: usize,
    complexity: f64,
    has_validation_pattern: bool,
    has_transformation_pattern: bool,
    has_error_handling_pattern: bool,
}

struct BodyAnalyzer {
    analysis: BodyAnalysis,
    depth: usize,
}

impl BodyAnalyzer {
    fn new() -> Self {
        Self {
            analysis: BodyAnalysis::default(),
            depth: 0,
        }
    }
}

impl<'ast> Visit<'ast> for BodyAnalyzer {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        self.analysis.total_call_count += 1;

        // Check for I/O calls
        if let Expr::Path(path) = &*node.func {
            let path_str = quote::quote!(#path).to_string();
            if path_str.contains("fs::")
                || path_str.contains("File::")
                || path_str.contains("read")
                || path_str.contains("write")
            {
                self.analysis.io_call_count += 1;
            }
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.analysis.total_call_count += 1;

        let method_name = node.method.to_string();
        if method_name.contains("read")
            || method_name.contains("write")
            || method_name == "open"
            || method_name == "create"
        {
            self.analysis.io_call_count += 1;
        }

        // Check for validation patterns
        if method_name.contains("is_") || method_name.contains("has_") || method_name == "validate"
        {
            self.analysis.has_validation_pattern = true;
        }

        // Check for transformation patterns
        if method_name == "map"
            || method_name == "filter"
            || method_name == "fold"
            || method_name == "collect"
        {
            self.analysis.has_transformation_pattern = true;
        }

        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.analysis.complexity += 1.0;
        syn::visit::visit_expr_if(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.analysis.complexity += (node.arms.len() - 1) as f64;

        // Check for error handling patterns
        for arm in &node.arms {
            if let syn::Pat::TupleStruct(pat) = &arm.pat {
                let pat_str = quote::quote!(#pat).to_string();
                if pat_str.contains("Err") || pat_str.contains("Error") {
                    self.analysis.has_error_handling_pattern = true;
                }
            }
        }

        syn::visit::visit_expr_match(self, node);
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.analysis.complexity += 1.0;
        syn::visit::visit_expr_loop(self, node);
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.analysis.complexity += 1.0;
        syn::visit::visit_expr_while(self, node);
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.analysis.complexity += 1.0;
        syn::visit::visit_expr_for_loop(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_intent_classification_setup() {
        let classifier = IntentClassifier::new();

        let function = syn::parse_quote! {
            fn setup_test_environment() {
                // setup code
            }
        };

        let intent = classifier.classify_function_intent(&function, None);
        assert_eq!(intent, FunctionIntent::Setup);
    }

    #[test]
    fn test_function_intent_classification_teardown() {
        let classifier = IntentClassifier::new();

        let function = syn::parse_quote! {
            fn cleanup_resources() {
                // cleanup code
            }
        };

        let intent = classifier.classify_function_intent(&function, None);
        assert_eq!(intent, FunctionIntent::Teardown);
    }

    #[test]
    fn test_function_intent_classification_business_logic() {
        let classifier = IntentClassifier::new();

        let function = syn::parse_quote! {
            fn process_user_request(request: Request) -> Response {
                // business logic
                Response::new()
            }
        };

        let intent = classifier.classify_function_intent(&function, None);
        assert_eq!(intent, FunctionIntent::BusinessLogic);
    }

    #[test]
    fn test_function_intent_classification_io_wrapper() {
        let classifier = IntentClassifier::new();

        let function = syn::parse_quote! {
            fn read_config_file(path: &str) -> Config {
                std::fs::read_to_string(path).unwrap()
            }
        };

        let intent = classifier.classify_function_intent(&function, None);
        assert_eq!(intent, FunctionIntent::IOWrapper);
    }
}
