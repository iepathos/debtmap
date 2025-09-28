//! Python-Aware Dead Code Detection Module
//!
//! This module implements Python-specific dead code detection that correctly
//! identifies implicitly called methods, framework patterns, and Python runtime
//! conventions to eliminate false positives.

use crate::core::{FunctionMetrics, Language};
use crate::priority::call_graph::{CallGraph, FunctionId};
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Confidence level for dead code removal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalConfidence {
    /// Definitely unused, safe to remove
    Safe,
    /// Probably unused, manual check recommended
    Likely,
    /// May be implicitly called, unsafe to remove
    Unsafe,
    /// Framework method, don't remove
    Framework,
    /// Python magic method, never remove
    Magic,
}

lazy_static! {
    /// Python magic methods that should never be flagged as dead code
    static ref MAGIC_METHODS: HashSet<&'static str> = {
        let mut set = HashSet::new();
        // Object lifecycle
        set.insert("__init__");
        set.insert("__new__");
        set.insert("__del__");

        // Operator overloading
        set.insert("__add__");
        set.insert("__sub__");
        set.insert("__mul__");
        set.insert("__truediv__");
        set.insert("__floordiv__");
        set.insert("__mod__");
        set.insert("__pow__");
        set.insert("__lshift__");
        set.insert("__rshift__");
        set.insert("__and__");
        set.insert("__xor__");
        set.insert("__or__");
        set.insert("__radd__");
        set.insert("__rsub__");
        set.insert("__rmul__");
        set.insert("__rtruediv__");
        set.insert("__rfloordiv__");
        set.insert("__rmod__");
        set.insert("__rpow__");
        set.insert("__rlshift__");
        set.insert("__rrshift__");
        set.insert("__rand__");
        set.insert("__rxor__");
        set.insert("__ror__");
        set.insert("__iadd__");
        set.insert("__isub__");
        set.insert("__imul__");
        set.insert("__itruediv__");
        set.insert("__ifloordiv__");
        set.insert("__imod__");
        set.insert("__ipow__");
        set.insert("__ilshift__");
        set.insert("__irshift__");
        set.insert("__iand__");
        set.insert("__ixor__");
        set.insert("__ior__");
        set.insert("__neg__");
        set.insert("__pos__");
        set.insert("__abs__");
        set.insert("__invert__");

        // Container protocols
        set.insert("__len__");
        set.insert("__getitem__");
        set.insert("__setitem__");
        set.insert("__delitem__");
        set.insert("__contains__");
        set.insert("__iter__");
        set.insert("__reversed__");
        set.insert("__missing__");

        // Context managers
        set.insert("__enter__");
        set.insert("__exit__");
        set.insert("__aenter__");
        set.insert("__aexit__");

        // Descriptors
        set.insert("__get__");
        set.insert("__set__");
        set.insert("__delete__");
        set.insert("__set_name__");

        // Serialization
        set.insert("__getstate__");
        set.insert("__setstate__");
        set.insert("__reduce__");
        set.insert("__reduce_ex__");
        set.insert("__getnewargs__");
        set.insert("__getnewargs_ex__");

        // String representation
        set.insert("__str__");
        set.insert("__repr__");
        set.insert("__format__");
        set.insert("__bytes__");

        // Comparison
        set.insert("__eq__");
        set.insert("__ne__");
        set.insert("__lt__");
        set.insert("__le__");
        set.insert("__gt__");
        set.insert("__ge__");
        set.insert("__hash__");
        set.insert("__bool__");

        // Attribute access
        set.insert("__getattr__");
        set.insert("__getattribute__");
        set.insert("__setattr__");
        set.insert("__delattr__");
        set.insert("__dir__");

        // Callable
        set.insert("__call__");

        // Async/await
        set.insert("__aiter__");
        set.insert("__anext__");
        set.insert("__await__");

        // Numeric conversions
        set.insert("__complex__");
        set.insert("__int__");
        set.insert("__float__");
        set.insert("__round__");
        set.insert("__trunc__");
        set.insert("__floor__");
        set.insert("__ceil__");

        // Copy
        set.insert("__copy__");
        set.insert("__deepcopy__");

        // Class creation
        set.insert("__init_subclass__");
        set.insert("__prepare__");
        set.insert("__mro_entries__");
        set.insert("__class_getitem__");

        // Instance checks
        set.insert("__instancecheck__");
        set.insert("__subclasscheck__");

        // Buffer protocol
        set.insert("__buffer__");
        set.insert("__release_buffer__");

        set
    };
}

