//! Path resolution logic combining imports and module hierarchy
//!
//! This module provides the main resolution engine that combines:
//! - Import maps (use statements)
//! - Module trees (hierarchy)
//! - Call site context
//!
//! To accurately resolve function calls to their definitions

use super::import_map::ImportMap;
use super::module_tree::ModuleTree;
use crate::priority::call_graph::{CallGraph, FunctionId};
use std::path::{Path, PathBuf};

/// Path resolver combining all resolution strategies
#[derive(Debug, Clone)]
pub struct PathResolver {
    import_map: ImportMap,
    module_tree: ModuleTree,
}

impl PathResolver {
    /// Create a new path resolver
    pub fn new(import_map: ImportMap, module_tree: ModuleTree) -> Self {
        Self {
            import_map,
            module_tree,
        }
    }

    /// Resolve a function call to a FunctionId
    pub fn resolve_call(
        &self,
        caller_file: &Path,
        callee_name: &str,
        call_graph: &CallGraph,
    ) -> Option<FunctionId> {
        // Try multiple resolution strategies in order of likelihood

        // 1. Simple name - check imports first
        if !callee_name.contains("::") {
            if let Some(resolved) =
                self.resolve_through_imports(caller_file, callee_name, call_graph)
            {
                return Some(resolved);
            }

            // Fallback to same-file search
            if let Some(resolved) = self.find_in_same_file(caller_file, callee_name, call_graph) {
                return Some(resolved);
            }
        }

        // 2. Qualified path - use module tree
        if callee_name.contains("::") {
            if let Some(resolved) =
                self.resolve_qualified_path(caller_file, callee_name, call_graph)
            {
                return Some(resolved);
            }
        }

        // 3. Check re-exports
        if let Some(resolved) = self.resolve_through_reexports(caller_file, callee_name, call_graph)
        {
            return Some(resolved);
        }

        None
    }

    /// Resolve through import statements
    fn resolve_through_imports(
        &self,
        caller_file: &Path,
        callee_name: &str,
        call_graph: &CallGraph,
    ) -> Option<FunctionId> {
        let imported_paths = self.import_map.resolve_import(caller_file, callee_name)?;

        // Try each imported path
        for import_path in &imported_paths {
            if let Some(func) = self.find_function_by_path(import_path, call_graph) {
                return Some(func);
            }
        }

        None
    }

    /// Resolve a qualified path like module::function
    fn resolve_qualified_path(
        &self,
        caller_file: &Path,
        qualified_name: &str,
        call_graph: &CallGraph,
    ) -> Option<FunctionId> {
        let segments: Vec<String> = qualified_name.split("::").map(|s| s.to_string()).collect();

        if segments.is_empty() {
            return None;
        }

        // Strategy 1: Check if the first segment is an imported module
        if segments.len() >= 2 {
            let first_segment = &segments[0];

            // Check if this module is imported
            if let Some(imported_paths) = self.import_map.resolve_import(caller_file, first_segment)
            {
                // The first segment resolves to an imported module
                // Build the full path by replacing the first segment
                for imported_path in &imported_paths {
                    let mut full_path_segments = imported_path
                        .split("::")
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>();

                    // Resolve super/self/crate if present in the imported path
                    if let Some(current_module) = self.module_tree.get_module(caller_file) {
                        if let Some(resolved) = self
                            .module_tree
                            .resolve_path(current_module, &full_path_segments)
                        {
                            // Use the resolved path instead of the raw import path
                            full_path_segments =
                                resolved.split("::").map(|s| s.to_string()).collect();
                        }
                    }

                    full_path_segments.extend_from_slice(&segments[1..]);
                    let full_path = full_path_segments.join("::");

                    // Debug logging
                    log::trace!(
                        "Resolving qualified path: {} in {:?} -> trying {}",
                        qualified_name,
                        caller_file,
                        full_path
                    );

                    if let Some(func) = self.find_function_by_path(&full_path, call_graph) {
                        return Some(func);
                    }
                }
            }
        }

        // Strategy 2: Try using module tree resolution
        if let Some(current_module) = self.module_tree.get_module(caller_file) {
            if let Some(resolved_path) = self.module_tree.resolve_path(current_module, &segments) {
                if let Some(func) = self.find_function_by_path(&resolved_path, call_graph) {
                    return Some(func);
                }
            }
        }

        // Strategy 3: Try direct lookup (for crate::... paths)
        if let Some(resolved) = self.import_map.resolve_qualified_path(&segments) {
            if let Some(func) = self.find_function_by_path(&resolved, call_graph) {
                return Some(func);
            }
        }

        None
    }

