//! AST-based context detection for functions and code blocks

use crate::context::{
    detect_function_role, FileType, FrameworkPattern, FunctionContext, FunctionRole,
};
use syn::{
    visit::Visit, Attribute, Block, Expr, ExprCall, ExprMethodCall, ImplItem, ItemFn,
    ItemImpl, Path,
};

/// Detects context information from Rust AST
pub struct ContextDetector {
    /// Current module path being analyzed
    module_path: Vec<String>,
    /// Detected contexts for functions
    pub contexts: Vec<(String, FunctionContext)>,
    /// File type for the current file
    file_type: FileType,
}

impl ContextDetector {
    /// Create a new context detector
    pub fn new(file_type: FileType) -> Self {
        Self {
            module_path: Vec::new(),
            contexts: Vec::new(),
            file_type,
        }
    }

    /// Analyze a function and detect its context
    pub fn analyze_function(&mut self, func: &ItemFn) -> FunctionContext {
        let func_name = func.sig.ident.to_string();

        // Check for test attribute
        let is_test = has_test_attribute(&func.attrs);

        // Detect function role
        let role = detect_function_role(&func_name, is_test);

        // Check if async
        let is_async = func.sig.asyncness.is_some();

        // Detect framework patterns
        let framework_pattern = detect_framework_pattern(&func_name, &func.attrs, &func.block);

        // Build context
        let context = FunctionContext::new()
            .with_role(role)
            .with_file_type(self.file_type)
            .with_async(is_async)
            .with_function_name(func_name.clone())
            .with_module_path(self.module_path.clone());

        let context = if let Some(pattern) = framework_pattern {
            context.with_framework_pattern(pattern)
        } else {
            context
        };

        // Store the context
        self.contexts.push((func_name, context.clone()));

        context
    }

    /// Get context for a function by name
    pub fn get_context(&self, func_name: &str) -> Option<&FunctionContext> {
        self.contexts
            .iter()
            .find(|(name, _)| name == func_name)
            .map(|(_, context)| context)
    }

    /// Detect if a function is a configuration loader based on its implementation
    pub fn detect_config_loader_from_body(block: &Block) -> bool {
        let mut detector = ConfigLoaderDetector::default();
        detector.visit_block(block);
        detector.is_config_loader()
    }
}

/// Visitor to detect configuration loading patterns
#[derive(Default)]
struct ConfigLoaderDetector {
    has_file_read: bool,
    has_env_read: bool,
    has_toml_parse: bool,
    has_json_parse: bool,
    has_config_type: bool,
}

impl ConfigLoaderDetector {
    fn is_config_loader(&self) -> bool {
        // If we see file reading and parsing, it's likely a config loader
        (self.has_file_read || self.has_env_read)
            && (self.has_toml_parse || self.has_json_parse || self.has_config_type)
    }
}

impl<'ast> Visit<'ast> for ConfigLoaderDetector {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            let path_str = path_to_string(&path.path);

            // Check for file operations
            if path_str.contains("read_to_string")
                || path_str.contains("File::open")
                || path_str.contains("fs::read")
            {
                self.has_file_read = true;
            }

            // Check for env operations
            if path_str.contains("env::var") || path_str.contains("std::env") {
                self.has_env_read = true;
            }

            // Check for parsing operations
            if path_str.contains("toml::from_str") || path_str.contains("toml::parse") {
                self.has_toml_parse = true;
            }
            if path_str.contains("serde_json::from_str") || path_str.contains("json::parse") {
                self.has_json_parse = true;
            }
        }

        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();

        // Check for config-related method calls
        if method_name == "parse" || method_name == "from_str" || method_name == "deserialize" {
            // Check if the receiver might be config-related
            if let Expr::Path(path) = &*node.receiver {
                let path_str = path_to_string(&path.path);
                if path_str.contains("config") || path_str.contains("Config") {
                    self.has_config_type = true;
                }
            }
        }

        syn::visit::visit_expr_method_call(self, node);
    }
}

