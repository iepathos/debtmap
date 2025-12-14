//! Module Structure Analysis
//!
//! Provides detailed analysis of module structure including:
//! - Accurate function counting (module-level, impl methods, trait methods)
//! - Component detection (structs, enums, impl blocks)
//! - Responsibility identification
//! - Coupling analysis for refactoring recommendations
//! - Facade detection for well-organized modules (Spec 170)
//!
//! ## Architecture
//!
//! This module is organized following the Stillwater philosophy:
//! - `types`: Pure data types and their core operations
//! - `rust_analyzer`: Rust-specific analysis using `syn`
//! - `python_analyzer`: Python analysis via pattern matching
//! - `js_analyzer`: JavaScript/TypeScript analysis via pattern matching
//! - `facade`: Module facade detection logic
//!
//! ## Usage
//!
//! ```ignore
//! use debtmap::analysis::module_structure::ModuleStructureAnalyzer;
//!
//! let analyzer = ModuleStructureAnalyzer::new_rust();
//! let structure = analyzer.analyze_rust_file(content, path);
//! ```

mod facade;
mod js_analyzer;
mod python_analyzer;
mod rust_analyzer;
mod types;

// Re-export all public types
pub use facade::{classify_organization_quality, extract_path_attribute};
pub use types::{
    ComponentCouplingAnalysis, ComponentDependencyGraph, Difficulty, FunctionCounts, FunctionGroup,
    ModuleComponent, ModuleFacadeInfo, ModuleStructure, OrganizationQuality, PathDeclaration,
    SplitRecommendation,
};

// Re-export pure helper functions
pub use types::{difficulty_from_coupling, suggest_module_name};

/// Module structure analyzer supporting multiple languages
///
/// Provides a unified interface for analyzing module structure across
/// Rust, Python, JavaScript, and TypeScript files.
pub struct ModuleStructureAnalyzer {
    _language: String, // For future multi-language support
}

impl ModuleStructureAnalyzer {
    pub fn new_rust() -> Self {
        Self {
            _language: "rust".to_string(),
        }
    }

    pub fn new_python() -> Self {
        Self {
            _language: "python".to_string(),
        }
    }

    pub fn new_javascript() -> Self {
        Self {
            _language: "javascript".to_string(),
        }
    }

    pub fn new_typescript() -> Self {
        Self {
            _language: "typescript".to_string(),
        }
    }

    /// Analyze a Rust source file to extract detailed module structure
    pub fn analyze_rust_file(&self, content: &str, file_path: &std::path::Path) -> ModuleStructure {
        rust_analyzer::analyze_rust_file(content, file_path)
    }

    /// Analyze a parsed Rust AST directly
    pub fn analyze_rust_ast(&self, ast: &syn::File) -> ModuleStructure {
        rust_analyzer::analyze_rust_ast(ast)
    }

    /// Analyze a Python source file to extract module structure
    pub fn analyze_python_file(
        &self,
        content: &str,
        file_path: &std::path::Path,
    ) -> ModuleStructure {
        python_analyzer::analyze_python_file(content, file_path)
    }

    /// Analyze a JavaScript source file to extract module structure
    pub fn analyze_javascript_file(
        &self,
        content: &str,
        file_path: &std::path::Path,
    ) -> ModuleStructure {
        js_analyzer::analyze_javascript_file(content, file_path)
    }

    /// Analyze a TypeScript source file to extract module structure
    pub fn analyze_typescript_file(
        &self,
        content: &str,
        file_path: &std::path::Path,
    ) -> ModuleStructure {
        js_analyzer::analyze_typescript_file(content, file_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let _rust = ModuleStructureAnalyzer::new_rust();
        let _python = ModuleStructureAnalyzer::new_python();
        let _js = ModuleStructureAnalyzer::new_javascript();
        let _ts = ModuleStructureAnalyzer::new_typescript();
    }

    #[test]
    fn test_analyze_simple_rust_file() {
        let code = r#"
            pub struct Foo {
                field: u32,
            }

            impl Foo {
                pub fn new() -> Self {
                    Self { field: 0 }
                }
            }

            pub fn helper() {}
        "#;

        let analyzer = ModuleStructureAnalyzer::new_rust();
        let structure = analyzer.analyze_rust_file(code, std::path::Path::new("test.rs"));

        assert_eq!(structure.function_counts.impl_methods, 1);
        assert_eq!(structure.function_counts.module_level_functions, 1);
        assert!(structure.responsibility_count >= 1);
    }
}
