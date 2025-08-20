//! Framework Pattern Detection
//!
//! This module detects functions that are managed by frameworks (test functions,
//! web handlers, event handlers, etc.) to prevent them from being marked as dead code.

use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use im::{HashMap, HashSet, Vector};
use std::path::Path;
use syn::visit::Visit;
use syn::{Attribute, File, ItemFn};

/// Types of framework patterns that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PatternType {
    /// Test function (marked with `#[test]`, `#[tokio::test]`, etc.)
    TestFunction,
    /// Benchmark function (marked with `#[bench]`)
    BenchmarkFunction,
    /// Web handler (marked with route attributes like `#[get]`, `#[post]`, etc.)
    WebHandler,
    /// Event handler (async functions with specific patterns)
    EventHandler,
    /// Macro callback (functions called by procedural macros)
    MacroCallback,
    /// Serialization function (serde derive callbacks)
    SerializationFunction,
    /// Constructor function (derive macro generated)
    ConstructorFunction,
    /// Foreign Function Interface
    FfiFunction,
    /// Main function entry points
    MainFunction,
    /// Visit trait pattern (visitor pattern implementations)
    VisitTrait,
    /// Custom framework pattern
    CustomPattern { name: String },
}

/// Information about a detected framework pattern
#[derive(Debug, Clone)]
pub struct FrameworkPattern {
    /// Type of pattern detected
    pub pattern_type: PatternType,
    /// Function associated with this pattern
    pub function_id: Option<FunctionId>,
    /// Attributes that triggered this pattern detection
    pub triggering_attributes: Vector<String>,
    /// Framework name (if identifiable)
    pub framework_name: Option<String>,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Configuration for framework pattern detection
#[derive(Debug, Clone)]
pub struct PatternConfig {
    /// Enable test function detection
    pub detect_tests: bool,
    /// Enable web handler detection
    pub detect_web_handlers: bool,
    /// Enable event handler detection
    pub detect_event_handlers: bool,
    /// Enable macro callback detection
    pub detect_macro_callbacks: bool,
    /// Enable serialization function detection
    pub detect_serialization: bool,
    /// Enable FFI function detection
    pub detect_ffi: bool,
    /// Custom attribute patterns to detect
    pub custom_patterns: HashMap<String, String>,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            detect_tests: true,
            detect_web_handlers: true,
            detect_event_handlers: true,
            detect_macro_callbacks: true,
            detect_serialization: true,
            detect_ffi: true,
            custom_patterns: HashMap::new(),
        }
    }
}

/// Detector for framework patterns
#[derive(Debug, Clone)]
pub struct FrameworkPatternDetector {
    config: PatternConfig,
    detected_patterns: Vector<FrameworkPattern>,
    function_to_patterns: HashMap<FunctionId, Vector<PatternType>>,
}

impl FrameworkPatternDetector {
    /// Create a new framework pattern detector
    pub fn new() -> Self {
        Self {
            config: PatternConfig::default(),
            detected_patterns: Vector::new(),
            function_to_patterns: HashMap::new(),
        }
    }

    /// Create a detector with custom configuration
    pub fn with_config(config: PatternConfig) -> Self {
        Self {
            config,
            detected_patterns: Vector::new(),
            function_to_patterns: HashMap::new(),
        }
    }

    /// Analyze a file for framework patterns
    pub fn analyze_file(&mut self, file_path: &Path, ast: &File) -> Result<()> {
        let mut visitor = PatternVisitor::new(file_path.to_path_buf(), &self.config);
        visitor.visit_file(ast);

        // Add discovered patterns
        for pattern in visitor.patterns {
            if let Some(func_id) = &pattern.function_id {
                // Update function to patterns mapping
                self.function_to_patterns
                    .entry(func_id.clone())
                    .or_default()
                    .push_back(pattern.pattern_type.clone());
            }

            self.detected_patterns.push_back(pattern);
        }

        Ok(())
    }

    /// Get all detected patterns
    pub fn get_detected_patterns(&self) -> Vector<FrameworkPattern> {
        self.detected_patterns.clone()
    }

    /// Check if a function might be managed by a framework
    pub fn might_be_framework_managed(&self, func_id: &FunctionId) -> bool {
        self.function_to_patterns.contains_key(func_id)
    }

    /// Get pattern types for a specific function
    pub fn get_function_patterns(&self, func_id: &FunctionId) -> Option<&Vector<PatternType>> {
        self.function_to_patterns.get(func_id)
    }

