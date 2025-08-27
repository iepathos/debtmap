//! Context-aware detection system for reducing false positives
//!
//! This module provides functionality to classify functions and files by their
//! role and purpose, enabling context-aware debt detection that understands
//! when certain patterns are acceptable vs problematic.

use std::path::Path;

pub mod async_detector;
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
        // - Build scripts (compile-time execution)
        // - Non-async contexts
        match self.role {
            FunctionRole::Main
            | FunctionRole::ConfigLoader
            | FunctionRole::TestFunction
            | FunctionRole::Initialization
            | FunctionRole::BuildScript => true,
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

/// Detect file type from path using pattern matching
pub fn detect_file_type(path: &Path) -> FileType {
    let path_str = path.to_string_lossy();

    // Use pattern matching with guards for cleaner classification
    match () {
        _ if is_test_file(&path_str) => FileType::Test,
        _ if is_benchmark_file(&path_str) => FileType::Benchmark,
        _ if is_example_file(&path_str) => FileType::Example,
        _ if path_str.ends_with("build.rs") => FileType::BuildScript,
        _ if is_documentation_file(&path_str) => FileType::Documentation,
        _ if is_configuration_file(&path_str) => FileType::Configuration,
        _ => FileType::Production,
    }
}

// Pure classification functions for testability
fn is_test_file(path: &str) -> bool {
    const TEST_PATTERNS_DIR: &[&str] = &["tests/", "tests\\"];
    const TEST_PATTERNS_FILE: &[&str] = &[
        "_test.rs",
        "_tests.rs",
        "test.py",
        "_test.py",
        ".test.js",
        ".test.ts",
        ".spec.js",
        ".spec.ts",
    ];

    TEST_PATTERNS_DIR
        .iter()
        .any(|pattern| path.contains(pattern))
        || TEST_PATTERNS_FILE
            .iter()
            .any(|pattern| path.ends_with(pattern))
}

fn is_benchmark_file(path: &str) -> bool {
    const BENCHMARK_PATTERNS_DIR: &[&str] =
        &["benches/", "benches\\", "benchmarks/", "benchmarks\\"];
    const BENCHMARK_PATTERNS_FILE: &[&str] = &["_bench.rs", "_benchmark.rs"];

    BENCHMARK_PATTERNS_DIR
        .iter()
        .any(|pattern| path.contains(pattern))
        || BENCHMARK_PATTERNS_FILE
            .iter()
            .any(|pattern| path.ends_with(pattern))
}

fn is_example_file(path: &str) -> bool {
    const EXAMPLE_PATTERNS_DIR: &[&str] = &["examples/", "examples\\"];
    const EXAMPLE_PATTERNS_FILE: &[&str] = &["_example.rs", "example.py"];

    EXAMPLE_PATTERNS_DIR
        .iter()
        .any(|pattern| path.contains(pattern))
        || EXAMPLE_PATTERNS_FILE
            .iter()
            .any(|pattern| path.ends_with(pattern))
}

fn is_documentation_file(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".rst")
}

fn is_configuration_file(path: &str) -> bool {
    const CONFIG_EXTENSIONS: &[&str] = &[".toml", ".yaml", ".yml", ".json", ".ini", ".cfg"];

    CONFIG_EXTENSIONS.iter().any(|ext| path.ends_with(ext))
}