/// Framework-specific lifecycle method patterns
#[derive(Debug, Clone)]
pub struct FrameworkPattern {
    pub name: String,
    pub lifecycle_methods: Vec<String>,
    pub event_patterns: Vec<Regex>,
    pub decorator_patterns: Vec<String>,
}

impl FrameworkPattern {
    /// Create pattern for wxPython framework
    pub fn wxpython() -> Self {
        Self {
            name: "wxPython".to_string(),
            lifecycle_methods: vec![
                "OnInit".to_string(),
                "OnExit".to_string(),
                "OnClose".to_string(),
                "OnDestroy".to_string(),
                "OnShow".to_string(),
                "OnHide".to_string(),
            ],
            event_patterns: vec![
                Regex::new(r"^on_.*").unwrap(),
                Regex::new(r"^On[A-Z].*").unwrap(),
            ],
            decorator_patterns: vec![],
        }
    }

    /// Create pattern for Django framework
    pub fn django() -> Self {
        Self {
            name: "Django".to_string(),
            lifecycle_methods: vec![
                "save".to_string(),
                "delete".to_string(),
                "clean".to_string(),
                "full_clean".to_string(),
                "validate_unique".to_string(),
                "clean_fields".to_string(),
                "__str__".to_string(),
                "get_absolute_url".to_string(),
            ],
            event_patterns: vec![
                Regex::new(r"^get_.*").unwrap(),
                Regex::new(r"^post_.*").unwrap(),
                Regex::new(r"^put_.*").unwrap(),
                Regex::new(r"^patch_.*").unwrap(),
                Regex::new(r"^delete_.*").unwrap(),
                Regex::new(r"^handle_.*").unwrap(),
            ],
            decorator_patterns: vec![
                "@login_required".to_string(),
                "@permission_required".to_string(),
                "@require_http_methods".to_string(),
                "@csrf_exempt".to_string(),
            ],
        }
    }

    /// Create pattern for Flask framework
    pub fn flask() -> Self {
        Self {
            name: "Flask".to_string(),
            lifecycle_methods: vec![
                "before_request".to_string(),
                "after_request".to_string(),
                "teardown_request".to_string(),
                "before_first_request".to_string(),
                "errorhandler".to_string(),
            ],
            event_patterns: vec![],
            decorator_patterns: vec![
                "@app.route".to_string(),
                "@blueprint.route".to_string(),
                "@app.before_request".to_string(),
                "@app.after_request".to_string(),
                "@app.errorhandler".to_string(),
            ],
        }
    }

    /// Create pattern for pytest
    pub fn pytest() -> Self {
        Self {
            name: "pytest".to_string(),
            lifecycle_methods: vec![
                "setup_method".to_string(),
                "teardown_method".to_string(),
                "setup_class".to_string(),
                "teardown_class".to_string(),
                "setup_module".to_string(),
                "teardown_module".to_string(),
                "setup_function".to_string(),
                "teardown_function".to_string(),
            ],
            event_patterns: vec![
                Regex::new(r"^test_.*").unwrap(),
                Regex::new(r"^.*_test$").unwrap(),
            ],
            decorator_patterns: vec![
                "@pytest.fixture".to_string(),
                "@pytest.mark".to_string(),
                "@fixture".to_string(),
            ],
        }
    }

    /// Create pattern for unittest
    pub fn unittest() -> Self {
        Self {
            name: "unittest".to_string(),
            lifecycle_methods: vec![
                "setUp".to_string(),
                "tearDown".to_string(),
                "setUpClass".to_string(),
                "tearDownClass".to_string(),
                "setUpModule".to_string(),
                "tearDownModule".to_string(),
            ],
            event_patterns: vec![Regex::new(r"^test.*").unwrap()],
            decorator_patterns: vec![
                "@unittest.skip".to_string(),
                "@unittest.skipIf".to_string(),
                "@unittest.expectedFailure".to_string(),
            ],
        }
    }

