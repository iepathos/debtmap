//! Framework Pattern Detection for Python
//!
//! This module provides comprehensive framework detection for Python codebases,
//! automatically identifying framework entry points, event handlers, and lifecycle
//! methods to improve call graph accuracy and reduce false positive dead code detection.

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Supported Python frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameworkType {
    /// wxPython GUI framework
    WxPython,
    /// Tkinter GUI framework
    Tkinter,
    /// PyQt/PySide GUI frameworks
    PyQt,
    /// Kivy mobile framework
    Kivy,
    /// Django web framework
    Django,
    /// Flask web framework
    Flask,
    /// FastAPI web framework
    FastAPI,
    /// Tornado async web framework
    Tornado,
    /// pytest testing framework
    Pytest,
    /// unittest testing framework
    Unittest,
    /// nose testing framework
    Nose,
    /// asyncio async framework
    Asyncio,
    /// trio async framework
    Trio,
    /// SQLAlchemy ORM
    SqlAlchemy,
    /// Click CLI framework
    Click,
    /// Celery task queue
    Celery,
}

impl FrameworkType {
    /// Get the canonical name of the framework
    pub fn name(&self) -> &'static str {
        match self {
            FrameworkType::WxPython => "wxPython",
            FrameworkType::Tkinter => "Tkinter",
            FrameworkType::PyQt => "PyQt",
            FrameworkType::Kivy => "Kivy",
            FrameworkType::Django => "Django",
            FrameworkType::Flask => "Flask",
            FrameworkType::FastAPI => "FastAPI",
            FrameworkType::Tornado => "Tornado",
            FrameworkType::Pytest => "pytest",
            FrameworkType::Unittest => "unittest",
            FrameworkType::Nose => "nose",
            FrameworkType::Asyncio => "asyncio",
            FrameworkType::Trio => "trio",
            FrameworkType::SqlAlchemy => "SQLAlchemy",
            FrameworkType::Click => "Click",
            FrameworkType::Celery => "Celery",
        }
    }

    /// Get import indicators that suggest this framework is in use
    pub fn import_indicators(&self) -> Vec<&'static str> {
        match self {
            FrameworkType::WxPython => vec!["wx", "wxPython"],
            FrameworkType::Tkinter => vec!["tkinter", "Tkinter"],
            FrameworkType::PyQt => vec!["PyQt5", "PyQt6", "PySide2", "PySide6"],
            FrameworkType::Kivy => vec!["kivy"],
            FrameworkType::Django => vec!["django"],
            FrameworkType::Flask => vec!["flask"],
            FrameworkType::FastAPI => vec!["fastapi"],
            FrameworkType::Tornado => vec!["tornado"],
            FrameworkType::Pytest => vec!["pytest"],
            FrameworkType::Unittest => vec!["unittest"],
            FrameworkType::Nose => vec!["nose"],
            FrameworkType::Asyncio => vec!["asyncio"],
            FrameworkType::Trio => vec!["trio"],
            FrameworkType::SqlAlchemy => vec!["sqlalchemy"],
            FrameworkType::Click => vec!["click"],
            FrameworkType::Celery => vec!["celery"],
        }
    }
}

/// Pattern definition for a framework
#[derive(Debug, Clone)]
pub struct FrameworkPattern {
    /// Framework type
    pub framework_type: FrameworkType,
    /// Lifecycle methods that are entry points
    pub lifecycle_methods: Vec<String>,
    /// Regex patterns for event handler naming conventions
    pub event_patterns: Vec<Regex>,
    /// Decorator patterns that mark entry points
    pub decorator_patterns: Vec<String>,
}

impl FrameworkPattern {
    /// Create pattern for wxPython
    pub fn wxpython() -> Self {
        Self {
            framework_type: FrameworkType::WxPython,
            lifecycle_methods: vec![
                "OnInit".to_string(),
                "OnExit".to_string(),
                "OnClose".to_string(),
                "OnDestroy".to_string(),
                "OnShow".to_string(),
                "OnHide".to_string(),
                "OnPaint".to_string(),
                "OnSize".to_string(),
                "OnMove".to_string(),
            ],
            event_patterns: vec![
                Regex::new(r"^on_.*").unwrap(),
                Regex::new(r"^On[A-Z].*").unwrap(),
            ],
            decorator_patterns: vec![],
        }
    }

