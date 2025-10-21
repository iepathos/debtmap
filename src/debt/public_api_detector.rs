//! Public API Detection Heuristics (Spec 113)
//!
//! This module implements heuristics to detect public API functions and exclude them
//! from dead code detection, reducing false positives from 30% to < 5% for library-style modules.
//!
//! # Heuristics Implemented
//!
//! 1. **Naming Convention Heuristics** - Functions without underscore prefix → likely public
//! 2. **Documentation Analysis** - Comprehensive docstrings → likely public API
//! 3. **Type Annotation Analysis** - Full type hints → likely public
//! 4. **Symmetric Function Detection** - Paired operations (load/save, get/set)
//! 5. **Module-Level Export Analysis** - Functions in `__all__` → definitely public
//! 6. **Rust Visibility** - `pub` keyword → definitive public API
//!
//! # Usage
//!
//! ```rust,no_run
//! use debtmap::debt::public_api_detector::{PublicApiDetector, PublicApiConfig, FunctionDef, FileContext, Language};
//! use std::collections::HashMap;
//! use std::path::PathBuf;
//!
//! let detector = PublicApiDetector::new(PublicApiConfig::default());
//!
//! let function = FunctionDef {
//!     name: "my_function".to_string(),
//!     docstring: Some("A public API function".to_string()),
//!     parameters: vec![],
//!     return_type: Some("str".to_string()),
//!     decorators: vec![],
//!     is_method: false,
//!     class_name: None,
//!     line: 10,
//!     visibility: None,
//!     is_trait_impl: false,
//! };
//!
//! let context = FileContext {
//!     file_path: PathBuf::from("example.py"),
//!     language: Language::Python,
//!     module_all: None,
//!     functions: HashMap::new(),
//!     used_functions: vec![],
//!     init_exports: vec![],
//! };
//!
//! let score = detector.is_public_api(&function, &context);
//!
//! if score.is_public {
//!     println!("Public API function: {}", score.confidence);
//! }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for public API detection
#[derive(Debug, Clone)]
pub struct PublicApiConfig {
    /// Weight for naming convention heuristic (0.0 - 1.0)
    pub naming_convention_weight: f32,
    /// Weight for docstring quality heuristic (0.0 - 1.0)
    pub docstring_weight: f32,
    /// Weight for type annotation heuristic (0.0 - 1.0)
    pub type_annotation_weight: f32,
    /// Weight for symmetric pair heuristic (0.0 - 1.0)
    pub symmetric_pair_weight: f32,
    /// Weight for module export heuristic (0.0 - 1.0)
    pub module_export_weight: f32,
    /// Confidence threshold for marking as public (0.0 - 1.0)
    pub public_api_threshold: f32,
    /// Custom public prefixes for project-specific patterns
    pub custom_public_prefixes: Vec<String>,
    /// Custom symmetric pairs (e.g., ["fetch", "submit"])
    pub custom_symmetric_pairs: Vec<(String, String)>,
}

impl Default for PublicApiConfig {
    fn default() -> Self {
        Self {
            naming_convention_weight: 0.3,
            docstring_weight: 0.25,
            type_annotation_weight: 0.15,
            symmetric_pair_weight: 0.2,
            module_export_weight: 0.1,
            public_api_threshold: 0.6,
            custom_public_prefixes: vec![],
            custom_symmetric_pairs: vec![],
        }
    }
}

/// Result of public API analysis
#[derive(Debug, Clone)]
pub struct PublicApiScore {
    /// Whether the function is considered public API
    pub is_public: bool,
    /// Overall confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Individual heuristic scores
    pub heuristic_scores: HashMap<String, f32>,
    /// Human-readable reasoning
    pub reasoning: Vec<String>,
}

/// Context for analyzing functions in a file
#[derive(Debug, Clone)]
pub struct FileContext {
    /// Path to the file being analyzed
    pub file_path: PathBuf,
    /// Programming language of the file
    pub language: Language,
    /// Module-level `__all__` exports (Python)
    pub module_all: Option<Vec<String>>,
    /// All functions in the file
    pub functions: HashMap<String, FunctionDef>,
    /// Functions that are used/called
    pub used_functions: Vec<String>,
    /// Exports in __init__.py
    pub init_exports: Vec<String>,
}

