//! Import tracking for function call resolution
//!
//! This module provides import analysis capabilities to track:
//! - use statements and their targets
//! - re-exports (pub use)
//! - aliased imports (use x as y)
//! - glob imports (use module::*)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Tracks imports and re-exports for call resolution
#[derive(Debug, Clone)]
pub struct ImportMap {
    /// Maps (file_path, imported_name) -> canonical function path
    /// Example: (src/main.rs, "handle_analyze") -> full module path
    imports: HashMap<(PathBuf, String), Vec<String>>,

    /// Tracks re-exports: (module_path, exported_name) -> original module path
    /// Example: ("commands", "handle_analyze") -> "commands::analyze"
    re_exports: HashMap<(String, String), String>,

    /// Maps file paths to their module paths
    /// Example: src/commands/analyze.rs -> "commands::analyze"
    file_to_module: HashMap<PathBuf, String>,
}

impl ImportMap {
    /// Create a new empty import map
    pub fn new() -> Self {
        Self {
            imports: HashMap::new(),
            re_exports: HashMap::new(),
            file_to_module: HashMap::new(),
        }
    }

    /// Register a file's module path
    pub fn register_file(&mut self, file_path: PathBuf, module_path: String) {
        self.file_to_module.insert(file_path, module_path);
    }

    /// Analyze use statements in a file
    pub fn analyze_imports(&mut self, file_path: &Path, ast: &syn::File) {
        for item in &ast.items {
            if let syn::Item::Use(use_item) = item {
                self.process_use_statement(file_path, use_item);
            }
        }
    }

    /// Process a single use statement
    fn process_use_statement(&mut self, file_path: &Path, use_item: &syn::ItemUse) {
        self.process_use_tree(file_path, &use_item.tree, &[]);
    }

    /// Recursively process use tree
    fn process_use_tree(&mut self, file_path: &Path, tree: &syn::UseTree, prefix: &[String]) {
        match tree {
            syn::UseTree::Path(path) => {
                let mut new_prefix = prefix.to_vec();
                new_prefix.push(path.ident.to_string());
                self.process_use_tree(file_path, &path.tree, &new_prefix);
            }
            syn::UseTree::Name(name) => {
                let mut full_path = prefix.to_vec();
                let imported_name = name.ident.to_string();
                full_path.push(imported_name.clone());

                self.imports
                    .entry((file_path.to_path_buf(), imported_name))
                    .or_default()
                    .push(full_path.join("::"));
            }
            syn::UseTree::Rename(rename) => {
                let mut full_path = prefix.to_vec();
                full_path.push(rename.ident.to_string());

                let alias = rename.rename.to_string();
                self.imports
                    .entry((file_path.to_path_buf(), alias))
                    .or_default()
                    .push(full_path.join("::"));
            }
            syn::UseTree::Glob(_) => {
                // Glob imports require runtime resolution
                // Store the glob prefix for later resolution
                let glob_path = prefix.join("::");
                self.imports
                    .entry((file_path.to_path_buf(), "*".to_string()))
                    .or_default()
                    .push(glob_path);
            }
            syn::UseTree::Group(group) => {
                for tree in &group.items {
                    self.process_use_tree(file_path, tree, prefix);
                }
            }
        }
    }

    /// Record a re-export
    pub fn record_reexport(&mut self, module_path: String, exported_name: String, target: String) {
        self.re_exports.insert((module_path, exported_name), target);
    }

    /// Resolve an imported name in a file to its full module path
    pub fn resolve_import(&self, file_path: &Path, name: &str) -> Option<Vec<String>> {
        // First try direct import lookup
        if let Some(paths) = self
            .imports
            .get(&(file_path.to_path_buf(), name.to_string()))
        {
            return Some(paths.clone());
        }

        // If not found, check glob imports
        self.resolve_through_glob_imports(file_path, name)
    }

    /// Resolve a name through glob imports
    fn resolve_through_glob_imports(&self, file_path: &Path, name: &str) -> Option<Vec<String>> {
        // Get all glob import prefixes for this file
        let glob_prefixes = self
            .imports
            .get(&(file_path.to_path_buf(), "*".to_string()))?;

        // For each glob prefix, construct potential qualified paths
        let mut results = Vec::new();
        for prefix in glob_prefixes {
            let qualified = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}::{}", prefix, name)
            };
            results.push(qualified);
        }

        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }

    /// Resolve through re-exports
    pub fn resolve_reexport(&self, module_path: &str, name: &str) -> Option<String> {
        self.re_exports
            .get(&(module_path.to_string(), name.to_string()))
            .cloned()
    }

    /// Get module path for a file
    pub fn get_module_path(&self, file_path: &Path) -> Option<&String> {
        self.file_to_module.get(file_path)
    }

    /// Try to resolve a qualified path call
    pub fn resolve_qualified_path(&self, path_segments: &[String]) -> Option<String> {
        if path_segments.is_empty() {
            return None;
        }

        // Handle special path prefixes
        match path_segments[0].as_str() {
            "crate" => {
                // Absolute path from crate root
                Some(path_segments[1..].join("::"))
            }
            "super" => {
                // Relative path - requires context from caller
                // This is handled by module_tree
                None
            }
            "self" => {
                // Current module - requires context
                None
            }
            _ => {
                // Regular qualified path
                Some(path_segments.join("::"))
            }
        }
    }
}