impl<'ast> Visit<'ast> for ContextDetector {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        self.analyze_function(node);
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        // Track impl blocks for context
        if let Some((_, path, _)) = &node.trait_ {
            let trait_name = path_to_string(path);

            // Check for test implementations
            if trait_name.contains("Test") || trait_name.contains("Benchmark") {
                // Mark all methods in this impl as test-related
                for item in &node.items {
                    if let ImplItem::Fn(method) = item {
                        let func_name = method.sig.ident.to_string();
                        let context = FunctionContext::new()
                            .with_role(FunctionRole::TestFunction)
                            .with_file_type(self.file_type)
                            .with_function_name(func_name.clone());
                        self.contexts.push((func_name, context));
                    }
                }
            }
        }

        syn::visit::visit_item_impl(self, node);
    }
}

/// Check if a function has a test attribute
fn has_test_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().segments.iter().any(|segment| {
            let ident = segment.ident.to_string();
            ident == "test" || ident == "tokio_test" || ident == "async_std_test"
        })
    })
}

/// Detect framework patterns from function attributes and body
fn detect_framework_pattern(
    name: &str,
    attrs: &[Attribute],
    block: &Block,
) -> Option<FrameworkPattern> {
    // Check for Rust main
    if name == "main" {
        // Check if it sets up an async runtime
        if block_contains_async_runtime(block) {
            return Some(FrameworkPattern::AsyncRuntime);
        }
        return Some(FrameworkPattern::RustMain);
    }

    // Check for web handler attributes
    for attr in attrs {
        let attr_str = attr
            .path()
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        if attr_str.contains("get")
            || attr_str.contains("post")
            || attr_str.contains("put")
            || attr_str.contains("delete")
            || attr_str.contains("route")
            || attr_str.contains("handler")
        {
            return Some(FrameworkPattern::WebHandler);
        }
    }

    // Check for CLI handler patterns
    if name.contains("command") || name.contains("cmd") || name.contains("cli") {
        return Some(FrameworkPattern::CliHandler);
    }

    // Check for config initialization
    if ContextDetector::detect_config_loader_from_body(block) {
        return Some(FrameworkPattern::ConfigInit);
    }

    // Check for test framework
    if has_test_attribute(attrs) {
        return Some(FrameworkPattern::TestFramework);
    }

    None
}

/// Check if a block contains async runtime setup
fn block_contains_async_runtime(block: &Block) -> bool {
    let mut detector = AsyncRuntimeDetector::default();
    detector.visit_block(block);
    detector.has_runtime
}

/// Visitor to detect async runtime setup
#[derive(Default)]
struct AsyncRuntimeDetector {
    has_runtime: bool,
}

impl<'ast> Visit<'ast> for AsyncRuntimeDetector {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method = node.method.to_string();
        if method == "block_on" || method == "spawn" || method == "spawn_blocking" {
            self.has_runtime = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(path) = &*node.func {
            let path_str = path_to_string(&path.path);
            if path_str.contains("tokio::runtime")
                || path_str.contains("async_std::task")
                || path_str.contains("Runtime::new")
            {
                self.has_runtime = true;
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

/// Convert a path to a string representation
fn path_to_string(path: &Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn;

    #[test]
    fn test_context_detection() {
        let code = r#"
            #[test]
            fn test_something() {
                assert_eq!(1, 1);
            }
            
            fn main() {
                println!("Hello");
            }
            
            async fn handle_request() {
                // handler code
            }
            
            fn load_config() -> Config {
                let content = fs::read_to_string("config.toml")?;
                toml::from_str(&content)?
            }
        "#;

        let file = syn::parse_file(code).unwrap();
        let mut detector = ContextDetector::new(FileType::Production);

        for item in file.items {
            if let syn::Item::Fn(func) = item {
                detector.visit_item_fn(&func);
            }
        }

        // Check detected contexts
        assert_eq!(detector.contexts.len(), 4);

        let test_ctx = detector.get_context("test_something").unwrap();
        assert_eq!(test_ctx.role, FunctionRole::TestFunction);
        assert!(test_ctx.is_test());

        let main_ctx = detector.get_context("main").unwrap();
        assert_eq!(main_ctx.role, FunctionRole::Main);
        assert!(main_ctx.is_entry_point());

        let handler_ctx = detector.get_context("handle_request").unwrap();
        assert!(handler_ctx.is_async);
        assert_eq!(handler_ctx.role, FunctionRole::Handler);

        let config_ctx = detector.get_context("load_config").unwrap();
        assert_eq!(config_ctx.role, FunctionRole::ConfigLoader);
        assert!(config_ctx.allows_blocking_io());
    }
}
