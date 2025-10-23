use super::import_tracker::{ExportedSymbol, ImportTracker, ImportedSymbol};
use super::namespace::{build_module_namespace, ImportUsage, ModuleNamespace};
use crate::analysis::python_imports::EnhancedImportResolver;
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

#[derive(Debug, Default)]
pub struct CrossModuleContext {
    pub symbols: HashMap<String, FunctionId>,
    pub dependencies: HashMap<PathBuf, Vec<PathBuf>>,
    pub imports: HashMap<PathBuf, Vec<ImportedSymbol>>,
    pub exports: HashMap<PathBuf, Vec<ExportedSymbol>>,
    pub module_trackers: HashMap<PathBuf, ImportTracker>,
    /// Module namespaces for import resolution
    pub namespaces: HashMap<PathBuf, ModuleNamespace>,
    /// Import usage tracking
    pub import_usage: HashMap<PathBuf, Vec<ImportUsage>>,
    /// Resolution cache for performance
    pub resolution_cache: RwLock<HashMap<(PathBuf, String), Option<FunctionId>>>,
    /// Enhanced import resolver for advanced import resolution
    pub enhanced_resolver: RwLock<Option<EnhancedImportResolver>>,
}

impl Clone for CrossModuleContext {
    fn clone(&self) -> Self {
        Self {
            symbols: self.symbols.clone(),
            dependencies: self.dependencies.clone(),
            imports: self.imports.clone(),
            exports: self.exports.clone(),
            module_trackers: self.module_trackers.clone(),
            namespaces: self.namespaces.clone(),
            import_usage: self.import_usage.clone(),
            resolution_cache: RwLock::new(self.resolution_cache.read().unwrap().clone()),
            enhanced_resolver: RwLock::new(self.enhanced_resolver.read().unwrap().clone()),
        }
    }
}

impl CrossModuleContext {
    pub fn new() -> Self {
        Self {
            resolution_cache: RwLock::new(HashMap::new()),
            enhanced_resolver: RwLock::new(None),
            ..Default::default()
        }
    }

    /// Enable enhanced import resolution
    pub fn enable_enhanced_resolution(&self) {
        *self.enhanced_resolver.write().unwrap() = Some(EnhancedImportResolver::new());
    }

    /// Check if enhanced resolution is enabled
    pub fn has_enhanced_resolution(&self) -> bool {
        self.enhanced_resolver.read().unwrap().is_some()
    }

    pub fn add_module(
        &mut self,
        path: PathBuf,
        tracker: ImportTracker,
        exports: Vec<ExportedSymbol>,
        namespace: ModuleNamespace,
    ) {
        let imports = tracker.get_imports().to_vec();

        for import in &imports {
            let dep_path = self.resolve_import_path(&path, &import.module);
            if let Some(dep) = dep_path {
                self.dependencies.entry(path.clone()).or_default().push(dep);
            }
        }

        self.imports.insert(path.clone(), imports);
        self.exports.insert(path.clone(), exports);
        self.module_trackers.insert(path.clone(), tracker);
        self.namespaces.insert(path, namespace);
    }

    fn resolve_import_path(&self, from_path: &Path, module_name: &str) -> Option<PathBuf> {
        if module_name == "." || module_name.is_empty() {
            return Some(from_path.to_path_buf());
        }

        let parent = from_path.parent()?;
        let module_parts: Vec<&str> = module_name.split('.').collect();

        let mut path = parent.to_path_buf();
        for part in module_parts {
            path.push(part);
        }
        path.set_extension("py");

        if path.exists() {
            Some(path)
        } else {
            path.set_extension("");
            if path.join("__init__.py").exists() {
                Some(path.join("__init__.py"))
            } else {
                None
            }
        }
    }

    pub fn register_function(&mut self, module_path: &Path, name: &str, func_id: FunctionId) {
        let qualified_name = format!(
            "{}.{}",
            module_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown"),
            name
        );
        self.symbols.insert(qualified_name.clone(), func_id.clone());
        self.symbols.insert(name.to_string(), func_id.clone());

        let full_path_name = format!("{}:{}", module_path.display(), name);
        self.symbols.insert(full_path_name, func_id);
    }