impl Default for ImportMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_code(code: &str) -> syn::File {
        syn::parse_str(code).expect("Failed to parse code")
    }

    #[test]
    fn test_simple_import() {
        let mut map = ImportMap::new();
        let file = PathBuf::from("test.rs");

        let code = r#"
            use std::collections::HashMap;
        "#;
        let ast = parse_code(code);
        map.analyze_imports(&file, &ast);

        let resolved = map.resolve_import(&file, "HashMap");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap()[0], "std::collections::HashMap");
    }

    #[test]
    fn test_aliased_import() {
        let mut map = ImportMap::new();
        let file = PathBuf::from("test.rs");

        let code = r#"
            use std::collections::HashMap as Map;
        "#;
        let ast = parse_code(code);
        map.analyze_imports(&file, &ast);

        let resolved = map.resolve_import(&file, "Map");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap()[0], "std::collections::HashMap");
    }

    #[test]
    fn test_grouped_imports() {
        let mut map = ImportMap::new();
        let file = PathBuf::from("test.rs");

        let code = r#"
            use std::collections::{HashMap, HashSet};
        "#;
        let ast = parse_code(code);
        map.analyze_imports(&file, &ast);

        assert!(map.resolve_import(&file, "HashMap").is_some());
        assert!(map.resolve_import(&file, "HashSet").is_some());
    }

    #[test]
    fn test_glob_import() {
        let mut map = ImportMap::new();
        let file = PathBuf::from("test.rs");

        let code = r#"
            use std::collections::*;
        "#;
        let ast = parse_code(code);
        map.analyze_imports(&file, &ast);

        let resolved = map.resolve_import(&file, "*");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap()[0], "std::collections");
    }

    #[test]
    fn test_reexport_tracking() {
        let mut map = ImportMap::new();

        map.record_reexport(
            "commands".to_string(),
            "handle_analyze".to_string(),
            "commands::analyze".to_string(),
        );

        let resolved = map.resolve_reexport("commands", "handle_analyze");
        assert_eq!(resolved, Some("commands::analyze".to_string()));
    }

    #[test]
    fn test_qualified_path_resolution() {
        let map = ImportMap::new();

        // Crate-relative path
        let path = vec![
            "crate".to_string(),
            "commands".to_string(),
            "analyze".to_string(),
        ];
        let resolved = map.resolve_qualified_path(&path);
        assert_eq!(resolved, Some("commands::analyze".to_string()));

        // Regular qualified path
        let path = vec!["module".to_string(), "function".to_string()];
        let resolved = map.resolve_qualified_path(&path);
        assert_eq!(resolved, Some("module::function".to_string()));
    }

    #[test]
    fn test_file_to_module_mapping() {
        let mut map = ImportMap::new();
        let file = PathBuf::from("src/commands/analyze.rs");

        map.register_file(file.clone(), "commands::analyze".to_string());

        let module_path = map.get_module_path(&file);
        assert_eq!(module_path, Some(&"commands::analyze".to_string()));
    }

    #[test]
    fn test_super_import() {
        let mut map = ImportMap::new();
        let file = PathBuf::from("src/builders/unified_analysis.rs");

        let code = r#"
            use super::{call_graph, parallel_call_graph};
        "#;
        let ast = parse_code(code);
        map.analyze_imports(&file, &ast);

        let resolved = map.resolve_import(&file, "call_graph");
        println!("Resolved call_graph import: {:?}", resolved);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap()[0], "super::call_graph");
    }

    #[test]
    fn test_glob_import_resolution() {
        let mut map = ImportMap::new();
        let file = PathBuf::from("test.rs");

        let code = r#"
            use std::collections::*;
        "#;
        let ast = parse_code(code);
        map.analyze_imports(&file, &ast);

        // Should resolve HashMap through glob import
        let resolved = map.resolve_import(&file, "HashMap");
        assert!(resolved.is_some());
        let paths = resolved.unwrap();
        assert!(paths.contains(&"std::collections::HashMap".to_string()));
    }

    #[test]
    fn test_multiple_glob_imports() {
        let mut map = ImportMap::new();
        let file = PathBuf::from("test.rs");

        let code = r#"
            use std::collections::*;
            use std::io::*;
        "#;
        let ast = parse_code(code);
        map.analyze_imports(&file, &ast);

        // Should have multiple possible resolutions for same name
        let resolved = map.resolve_import(&file, "Error");
        assert!(resolved.is_some());
        let paths = resolved.unwrap();
        // Should include both std::collections::Error and std::io::Error
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_specific_import_precedence_over_glob() {
        let mut map = ImportMap::new();
        let file = PathBuf::from("test.rs");

        let code = r#"
            use std::collections::HashMap;
            use std::collections::*;
        "#;
        let ast = parse_code(code);
        map.analyze_imports(&file, &ast);

        // Specific import should be found first
        let resolved = map.resolve_import(&file, "HashMap");
        assert!(resolved.is_some());
        let paths = resolved.unwrap();
        assert!(paths.contains(&"std::collections::HashMap".to_string()));
    }
}