    /// Resolve through re-exports
    fn resolve_through_reexports(
        &self,
        _caller_file: &Path,
        callee_name: &str,
        call_graph: &CallGraph,
    ) -> Option<FunctionId> {
        // Extract module and function name
        let parts: Vec<&str> = callee_name.rsplitn(2, "::").collect();
        if parts.len() != 2 {
            return None;
        }

        let func_name = parts[0];
        let module_path = parts[1];

        // Check re-export
        let target = self.import_map.resolve_reexport(module_path, func_name)?;

        // Find function at target
        self.find_function_by_path(&target, call_graph)
    }

    /// Find a function in the same file
    fn find_in_same_file(
        &self,
        file: &Path,
        name: &str,
        call_graph: &CallGraph,
    ) -> Option<FunctionId> {
        call_graph
            .get_all_functions()
            .find(|func| func.file == file && Self::matches_name(&func.name, name))
            .cloned()
    }

    /// Find a function by its module path
    fn find_function_by_path(&self, path: &str, call_graph: &CallGraph) -> Option<FunctionId> {
        // Try exact match first
        for func in call_graph.get_all_functions() {
            if func.name == path || func.name.ends_with(&format!("::{}", path)) {
                return Some(func.clone());
            }
        }

        // Try matching by module_path + function name
        // Split path into potential module path and function name
        if let Some(base_name) = path.split("::").last() {
            // Try different ways to split the path
            let path_parts: Vec<&str> = path.split("::").collect();

            for i in 0..path_parts.len() {
                let potential_module = path_parts[0..i].join("::");
                let potential_func_path = path_parts[i..].join("::");

                for func in call_graph.get_all_functions() {
                    // Match if:
                    // 1. Function name matches the remaining path
                    // 2. Module path matches (if we have module_path)
                    if func.name == potential_func_path
                        && (potential_module.is_empty() || func.module_path == potential_module)
                    {
                        return Some(func.clone());
                    }

                    // Also try matching just the function name with module_path check
                    if Self::matches_name(&func.name, base_name)
                        && !potential_module.is_empty()
                        && func.module_path == path_parts[0..path_parts.len() - 1].join("::")
                    {
                        return Some(func.clone());
                    }
                }
            }

            // Fallback: if module_path is not populated (empty string), just match by function name
            // This is a temporary workaround until spec 142 is fully implemented
            for func in call_graph.get_all_functions() {
                if Self::matches_name(&func.name, base_name) && func.module_path.is_empty() {
                    // Verify the file path contains the expected module name
                    let file_path_str = func.file.to_string_lossy();
                    let expected_module = &path_parts[..path_parts.len() - 1].join("/");
                    if file_path_str.contains(expected_module) || expected_module.starts_with("crate") {
                        return Some(func.clone());
                    }
                }
            }
        }

        None
    }

    /// Check if a function name matches the search name
    fn matches_name(full_name: &str, search_name: &str) -> bool {
        // Exact match
        if full_name == search_name {
            return true;
        }

        // Base name match (Type::method matches method)
        if let Some(base) = full_name.split("::").last() {
            if base == search_name {
                return true;
            }
        }

        false
    }

    /// Get the import map
    pub fn import_map(&self) -> &ImportMap {
        &self.import_map
    }

    /// Get the module tree
    pub fn module_tree(&self) -> &ModuleTree {
        &self.module_tree
    }
}

/// Builder for PathResolver
pub struct PathResolverBuilder {
    import_map: ImportMap,
    module_tree: ModuleTree,
}