    /// Create pattern for Tkinter
    pub fn tkinter() -> Self {
        Self {
            framework_type: FrameworkType::Tkinter,
            lifecycle_methods: vec![
                "mainloop".to_string(),
                "quit".to_string(),
                "destroy".to_string(),
            ],
            event_patterns: vec![
                Regex::new(r"^on_.*").unwrap(),
                Regex::new(r"^handle_.*").unwrap(),
            ],
            decorator_patterns: vec![],
        }
    }

    /// Create pattern for PyQt/PySide
    pub fn pyqt() -> Self {
        Self {
            framework_type: FrameworkType::PyQt,
            lifecycle_methods: vec![
                "exec_".to_string(),
                "exec".to_string(),
                "closeEvent".to_string(),
                "paintEvent".to_string(),
                "resizeEvent".to_string(),
                "mousePressEvent".to_string(),
                "mouseReleaseEvent".to_string(),
            ],
            event_patterns: vec![
                Regex::new(r"^.*Event$").unwrap(),
                Regex::new(r"^on_.*").unwrap(),
            ],
            decorator_patterns: vec!["@pyqtSlot".to_string(), "@Slot".to_string()],
        }
    }

    /// Create pattern for Kivy
    pub fn kivy() -> Self {
        Self {
            framework_type: FrameworkType::Kivy,
            lifecycle_methods: vec![
                "build".to_string(),
                "on_start".to_string(),
                "on_stop".to_string(),
                "on_pause".to_string(),
                "on_resume".to_string(),
            ],
            event_patterns: vec![Regex::new(r"^on_.*").unwrap()],
            decorator_patterns: vec![],
        }
    }