    /// Get functions of a specific pattern type
    pub fn get_functions_by_pattern(&self, pattern_type: &PatternType) -> Vector<FunctionId> {
        self.detected_patterns
            .iter()
            .filter(|pattern| &pattern.pattern_type == pattern_type)
            .filter_map(|pattern| pattern.function_id.clone())
            .collect()
    }

    /// Get statistics about detected patterns
    pub fn get_statistics(&self) -> PatternStatistics {
        let mut pattern_counts = HashMap::new();
        let total_patterns = self.detected_patterns.len();
        let framework_managed_functions = self.function_to_patterns.len();

        for pattern in &self.detected_patterns {
            *pattern_counts
                .entry(pattern.pattern_type.clone())
                .or_insert(0) += 1;
        }

        PatternStatistics {
            total_patterns,
            framework_managed_functions,
            pattern_counts,
        }
    }

    /// Add a custom pattern configuration
    pub fn add_custom_pattern(&mut self, attribute_name: String, description: String) {
        self.config
            .custom_patterns
            .insert(attribute_name, description);
    }

    /// Get functions that should be excluded from dead code analysis
    pub fn get_exclusions(&self) -> HashSet<FunctionId> {
        let mut exclusions = HashSet::new();

        for pattern in &self.detected_patterns {
            if let Some(func_id) = &pattern.function_id {
                match pattern.pattern_type {
                    PatternType::TestFunction
                    | PatternType::BenchmarkFunction
                    | PatternType::WebHandler
                    | PatternType::EventHandler
                    | PatternType::MacroCallback
                    | PatternType::MainFunction
                    | PatternType::FfiFunction
                    | PatternType::VisitTrait => {
                        exclusions.insert(func_id.clone());
                    }
                    PatternType::SerializationFunction | PatternType::ConstructorFunction => {
                        // These might be conditionally excluded based on confidence
                        if pattern.confidence > 0.7 {
                            exclusions.insert(func_id.clone());
                        }
                    }
                    PatternType::CustomPattern { .. } => {
                        // Custom patterns are excluded if confidence is high
                        if pattern.confidence > 0.8 {
                            exclusions.insert(func_id.clone());
                        }
                    }
                }
            }
        }

        exclusions
    }

    /// Mark a function as a Visit trait implementation
    pub fn add_visit_trait_function(&mut self, func_id: FunctionId) {
        let pattern = FrameworkPattern {
            pattern_type: PatternType::VisitTrait,
            function_id: Some(func_id.clone()),
            triggering_attributes: Vector::new(),
            framework_name: Some("visitor_pattern".to_string()),
            confidence: 1.0,
            metadata: HashMap::new(),
        };

        self.function_to_patterns
            .entry(func_id.clone())
            .or_default()
            .push_back(PatternType::VisitTrait);

        self.detected_patterns.push_back(pattern);
    }

    /// Check if a function is likely a visitor pattern method by name
    pub fn is_visitor_pattern_method(func_name: &str) -> bool {
        // Common visitor pattern method prefixes
        func_name.starts_with("visit_")
            || func_name.starts_with("walk_")
            || func_name.starts_with("traverse_")
            || func_name == "visit"
            || func_name == "walk"
    }
}

/// Statistics about detected framework patterns
#[derive(Debug, Clone)]
pub struct PatternStatistics {
    pub total_patterns: usize,
    pub framework_managed_functions: usize,
    pub pattern_counts: HashMap<PatternType, usize>,
}

/// Visitor for detecting framework patterns in AST
struct PatternVisitor {
    file_path: std::path::PathBuf,
    config: PatternConfig,
    patterns: Vec<FrameworkPattern>,
}

