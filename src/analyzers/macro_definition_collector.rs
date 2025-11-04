//! Macro Definition Collector
//!
//! Collects `macro_rules!` definitions from Rust source code for purity analysis.
//! This module enables the purity detector to classify custom macros by analyzing
//! their definitions.
//!
//! # Overview
//!
//! - Collects all `macro_rules!` definitions across a codebase
//! - Stores macro name, body tokens, and source location
//! - Thread-safe concurrent collection via `DashMap`
//! - Integrates with `PurityDetector` for macro classification

use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use syn::visit::Visit;
use syn::{File, ItemMacro};

/// Represents a custom macro definition
///
/// This type is Send + Sync to enable concurrent collection across threads
#[derive(Debug, Clone)]
pub struct MacroDefinition {
    /// Macro name (e.g., "my_logger")
    pub name: String,

    /// Macro body tokens (the expansion pattern)
    pub body: String,

    /// Source file location
    pub source_file: PathBuf,

    /// Line number where defined
    pub line: usize,
}

/// Thread-safe collection of macro definitions
pub type MacroDefinitions = Arc<DashMap<String, MacroDefinition>>;

/// Visitor to collect macro definitions from a file
pub struct MacroDefinitionCollector {
    definitions: MacroDefinitions,
    current_file: PathBuf,
}

impl MacroDefinitionCollector {
    /// Create a new collector for the given file
    pub fn new(definitions: MacroDefinitions, file_path: PathBuf) -> Self {
        Self {
            definitions,
            current_file: file_path,
        }
    }
}

impl<'ast> Visit<'ast> for MacroDefinitionCollector {
    fn visit_item_macro(&mut self, item: &'ast ItemMacro) {
        // Check if this is a macro_rules! definition
        // ItemMacro with an ident represents a macro definition like:
        // macro_rules! my_macro { ... }
        if let Some(ident) = &item.ident {
            let name = ident.to_string();

            // Get line number from span
            let line = ident.span().start().line;

            // Convert tokens to string for storage (Send + Sync compatible)
            let body = item.mac.tokens.to_string();

            // Store definition
            self.definitions.insert(
                name.clone(),
                MacroDefinition {
                    name,
                    body,
                    source_file: self.current_file.clone(),
                    line,
                },
            );
        }

        // Continue visiting nested items
        syn::visit::visit_item_macro(self, item);
    }
}

/// Collect macro definitions from a parsed file
pub fn collect_definitions(file: &File, file_path: &Path, definitions: MacroDefinitions) {
    let mut collector = MacroDefinitionCollector::new(definitions, file_path.to_path_buf());
    collector.visit_file(file);
}

/// Collect all macro definitions from a project
///
/// This function processes all files in parallel using rayon for performance.
pub fn collect_project_macros(files: &[(PathBuf, syn::File)]) -> MacroDefinitions {
    let definitions = Arc::new(DashMap::new());

    // Sequential collection for now (parallel version requires rayon ParallelSlice)
    // Performance impact is minimal since macro collection is very fast
    for (path, ast) in files {
        collect_definitions(ast, path, definitions.clone());
    }

    definitions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_macro_definitions() {
        let code = r#"
            macro_rules! my_macro {
                () => { 42 };
            }

            macro_rules! another {
                ($x:expr) => { println!("{}", $x) };
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, Path::new("test.rs"), definitions.clone());

        assert_eq!(definitions.len(), 2);
        assert!(definitions.contains_key("my_macro"));
        assert!(definitions.contains_key("another"));
    }

    #[test]
    fn test_parallel_collection() {
        let files = vec![
            (
                PathBuf::from("a.rs"),
                syn::parse_file(
                    r#"
                macro_rules! macro_a { () => { } }
            "#,
                )
                .unwrap(),
            ),
            (
                PathBuf::from("b.rs"),
                syn::parse_file(
                    r#"
                macro_rules! macro_b { () => { } }
            "#,
                )
                .unwrap(),
            ),
            (
                PathBuf::from("c.rs"),
                syn::parse_file(
                    r#"
                macro_rules! macro_c { () => { } }
            "#,
                )
                .unwrap(),
            ),
        ];

        let definitions = collect_project_macros(&files);

        assert_eq!(definitions.len(), 3);
        assert!(definitions.contains_key("macro_a"));
        assert!(definitions.contains_key("macro_b"));
        assert!(definitions.contains_key("macro_c"));
    }

    #[test]
    fn test_macro_source_tracking() {
        let code = r#"
            macro_rules! my_macro {
                () => { 42 };
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        let path = PathBuf::from("src/macros.rs");
        collect_definitions(&ast, &path, definitions.clone());

        let def = definitions.get("my_macro").unwrap();
        assert_eq!(def.source_file, path);
        assert_eq!(def.name, "my_macro");
    }

    #[test]
    fn test_empty_file() {
        let code = r#"
            fn some_function() {
                let x = 42;
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, Path::new("test.rs"), definitions.clone());

        assert_eq!(definitions.len(), 0);
    }

    #[test]
    fn test_macro_with_complex_body() {
        let code = r#"
            macro_rules! complex_logger {
                ($($arg:tt)*) => {
                    {
                        eprintln!("[LOG] {}", format!($($arg)*));
                        std::io::stderr().flush().ok();
                    }
                };
            }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let definitions = Arc::new(DashMap::new());
        collect_definitions(&ast, Path::new("test.rs"), definitions.clone());

        assert_eq!(definitions.len(), 1);
        let def = definitions.get("complex_logger").unwrap();
        assert_eq!(def.name, "complex_logger");
        assert!(!def.body.is_empty());
    }
}