    /// Create pattern for FastAPI
    pub fn fastapi() -> Self {
        Self {
            name: "FastAPI".to_string(),
            lifecycle_methods: vec!["startup".to_string(), "shutdown".to_string()],
            event_patterns: vec![],
            decorator_patterns: vec![
                "@app.get".to_string(),
                "@app.post".to_string(),
                "@app.put".to_string(),
                "@app.delete".to_string(),
                "@app.patch".to_string(),
                "@app.options".to_string(),
                "@app.head".to_string(),
                "@router.get".to_string(),
                "@router.post".to_string(),
                "@router.put".to_string(),
                "@router.delete".to_string(),
            ],
        }
    }

    /// Create pattern for SQLAlchemy
    pub fn sqlalchemy() -> Self {
        Self {
            name: "SQLAlchemy".to_string(),
            lifecycle_methods: vec![
                "__tablename__".to_string(),
                "__mapper_args__".to_string(),
                "__table_args__".to_string(),
            ],
            event_patterns: vec![],
            decorator_patterns: vec![
                "@validates".to_string(),
                "@hybrid_property".to_string(),
                "@hybrid_method".to_string(),
                "@event.listens_for".to_string(),
            ],
        }
    }
}

/// Python dead code detector with framework awareness
pub struct PythonDeadCodeDetector {
    magic_methods: &'static HashSet<&'static str>,
    framework_patterns: HashMap<String, FrameworkPattern>,
    _decorator_handlers: Vec<String>,
    active_frameworks: HashSet<String>,
}

impl PythonDeadCodeDetector {
    /// Create a new Python dead code detector
    pub fn new() -> Self {
        let mut framework_patterns = HashMap::new();

        // Add all framework patterns
        framework_patterns.insert("wxpython".to_string(), FrameworkPattern::wxpython());
        framework_patterns.insert("django".to_string(), FrameworkPattern::django());
        framework_patterns.insert("flask".to_string(), FrameworkPattern::flask());
        framework_patterns.insert("pytest".to_string(), FrameworkPattern::pytest());
        framework_patterns.insert("unittest".to_string(), FrameworkPattern::unittest());
        framework_patterns.insert("fastapi".to_string(), FrameworkPattern::fastapi());
        framework_patterns.insert("sqlalchemy".to_string(), FrameworkPattern::sqlalchemy());

        // Common property decorators
        let decorator_handlers = vec![
            "@property".to_string(),
            "@cached_property".to_string(),
            "@classmethod".to_string(),
            "@staticmethod".to_string(),
            "@abstractmethod".to_string(),
        ];

        // Always activate common frameworks for better detection
        let mut active_frameworks = HashSet::new();
        active_frameworks.insert("wxpython".to_string());
        active_frameworks.insert("unittest".to_string());
        active_frameworks.insert("pytest".to_string());
        active_frameworks.insert("django".to_string());
        active_frameworks.insert("flask".to_string());

        Self {
            magic_methods: &MAGIC_METHODS,
            framework_patterns,
            _decorator_handlers: decorator_handlers,
            active_frameworks,
        }
    }

    /// Set active frameworks for detection
    pub fn with_frameworks(mut self, frameworks: Vec<String>) -> Self {
        self.active_frameworks = frameworks.into_iter().collect();
        self
    }

    /// Auto-detect frameworks from imports or file patterns
    pub fn auto_detect_frameworks(&mut self, file_path: &Path, _imports: &[String]) {
        let path_str = file_path.to_string_lossy();

        // Detect based on file patterns
        if path_str.contains("test") || path_str.contains("spec") {
            self.active_frameworks.insert("pytest".to_string());
            self.active_frameworks.insert("unittest".to_string());
        }

        // Add more auto-detection logic based on imports
        // This would require parsing imports which we can enhance later
    }

    /// Check if a function is implicitly called
    pub fn is_implicitly_called(&self, func: &FunctionMetrics) -> bool {
        // Extract method name from potentially qualified name
        let method_name = if let Some(pos) = func.name.rfind('.') {
            &func.name[pos + 1..]
        } else {
            &func.name
        };

        // Check if it's a magic method
        if self.is_magic_method(method_name) {
            return true;
        }

        // Check if it's a main entry point
        if self.is_main_entry_point(func) {
            return true;
        }

        // Check framework patterns
        if self.is_framework_method(method_name, func) {
            return true;
        }

        // Check common patterns
        if self.is_common_implicit_pattern(method_name, func) {
            return true;
        }

        false
    }