impl PatternVisitor {
    fn new(file_path: std::path::PathBuf, config: &PatternConfig) -> Self {
        Self {
            file_path,
            config: config.clone(),
            patterns: Vec::new(),
        }
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn analyze_function_attributes(&mut self, func: &ItemFn) {
        let func_name = func.sig.ident.to_string();
        let line = self.get_line_number(func.sig.ident.span());

        let func_id = FunctionId {
            file: self.file_path.clone(),
            name: func_name.clone(),
            line,
        };

        // Check for visitor pattern methods by name
        if FrameworkPatternDetector::is_visitor_pattern_method(&func_name) {
            let pattern = FrameworkPattern {
                pattern_type: PatternType::VisitTrait,
                function_id: Some(func_id.clone()),
                triggering_attributes: Vector::new(),
                framework_name: Some("visitor_pattern".to_string()),
                confidence: 0.9, // High confidence based on naming convention
                metadata: HashMap::new(),
            };
            self.patterns.push(pattern);
        }

        // Check for main function
        if func_name == "main" {
            let pattern = FrameworkPattern {
                pattern_type: PatternType::MainFunction,
                function_id: Some(func_id.clone()),
                triggering_attributes: Vector::new(),
                framework_name: None,
                confidence: 1.0,
                metadata: HashMap::new(),
            };
            self.patterns.push(pattern);
        }

        // Check FFI functions
        if func.sig.abi.is_some() {
            let pattern = FrameworkPattern {
                pattern_type: PatternType::FfiFunction,
                function_id: Some(func_id.clone()),
                triggering_attributes: Vector::new(),
                framework_name: None,
                confidence: 1.0,
                metadata: HashMap::new(),
            };
            self.patterns.push(pattern);
        }

        // Analyze attributes
        for attr in &func.attrs {
            if let Some(pattern) = self.analyze_attribute(attr, &func_id) {
                self.patterns.push(pattern);
            }
        }
    }

    fn analyze_attribute(
        &self,
        attr: &Attribute,
        func_id: &FunctionId,
    ) -> Option<FrameworkPattern> {
        let attr_name = self.extract_attribute_name(attr)?;

        // Test function patterns
        if self.config.detect_tests && self.is_test_attribute(&attr_name) {
            return Some(FrameworkPattern {
                pattern_type: PatternType::TestFunction,
                function_id: Some(func_id.clone()),
                triggering_attributes: vec![attr_name.clone()].into_iter().collect(),
                framework_name: self.detect_test_framework(&attr_name),
                confidence: 1.0,
                metadata: HashMap::new(),
            });
        }

        // Benchmark function patterns
        if attr_name == "bench" {
            return Some(FrameworkPattern {
                pattern_type: PatternType::BenchmarkFunction,
                function_id: Some(func_id.clone()),
                triggering_attributes: vec![attr_name.clone()].into_iter().collect(),
                framework_name: Some("criterion".to_string()),
                confidence: 1.0,
                metadata: HashMap::new(),
            });
        }

        // Web handler patterns
        if self.config.detect_web_handlers && self.is_web_handler_attribute(&attr_name) {
            return Some(FrameworkPattern {
                pattern_type: PatternType::WebHandler,
                function_id: Some(func_id.clone()),
                triggering_attributes: vec![attr_name.clone()].into_iter().collect(),
                framework_name: self.detect_web_framework(&attr_name),
                confidence: 0.9,
                metadata: self.extract_route_metadata(attr),
            });
        }

        // Serialization patterns
        if self.config.detect_serialization && self.is_serialization_attribute(&attr_name) {
            return Some(FrameworkPattern {
                pattern_type: PatternType::SerializationFunction,
                function_id: Some(func_id.clone()),
                triggering_attributes: vec![attr_name.clone()].into_iter().collect(),
                framework_name: Some("serde".to_string()),
                confidence: 0.8,
                metadata: HashMap::new(),
            });
        }

        // Macro callback patterns
        if self.config.detect_macro_callbacks && self.is_macro_callback_attribute(&attr_name) {
            return Some(FrameworkPattern {
                pattern_type: PatternType::MacroCallback,
                function_id: Some(func_id.clone()),
                triggering_attributes: vec![attr_name.clone()].into_iter().collect(),
                framework_name: None,
                confidence: 0.7,
                metadata: HashMap::new(),
            });
        }

        // Custom patterns
        if self.config.custom_patterns.contains_key(&attr_name) {
            let description = self.config.custom_patterns.get(&attr_name).unwrap();
            return Some(FrameworkPattern {
                pattern_type: PatternType::CustomPattern {
                    name: attr_name.clone(),
                },
                function_id: Some(func_id.clone()),
                triggering_attributes: vec![attr_name.clone()].into_iter().collect(),
                framework_name: None,
                confidence: 0.6,
                metadata: vec![("description".to_string(), description.clone())]
                    .into_iter()
                    .collect(),
            });
        }

        None
    }

    fn extract_attribute_name(&self, attr: &Attribute) -> Option<String> {
        // Extract attribute name from the path
        // Since parse_meta is deprecated in newer syn versions, use path directly
        if attr.path().segments.len() == 1 {
            Some(attr.path().segments.first()?.ident.to_string())
        } else {
            // For multi-segment paths like tokio::test
            let segments: Vec<String> = attr
                .path()
                .segments
                .iter()
                .map(|seg| seg.ident.to_string())
                .collect();
            Some(segments.join("::"))
        }
    }

    fn is_test_attribute(&self, attr_name: &str) -> bool {
        matches!(
            attr_name,
            "test"
                | "tokio::test"
                | "async_test"
                | "wasm_bindgen_test"
                | "proptest"
                | "quickcheck"
                | "rstest"
                | "serial_test"
        )
    }

    fn is_web_handler_attribute(&self, attr_name: &str) -> bool {
        matches!(
            attr_name,
            "get"
                | "post"
                | "put"
                | "delete"
                | "patch"
                | "head"
                | "options"
                | "route"
                | "handler"
                | "web"
                | "actix_web"
                | "rocket"
                | "warp"
                | "axum"
                | "tide"
                | "hyper"
        )
    }

    fn is_serialization_attribute(&self, attr_name: &str) -> bool {
        matches!(
            attr_name,
            "serde"
                | "derive"
                | "serialize"
                | "deserialize"
                | "serde_json"
                | "bincode"
                | "toml"
                | "yaml"
        )
    }

    fn is_macro_callback_attribute(&self, attr_name: &str) -> bool {
        matches!(
            attr_name,
            "derive"
                | "proc_macro"
                | "proc_macro_derive"
                | "proc_macro_attribute"
                | "no_mangle"
                | "export_name"
                | "link_name"
        )
    }

    fn detect_test_framework(&self, attr_name: &str) -> Option<String> {
        match attr_name {
            "test" => Some("std".to_string()),
            "tokio::test" => Some("tokio".to_string()),
            "async_test" => Some("async-std".to_string()),
            "wasm_bindgen_test" => Some("wasm-bindgen".to_string()),
            "proptest" => Some("proptest".to_string()),
            "quickcheck" => Some("quickcheck".to_string()),
            "rstest" => Some("rstest".to_string()),
            "serial_test" => Some("serial_test".to_string()),
            _ => None,
        }
    }

    fn detect_web_framework(&self, attr_name: &str) -> Option<String> {
        match attr_name {
            "get" | "post" | "put" | "delete" | "patch" | "head" | "options" | "route" => {
                // Could be multiple frameworks, would need more context
                Some("web_framework".to_string())
            }
            "actix_web" => Some("actix-web".to_string()),
            "rocket" => Some("rocket".to_string()),
            "warp" => Some("warp".to_string()),
            "axum" => Some("axum".to_string()),
            "tide" => Some("tide".to_string()),
            "hyper" => Some("hyper".to_string()),
            _ => None,
        }
    }

    fn extract_route_metadata(&self, _attr: &Attribute) -> HashMap<String, String> {
        // In newer versions of syn, parse_meta is deprecated
        // For now, return empty metadata - this would need proper parsing with the new API
        HashMap::new()
    }
}

impl<'ast> Visit<'ast> for PatternVisitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        self.analyze_function_attributes(item);

