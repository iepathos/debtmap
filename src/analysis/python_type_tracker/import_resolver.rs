//! Import Resolution Module
//!
//! Handles tracking and resolution of Python import statements.
//! Separates import management logic from the main type tracker.

use rustpython_parser::ast;
use std::collections::HashMap;

/// Manages import tracking and resolution for Python modules
#[derive(Debug, Clone)]
pub struct ImportResolver {
    /// Imported modules and their aliases (alias -> module_name)
    imports: HashMap<String, String>,
    /// Imported symbols from modules (name -> (module, original_name))
    from_imports: HashMap<String, (String, Option<String>)>,
}

impl ImportResolver {
    /// Create a new import resolver
    pub fn new() -> Self {
        Self {
            imports: HashMap::new(),
            from_imports: HashMap::new(),
        }
    }

    /// Register a module import (import module as alias)
    pub fn register_import(&mut self, module_name: String, alias: Option<String>) {
        let key = alias.unwrap_or_else(|| module_name.clone());
        self.imports.insert(key, module_name);
    }

    /// Register a from import (from module import name as alias)
    pub fn register_from_import(
        &mut self,
        module_name: String,
        name: String,
        alias: Option<String>,
    ) {
        let key = alias.clone().unwrap_or_else(|| name.clone());
        self.from_imports.insert(key, (module_name, Some(name)));
    }

    /// Resolve an imported name to its fully qualified form
    pub fn resolve_imported_name(&self, name: &str) -> Option<String> {
        // Check if it's an aliased module import
        if let Some(module_name) = self.imports.get(name) {
            return Some(module_name.clone());
        }

        // Check if it's a from import
        if let Some((module, original_name)) = self.from_imports.get(name) {
            if let Some(orig) = original_name {
                return Some(format!("{}.{}", module, orig));
            }
            return Some(module.clone());
        }

        None
    }

    /// Track import statement (import module as alias)
    pub fn track_import_stmt(&mut self, import: &ast::StmtImport) {
        for alias in &import.names {
            let module_name = alias.name.to_string();
            let alias_name = alias.asname.as_ref().map(|s| s.to_string());
            self.register_import(module_name, alias_name);
        }
    }

    /// Track from import statement (from module import name as alias)
    pub fn track_import_from_stmt(&mut self, import_from: &ast::StmtImportFrom) {
        let module_name = import_from
            .module
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| String::from("."));

        for alias in &import_from.names {
            let name = alias.name.to_string();
            let alias_name = alias.asname.as_ref().map(|s| s.to_string());

            // Handle wildcard imports
            if name == "*" {
                // For wildcard imports, we just track the module
                self.register_import(module_name.clone(), None);
            } else {
                self.register_from_import(module_name.clone(), name, alias_name);
            }
        }
    }

    /// Check if a name is an imported function/class
    pub fn is_imported_name(&self, name: &str) -> bool {
        self.imports.contains_key(name) || self.from_imports.contains_key(name)
    }

    /// Get the module for an imported name
    pub fn get_import_module(&self, name: &str) -> Option<String> {
        if let Some(module) = self.imports.get(name) {
            return Some(module.clone());
        }
        if let Some((module, _)) = self.from_imports.get(name) {
            return Some(module.clone());
        }
        None
    }

    /// Get all import strings for framework detection
    pub fn get_all_imports(&self) -> Vec<String> {
        let mut imports = Vec::new();

        // Add module imports
        for (alias, module) in &self.imports {
            if alias == module {
                imports.push(format!("import {}", module));
            } else {
                imports.push(format!("import {} as {}", module, alias));
            }
        }

        // Add from imports
        for (alias, (module, original_name)) in &self.from_imports {
            if let Some(name) = original_name {
                if alias == name {
                    imports.push(format!("from {} import {}", module, name));
                } else {
                    imports.push(format!("from {} import {} as {}", module, name, alias));
                }
            }
        }

        imports
    }
}

impl Default for ImportResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_simple_import() {
        let mut resolver = ImportResolver::new();
        resolver.register_import("os".to_string(), None);

        assert!(resolver.is_imported_name("os"));
        assert_eq!(resolver.get_import_module("os"), Some("os".to_string()));
    }

    #[test]
    fn test_register_aliased_import() {
        let mut resolver = ImportResolver::new();
        resolver.register_import("numpy".to_string(), Some("np".to_string()));

        assert!(resolver.is_imported_name("np"));
        assert_eq!(resolver.get_import_module("np"), Some("numpy".to_string()));
    }

    #[test]
    fn test_register_from_import() {
        let mut resolver = ImportResolver::new();
        resolver.register_from_import("os.path".to_string(), "join".to_string(), None);

        assert!(resolver.is_imported_name("join"));
        assert_eq!(
            resolver.resolve_imported_name("join"),
            Some("os.path.join".to_string())
        );
    }

    #[test]
    fn test_register_aliased_from_import() {
        let mut resolver = ImportResolver::new();
        resolver.register_from_import(
            "collections".to_string(),
            "defaultdict".to_string(),
            Some("dd".to_string()),
        );

        assert!(resolver.is_imported_name("dd"));
        assert_eq!(
            resolver.resolve_imported_name("dd"),
            Some("collections.defaultdict".to_string())
        );
    }

    #[test]
    fn test_get_all_imports() {
        let mut resolver = ImportResolver::new();
        resolver.register_import("os".to_string(), None);
        resolver.register_import("numpy".to_string(), Some("np".to_string()));
        resolver.register_from_import("sys".to_string(), "argv".to_string(), None);

        let imports = resolver.get_all_imports();
        assert!(imports.contains(&"import os".to_string()));
        assert!(imports.contains(&"import numpy as np".to_string()));
        assert!(imports.contains(&"from sys import argv".to_string()));
    }
}