    /// Check if a method is a Python magic method
    fn is_magic_method(&self, method_name: &str) -> bool {
        self.magic_methods.contains(method_name)
    }

    /// Check if function is called from if __name__ == "__main__" block
    pub fn is_main_entry_point(&self, func: &FunctionMetrics) -> bool {
        // Check if this is a main() function that would be called from if __name__ == "__main__"
        func.name == "main" || func.name == "cli" || func.name == "run"
    }

    /// Check if a method matches framework patterns
    fn is_framework_method(&self, method_name: &str, func: &FunctionMetrics) -> bool {
        for framework_name in &self.active_frameworks {
            if let Some(pattern) = self.framework_patterns.get(framework_name) {
                // Check lifecycle methods
                if pattern.lifecycle_methods.iter().any(|m| m == method_name) {
                    return true;
                }

                // Check event patterns
                if pattern
                    .event_patterns
                    .iter()
                    .any(|re| re.is_match(method_name))
                {
                    return true;
                }

                // Check decorator patterns (would need decorator info from AST)
                // This is a simplified check based on naming conventions
                if framework_name == "django" && method_name.starts_with("get_") {
                    return true;
                }
                if framework_name == "flask" && func.name.contains("route") {
                    return true;
                }
            }
        }

        false
    }

    /// Check common implicit call patterns
    fn is_common_implicit_pattern(&self, method_name: &str, func: &FunctionMetrics) -> bool {
        // Property-like methods
        if method_name.starts_with("get_") || method_name.starts_with("set_") {
            return true;
        }

        // Event handlers
        if method_name.starts_with("on_") || method_name.starts_with("handle_") {
            return true;
        }

        // Callback patterns
        if method_name.ends_with("_callback") || method_name.ends_with("_handler") {
            return true;
        }

        // Test methods (additional patterns)
        if method_name.starts_with("test")
            || method_name.starts_with("setup")
            || method_name.starts_with("teardown")
        {
            return true;
        }

        // Module-level special names
        if func.name == "__main__" || func.name == "__all__" {
            return true;
        }

        false
    }

    /// Get confidence level for dead code removal
    pub fn get_removal_confidence(&self, func: &FunctionMetrics) -> RemovalConfidence {
        let method_name = if let Some(pos) = func.name.rfind('.') {
            &func.name[pos + 1..]
        } else {
            &func.name
        };

        // Magic methods - never remove
        if self.is_magic_method(method_name) {
            return RemovalConfidence::Magic;
        }

        // Framework methods
        if self.is_framework_method(method_name, func) {
            return RemovalConfidence::Framework;
        }

        // Common implicit patterns
        if self.is_common_implicit_pattern(method_name, func) {
            return RemovalConfidence::Unsafe;
        }

        // Public API (no underscore prefix) - likely safe but needs review
        if !method_name.starts_with('_') && func.visibility.as_ref().is_some_and(|v| v == "pub") {
            return RemovalConfidence::Likely;
        }

        // Private methods with no calls - safe to remove
        RemovalConfidence::Safe
    }

    /// Generate usage hints for potentially dead code
    pub fn generate_usage_hints(&self, func: &FunctionMetrics) -> Vec<String> {
        let mut hints = Vec::new();
        let method_name = if let Some(pos) = func.name.rfind('.') {
            &func.name[pos + 1..]
        } else {
            &func.name
        };

        if self.is_magic_method(method_name) {
            hints.push(format!(
                "Magic method '{}' is called by Python runtime",
                method_name
            ));
        }

        for framework_name in &self.active_frameworks {
            if let Some(pattern) = self.framework_patterns.get(framework_name) {
                if pattern.lifecycle_methods.contains(&method_name.to_string()) {
                    hints.push(format!("{} framework lifecycle method", pattern.name));
                }
                if pattern
                    .event_patterns
                    .iter()
                    .any(|re| re.is_match(method_name))
                {
                    hints.push(format!("Matches {} event pattern", pattern.name));
                }
            }
        }

        if method_name.starts_with("on_") {
            hints.push("Likely an event handler (on_* pattern)".to_string());
        }

        if method_name.starts_with("test") {
            hints.push("Test method - called by test runner".to_string());
        }

        if hints.is_empty() && !func.name.starts_with('_') {
            hints.push("Public API - may be used externally".to_string());
        }

        hints
    }

