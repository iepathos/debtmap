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
                let location =
                    self.current_function_location
                        .clone()
                        .unwrap_or(SourceLocation {
                            line: 1,
                            column: None,
                            end_line: None,
                            end_column: None,
                            confidence: LocationConfidence::Unavailable,
                        });

                self.debt_items.push(DebtItem {
                    id: format!(
                        "security-validation-{}-{}",
                        self.path.display(),
                        location.line
                    ),
                    debt_type: DebtType::Security,
                    priority: Priority::High,
                    file: self.path.clone(),
                    line: location.line,
                    column: location.column,
                    message: format!("Missing input validation in function '{}'", func_name),
                    context: Some("External input should be validated before use".to_string()),
                });
            }
        }
    }

    fn is_external_input_source(&self, name: &str) -> bool {
        let input_sources = [
            "request",
            "req",
            "body",
            "params",
            "query",
            "headers",
            "user_input",
            "input",
            "data",
            "payload",
            "form",
            "stdin",
            "args",
            "env",
            "file",
            "socket",
            "stream",
        ];

        let name_lower = name.to_lowercase();
        input_sources
            .iter()
            .any(|&source| name_lower.contains(source))
    }

    fn is_validation_method(&self, name: &str) -> bool {
        let validation_methods = [
            "validate", "verify", "check", "sanitize", "escape", "is_valid", "parse", "try_from",
            "from_str", "filter", "clean", "strip", "trim",
        ];

        let name_lower = name.to_lowercase();
        validation_methods
            .iter()
            .any(|&method| name_lower.contains(method))
    }
}

impl<'ast> Visit<'ast> for ValidationVisitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        let prev_function = self.current_function.clone();
        let prev_location = self.current_function_location.clone();
        let prev_validation = self.has_validation;
        let prev_input = self.has_external_input;

        self.current_function = Some(i.sig.ident.to_string());

        // Extract actual function location
        self.current_function_location = Some(
            self.location_extractor
                .extract_item_location(&syn::Item::Fn(i.clone())),
        );

        self.has_validation = false;
        self.has_external_input = false;

        // Check function parameters for external input
        for input in &i.sig.inputs {
            if let syn::FnArg::Typed(pat_type) = input {
                if let Pat::Ident(PatIdent { ident, .. }) = &*pat_type.pat {
                    if self.is_external_input_source(&ident.to_string()) {
                        self.has_external_input = true;
                    }
                }
            }
        }

        // Check if function name suggests it handles external input
        if self.is_external_input_source(&i.sig.ident.to_string()) {
            self.has_external_input = true;
        }

        syn::visit::visit_item_fn(self, i);

        // Check after visiting the function body
        self.check_function_validation();

        self.current_function = prev_function;
        self.current_function_location = prev_location;
        self.has_validation = prev_validation;
        self.has_external_input = prev_input;
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        let method_name = i.method.to_string();

        // Check for validation methods
        if self.is_validation_method(&method_name) {
            self.has_validation = true;
        }

        // Check for external input access
        if self.is_external_input_source(&method_name) {
            self.has_external_input = true;
        }

        // Check for dangerous operations without validation
        let dangerous_ops = ["execute", "eval", "system", "command", "shell"];
        if dangerous_ops.contains(&method_name.as_str()) && !self.has_validation {
            self.debt_items.push(DebtItem {
                id: format!("security-dangerous-{}-{}", self.path.display(), 0),
                debt_type: DebtType::Security,
                priority: Priority::Critical,
                file: self.path.clone(),
                line: 0,
                column: None,
                message: format!(
                    "Critical: Dangerous operation '{}' without validation",
                    method_name
                ),
                context: Some(
                    "Validate and sanitize all inputs before dangerous operations".to_string(),
                ),
            });
        }

        syn::visit::visit_expr_method_call(self, i);
    }

    fn visit_block(&mut self, i: &'ast Block) {
        // Check for validation patterns in blocks
        let block_str = quote::quote!(#i).to_string();
        if self.is_validation_method(&block_str) {
            self.has_validation = true;
        }

        syn::visit::visit_block(self, i);
    }
}
