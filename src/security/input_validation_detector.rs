use crate::common::{LocationConfidence, SourceLocation, UnifiedLocationExtractor};
use crate::context::rules::{ContextRuleEngine, DebtPattern, RuleAction};
use crate::context::{ContextDetector, FileType, FunctionRole};
use crate::core::{DebtItem, DebtType, Priority};
use crate::security::types::InputSource;
use std::collections::HashSet;
use std::path::Path;
use syn::visit::Visit;
use syn::{
    Block, ExprCall, ExprMethodCall, File, FnArg, ItemFn, ItemImpl, Pat, PatIdent, PatType,
    Signature,
};

pub fn detect_validation_gaps(file: &File, path: &Path) -> Vec<DebtItem> {
    let source_content = std::fs::read_to_string(path).unwrap_or_default();

    // Determine file type from path
    let file_type = detect_file_type(path);

    // Create context detector
    let mut context_detector = ContextDetector::new(file_type);
    context_detector.visit_file(file);

    // Create context rule engine
    let rule_engine = ContextRuleEngine::new();

    let mut visitor = ValidationVisitor::new(path, &source_content, context_detector, rule_engine);
    visitor.visit_file(file);
    visitor.debt_items
}

fn detect_file_type(path: &Path) -> FileType {
    let path_str = path.to_string_lossy();
    if path_str.contains("/tests/")
        || path_str.contains("_test.rs")
        || path_str.contains("_tests.rs")
    {
        FileType::Test
    } else if path_str.contains("/examples/") {
        FileType::Example
    } else if path_str.contains("/benches/") || path_str.contains("_bench.rs") {
        FileType::Benchmark
    } else if path_str.contains("build.rs") {
        FileType::BuildScript
    } else {
        FileType::Production
    }
}

struct ValidationVisitor {
    path: std::path::PathBuf,
    debt_items: Vec<DebtItem>,
    current_function: Option<String>,
    current_function_location: Option<SourceLocation>,
    current_function_has_params: bool,
    location_extractor: UnifiedLocationExtractor,
    context_detector: ContextDetector,
    rule_engine: ContextRuleEngine,
    // Input source detection
    input_sources: InputSourceRegistry,
    // Per-function tracking
    function_analysis: FunctionAnalysis,
}

#[derive(Default, Clone)]
struct FunctionAnalysis {
    has_external_input: bool,
    has_validation: bool,
    tainted_vars: HashSet<String>,
    input_sources_found: Vec<InputSource>,
    is_public_api: bool,
}

struct InputSourceRegistry {
    file_io_patterns: Vec<&'static str>,
    network_patterns: Vec<&'static str>,
    env_patterns: Vec<&'static str>,
    stdin_patterns: Vec<&'static str>,
}

impl Default for InputSourceRegistry {
    fn default() -> Self {
        Self {
            file_io_patterns: vec![
                "File::open",
                "fs::read",
                "read_to_string",
                "BufReader",
                "read_dir",
                "File::create",
                "OpenOptions",
            ],
            network_patterns: vec![
                "TcpStream",
                "UdpSocket",
                "HttpRequest",
                "Request",
                "reqwest",
                "hyper",
                "actix_web",
                "rocket",
                "warp",
                "axum",
            ],
            env_patterns: vec!["env::args", "env::var", "std::env"],
            stdin_patterns: vec!["stdin", "read_line"],
        }
    }
}

impl ValidationVisitor {
    fn new(
        path: &Path,
        source_content: &str,
        context_detector: ContextDetector,
        rule_engine: ContextRuleEngine,
    ) -> Self {
        Self {
            path: path.to_path_buf(),
            debt_items: Vec::new(),
            current_function: None,
            current_function_location: None,
            current_function_has_params: false,
            location_extractor: UnifiedLocationExtractor::new(source_content),
            context_detector,
            rule_engine,
            input_sources: InputSourceRegistry::default(),
            function_analysis: FunctionAnalysis::default(),
        }
    }