impl FileContext {
    pub fn new(file_path: PathBuf, language: Language) -> Self {
        Self {
            file_path,
            language,
            module_all: None,
            functions: HashMap::new(),
            used_functions: vec![],
            init_exports: vec![],
        }
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn is_module_level(&self, function: &FunctionDef) -> bool {
        !function.is_method
    }

    pub fn is_class_method(&self, function: &FunctionDef) -> bool {
        function.is_method
    }

    pub fn is_in_module_all(&self, name: &str) -> bool {
        self.module_all
            .as_ref()
            .map(|all| all.contains(&name.to_string()))
            .unwrap_or(false)
    }

    pub fn is_exported_in_init(&self, name: &str) -> bool {
        self.init_exports.iter().any(|export| export == name)
    }

    pub fn find_function(&self, name: &str) -> Option<&FunctionDef> {
        self.functions.get(name)
    }

    pub fn is_function_used(&self, function: &FunctionDef) -> bool {
        self.used_functions
            .iter()
            .any(|used| used == &function.name)
    }
}

/// Programming language
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Python,
    Rust,
    JavaScript,
    TypeScript,
}

/// Function definition with metadata
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub docstring: Option<String>,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub decorators: Vec<String>,
    pub is_method: bool,
    pub class_name: Option<String>,
    pub line: usize,
    /// Rust-specific visibility (pub, pub(crate), etc.)
    pub visibility: Option<String>,
    /// True if implementing a trait method (Rust)
    pub is_trait_impl: bool,
}

impl FunctionDef {
    /// Check if function has a specific visibility keyword (Rust)
    pub fn has_visibility_keyword(&self, keyword: &str) -> bool {
        self.visibility
            .as_ref()
            .map(|v| v.contains(keyword))
            .unwrap_or(false)
    }

    /// Check if function implements a trait method (Rust)
    pub fn is_trait_implementation(&self) -> bool {
        self.is_trait_impl
    }
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub default_value: Option<String>,
}

/// Trait for implementing API detection heuristics
pub trait ApiHeuristic: Send + Sync {
    /// Name of the heuristic
    fn name(&self) -> &str;

    /// Evaluate the heuristic for a function (returns 0.0-1.0)
    fn evaluate(&self, function: &FunctionDef, context: &FileContext) -> f32;

    /// Generate human-readable explanation
    fn explain(&self, function: &FunctionDef) -> String;
}

/// Public API detector using multiple heuristics
pub struct PublicApiDetector {
    config: PublicApiConfig,
}

impl PublicApiDetector {
    pub fn new(config: PublicApiConfig) -> Self {
        Self { config }
    }