    pub fn resolve_function(&self, module_path: &Path, name: &str) -> Option<FunctionId> {
        // Check resolution cache first
        let cache_key = (module_path.to_path_buf(), name.to_string());
        {
            let cache = self.resolution_cache.read().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }

        // Try enhanced resolver first if available
        {
            let mut resolver = self.enhanced_resolver.write().unwrap();
            if let Some(ref mut resolver) = *resolver {
                if let Some(resolved) = resolver.resolve_symbol(module_path, name) {
                    // Look up the FunctionId from the resolved symbol
                    let func_id = self
                        .symbols
                        .get(&format!(
                            "{}:{}",
                            resolved.module_path.display(),
                            resolved.name
                        ))
                        .or_else(|| self.symbols.get(&resolved.name))
                        .cloned();

                    if func_id.is_some() {
                        self.resolution_cache
                            .write()
                            .unwrap()
                            .insert(cache_key, func_id.clone());
                        return func_id;
                    }
                }
            }
        }

        // Try namespace-based resolution
        if let Some(namespace) = self.namespaces.get(module_path) {
            if let Some((source_module, original_name)) = namespace.resolve_import(name) {
                // Resolve through the source module
                if let Some(func_id) =
                    self.resolve_function_in_module(&source_module, &original_name)
                {
                    self.resolution_cache
                        .write()
                        .unwrap()
                        .insert(cache_key, Some(func_id.clone()));
                    return Some(func_id);
                }
            }

            // Check wildcard imports
            for wildcard_module in &namespace.wildcard_imports {
                if let Some(func_id) = self.resolve_function_in_module(wildcard_module, name) {
                    self.resolution_cache
                        .write()
                        .unwrap()
                        .insert(cache_key, Some(func_id.clone()));
                    return Some(func_id);
                }
            }
        }

        // Fall back to tracker-based resolution
        if let Some(tracker) = self.module_trackers.get(module_path) {
            if let Some(resolved) = tracker.resolve_name(name) {
                // Try direct resolution first
                if let Some(func_id) = self.symbols.get(&resolved) {
                    self.resolution_cache
                        .write()
                        .unwrap()
                        .insert(cache_key, Some(func_id.clone()));
                    return Some(func_id.clone());
                }

                // Try without module qualification if it's a module.function format
                if let Some(dot_pos) = resolved.rfind('.') {
                    let func_name = &resolved[dot_pos + 1..];
                    if let Some(func_id) = self.symbols.get(func_name) {
                        self.resolution_cache
                            .write()
                            .unwrap()
                            .insert(cache_key, Some(func_id.clone()));
                        return Some(func_id.clone());
                    }
                }

                // Try to find it in any module's exports
                for exports in self.exports.values() {
                    for export in exports {
                        if export.qualified_name == resolved || export.name == resolved {
                            if let Some(func_id) = self
                                .symbols
                                .get(&export.qualified_name)
                                .or_else(|| self.symbols.get(&export.name))
                            {
                                self.resolution_cache
                                    .write()
                                    .unwrap()
                                    .insert(cache_key, Some(func_id.clone()));
                                return Some(func_id.clone());
                            }
                        }
                    }
                }
            }
        }

        // Try direct lookup
        let result = self
            .symbols
            .get(name)
            .or_else(|| {
                let qualified = format!(
                    "{}.{}",
                    module_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown"),
                    name
                );
                self.symbols.get(&qualified)
            })
            .cloned();

        self.resolution_cache
            .write()
            .unwrap()
            .insert(cache_key, result.clone());
        result
    }

    /// Resolve a function within a specific module
    fn resolve_function_in_module(&self, module_path: &Path, name: &str) -> Option<FunctionId> {
        // Check exports of the target module
        if let Some(exports) = self.exports.get(module_path) {
            for export in exports {
                if export.name == name || export.qualified_name == name {
                    // Try to find the FunctionId for this export
                    return self
                        .symbols
                        .get(&export.qualified_name)
                        .or_else(|| self.symbols.get(&export.name))
                        .or_else(|| {
                            let full_path_name = format!("{}:{}", module_path.display(), name);
                            self.symbols.get(&full_path_name)
                        })
                        .cloned();
                }
            }
        }

        // Try direct lookup with module qualification
        let qualified = format!(
            "{}.{}",
            module_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown"),
            name
        );
        self.symbols
            .get(&qualified)
            .or_else(|| {
                let full_path_name = format!("{}:{}", module_path.display(), name);
                self.symbols.get(&full_path_name)
            })
            .cloned()
    }

    pub fn resolve_method(&self, class_name: &str, method_name: &str) -> Option<FunctionId> {
        let qualified = format!("{}.{}", class_name, method_name);
        self.symbols.get(&qualified).cloned()
    }