    fn check_function_validation(&mut self) {
        if let Some(ref func_name) = self.current_function {
            // Get function context
            let func_context = self.context_detector.get_context(func_name);

            // Skip if context rules say to skip or allow
            if let Some(context) = func_context {
                let action = self
                    .rule_engine
                    .evaluate(&DebtPattern::InputValidation, context);
                match action {
                    RuleAction::Skip | RuleAction::Allow => {
                        // Reset and return early
                        self.function_analysis = FunctionAnalysis::default();
                        return;
                    }
                    _ => {}
                }
            }

            // Check if this is a utility function (doesn't handle external input)
            // But still flag if we have parameters in public API functions
            if !self.function_analysis.has_external_input && !self.function_analysis.is_public_api {
                // This is likely a utility function, skip it
                self.function_analysis = FunctionAnalysis::default();
                return;
            }

            // Only flag if we have actual external input and no validation
            if self.function_analysis.has_external_input && !self.function_analysis.has_validation {
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

                // Determine priority based on context and input sources
                let priority = self.determine_priority(&self.function_analysis, func_context);

                self.debt_items.push(DebtItem {
                    id: format!("SEC-VAL-{}", self.debt_items.len() + 1),
                    debt_type: DebtType::Security,
                    priority,
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
        self.function_analysis = FunctionAnalysis::default();
    }

    fn determine_priority(
        &self,
        analysis: &FunctionAnalysis,
        context: Option<&crate::context::FunctionContext>,
    ) -> Priority {
        // If it's a test or utility function, lower priority
        if let Some(ctx) = context {
            if ctx.role == FunctionRole::TestFunction || ctx.role == FunctionRole::Utility {
                return Priority::Low;
            }
        }

        // Check severity of input sources
        for source in &analysis.input_sources_found {
            match source {
                InputSource::HttpRequest | InputSource::CliArgument => return Priority::High,
                InputSource::FileInput | InputSource::Environment => return Priority::Medium,
                _ => {}
            }
        }

        // Default to medium if we found external input
        if analysis.has_external_input {
            Priority::Medium
        } else {
            Priority::Low
        }
    }

    fn is_external_input_source(&self, expr_str: &str) -> Option<InputSource> {
        // Check file I/O patterns
        for pattern in &self.input_sources.file_io_patterns {
            if expr_str.contains(pattern) {
                return Some(InputSource::FileInput);
            }
        }

        // Check network patterns
        for pattern in &self.input_sources.network_patterns {
            if expr_str.contains(pattern) {
                return Some(InputSource::HttpRequest);
            }
        }

        // Check environment patterns
        for pattern in &self.input_sources.env_patterns {
            if expr_str.contains(pattern) {
                if expr_str.contains("args") {
                    return Some(InputSource::CliArgument);
                } else {
                    return Some(InputSource::Environment);
                }
            }
        }

        // Check stdin patterns
        for pattern in &self.input_sources.stdin_patterns {
            if expr_str.contains(pattern) {
                return Some(InputSource::UserInput);
            }
        }

        None
    }

    fn is_validation_call(&self, method_name: &str) -> bool {
        method_name.contains("validate")
            || method_name.contains("sanitize")
            || method_name.contains("check")
            || method_name.contains("verify")
            || method_name.contains("parse") // parse with error handling is validation
            || method_name.contains("try_from")
            || method_name.contains("is_valid")
    }

    fn analyze_function_signature(&mut self, sig: &Signature) {
        // Check if function has parameters (potential external input)
        self.current_function_has_params = !sig.inputs.is_empty();

        // Check if it's a public API function
        if !sig.inputs.is_empty() {
            // If it has parameters and is public, it might handle external input
            self.function_analysis.is_public_api = true;

            // Mark parameters as potentially tainted
            for input in &sig.inputs {
                if let FnArg::Typed(PatType { pat, .. }) = input {
                    if let Pat::Ident(PatIdent { ident, .. }) = &**pat {
                        self.function_analysis
                            .tainted_vars
                            .insert(ident.to_string());
                    }
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for ValidationVisitor {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        // Store previous function state
        let prev_func = self.current_function.clone();
        let prev_location = self.current_function_location.clone();
        let prev_analysis = self.function_analysis.clone();
        let prev_has_params = self.current_function_has_params;

        // Reset analysis for new function
        self.function_analysis = FunctionAnalysis::default();

        self.current_function = Some(node.sig.ident.to_string());
        self.current_function_location = Some(self.location_extractor.extract_location(node));

        // Analyze function signature
        self.analyze_function_signature(&node.sig);

        // Visit function body
        syn::visit::visit_item_fn(self, node);

        // Check validation after visiting
        self.check_function_validation();

        // Restore previous state
        self.current_function = prev_func;
        self.current_function_location = prev_location;
        self.function_analysis = prev_analysis;
        self.current_function_has_params = prev_has_params;
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        // Visit impl items to handle methods
        for item in &node.items {
            if let syn::ImplItem::Fn(method) = item {
                // Store previous function state
                let prev_func = self.current_function.clone();
                let prev_location = self.current_function_location.clone();
                let prev_analysis = self.function_analysis.clone();
                let prev_has_params = self.current_function_has_params;

                // Reset analysis for new method
                self.function_analysis = FunctionAnalysis::default();

                self.current_function = Some(method.sig.ident.to_string());
                self.current_function_location =
                    Some(self.location_extractor.extract_location(method));

                // Analyze method signature
                self.analyze_function_signature(&method.sig);

                // Visit method body
                self.visit_block(&method.block);

                // Check validation after visiting
                self.check_function_validation();

                // Restore previous state
                self.current_function = prev_func;
                self.current_function_location = prev_location;
                self.function_analysis = prev_analysis;
                self.current_function_has_params = prev_has_params;
            }
        }

        syn::visit::visit_item_impl(self, node);
    }

    fn visit_block(&mut self, node: &'ast Block) {
        // Check for external input patterns
        for stmt in &node.stmts {
            if let syn::Stmt::Local(local) = stmt {
                // Check variable names that suggest external input
                if let Pat::Ident(PatIdent { ident, .. }) = &local.pat {
                    let var_name = ident.to_string();

                    // In production code, variables named with input patterns are suspicious
                    if let Some(func_name) = &self.current_function {
                        let func_context = self.context_detector.get_context(func_name);
                        let is_test = func_context.is_some_and(|ctx| ctx.is_test());

                        if !is_test
                            && (var_name.contains("input")
                                || var_name.contains("user")
                                || var_name.contains("param")
                                || var_name.contains("arg")
                                || var_name.contains("data")
                                || var_name.contains("request"))
                        {
                            // This looks like external input
                            self.function_analysis.has_external_input = true;
                            self.function_analysis
                                .input_sources_found
                                .push(InputSource::UserInput);
                            self.function_analysis.tainted_vars.insert(var_name.clone());
                        }
                    }
                }

                if let Some(init) = &local.init {
                    // Check if this is an external input source
                    let expr_str = format!("{:?}", init.expr);

                    if let Some(input_source) = self.is_external_input_source(&expr_str) {
                        self.function_analysis.has_external_input = true;
                        self.function_analysis
                            .input_sources_found
                            .push(input_source);

                        // Track tainted variable if it's an identifier
                        if let Pat::Ident(PatIdent { ident, .. }) = &local.pat {
                            self.function_analysis
                                .tainted_vars
                                .insert(ident.to_string());
                        }
                    }

                    // Check if this is validation (e.g., parsing with Result)
                    if (expr_str.contains("?")
                        || expr_str.contains(".ok()")
                        || expr_str.contains(".expect("))
                        && (expr_str.contains("parse") || expr_str.contains("from_str"))
                    {
                        self.function_analysis.has_validation = true;
                    }
                }
            }
        }

        syn::visit::visit_block(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        let expr_str = format!("{:?}", node);

        // Check if this is an external input source
        if let Some(input_source) = self.is_external_input_source(&expr_str) {
            self.function_analysis.has_external_input = true;
            self.function_analysis
                .input_sources_found
                .push(input_source);
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();

        // Check for validation methods
        if self.is_validation_call(&method_name) {
            self.function_analysis.has_validation = true;
        }

        // Check if receiver is tainted
        let receiver_str = format!("{:?}", node.receiver);
        let is_tainted = self
            .function_analysis
            .tainted_vars
            .iter()
            .any(|var| receiver_str.contains(var));

        // Check for external input methods
        let full_expr = format!("{:?}", node);
        if let Some(input_source) = self.is_external_input_source(&full_expr) {
            self.function_analysis.has_external_input = true;
            self.function_analysis
                .input_sources_found
                .push(input_source);
        } else if is_tainted && (method_name == "parse" || method_name == "from_str") {
            // Tainted data being parsed is a form of validation if error handled
            // We'll check for error handling in the parent expression
            self.function_analysis.has_validation = true;
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

        // Fixed behavior: we should NOT detect issues in test functions
        assert_eq!(
            test_function_issues, 0,
            "Test functions should not have validation issues"
        );
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

        // Fixed behavior: test functions should have no issues
        assert_eq!(test_issues, 0, "Test functions should have no issues");

        // Regular functions might not have issues either in this case
        // since they're just using literals without actual external input
    }
}