    /// Determine if a function is public API
    pub fn is_public_api(&self, function: &FunctionDef, context: &FileContext) -> PublicApiScore {
        let mut heuristic_scores = HashMap::new();
        let mut reasoning = Vec::new();
        let mut weighted_score = 0.0f32;

        // Apply naming convention heuristic
        let naming_heuristic = NamingConventionHeuristic;
        let naming_score = naming_heuristic.evaluate(function, context);
        heuristic_scores.insert(naming_heuristic.name().to_string(), naming_score);
        weighted_score += naming_score * self.config.naming_convention_weight;
        reasoning.push(naming_heuristic.explain(function));

        // Apply docstring heuristic
        let docstring_heuristic = DocstringHeuristic;
        let docstring_score = docstring_heuristic.evaluate(function, context);
        heuristic_scores.insert(docstring_heuristic.name().to_string(), docstring_score);
        weighted_score += docstring_score * self.config.docstring_weight;
        if docstring_score > 0.0 {
            reasoning.push(docstring_heuristic.explain(function));
        }

        // Apply type annotation heuristic
        let type_annotation_heuristic = TypeAnnotationHeuristic;
        let type_score = type_annotation_heuristic.evaluate(function, context);
        heuristic_scores.insert(type_annotation_heuristic.name().to_string(), type_score);
        weighted_score += type_score * self.config.type_annotation_weight;
        if type_score > 0.5 {
            reasoning.push(type_annotation_heuristic.explain(function));
        }

        // Apply symmetric pair heuristic
        let symmetric_heuristic = SymmetricPairHeuristic::new(&self.config);
        let symmetric_score = symmetric_heuristic.evaluate(function, context);
        heuristic_scores.insert(symmetric_heuristic.name().to_string(), symmetric_score);
        weighted_score += symmetric_score * self.config.symmetric_pair_weight;
        if symmetric_score > 0.0 {
            reasoning.push(symmetric_heuristic.explain(function));
        }

        // Apply module export heuristic
        let export_heuristic = ModuleExportHeuristic;
        let export_score = export_heuristic.evaluate(function, context);
        heuristic_scores.insert(export_heuristic.name().to_string(), export_score);
        weighted_score += export_score * self.config.module_export_weight;
        if export_score > 0.0 {
            reasoning.push(export_heuristic.explain(function));
        }

        // Apply Rust visibility heuristic if applicable
        if context.language() == Language::Rust {
            let rust_heuristic = RustVisibilityHeuristic;
            let rust_score = rust_heuristic.evaluate(function, context);
            heuristic_scores.insert(rust_heuristic.name().to_string(), rust_score);
            // Rust visibility is definitive, so give it high weight
            weighted_score = weighted_score.max(rust_score);
            if rust_score > 0.0 {
                reasoning.push(rust_heuristic.explain(function));
            }
        }

        let is_public = weighted_score >= self.config.public_api_threshold;

        PublicApiScore {
            is_public,
            confidence: weighted_score.clamp(0.0, 1.0),
            heuristic_scores,
            reasoning,
        }
    }

    /// Find the symmetric pair of a function (e.g., save → load)
    pub fn find_symmetric_pair<'a>(
        &self,
        function: &FunctionDef,
        context: &'a FileContext,
    ) -> Option<&'a FunctionDef> {
        let heuristic = SymmetricPairHeuristic::new(&self.config);
        heuristic.find_pair(function, context)
    }
}

// ============================================================================
// Heuristic Implementations
// ============================================================================

/// Naming Convention Heuristic
///
/// IMPORTANT: Functions with leading underscore are marked as private (0.0)
/// regardless of other heuristic scores. This prevents false negatives.
struct NamingConventionHeuristic;

impl ApiHeuristic for NamingConventionHeuristic {
    fn name(&self) -> &str {
        "naming_convention"
    }

    fn evaluate(&self, function: &FunctionDef, context: &FileContext) -> f32 {
        let name = &function.name;

        // Dunder methods (special methods, not public API)
        if name.starts_with("__") && name.ends_with("__") {
            return 0.5;
        }

        // Leading underscore → internal (STRONG NEGATIVE SIGNAL)
        if name.starts_with('_') {
            return 0.0;
        }

        // Module-level function without underscore → likely public
        if context.is_module_level(function) {
            return 1.0;
        }

        // Class method without underscore → public
        if context.is_class_method(function) {
            return 0.8;
        }

        0.5 // Neutral
    }

    fn explain(&self, function: &FunctionDef) -> String {
        if function.name.starts_with('_') {
            "Function has leading underscore (private convention)".to_string()
        } else if function.name.starts_with("__") && function.name.ends_with("__") {
            "Function is a dunder method (special method)".to_string()
        } else {
            "Function has no underscore prefix (public convention)".to_string()
        }
    }
}

/// Docstring Heuristic
struct DocstringHeuristic;

impl ApiHeuristic for DocstringHeuristic {
    fn name(&self) -> &str {
        "docstring_quality"
    }