    /// Enhanced dead code detection for Python
    pub fn is_dead_code_with_confidence(
        &self,
        func: &FunctionMetrics,
        call_graph: &CallGraph,
        func_id: &FunctionId,
    ) -> Option<(bool, RemovalConfidence)> {
        // Only check Python files
        if Language::from_path(&func.file) != Language::Python {
            return None;
        }

        // Check if implicitly called
        if self.is_implicitly_called(func) {
            return Some((false, self.get_removal_confidence(func)));
        }

        // Check call graph
        let has_callers = !call_graph.get_callers(func_id).is_empty();
        if has_callers {
            return Some((false, RemovalConfidence::Safe));
        }

        // No callers and not implicitly called
        let confidence = self.get_removal_confidence(func);
        Some((true, confidence))
    }
}

impl Default for PythonDeadCodeDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for implicit call detection
pub trait ImplicitCallDetector {
    fn is_implicitly_called(&self, method: &str, context: &Context) -> bool;
    fn get_implicit_callers(&self, method: &str) -> Vec<String>;
    fn confidence_level(&self, method: &str) -> RemovalConfidence;
}

/// Context for dead code analysis
pub struct Context<'a> {
    pub file_path: &'a Path,
    pub class_name: Option<&'a str>,
    pub module_name: Option<&'a str>,
    pub imports: Vec<String>,
    pub decorators: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_method_detection() {
        let detector = PythonDeadCodeDetector::new();

        // Test various magic methods
        assert!(detector.is_magic_method("__init__"));
        assert!(detector.is_magic_method("__str__"));
        assert!(detector.is_magic_method("__getitem__"));
        assert!(detector.is_magic_method("__enter__"));
        assert!(detector.is_magic_method("__aiter__"));

        // Non-magic methods
        assert!(!detector.is_magic_method("init"));
        assert!(!detector.is_magic_method("_private"));
        assert!(!detector.is_magic_method("public_method"));
    }

    #[test]
    fn test_framework_method_detection() {
        let detector = PythonDeadCodeDetector::new()
            .with_frameworks(vec!["django".to_string(), "wxpython".to_string()]);

        let mut func = FunctionMetrics::new(
            "MyClass.OnInit".to_string(),
            Path::new("app.py").to_path_buf(),
            0,
        );
        func.visibility = Some("pub".to_string());

        assert!(detector.is_framework_method("OnInit", &func));
        assert!(detector.is_framework_method("save", &func));
        assert!(!detector.is_framework_method("random_method", &func));
    }

    #[test]
    fn test_removal_confidence_levels() {
        let detector = PythonDeadCodeDetector::new();

        // Magic method
        let magic_func = FunctionMetrics::new(
            "MyClass.__init__".to_string(),
            Path::new("test.py").to_path_buf(),
            0,
        );
        assert_eq!(
            detector.get_removal_confidence(&magic_func),
            RemovalConfidence::Magic
        );

        // Event handler
        let event_func = FunctionMetrics::new(
            "Panel.on_click".to_string(),
            Path::new("ui.py").to_path_buf(),
            0,
        );
        assert_eq!(
            detector.get_removal_confidence(&event_func),
            RemovalConfidence::Unsafe
        );

        // Private method
        let mut private_func = FunctionMetrics::new(
            "MyClass._helper".to_string(),
            Path::new("core.py").to_path_buf(),
            0,
        );
        private_func.visibility = None;
        assert_eq!(
            detector.get_removal_confidence(&private_func),
            RemovalConfidence::Safe
        );
    }

    #[test]
    fn test_usage_hints_generation() {
        let detector = PythonDeadCodeDetector::new().with_frameworks(vec!["wxpython".to_string()]);

        let func = FunctionMetrics::new(
            "App.OnInit".to_string(),
            Path::new("app.py").to_path_buf(),
            0,
        );

        let hints = detector.generate_usage_hints(&func);
        assert!(!hints.is_empty());
        assert!(hints.iter().any(|h| h.contains("wxPython")));
    }
}