    pub fn merge_call_graphs(&self, graphs: Vec<CallGraph>) -> CallGraph {
        let mut merged = CallGraph::new();

        for graph in graphs {
            // First copy all functions with their metadata
            for func_id in graph.get_all_functions() {
                if let Some((is_entry, is_test, complexity, lines)) =
                    graph.get_function_info(func_id)
                {
                    merged.add_function(func_id.clone(), is_entry, is_test, complexity, lines);
                } else {
                    // Add with default values if info not found
                    merged.add_function(func_id.clone(), false, false, 0, 0);
                }
            }

            // Then copy all calls
            for call in graph.get_all_calls() {
                merged.add_call(call.clone());
            }
        }

        for (path, imports) in &self.imports {
            for import in imports {
                if let Some(exported_funcs) = self.find_exported_functions(&import.module) {
                    for export in exported_funcs {
                        if let Some(importer_func) = self.find_module_init(path) {
                            if let Some(imported_func) = self.symbols.get(&export.qualified_name) {
                                merged.add_call(FunctionCall {
                                    caller: importer_func.clone(),
                                    callee: imported_func.clone(),
                                    call_type: CallType::Direct,
                                });
                            }
                        }
                    }
                }
            }
        }

        merged
    }

    fn find_exported_functions(&self, module_name: &str) -> Option<&Vec<ExportedSymbol>> {
        for (path, exports) in &self.exports {
            if path
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|name| name == module_name || module_name.ends_with(name))
            {
                return Some(exports);
            }
        }
        None
    }

    fn find_module_init(&self, module_path: &Path) -> Option<&FunctionId> {
        let init_name = format!("{}:<module>", module_path.display());
        self.symbols.get(&init_name)
    }
}

pub struct CrossModuleAnalyzer {
    context: CrossModuleContext,
}

impl Default for CrossModuleAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossModuleAnalyzer {
    pub fn new() -> Self {
        Self {
            context: CrossModuleContext::new(),
        }
    }

    /// Estimate line number for a function by searching for def patterns
    fn estimate_line_number(&self, source_lines: &[String], func_name: &str) -> usize {
        // Handle class methods (e.g., "ClassName.method_name")
        let search_name = if let Some(dot_pos) = func_name.rfind('.') {
            &func_name[dot_pos + 1..]
        } else {
            func_name
        };

        let def_pattern = format!("def {}", search_name);
        let async_def_pattern = format!("async def {}", search_name);

        for (idx, line) in source_lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with(&def_pattern) || trimmed.starts_with(&async_def_pattern) {
                return idx + 1; // Line numbers are 1-based
            }
        }

        0 // Return 0 if not found
    }

    pub fn analyze_file(&mut self, path: &Path, content: &str) -> anyhow::Result<()> {
        let module = rustpython_parser::parse(
            content,
            rustpython_parser::Mode::Module,
            path.to_str().unwrap_or("unknown"),
        )?;

        let mut tracker = ImportTracker::new(path.to_path_buf());

        // Create source lines for line number extraction
        let source_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        if let ast::Mod::Module(module_ast) = &module {
            for stmt in &module_ast.body {
                match stmt {
                    ast::Stmt::Import(import) => {
                        tracker.track_import(import);
                    }
                    ast::Stmt::ImportFrom(import_from) => {
                        tracker.track_import_from(import_from);
                    }
                    _ => {}
                }
            }
        }

        let exports = super::import_tracker::extract_exports(&module);

        // Build module namespace
        let namespace = build_module_namespace(&module, path);

        // Analyze with enhanced resolver if enabled
        {
            let mut resolver = self.context.enhanced_resolver.write().unwrap();
            if let Some(ref mut resolver) = *resolver {
                resolver.analyze_imports(&module, path);
            }
        }

        // Register exported functions in the global symbol table
        for export in &exports {
            if export.is_function || export.is_class {
                // Extract line number for the function
                let line = self.estimate_line_number(&source_lines, &export.name);
                let func_id =
                    FunctionId::new(path.to_path_buf(), export.qualified_name.clone(), line);
                self.context.register_function(path, &export.name, func_id);
            }
        }

        self.context
            .add_module(path.to_path_buf(), tracker, exports, namespace);

        Ok(())
    }

    pub fn get_context(&self) -> &CrossModuleContext {
        &self.context
    }

    pub fn take_context(self) -> CrossModuleContext {
        self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_module_context() {
        let mut context = CrossModuleContext::new();
        let path = PathBuf::from("test.py");
        let tracker = ImportTracker::new(path.clone());
        let exports = vec![];
        let namespace = ModuleNamespace::new();

        context.add_module(path.clone(), tracker, exports, namespace);
        assert!(context.module_trackers.contains_key(&path));
        assert!(context.namespaces.contains_key(&path));
    }

    #[test]
    fn test_function_registration() {
        let mut context = CrossModuleContext::new();
        let path = Path::new("module.py");
        let func_id = FunctionId::new(path.to_path_buf(), "test_func".to_string(), 0);

        context.register_function(path, "test_func", func_id.clone());

        let resolved = context.resolve_function(path, "test_func");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), func_id);
    }
}
