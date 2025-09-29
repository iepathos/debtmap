//! Module namespace management for Python import resolution
//!
//! Provides namespace building and resolution for imported symbols in Python modules.

use crate::priority::call_graph::FunctionId;
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Module namespace containing imported symbols and their resolution
#[derive(Debug, Clone, Default)]
pub struct ModuleNamespace {
    /// Direct imports: name -> (module_path, original_name)
    pub imports: HashMap<String, (PathBuf, String)>,
    /// Wildcard imports: module_path -> all exported names
    pub wildcard_imports: Vec<PathBuf>,
    /// Import aliases: alias -> original_name
    pub aliases: HashMap<String, String>,
    /// Scope-specific imports (function-level)
    pub scoped_imports: HashMap<String, ModuleNamespace>,
    /// Module imports: alias/name -> module_path
    pub module_imports: HashMap<String, PathBuf>,
}

impl ModuleNamespace {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a direct import (from module import name as alias)
    pub fn add_import(
        &mut self,
        name: String,
        module_path: PathBuf,
        original_name: String,
        alias: Option<String>,
    ) {
        let key = alias.unwrap_or_else(|| name.clone());
        self.imports.insert(key, (module_path, original_name));
    }

    /// Add a wildcard import (from module import *)
    pub fn add_wildcard_import(&mut self, module_path: PathBuf) {
        self.wildcard_imports.push(module_path);
    }

    /// Add an alias mapping
    pub fn add_alias(&mut self, alias: String, original: String) {
        self.aliases.insert(alias, original);
    }

    /// Add a module import (import module as alias)
    pub fn add_module_import(
        &mut self,
        module_name: String,
        module_path: PathBuf,
        alias: Option<String>,
    ) {
        let key = alias.unwrap_or_else(|| module_name.clone());
        self.module_imports.insert(key, module_path);
    }

    /// Resolve an import to its source module and original name
    pub fn resolve_import(&self, call_name: &str) -> Option<(PathBuf, String)> {
        // Check if it's a direct name import
        if let Some((path, name)) = self.imports.get(call_name) {
            return Some((path.clone(), name.clone()));
        }

        // Check if it's accessing a module attribute (e.g., module.function)
        if let Some(dot_pos) = call_name.find('.') {
            let module_part = &call_name[..dot_pos];
            let attr_part = &call_name[dot_pos + 1..];

            // Check if module_part is an imported module
            if let Some(module_path) = self.module_imports.get(module_part) {
                return Some((module_path.clone(), attr_part.to_string()));
            }
        }

        // Check aliases
        if let Some(original) = self.aliases.get(call_name) {
            return self.resolve_import(original);
        }

        None
    }

    /// Check if a name might be from a wildcard import
    pub fn has_wildcard_from(&self, module_path: &PathBuf) -> bool {
        self.wildcard_imports.contains(module_path)
    }

    /// Merge with a parent namespace (for scoped imports)
    pub fn merge_with_parent(&mut self, parent: &ModuleNamespace) {
        // Add parent imports that don't conflict
        for (name, value) in &parent.imports {
            self.imports
                .entry(name.clone())
                .or_insert_with(|| value.clone());
        }

        // Add parent wildcard imports
        for wildcard in &parent.wildcard_imports {
            if !self.wildcard_imports.contains(wildcard) {
                self.wildcard_imports.push(wildcard.clone());
            }
        }

        // Add parent aliases
        for (alias, original) in &parent.aliases {
            self.aliases
                .entry(alias.clone())
                .or_insert_with(|| original.clone());
        }

        // Add parent module imports
        for (name, path) in &parent.module_imports {
            self.module_imports
                .entry(name.clone())
                .or_insert_with(|| path.clone());
        }
    }
}

/// Import usage tracking
#[derive(Debug, Clone)]
pub struct ImportUsage {
    pub import_module: String,
    pub imported_name: String,
    pub alias: Option<String>,
    pub usage_sites: Vec<(String, usize)>, // (function_name, line)
    pub resolved_targets: HashMap<String, FunctionId>,
}