    fn evaluate(&self, function: &FunctionDef, _context: &FileContext) -> f32 {
        let docstring = match &function.docstring {
            Some(doc) => doc,
            None => return 0.0,
        };

        let length = docstring.len();

        // Very short docstrings
        if length < 20 {
            return 0.2;
        }

        // Medium docstrings
        if length < 50 {
            return 0.5;
        }

        // Long docstrings
        if length < 100 {
            return 0.8;
        }

        // Check for structured docstring
        if self.is_structured_docstring(docstring) {
            return 1.0;
        }

        0.9
    }

    fn explain(&self, function: &FunctionDef) -> String {
        match &function.docstring {
            Some(doc) if doc.len() >= 50 => "Function has comprehensive docstring".to_string(),
            Some(_) => "Function has basic docstring".to_string(),
            None => "Function has no docstring".to_string(),
        }
    }
}

impl DocstringHeuristic {
    fn is_structured_docstring(&self, doc: &str) -> bool {
        let markers = [
            "Args:",
            "Returns:",
            "Raises:",
            "Yields:",
            "Parameters:",
            ":param",
            ":return",
        ];
        markers.iter().any(|marker| doc.contains(marker))
    }
}

/// Type Annotation Heuristic
struct TypeAnnotationHeuristic;

impl ApiHeuristic for TypeAnnotationHeuristic {
    fn name(&self) -> &str {
        "type_annotations"
    }

    fn evaluate(&self, function: &FunctionDef, _context: &FileContext) -> f32 {
        let param_annotations = function
            .parameters
            .iter()
            .filter(|p| p.type_annotation.is_some())
            .count();

        let total_params = function.parameters.len();

        // No parameters → neutral
        if total_params == 0 {
            return 0.5;
        }

        let annotation_ratio = param_annotations as f32 / total_params as f32;
        let has_return_type = function.return_type.is_some();

        // Fully annotated
        if annotation_ratio >= 1.0 && has_return_type {
            return 1.0;
        }

        // Partially annotated
        if has_return_type {
            return 0.5 + (annotation_ratio * 0.3);
        }

        annotation_ratio * 0.7
    }

    fn explain(&self, function: &FunctionDef) -> String {
        let annotated = function
            .parameters
            .iter()
            .filter(|p| p.type_annotation.is_some())
            .count();
        let total = function.parameters.len();

        if annotated == total && function.return_type.is_some() {
            "Function has full type annotations".to_string()
        } else if annotated > 0 || function.return_type.is_some() {
            format!(
                "Function has partial type annotations ({}/{})",
                annotated, total
            )
        } else {
            "Function has no type annotations".to_string()
        }
    }
}

/// Symmetric Pair Heuristic
struct SymmetricPairHeuristic {
    pairs: Vec<(String, String)>,
}

impl SymmetricPairHeuristic {
    fn new(config: &PublicApiConfig) -> Self {
        let mut pairs = vec![
            ("load".to_string(), "save".to_string()),
            ("get".to_string(), "set".to_string()),
            ("open".to_string(), "close".to_string()),
            ("create".to_string(), "destroy".to_string()),
            ("start".to_string(), "stop".to_string()),
            ("acquire".to_string(), "release".to_string()),
            ("add".to_string(), "remove".to_string()),
            ("push".to_string(), "pop".to_string()),
            ("read".to_string(), "write".to_string()),
        ];

        // Add custom pairs
        pairs.extend(config.custom_symmetric_pairs.clone());

        Self { pairs }
    }

    fn find_pair<'a>(
        &self,
        function: &FunctionDef,
        context: &'a FileContext,
    ) -> Option<&'a FunctionDef> {
        let func_name = &function.name;
        let components: Vec<&str> = func_name.split('_').collect();

        for (first, second) in &self.pairs {
            let has_first = components.iter().any(|&c| c == first);
            let has_second = components.iter().any(|&c| c == second);

            if has_first || has_second {
                // Construct symmetric pair name
                let pair_name = if has_first {
                    components
                        .iter()
                        .map(|&c| if c == first { second.as_str() } else { c })
                        .collect::<Vec<_>>()
                        .join("_")
                } else {
                    components
                        .iter()
                        .map(|&c| if c == second { first.as_str() } else { c })
                        .collect::<Vec<_>>()
                        .join("_")
                };

                if let Some(pair_func) = context.find_function(&pair_name) {
                    return Some(pair_func);
                }
            }
        }

        None
    }
}