    /// Create pattern for Django
    pub fn django() -> Self {
        Self {
            framework_type: FrameworkType::Django,
            lifecycle_methods: vec![
                "save".to_string(),
                "delete".to_string(),
                "clean".to_string(),
                "full_clean".to_string(),
                "validate_unique".to_string(),
                "clean_fields".to_string(),
                "get_absolute_url".to_string(),
                "ready".to_string(),
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

    /// Create pattern for Flask
    pub fn flask() -> Self {
        Self {
            framework_type: FrameworkType::Flask,
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

    /// Create pattern for FastAPI
    pub fn fastapi() -> Self {
        Self {
            framework_type: FrameworkType::FastAPI,
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
                "@app.on_event".to_string(),
            ],
        }
    }

    /// Create pattern for Tornado
    pub fn tornado() -> Self {
        Self {
            framework_type: FrameworkType::Tornado,
            lifecycle_methods: vec![
                "initialize".to_string(),
                "prepare".to_string(),
                "on_finish".to_string(),
            ],
            event_patterns: vec![
                Regex::new(r"^get$").unwrap(),
                Regex::new(r"^post$").unwrap(),
                Regex::new(r"^put$").unwrap(),
                Regex::new(r"^delete$").unwrap(),
                Regex::new(r"^patch$").unwrap(),
                Regex::new(r"^head$").unwrap(),
                Regex::new(r"^options$").unwrap(),
            ],
            decorator_patterns: vec!["@gen.coroutine".to_string()],
        }
    }

    /// Create pattern for pytest
    pub fn pytest() -> Self {
        Self {
            framework_type: FrameworkType::Pytest,
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
            framework_type: FrameworkType::Unittest,
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

    /// Create pattern for asyncio
    pub fn asyncio() -> Self {
        Self {
            framework_type: FrameworkType::Asyncio,
            lifecycle_methods: vec!["run".to_string()],
            event_patterns: vec![Regex::new(r"^async_.*").unwrap()],
            decorator_patterns: vec![],
        }
    }

    /// Create pattern for SQLAlchemy
    pub fn sqlalchemy() -> Self {
        Self {
            framework_type: FrameworkType::SqlAlchemy,
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

    /// Create pattern for Click CLI framework
    pub fn click() -> Self {
        Self {
            framework_type: FrameworkType::Click,
            lifecycle_methods: vec![],
            event_patterns: vec![],
            decorator_patterns: vec![
                "@click.command".to_string(),
                "@click.group".to_string(),
                "@click.option".to_string(),
                "@click.argument".to_string(),
            ],
        }
    }

    /// Create pattern for Celery
    pub fn celery() -> Self {
        Self {
            framework_type: FrameworkType::Celery,
            lifecycle_methods: vec![],
            event_patterns: vec![],
            decorator_patterns: vec!["@app.task".to_string(), "@celery.task".to_string()],
        }
    }

    /// Check if a method name matches this pattern
    pub fn matches_method(&self, method_name: &str) -> bool {
        // Check lifecycle methods
        if self.lifecycle_methods.iter().any(|m| m == method_name) {
            return true;
        }

        // Check event patterns
        if self
            .event_patterns
            .iter()
            .any(|pattern| pattern.is_match(method_name))
        {
            return true;
        }

        false
    }
}

/// Registry of framework patterns for detection
pub struct FrameworkPatternRegistry {
    /// Map of framework types to their patterns
    patterns: HashMap<FrameworkType, FrameworkPattern>,
    /// Currently active/detected frameworks
    active_frameworks: HashSet<FrameworkType>,
    /// Custom pattern rules (user-defined)
    custom_patterns: Vec<CustomPattern>,
}

/// User-defined custom pattern
#[derive(Debug, Clone)]
pub struct CustomPattern {
    /// Name of the custom pattern
    pub name: String,
    /// Method names that are entry points
    pub method_names: Vec<String>,
    /// Regex patterns for method names
    pub method_patterns: Vec<Regex>,
    /// Decorator patterns
    pub decorator_patterns: Vec<String>,
}

impl FrameworkPatternRegistry {
    /// Create a new registry with all built-in patterns
    pub fn new() -> Self {
        let mut patterns = HashMap::new();

        // Register all built-in framework patterns
        patterns.insert(FrameworkType::WxPython, FrameworkPattern::wxpython());
        patterns.insert(FrameworkType::Tkinter, FrameworkPattern::tkinter());
        patterns.insert(FrameworkType::PyQt, FrameworkPattern::pyqt());
        patterns.insert(FrameworkType::Kivy, FrameworkPattern::kivy());
        patterns.insert(FrameworkType::Django, FrameworkPattern::django());
        patterns.insert(FrameworkType::Flask, FrameworkPattern::flask());
        patterns.insert(FrameworkType::FastAPI, FrameworkPattern::fastapi());
        patterns.insert(FrameworkType::Tornado, FrameworkPattern::tornado());
        patterns.insert(FrameworkType::Pytest, FrameworkPattern::pytest());
        patterns.insert(FrameworkType::Unittest, FrameworkPattern::unittest());
        patterns.insert(FrameworkType::Asyncio, FrameworkPattern::asyncio());
        patterns.insert(FrameworkType::SqlAlchemy, FrameworkPattern::sqlalchemy());
        patterns.insert(FrameworkType::Click, FrameworkPattern::click());
        patterns.insert(FrameworkType::Celery, FrameworkPattern::celery());

        // Activate common frameworks by default for better detection
        let mut active_frameworks = HashSet::new();
        active_frameworks.insert(FrameworkType::WxPython);
        active_frameworks.insert(FrameworkType::Pytest);
        active_frameworks.insert(FrameworkType::Unittest);
        active_frameworks.insert(FrameworkType::Django);
        active_frameworks.insert(FrameworkType::Flask);

        Self {
            patterns,
            active_frameworks,
            custom_patterns: Vec::new(),
        }
    }

    /// Detect frameworks from import statements
    pub fn detect_frameworks(&mut self, imports: &[String]) -> Vec<FrameworkType> {
        let mut detected = Vec::new();

        for framework_type in [
            FrameworkType::WxPython,
            FrameworkType::Tkinter,
            FrameworkType::PyQt,
            FrameworkType::Kivy,
            FrameworkType::Django,
            FrameworkType::Flask,
            FrameworkType::FastAPI,
            FrameworkType::Tornado,
            FrameworkType::Pytest,
            FrameworkType::Unittest,
            FrameworkType::Asyncio,
            FrameworkType::SqlAlchemy,
            FrameworkType::Click,
            FrameworkType::Celery,
        ] {
            let indicators = framework_type.import_indicators();

            // Check if any import matches the indicators
            for import in imports {
                if indicators.iter().any(|&indicator| {
                    import.contains(indicator)
                        || import.starts_with(&format!("from {}", indicator))
                        || import.starts_with(&format!("import {}", indicator))
                }) {
                    detected.push(framework_type);
                    self.active_frameworks.insert(framework_type);
                    break;
                }
            }
        }

        detected
    }

    /// Auto-detect frameworks from file path and imports
    pub fn auto_detect_frameworks(&mut self, file_path: &Path, imports: &[String]) {
        // Detect from imports
        self.detect_frameworks(imports);

        // Detect from file path patterns
        let path_str = file_path.to_string_lossy();

        if path_str.contains("test") || path_str.contains("spec") {
            self.active_frameworks.insert(FrameworkType::Pytest);
            self.active_frameworks.insert(FrameworkType::Unittest);
        }

        if path_str.contains("django") || path_str.contains("manage.py") {
            self.active_frameworks.insert(FrameworkType::Django);
        }

        if path_str.contains("flask") || path_str.contains("app.py") {
            self.active_frameworks.insert(FrameworkType::Flask);
        }
    }

    /// Check if a method is an entry point based on active frameworks
    pub fn is_entry_point(&self, func_name: &str, decorators: &[String]) -> bool {
        // Extract method name from qualified name
        let method_name = if let Some(pos) = func_name.rfind('.') {
            &func_name[pos + 1..]
        } else {
            func_name
        };

        // Check active frameworks
        for framework_type in &self.active_frameworks {
            if let Some(pattern) = self.patterns.get(framework_type) {
                if pattern.matches_method(method_name) {
                    return true;
                }

                // Check decorator patterns
                for decorator in decorators {
                    if pattern
                        .decorator_patterns
                        .iter()
                        .any(|d| decorator.contains(d))
                    {
                        return true;
                    }
                }
            }
        }

        // Check custom patterns
        for custom in &self.custom_patterns {
            if custom.method_names.iter().any(|m| m == method_name) {
                return true;
            }

            if custom
                .method_patterns
                .iter()
                .any(|pattern| pattern.is_match(method_name))
            {
                return true;
            }

            for decorator in decorators {
                if custom
                    .decorator_patterns
                    .iter()
                    .any(|d| decorator.contains(d))
                {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a method is an event handler (common patterns)
    pub fn is_event_handler(&self, func_name: &str) -> bool {
        let method_name = if let Some(pos) = func_name.rfind('.') {
            &func_name[pos + 1..]
        } else {
            func_name
        };

        // Common event handler patterns across frameworks
        method_name.starts_with("on_")
            || method_name.starts_with("handle_")
            || method_name.starts_with("process_")
            || method_name.ends_with("_callback")
            || method_name.ends_with("_handler")
            || method_name.ends_with("Event")
    }

    /// Add a custom pattern to the registry
    pub fn add_custom_pattern(&mut self, pattern: CustomPattern) {
        self.custom_patterns.push(pattern);
    }

    /// Get all active frameworks
    pub fn get_active_frameworks(&self) -> &HashSet<FrameworkType> {
        &self.active_frameworks
    }

    /// Manually activate a framework
    pub fn activate_framework(&mut self, framework_type: FrameworkType) {
        self.active_frameworks.insert(framework_type);
    }

    /// Get pattern for a specific framework
    pub fn get_pattern(&self, framework_type: FrameworkType) -> Option<&FrameworkPattern> {
        self.patterns.get(&framework_type)
    }
}

impl Default for FrameworkPatternRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_import_detection() {
        let mut registry = FrameworkPatternRegistry::new();

        let imports = vec![
            "import wx".to_string(),
            "from django.db import models".to_string(),
            "import pytest".to_string(),
        ];

        let detected = registry.detect_frameworks(&imports);

        assert!(detected.contains(&FrameworkType::WxPython));
        assert!(detected.contains(&FrameworkType::Django));
        assert!(detected.contains(&FrameworkType::Pytest));
        assert!(!detected.contains(&FrameworkType::Flask));
    }

    #[test]
    fn test_wxpython_pattern_matching() {
        let pattern = FrameworkPattern::wxpython();

        assert!(pattern.matches_method("OnInit"));
        assert!(pattern.matches_method("OnPaint"));
        assert!(pattern.matches_method("on_click"));
        assert!(pattern.matches_method("on_paint"));
        assert!(!pattern.matches_method("normal_method"));
    }

    #[test]
    fn test_django_pattern_matching() {
        let pattern = FrameworkPattern::django();

        assert!(pattern.matches_method("save"));
        assert!(pattern.matches_method("clean"));
        assert!(pattern.matches_method("get_absolute_url"));
        assert!(pattern.matches_method("post_save"));
        assert!(pattern.matches_method("handle_request"));
        assert!(!pattern.matches_method("normal_method"));
    }

    #[test]
    fn test_pytest_pattern_matching() {
        let pattern = FrameworkPattern::pytest();

        assert!(pattern.matches_method("test_something"));
        assert!(pattern.matches_method("test_another"));
        assert!(pattern.matches_method("something_test"));
        assert!(pattern.matches_method("setup_method"));
        assert!(pattern.matches_method("teardown_method"));
        assert!(!pattern.matches_method("helper_function"));
    }

    #[test]
    fn test_entry_point_detection() {
        let mut registry = FrameworkPatternRegistry::new();
        registry.activate_framework(FrameworkType::WxPython);
        registry.activate_framework(FrameworkType::Flask);

        assert!(registry.is_entry_point("MyApp.OnInit", &[]));
        assert!(registry.is_entry_point("view_func", &["@app.route('/home')".to_string()]));
        assert!(!registry.is_entry_point("helper_function", &[]));
    }

    #[test]
    fn test_event_handler_detection() {
        let registry = FrameworkPatternRegistry::new();

        assert!(registry.is_event_handler("on_click"));
        assert!(registry.is_event_handler("handle_message"));
        assert!(registry.is_event_handler("process_data"));
        assert!(registry.is_event_handler("button_callback"));
        assert!(registry.is_event_handler("mouseEvent"));
        assert!(!registry.is_event_handler("normal_method"));
    }

    #[test]
    fn test_custom_pattern_addition() {
        let mut registry = FrameworkPatternRegistry::new();

        let custom = CustomPattern {
            name: "MyFramework".to_string(),
            method_names: vec!["custom_init".to_string()],
            method_patterns: vec![Regex::new(r"^custom_.*").unwrap()],
            decorator_patterns: vec!["@my_decorator".to_string()],
        };

        registry.add_custom_pattern(custom);

        assert!(registry.is_entry_point("custom_init", &[]));
        assert!(registry.is_entry_point("custom_handler", &[]));
        assert!(registry.is_entry_point("func", &["@my_decorator".to_string()]));
    }

    #[test]
    fn test_auto_detect_from_file_path() {
        let mut registry = FrameworkPatternRegistry::new();

        registry.auto_detect_frameworks(Path::new("tests/test_module.py"), &[]);
        assert!(registry
            .get_active_frameworks()
            .contains(&FrameworkType::Pytest));
        assert!(registry
            .get_active_frameworks()
            .contains(&FrameworkType::Unittest));

        let mut registry2 = FrameworkPatternRegistry::new();
        registry2.auto_detect_frameworks(Path::new("myapp/manage.py"), &[]);
        assert!(registry2
            .get_active_frameworks()
            .contains(&FrameworkType::Django));
    }

    #[test]
    fn test_fastapi_pattern_matching() {
        let pattern = FrameworkPattern::fastapi();

        assert!(pattern.matches_method("startup"));
        assert!(pattern.matches_method("shutdown"));
    }

    #[test]
    fn test_all_frameworks_have_patterns() {
        let registry = FrameworkPatternRegistry::new();

        assert!(registry.get_pattern(FrameworkType::WxPython).is_some());
        assert!(registry.get_pattern(FrameworkType::Django).is_some());
        assert!(registry.get_pattern(FrameworkType::Flask).is_some());
        assert!(registry.get_pattern(FrameworkType::FastAPI).is_some());
        assert!(registry.get_pattern(FrameworkType::Pytest).is_some());
        assert!(registry.get_pattern(FrameworkType::Unittest).is_some());
        assert!(registry.get_pattern(FrameworkType::Asyncio).is_some());
        assert!(registry.get_pattern(FrameworkType::SqlAlchemy).is_some());
        assert!(registry.get_pattern(FrameworkType::Click).is_some());
        assert!(registry.get_pattern(FrameworkType::Celery).is_some());
    }
}