/// Build module namespace from AST
pub fn build_module_namespace(module: &ast::Mod, module_path: &Path) -> ModuleNamespace {
    let mut namespace = ModuleNamespace::new();

    if let ast::Mod::Module(module) = module {
        for stmt in &module.body {
            match stmt {
                ast::Stmt::Import(import) => {
                    process_import(&mut namespace, import, module_path);
                }
                ast::Stmt::ImportFrom(import_from) => {
                    process_import_from(&mut namespace, import_from, module_path);
                }
                _ => {}
            }
        }
    }

    namespace
}

fn process_import(namespace: &mut ModuleNamespace, import: &ast::StmtImport, base_path: &Path) {
    for alias in &import.names {
        let module_name = alias.name.as_str().to_string();
        let alias_name = alias.asname.as_ref().map(|n| n.as_str().to_string());

        // Create a path for the imported module
        let module_path = resolve_module_path(base_path, &module_name);

        namespace.add_module_import(module_name, module_path, alias_name);
    }
}

fn process_import_from(
    namespace: &mut ModuleNamespace,
    import_from: &ast::StmtImportFrom,
    base_path: &Path,
) {
    let module_name = import_from
        .module
        .as_ref()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| ".".to_string());

    let module_path = resolve_module_path(base_path, &module_name);

    for alias in &import_from.names {
        let name = alias.name.as_str();
        let alias_name = alias.asname.as_ref().map(|n| n.as_str().to_string());

        if name == "*" {
            namespace.add_wildcard_import(module_path.clone());
        } else {
            namespace.add_import(
                name.to_string(),
                module_path.clone(),
                name.to_string(),
                alias_name,
            );
        }
    }
}

fn resolve_module_path(base_path: &Path, module_name: &str) -> PathBuf {
    if module_name == "." || module_name.is_empty() {
        return base_path.to_path_buf();
    }

    let parent = base_path.parent().unwrap_or(base_path);
    let module_parts: Vec<&str> = module_name.split('.').collect();

    let mut path = parent.to_path_buf();
    for part in module_parts {
        path.push(part);
    }
    path.set_extension("py");

    // Check if the file exists, otherwise try as a package
    if !path.exists() {
        path.set_extension("");
        path.push("__init__.py");
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_namespace_import_resolution() {
        let mut namespace = ModuleNamespace::new();
        let module_path = PathBuf::from("/project/utils.py");

        // Add a direct import: from utils import helper as h
        namespace.add_import(
            "helper".to_string(),
            module_path.clone(),
            "helper".to_string(),
            Some("h".to_string()),
        );

        // Test alias resolution
        let resolved = namespace.resolve_import("h");
        assert!(resolved.is_some());
        let (path, name) = resolved.unwrap();
        assert_eq!(path, module_path);
        assert_eq!(name, "helper");
    }

    #[test]
    fn test_module_import_resolution() {
        let mut namespace = ModuleNamespace::new();
        let module_path = PathBuf::from("/project/utils.py");

        // Add a module import: import utils as u
        namespace.add_module_import(
            "utils".to_string(),
            module_path.clone(),
            Some("u".to_string()),
        );

        // Test module.function resolution
        let resolved = namespace.resolve_import("u.helper");
        assert!(resolved.is_some());
        let (path, name) = resolved.unwrap();
        assert_eq!(path, module_path);
        assert_eq!(name, "helper");
    }

    #[test]
    fn test_wildcard_import() {
        let mut namespace = ModuleNamespace::new();
        let module_path = PathBuf::from("/project/utils.py");

        // Add a wildcard import: from utils import *
        namespace.add_wildcard_import(module_path.clone());

        // Check wildcard tracking
        assert!(namespace.has_wildcard_from(&module_path));
    }
}