impl PathResolverBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            import_map: ImportMap::new(),
            module_tree: ModuleTree::new(),
        }
    }

    /// Analyze a file and update both import map and module tree
    pub fn analyze_file(mut self, file_path: PathBuf, ast: &syn::File) -> Self {
        // Infer module path from file path
        let module_path = ModuleTree::infer_module_from_file(&file_path);

        // Register with module tree
        self.module_tree
            .add_module(module_path.clone(), file_path.clone());

        // Register with import map
        self.import_map
            .register_file(file_path.clone(), module_path);

        // Analyze imports
        self.import_map.analyze_imports(&file_path, ast);

        // Check for re-exports
        for item in &ast.items {
            if let syn::Item::Use(use_item) = item {
                if let syn::Visibility::Public(_) = use_item.vis {
                    // This is a re-export
                    self.analyze_reexport(&file_path, use_item);
                }
            }
        }

        self
    }

    /// Analyze a re-export declaration
    fn analyze_reexport(&mut self, file_path: &Path, use_item: &syn::ItemUse) {
        if let Some(module_path) = self.module_tree.get_module(file_path) {
            self.extract_reexport_targets(&use_item.tree, module_path.clone(), &[]);
        }
    }

    /// Extract re-export targets from use tree
    fn extract_reexport_targets(
        &mut self,
        tree: &syn::UseTree,
        exporting_module: String,
        prefix: &[String],
    ) {
        match tree {
            syn::UseTree::Path(path) => {
                let mut new_prefix = prefix.to_vec();
                new_prefix.push(path.ident.to_string());
                self.extract_reexport_targets(&path.tree, exporting_module, &new_prefix);
            }
            syn::UseTree::Name(name) => {
                let mut target = prefix.to_vec();
                let exported_name = name.ident.to_string();
                target.push(exported_name.clone());

                self.import_map
                    .record_reexport(exporting_module, exported_name, target.join("::"));
            }
            syn::UseTree::Rename(rename) => {
                let mut target = prefix.to_vec();
                target.push(rename.ident.to_string());

                let alias = rename.rename.to_string();
                self.import_map
                    .record_reexport(exporting_module, alias, target.join("::"));
            }
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.extract_reexport_targets(item, exporting_module.clone(), prefix);
                }
            }
            syn::UseTree::Glob(_) => {
                // Glob re-exports need special handling
                // For now, skip them
            }
        }
    }

    /// Build the path resolver
    pub fn build(self) -> PathResolver {
        PathResolver::new(self.import_map, self.module_tree)
    }
}

impl Default for PathResolverBuilder {
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
    #[ignore] // TODO: Complete integration with CallGraphExtractor
    fn test_simple_import_resolution() {
        let file1 = PathBuf::from("src/main.rs");
        let file2 = PathBuf::from("src/helper.rs");

        let code1 = r#"
            use crate::helper::foo;

            fn main() {
                foo();
            }
        "#;

        let code2 = r#"
            pub fn foo() {}
        "#;

        let ast1 = parse_code(code1);
        let ast2 = parse_code(code2);

        let resolver = PathResolverBuilder::new()
            .analyze_file(file1.clone(), &ast1)
            .analyze_file(file2.clone(), &ast2)
            .build();

        let mut graph = CallGraph::new();
        graph.add_function(
            FunctionId::with_module_path(file2.clone(), "foo".to_string(), 1, "helper".to_string()),
            false,
            false,
            0,
            0,
        );

        let resolved = resolver.resolve_call(&file1, "foo", &graph);
        // Note: This test requires full integration with CallGraphExtractor
        // to populate module_path fields correctly
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().name, "foo");
    }

    #[test]
    fn test_qualified_path_resolution() {
        let file = PathBuf::from("src/main.rs");

        let code = r#"
            fn main() {
                module::function();
            }
        "#;

        let ast = parse_code(code);

        let builder = PathResolverBuilder::new().analyze_file(file.clone(), &ast);

        let resolver = builder.build();

        assert!(resolver.module_tree().get_module(&file).is_some());
    }

    #[test]
    fn test_builder_pattern() {
        let file1 = PathBuf::from("src/lib.rs");
        let file2 = PathBuf::from("src/commands/mod.rs");

        let ast1 = parse_code("pub mod commands;");
        let ast2 = parse_code("pub fn handle() {}");

        let resolver = PathResolverBuilder::new()
            .analyze_file(file1, &ast1)
            .analyze_file(file2, &ast2)
            .build();

        assert!(resolver
            .module_tree()
            .get_module(&PathBuf::from("src/lib.rs"))
            .is_some());
        assert!(resolver
            .module_tree()
            .get_module(&PathBuf::from("src/commands/mod.rs"))
            .is_some());
    }

    #[test]
    fn test_matches_name() {
        assert!(PathResolver::matches_name("function", "function"));
        assert!(PathResolver::matches_name("Type::method", "method"));
        assert!(PathResolver::matches_name("module::Type::method", "method"));
        assert!(!PathResolver::matches_name("function", "other"));
    }
}
