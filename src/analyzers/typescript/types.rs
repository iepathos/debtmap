//! TypeScript/JavaScript specific types
//!
//! Core data structures for JS/TS analysis.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Kind of JavaScript/TypeScript function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionKind {
    /// Regular function declaration: `function foo() {}`
    Declaration,
    /// Function expression: `const foo = function() {}`
    Expression,
    /// Arrow function: `const foo = () => {}`
    Arrow,
    /// Object method: `{ foo() {} }`
    Method,
    /// Class method: `class C { foo() {} }`
    ClassMethod,
    /// Constructor: `class C { constructor() {} }`
    Constructor,
    /// Getter: `get foo() {}`
    Getter,
    /// Setter: `set foo(v) {}`
    Setter,
    /// Generator function: `function* foo() {}`
    Generator,
    /// Async function: `async function foo() {}`
    Async,
    /// Async generator: `async function* foo() {}`
    AsyncGenerator,
}

impl FunctionKind {
    /// Check if this function kind is async
    pub fn is_async(&self) -> bool {
        matches!(self, FunctionKind::Async | FunctionKind::AsyncGenerator)
    }

    /// Check if this function kind is a generator
    pub fn is_generator(&self) -> bool {
        matches!(self, FunctionKind::Generator | FunctionKind::AsyncGenerator)
    }
}

/// Async pattern detected in JavaScript/TypeScript code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AsyncPattern {
    /// Async/await usage
    AsyncAwait {
        await_count: u32,
        nested_await_depth: u32,
    },
    /// Promise chain (.then/.catch/.finally)
    PromiseChain {
        chain_length: u32,
        has_catch: bool,
        has_finally: bool,
    },
    /// Promise.all/allSettled/race/any
    PromiseAll { promise_count: u32, method: String },
    /// Callback nesting (callback hell)
    CallbackNesting { depth: u32, callback_count: u32 },
}

/// JavaScript/TypeScript specific debt patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JsDebtPattern {
    /// TypeScript `any` type usage
    AnyTypeUsage { count: u32, locations: Vec<usize> },
    /// Type assertion usage (as, <Type>)
    TypeAssertion { count: u32, locations: Vec<usize> },
    /// Non-null assertion (!)
    NonNullAssertion { count: u32, locations: Vec<usize> },
    /// Callback hell (deeply nested callbacks)
    CallbackHell { depth: u32, line: usize },
    /// Unhandled promise rejection (no .catch or try/catch)
    UnhandledPromise { line: usize },
    /// Long promise chain
    LongPromiseChain { length: u32, line: usize },
    /// Console.log in production code
    ConsoleUsage { count: u32, locations: Vec<usize> },
    /// TODO/FIXME comments
    TodoComment { message: String, line: usize },
}

/// Function metrics specific to JavaScript/TypeScript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsFunctionMetrics {
    /// Function name (may be empty for anonymous functions)
    pub name: String,
    /// File path
    pub file: PathBuf,
    /// Line number
    pub line: usize,
    /// Kind of function
    pub kind: FunctionKind,
    /// Cyclomatic complexity
    pub cyclomatic: u32,
    /// Cognitive complexity
    pub cognitive: u32,
    /// Maximum nesting depth
    pub nesting: u32,
    /// Lines of code
    pub length: usize,
    /// Is this a test function
    pub is_test: bool,
    /// Is this an async function
    pub is_async: bool,
    /// Detected async patterns
    pub async_patterns: Vec<AsyncPattern>,
    /// Parameter count
    pub parameter_count: u32,
    /// Is this function exported
    pub is_exported: bool,
    /// TypeScript-specific patterns
    pub ts_patterns: Option<TypeScriptPatternResult>,
}

impl JsFunctionMetrics {
    /// Create a new JsFunctionMetrics
    pub fn new(name: String, file: PathBuf, line: usize, kind: FunctionKind) -> Self {
        Self {
            name,
            file,
            line,
            kind,
            cyclomatic: 1, // Base complexity
            cognitive: 0,
            nesting: 0,
            length: 0,
            is_test: false,
            is_async: false,
            async_patterns: Vec::new(),
            parameter_count: 0,
            is_exported: false,
            ts_patterns: None,
        }
    }