        // Continue visiting
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        // Check methods in impl blocks for visitor patterns
        for impl_item in &item.items {
            if let syn::ImplItem::Fn(method) = impl_item {
                let method_name = method.sig.ident.to_string();
                let line = self.get_line_number(method.sig.ident.span());

                let func_id = FunctionId {
                    file: self.file_path.clone(),
                    name: method_name.clone(),
                    line,
                };

                // Check if this is a visitor pattern method
                if FrameworkPatternDetector::is_visitor_pattern_method(&method_name) {
                    let pattern = FrameworkPattern {
                        pattern_type: PatternType::VisitTrait,
                        function_id: Some(func_id.clone()),
                        triggering_attributes: Vector::new(),
                        framework_name: Some("visitor_pattern".to_string()),
                        confidence: 0.9, // High confidence based on naming convention
                        metadata: HashMap::new(),
                    };
                    self.patterns.push(pattern);
                }
            }
        }

        // Continue visiting
        syn::visit::visit_item_impl(self, item);
    }
}

impl Default for FrameworkPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use syn::parse_quote;

    fn create_test_visitor() -> PatternVisitor {
        let config = PatternConfig::default();
        PatternVisitor::new(PathBuf::from("test.rs"), &config)
    }

    fn create_function_id() -> FunctionId {
        FunctionId {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
        }
    }

    #[test]
    fn test_analyze_attribute_test_function() {
        let visitor = create_test_visitor();
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[test]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::TestFunction);
        assert_eq!(pattern.function_id, Some(func_id));
        assert_eq!(pattern.framework_name, Some("std".to_string()));
        assert_eq!(pattern.confidence, 1.0);
        assert!(pattern.triggering_attributes.contains(&"test".to_string()));
    }

    #[test]
    fn test_analyze_attribute_tokio_test() {
        let visitor = create_test_visitor();
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[tokio::test]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::TestFunction);
        assert_eq!(pattern.framework_name, Some("tokio".to_string()));
        assert!(pattern
            .triggering_attributes
            .contains(&"tokio::test".to_string()));
    }

    #[test]
    fn test_analyze_attribute_benchmark() {
        let visitor = create_test_visitor();
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[bench]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::BenchmarkFunction);
        assert_eq!(pattern.framework_name, Some("criterion".to_string()));
        assert_eq!(pattern.confidence, 1.0);
    }

    #[test]
    fn test_analyze_attribute_web_handler_get() {
        let visitor = create_test_visitor();
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[get]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::WebHandler);
        assert_eq!(pattern.framework_name, Some("web_framework".to_string()));
        assert_eq!(pattern.confidence, 0.9);
    }

    #[test]
    fn test_analyze_attribute_serialization() {
        let visitor = create_test_visitor();
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[serde]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::SerializationFunction);
        assert_eq!(pattern.framework_name, Some("serde".to_string()));
        assert_eq!(pattern.confidence, 0.8);
    }

    #[test]
    fn test_analyze_attribute_macro_callback() {
        let visitor = create_test_visitor();
        let func_id = create_function_id();
        // Use proc_macro instead of derive since derive is also a serialization attribute
        let attr: Attribute = parse_quote!(#[proc_macro]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_some());
        let pattern = result.unwrap();
        assert_eq!(pattern.pattern_type, PatternType::MacroCallback);
        assert_eq!(pattern.framework_name, None);
        assert_eq!(pattern.confidence, 0.7);
    }

    #[test]
    fn test_analyze_attribute_custom_pattern() {
        let mut config = PatternConfig::default();
        config.custom_patterns.insert(
            "custom_attr".to_string(),
            "Custom attribute description".to_string(),
        );
        let visitor = PatternVisitor::new(PathBuf::from("test.rs"), &config);
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[custom_attr]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_some());
        let pattern = result.unwrap();
        match pattern.pattern_type {
            PatternType::CustomPattern { name } => {
                assert_eq!(name, "custom_attr");
            }
            _ => panic!("Expected CustomPattern"),
        }
        assert_eq!(pattern.confidence, 0.6);
        assert_eq!(
            pattern.metadata.get("description"),
            Some(&"Custom attribute description".to_string())
        );
    }

    #[test]
    fn test_analyze_attribute_unrecognized() {
        let visitor = create_test_visitor();
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[unknown_attr]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_attribute_with_disabled_detection() {
        let config = PatternConfig {
            detect_tests: false,
            ..Default::default()
        };
        let visitor = PatternVisitor::new(PathBuf::from("test.rs"), &config);
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[test]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_attribute_web_handler_disabled() {
        let config = PatternConfig {
            detect_web_handlers: false,
            ..Default::default()
        };
        let visitor = PatternVisitor::new(PathBuf::from("test.rs"), &config);
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[get]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_attribute_serialization_disabled() {
        let config = PatternConfig {
            detect_serialization: false,
            ..Default::default()
        };
        let visitor = PatternVisitor::new(PathBuf::from("test.rs"), &config);
        let func_id = create_function_id();
        let attr: Attribute = parse_quote!(#[serde]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_none());
    }

    #[test]
    fn test_analyze_attribute_macro_callback_disabled() {
        let config = PatternConfig {
            detect_macro_callbacks: false,
            ..Default::default()
        };
        let visitor = PatternVisitor::new(PathBuf::from("test.rs"), &config);
        let func_id = create_function_id();
        // Use proc_macro instead of derive since derive is also a serialization attribute
        let attr: Attribute = parse_quote!(#[proc_macro]);

        let result = visitor.analyze_attribute(&attr, &func_id);

        assert!(result.is_none());
    }
}