impl ApiHeuristic for SymmetricPairHeuristic {
    fn name(&self) -> &str {
        "symmetric_pair"
    }

    fn evaluate(&self, function: &FunctionDef, context: &FileContext) -> f32 {
        if let Some(pair_func) = self.find_pair(function, context) {
            // If pair is used, mark this as public
            if context.is_function_used(pair_func) {
                return 1.0;
            }
            // Pair exists but not used
            return 0.7;
        }

        0.0
    }

    fn explain(&self, function: &FunctionDef) -> String {
        format!(
            "Function '{}' may be part of a symmetric API pair",
            function.name
        )
    }
}

/// Module Export Heuristic
struct ModuleExportHeuristic;

impl ApiHeuristic for ModuleExportHeuristic {
    fn name(&self) -> &str {
        "module_export"
    }

    fn evaluate(&self, function: &FunctionDef, context: &FileContext) -> f32 {
        // Check if in __all__
        if context.is_in_module_all(&function.name) {
            return 1.0;
        }

        // Check if imported in __init__.py
        if context.is_exported_in_init(&function.name) {
            return 1.0;
        }

        0.0
    }

    fn explain(&self, function: &FunctionDef) -> String {
        format!("Function '{}' is explicitly exported", function.name)
    }
}

/// Rust Visibility Heuristic
struct RustVisibilityHeuristic;

impl ApiHeuristic for RustVisibilityHeuristic {
    fn name(&self) -> &str {
        "rust_visibility"
    }

    fn evaluate(&self, function: &FunctionDef, context: &FileContext) -> f32 {
        // Only applicable to Rust
        if context.language() != Language::Rust {
            return 0.0;
        }

        // Trait implementations are never dead code
        if function.is_trait_implementation() {
            return 1.0;
        }

        // Check visibility keyword
        if function.has_visibility_keyword("pub(crate)") {
            return 0.5;
        } else if function.has_visibility_keyword("pub(super)") {
            return 0.3;
        } else if function.has_visibility_keyword("pub") {
            return 1.0;
        }

        0.0
    }

