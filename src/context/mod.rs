//! Context-aware detection system for reducing false positives
//!
//! This module provides functionality to classify functions and files by their
//! role and purpose, enabling context-aware debt detection that understands
//! when certain patterns are acceptable vs problematic.

use std::path::Path;

pub mod detector;
pub mod rules;

pub use detector::ContextDetector;
pub use rules::{ContextRule, ContextRuleEngine, RuleAction};

/// Represents the context of a function or code block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionContext {
    /// The role this function plays in the system
    pub role: FunctionRole,
    /// The type of file this function is in
    pub file_type: FileType,
    /// Whether this function is async
    pub is_async: bool,
    /// Any framework patterns detected
    pub framework_pattern: Option<FrameworkPattern>,
    /// The function's name
    pub function_name: Option<String>,
    /// The module path to this function
    pub module_path: Vec<String>,
}

impl Default for FunctionContext {
    fn default() -> Self {
        Self {
            role: FunctionRole::Unknown,
            file_type: FileType::Production,
            is_async: false,
            framework_pattern: None,
            function_name: None,
            module_path: Vec::new(),
        }
    }
}

/// The role a function plays in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionRole {
    /// Main entry point function
    Main,
    /// Configuration loading function
    ConfigLoader,
    /// Test function
    TestFunction,
    /// Web/CLI handler function
    Handler,
    /// Initialization/setup function
    Initialization,
    /// Utility/helper function
    Utility,
    /// Build script function
    BuildScript,
    /// Example/documentation code
    Example,
    /// Unknown role
    Unknown,
}

/// The type of file being analyzed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Production code
    Production,
    /// Test file
    Test,
    /// Benchmark file
    Benchmark,
    /// Example file
    Example,
    /// Build script
    BuildScript,
    /// Documentation
    Documentation,
    /// Configuration file
    Configuration,
}

/// Framework patterns that affect analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameworkPattern {
    /// Rust's main function
    RustMain,
    /// Python's __main__ block
    PythonMain,
    /// Web framework handler (actix, rocket, etc.)
    WebHandler,
    /// CLI command handler (clap, etc.)
    CliHandler,
    /// Test framework pattern
    TestFramework,
    /// Async runtime entry point
    AsyncRuntime,
    /// Configuration initialization
    ConfigInit,
}

impl FunctionContext {
    /// Creates a new FunctionContext with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method to set the role
    pub fn with_role(mut self, role: FunctionRole) -> Self {
        self.role = role;
        self
    }

    /// Builder method to set the file type
    pub fn with_file_type(mut self, file_type: FileType) -> Self {
        self.file_type = file_type;
        self
    }

    /// Builder method to set async status
    pub fn with_async(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }

    /// Builder method to set framework pattern
    pub fn with_framework_pattern(mut self, pattern: FrameworkPattern) -> Self {
        self.framework_pattern = Some(pattern);
        self
    }

    /// Builder method to set function name
    pub fn with_function_name(mut self, name: String) -> Self {
        self.function_name = Some(name);
        self
    }

    /// Builder method to set module path
    pub fn with_module_path(mut self, path: Vec<String>) -> Self {
        self.module_path = path;
        self
    }

    /// Check if this context represents test code
    pub fn is_test(&self) -> bool {
        self.role == FunctionRole::TestFunction || self.file_type == FileType::Test
    }

    /// Check if this context represents a main entry point
    pub fn is_entry_point(&self) -> bool {
        matches!(
            self.role,
            FunctionRole::Main | FunctionRole::Handler | FunctionRole::Initialization
        ) || matches!(
            self.framework_pattern,
            Some(
                FrameworkPattern::RustMain
                    | FrameworkPattern::PythonMain
                    | FrameworkPattern::AsyncRuntime
            )
        )
    }

    /// Check if this context allows blocking I/O
    pub fn allows_blocking_io(&self) -> bool {
        // Blocking I/O is acceptable in:
        // - Main functions (they set up the async runtime)
        // - Config loaders (usually run at startup)
        // - Test functions (simplicity over performance)
        // - Initialization code
        // - Non-async contexts
        match self.role {
            FunctionRole::Main
            | FunctionRole::ConfigLoader
            | FunctionRole::TestFunction
            | FunctionRole::Initialization => true,
            _ => !self.is_async,
        }
    }

    /// Check if this context should skip security checks
    pub fn skip_security_checks(&self) -> bool {
        // Skip security checks for test code and examples
        matches!(
            self.file_type,
            FileType::Test | FileType::Example | FileType::Documentation
        )
    }

    /// Get the severity adjustment for this context
    pub fn severity_adjustment(&self) -> i32 {
        match (self.role, self.file_type) {
            // Test code gets lower severity
            (FunctionRole::TestFunction, _) | (_, FileType::Test) => -2,
            // Examples and documentation get lower severity
            (_, FileType::Example | FileType::Documentation) => -2,
            // Entry points and handlers get slightly higher severity
            (FunctionRole::Main | FunctionRole::Handler, _) => 1,
            // Default: no adjustment
            _ => 0,
        }
    }
}