    /// Check if function complexity exceeds threshold
    pub fn is_complex(&self, threshold: u32) -> bool {
        self.cyclomatic > threshold || self.cognitive > threshold
    }
}

/// TypeScript-specific pattern detection results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeScriptPatternResult {
    /// Count of `any` type usage
    pub any_count: u32,
    /// Count of type assertions
    pub assertion_count: u32,
    /// Count of non-null assertions
    pub non_null_assertion_count: u32,
    /// Whether using strict null checks
    pub has_strict_null: bool,
    /// Generic type parameters used
    pub generic_count: u32,
}

impl TypeScriptPatternResult {
    pub fn new() -> Self {
        Self {
            any_count: 0,
            assertion_count: 0,
            non_null_assertion_count: 0,
            has_strict_null: true,
            generic_count: 0,
        }
    }
}

impl Default for TypeScriptPatternResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for function analysis
#[derive(Debug, Clone)]
pub struct FunctionContext {
    /// Function name
    pub name: String,
    /// File path
    pub file: PathBuf,
    /// Line number
    pub line: usize,
    /// Function kind
    pub kind: FunctionKind,
    /// Is this function inside a class
    pub in_class: bool,
    /// Class name if inside a class
    pub class_name: Option<String>,
    /// Is this function exported
    pub is_exported: bool,
    /// Is this inside a test file or describe/it block
    pub in_test_context: bool,
}

/// Functional chain pattern (map/filter/reduce)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalChain {
    /// Methods in the chain (e.g., ["map", "filter", "reduce"])
    pub methods: Vec<String>,
    /// Total chain length
    pub length: u32,
    /// Starting line of the chain
    pub line: usize,
    /// Whether all callbacks appear pure
    pub appears_pure: bool,
}

/// Import/export dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsDependency {
    /// Module specifier (e.g., "lodash", "./utils", "@scope/pkg")
    pub specifier: String,
    /// Kind of import/export
    pub kind: JsDependencyKind,
    /// Imported/exported names
    pub names: Vec<String>,
    /// Line number
    pub line: usize,
}

/// Kind of JavaScript dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsDependencyKind {
    /// ES module import
    Import,
    /// CommonJS require
    Require,
    /// Dynamic import()
    DynamicImport,
    /// ES module export
    Export,
    /// Re-export
    ReExport,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_kind_is_async() {
        assert!(FunctionKind::Async.is_async());
        assert!(FunctionKind::AsyncGenerator.is_async());
        assert!(!FunctionKind::Declaration.is_async());
        assert!(!FunctionKind::Arrow.is_async());
    }

    #[test]
    fn test_function_kind_is_generator() {
        assert!(FunctionKind::Generator.is_generator());
        assert!(FunctionKind::AsyncGenerator.is_generator());
        assert!(!FunctionKind::Async.is_generator());
        assert!(!FunctionKind::Declaration.is_generator());
    }

    #[test]
    fn test_js_function_metrics_new() {
        let metrics = JsFunctionMetrics::new(
            "test".to_string(),
            PathBuf::from("test.js"),
            1,
            FunctionKind::Declaration,
        );
        assert_eq!(metrics.name, "test");
        assert_eq!(metrics.cyclomatic, 1);
        assert_eq!(metrics.cognitive, 0);
    }

    #[test]
    fn test_js_function_metrics_is_complex() {
        let mut metrics = JsFunctionMetrics::new(
            "test".to_string(),
            PathBuf::from("test.js"),
            1,
            FunctionKind::Declaration,
        );
        assert!(!metrics.is_complex(10));

        metrics.cyclomatic = 15;
        assert!(metrics.is_complex(10));
    }

    #[test]
    fn test_typescript_pattern_result_default() {
        let result = TypeScriptPatternResult::default();
        assert_eq!(result.any_count, 0);
        assert_eq!(result.assertion_count, 0);
        assert!(result.has_strict_null);
    }
}