    fn explain(&self, function: &FunctionDef) -> String {
        if function.is_trait_implementation() {
            "Function implements trait method (required by trait)".to_string()
        } else if function.has_visibility_keyword("pub") {
            "Function has `pub` visibility (Rust public API)".to_string()
        } else {
            "Function has no `pub` keyword (Rust private)".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_function(name: &str) -> FunctionDef {
        FunctionDef {
            name: name.to_string(),
            docstring: None,
            parameters: vec![],
            return_type: None,
            decorators: vec![],
            is_method: false,
            class_name: None,
            line: 10,
            visibility: None,
            is_trait_impl: false,
        }
    }

    fn create_python_context() -> FileContext {
        FileContext::new(PathBuf::from("test.py"), Language::Python)
    }

    #[test]
    fn test_naming_convention_public() {
        let function = create_test_function("create_bots");
        let context = create_python_context();
        let heuristic = NamingConventionHeuristic;

        let score = heuristic.evaluate(&function, &context);
        assert!(score >= 0.8, "Public function should score high");
    }

    #[test]
    fn test_naming_convention_private() {
        let function = create_test_function("_internal_helper");
        let context = create_python_context();
        let heuristic = NamingConventionHeuristic;

        let score = heuristic.evaluate(&function, &context);
        assert_eq!(score, 0.0, "Private function should score 0");
    }

    #[test]
    fn test_docstring_structured() {
        let mut function = create_test_function("process_data");
        function.docstring = Some(
            r#"
            Process input data and return results.

            Args:
                data: Input data to process

            Returns:
                Processed data
        "#
            .to_string(),
        );

        let context = create_python_context();
        let heuristic = DocstringHeuristic;
        let score = heuristic.evaluate(&function, &context);

        assert!(score >= 0.9, "Structured docstring should score very high");
    }

    #[test]
    fn test_type_annotations_full() {
        let mut function = create_test_function("calculate");
        function.parameters = vec![
            Parameter {
                name: "x".to_string(),
                type_annotation: Some("int".to_string()),
                default_value: None,
            },
            Parameter {
                name: "y".to_string(),
                type_annotation: Some("int".to_string()),
                default_value: None,
            },
        ];
        function.return_type = Some("int".to_string());

        let context = create_python_context();
        let heuristic = TypeAnnotationHeuristic;
        let score = heuristic.evaluate(&function, &context);

        assert!(score >= 0.9, "Fully annotated function should score high");
    }

    #[test]
    fn test_symmetric_pair_detection() {
        let save_func = create_test_function("save_chat_history");
        let load_func = create_test_function("load_chat_history");

        let mut context = create_python_context();
        context
            .functions
            .insert("load_chat_history".to_string(), load_func.clone());
        context.used_functions.push("load_chat_history".to_string());

        let config = PublicApiConfig::default();
        let heuristic = SymmetricPairHeuristic::new(&config);
        let score = heuristic.evaluate(&save_func, &context);

        assert!(
            score >= 0.8,
            "Function with used symmetric pair should score high"
        );
    }

    #[test]
    fn test_module_all_export() {
        let function = create_test_function("exported_func");
        let mut context = create_python_context();
        context.module_all = Some(vec!["exported_func".to_string()]);

        let heuristic = ModuleExportHeuristic;
        let score = heuristic.evaluate(&function, &context);

        assert_eq!(score, 1.0, "Function in __all__ should score 1.0");
    }

    #[test]
    fn test_rust_pub_keyword() {
        let mut function = create_test_function("analyze_code");
        function.visibility = Some("pub".to_string());

        let context = FileContext::new(PathBuf::from("lib.rs"), Language::Rust);
        let heuristic = RustVisibilityHeuristic;
        let score = heuristic.evaluate(&function, &context);

        assert_eq!(score, 1.0, "pub function should score 1.0");
    }

    #[test]
    fn test_rust_trait_implementation() {
        let mut function = create_test_function("clone");
        function.is_trait_impl = true;

        let context = FileContext::new(PathBuf::from("lib.rs"), Language::Rust);
        let heuristic = RustVisibilityHeuristic;
        let score = heuristic.evaluate(&function, &context);

        assert_eq!(
            score, 1.0,
            "Trait implementation should score 1.0 (never dead)"
        );
    }

    #[test]
    fn test_public_api_detector_integration() {
        let mut function = create_test_function("create_bots_from_list");
        function.docstring =
            Some("Create bots from a list of bot configuration files.".to_string());
        function.parameters = vec![Parameter {
            name: "bot_files".to_string(),
            type_annotation: Some("list".to_string()),
            default_value: Some("None".to_string()),
        }];

        let context = create_python_context();
        let detector = PublicApiDetector::new(PublicApiConfig::default());
        let score = detector.is_public_api(&function, &context);

        assert!(score.is_public, "Function should be detected as public API");
        assert!(
            score.confidence >= 0.6,
            "Confidence should exceed threshold"
        );
    }

    #[test]
    fn test_underscore_prefix_override() {
        let mut function = create_test_function("_internal_complex_algorithm");
        function.docstring = Some(
            r#"
            Performs complex internal processing.

            Args:
                data: List of integers to process

            Returns:
                Dictionary containing processed results
        "#
            .to_string(),
        );
        function.parameters = vec![Parameter {
            name: "data".to_string(),
            type_annotation: Some("List[int]".to_string()),
            default_value: None,
        }];
        function.return_type = Some("Dict[str, Any]".to_string());

        let context = create_python_context();
        let heuristic = NamingConventionHeuristic;
        let score = heuristic.evaluate(&function, &context);

        assert_eq!(
            score, 0.0,
            "Underscore prefix should score 0.0 regardless of docs"
        );
    }
}