/// Detect function role from name and patterns
pub fn detect_function_role(name: &str, is_test_attr: bool) -> FunctionRole {
    // Test functions
    if is_test_attr
        || name.starts_with("test_")
        || name.ends_with("_test")
        || name.starts_with("it_")
        || name.starts_with("should_")
    {
        return FunctionRole::TestFunction;
    }

    // Main function (Rust, Python, Java, etc.)
    if name == "main" || name == "__main__" || name == "Main" {
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

    #[test]
    fn test_is_test_file() {
        use super::is_test_file;

        // Test directory patterns
        assert!(is_test_file("tests/module.rs"));
        assert!(is_test_file("/src/tests/module.rs"));
        assert!(is_test_file("C:\\project\\tests\\file.rs"));

        // Test file suffixes for Rust
        assert!(is_test_file("mod_test.rs"));
        assert!(is_test_file("mod_tests.rs"));

        // Test file suffixes for Python
        assert!(is_test_file("test.py"));
        assert!(is_test_file("module_test.py"));

        // Test file suffixes for JavaScript/TypeScript
        assert!(is_test_file("component.test.js"));
        assert!(is_test_file("component.test.ts"));
        assert!(is_test_file("component.spec.js"));
        assert!(is_test_file("component.spec.ts"));

        // Negative cases
        assert!(!is_test_file("src/main.rs"));
        assert!(!is_test_file("lib.rs"));
        assert!(!is_test_file("testing_utils.rs"));
    }

    #[test]
    fn test_is_benchmark_file() {
        use super::is_benchmark_file;

        // Test directory patterns
        assert!(is_benchmark_file("benches/perf.rs"));
        assert!(is_benchmark_file("/src/benches/perf.rs"));
        assert!(is_benchmark_file("C:\\project\\benches\\perf.rs"));
        assert!(is_benchmark_file("benchmarks/perf.rs"));
        assert!(is_benchmark_file("/src/benchmarks/perf.rs"));
        assert!(is_benchmark_file("C:\\project\\benchmarks\\perf.rs"));

        // Test file suffixes
        assert!(is_benchmark_file("perf_bench.rs"));
        assert!(is_benchmark_file("perf_benchmark.rs"));

        // Negative cases
        assert!(!is_benchmark_file("bench.rs"));
        assert!(!is_benchmark_file("src/main.rs"));
        assert!(!is_benchmark_file("benches.toml"));
    }

    #[test]
    fn test_is_example_file() {
        use super::is_example_file;

        // Test directory patterns
        assert!(is_example_file("examples/demo.rs"));
        assert!(is_example_file("/src/examples/demo.rs"));
        assert!(is_example_file("C:\\project\\examples\\demo.rs"));

        // Test file suffixes
        assert!(is_example_file("demo_example.rs"));
        assert!(is_example_file("example.py"));

        // Negative cases
        assert!(!is_example_file("examples.rs"));
        assert!(!is_example_file("src/main.rs"));
        assert!(!is_example_file("example.txt"));
    }

    #[test]
    fn test_is_documentation_file() {
        use super::is_documentation_file;

        assert!(is_documentation_file("README.md"));
        assert!(is_documentation_file("docs/guide.md"));
        assert!(is_documentation_file("api.rst"));
        assert!(is_documentation_file("docs/tutorial.rst"));

        assert!(!is_documentation_file("main.rs"));
        assert!(!is_documentation_file("config.toml"));
    }

    #[test]
    fn test_is_configuration_file() {
        use super::is_configuration_file;

        assert!(is_configuration_file("Cargo.toml"));
        assert!(is_configuration_file("config.yaml"));
        assert!(is_configuration_file("settings.yml"));
        assert!(is_configuration_file("package.json"));
        assert!(is_configuration_file("setup.ini"));
        assert!(is_configuration_file("app.cfg"));

        assert!(!is_configuration_file("main.rs"));
        assert!(!is_configuration_file("README.md"));
        assert!(!is_configuration_file("config.rs"));
    }

    #[test]
    fn test_detect_file_type_comprehensive() {
        use std::path::Path;

        // Test files
        assert_eq!(
            detect_file_type(Path::new("tests/integration.rs")),
            FileType::Test
        );
        assert_eq!(
            detect_file_type(Path::new("module_test.rs")),
            FileType::Test
        );
        assert_eq!(detect_file_type(Path::new("app.test.js")), FileType::Test);
        assert_eq!(detect_file_type(Path::new("app.spec.ts")), FileType::Test);

        // Benchmark files
        assert_eq!(
            detect_file_type(Path::new("benches/perf.rs")),
            FileType::Benchmark
        );
        assert_eq!(
            detect_file_type(Path::new("perf_bench.rs")),
            FileType::Benchmark
        );

        // Example files
        assert_eq!(
            detect_file_type(Path::new("examples/demo.rs")),
            FileType::Example
        );
        assert_eq!(
            detect_file_type(Path::new("demo_example.rs")),
            FileType::Example
        );

        // Build scripts
        assert_eq!(
            detect_file_type(Path::new("build.rs")),
            FileType::BuildScript
        );

        // Documentation
        assert_eq!(
            detect_file_type(Path::new("README.md")),
            FileType::Documentation
        );
        assert_eq!(
            detect_file_type(Path::new("api.rst")),
            FileType::Documentation
        );

        // Configuration
        assert_eq!(
            detect_file_type(Path::new("Cargo.toml")),
            FileType::Configuration
        );
        assert_eq!(
            detect_file_type(Path::new("config.yaml")),
            FileType::Configuration
        );

        // Production (default)
        assert_eq!(
            detect_file_type(Path::new("src/main.rs")),
            FileType::Production
        );
        assert_eq!(detect_file_type(Path::new("lib.rs")), FileType::Production);
        assert_eq!(detect_file_type(Path::new("app.py")), FileType::Production);
    }
}