/// Detect file type from path
pub fn detect_file_type(path: &Path) -> FileType {
    let path_str = path.to_string_lossy();

    // Check for test files
    if path_str.contains("/tests/")
        || path_str.contains("\\tests\\")
        || path_str.ends_with("_test.rs")
        || path_str.ends_with("_tests.rs")
        || path_str.ends_with("test.py")
        || path_str.ends_with("_test.py")
        || path_str.ends_with(".test.js")
        || path_str.ends_with(".test.ts")
        || path_str.ends_with(".spec.js")
        || path_str.ends_with(".spec.ts")
    {
        return FileType::Test;
    }

    // Check for benchmark files
    if path_str.contains("/benches/")
        || path_str.contains("\\benches\\")
        || path_str.contains("/benchmarks/")
        || path_str.contains("\\benchmarks\\")
        || path_str.ends_with("_bench.rs")
        || path_str.ends_with("_benchmark.rs")
    {
        return FileType::Benchmark;
    }

    // Check for example files
    if path_str.contains("/examples/")
        || path_str.contains("\\examples\\")
        || path_str.ends_with("_example.rs")
        || path_str.ends_with("example.py")
    {
        return FileType::Example;
    }

    // Check for build scripts
    if path_str.ends_with("build.rs") {
        return FileType::BuildScript;
    }

    // Check for documentation
    if path_str.ends_with(".md") || path_str.ends_with(".rst") {
        return FileType::Documentation;
    }

    // Check for configuration
    if path_str.ends_with(".toml")
        || path_str.ends_with(".yaml")
        || path_str.ends_with(".yml")
        || path_str.ends_with(".json")
        || path_str.ends_with(".ini")
        || path_str.ends_with(".cfg")
    {
        return FileType::Configuration;
    }

    // Default to production
    FileType::Production
}

/// Detect function role from name and patterns
pub fn detect_function_role(name: &str, is_test_attr: bool) -> FunctionRole {
    // Test functions
    if is_test_attr || name.starts_with("test_") || name.ends_with("_test") {
        return FunctionRole::TestFunction;
    }

    // Main function
    if name == "main" || name == "__main__" {
        return FunctionRole::Main;
    }

    // Config loaders
    if name.contains("load_config")
        || name.contains("read_config")
        || name.contains("parse_config")
        || name.contains("init_config")
        || name == "configure"
        || name == "setup_configuration"
    {
        return FunctionRole::ConfigLoader;
    }

    // Initialization functions
    if name.starts_with("init_")
        || name.starts_with("setup_")
        || name.starts_with("initialize_")
        || name == "init"
        || name == "setup"
        || name == "initialize"
    {
        return FunctionRole::Initialization;
    }

    // Handler functions
    if name.contains("handle_")
        || name.contains("handler")
        || name.ends_with("_handler")
        || name.starts_with("on_")
        || name.starts_with("process_")
    {
        return FunctionRole::Handler;
    }

    // Utility functions (common patterns)
    if name.starts_with("helper_")
        || name.starts_with("util_")
        || name.contains("_helper")
        || name.contains("_util")
    {
        return FunctionRole::Utility;
    }

    FunctionRole::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(
            detect_file_type(Path::new("/src/tests/foo.rs")),
            FileType::Test
        );
        assert_eq!(
            detect_file_type(Path::new("/src/foo_test.rs")),
            FileType::Test
        );
        assert_eq!(
            detect_file_type(Path::new("/src/foo.test.js")),
            FileType::Test
        );
        assert_eq!(
            detect_file_type(Path::new("/benches/bench.rs")),
            FileType::Benchmark
        );
        assert_eq!(
            detect_file_type(Path::new("/examples/demo.rs")),
            FileType::Example
        );
        assert_eq!(
            detect_file_type(Path::new("build.rs")),
            FileType::BuildScript
        );
        assert_eq!(
            detect_file_type(Path::new("README.md")),
            FileType::Documentation
        );
        assert_eq!(
            detect_file_type(Path::new("config.toml")),
            FileType::Configuration
        );
        assert_eq!(
            detect_file_type(Path::new("/src/main.rs")),
            FileType::Production
        );
    }

    #[test]
    fn test_function_role_detection() {
        assert_eq!(
            detect_function_role("test_something", false),
            FunctionRole::TestFunction
        );
        assert_eq!(
            detect_function_role("something_test", false),
            FunctionRole::TestFunction
        );
        assert_eq!(detect_function_role("main", false), FunctionRole::Main);
        assert_eq!(
            detect_function_role("load_config", false),
            FunctionRole::ConfigLoader
        );
        assert_eq!(
            detect_function_role("init_database", false),
            FunctionRole::Initialization
        );
        assert_eq!(
            detect_function_role("handle_request", false),
            FunctionRole::Handler
        );
        assert_eq!(
            detect_function_role("helper_function", false),
            FunctionRole::Utility
        );
        assert_eq!(
            detect_function_role("some_function", false),
            FunctionRole::Unknown
        );
    }

    #[test]
    fn test_context_methods() {
        let test_context = FunctionContext::new()
            .with_role(FunctionRole::TestFunction)
            .with_file_type(FileType::Test);
        assert!(test_context.is_test());
        assert!(test_context.allows_blocking_io());
        assert!(test_context.skip_security_checks());
        assert_eq!(test_context.severity_adjustment(), -2);

        let main_context = FunctionContext::new()
            .with_role(FunctionRole::Main)
            .with_framework_pattern(FrameworkPattern::RustMain);
        assert!(main_context.is_entry_point());
        assert!(main_context.allows_blocking_io());
        assert!(!main_context.skip_security_checks());
        assert_eq!(main_context.severity_adjustment(), 1);

        let async_handler = FunctionContext::new()
            .with_role(FunctionRole::Handler)
            .with_async(true);
        assert!(async_handler.is_entry_point());
        assert!(!async_handler.allows_blocking_io());
    }
}
