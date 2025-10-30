use super::import_tracker::{ExportedSymbol, ImportTracker, ImportedSymbol};
use super::namespace::{build_module_namespace, ImportUsage, ModuleNamespace};
use super::observer_registry::ObserverRegistry;
use crate::analysis::python_imports::EnhancedImportResolver;
use crate::analysis::type_flow_tracker::{TypeFlowTracker, TypeId};
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

/// Pending observer dispatch information for cross-file resolution
#[derive(Debug, Clone)]
pub struct PendingObserverDispatch {
    pub for_stmt: ast::StmtFor,
    pub caller: FunctionId,
    pub current_class: Option<String>,
}

#[derive(Debug)]
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
    /// Shared observer registry across all files
    pub observer_registry: Arc<RwLock<ObserverRegistry>>,
    /// Shared type flow tracker across all files
    pub type_flow: Arc<RwLock<TypeFlowTracker>>,
    /// Pending observer dispatches to resolve after all files are analyzed
    pub pending_observer_dispatches: Arc<Mutex<Vec<PendingObserverDispatch>>>,
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
            observer_registry: Arc::clone(&self.observer_registry),
            type_flow: Arc::clone(&self.type_flow),
            pending_observer_dispatches: Arc::clone(&self.pending_observer_dispatches),
        }
    }
}

impl Default for CrossModuleContext {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossModuleContext {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            dependencies: HashMap::new(),
            imports: HashMap::new(),
            exports: HashMap::new(),
            module_trackers: HashMap::new(),
            namespaces: HashMap::new(),
            import_usage: HashMap::new(),
            resolution_cache: RwLock::new(HashMap::new()),
            enhanced_resolver: RwLock::new(None),
            observer_registry: Arc::new(RwLock::new(ObserverRegistry::new())),
            type_flow: Arc::new(RwLock::new(TypeFlowTracker::new())),
            pending_observer_dispatches: Arc::new(Mutex::new(Vec::new())),
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

    /// Get shared observer registry for a file analysis
    pub fn observer_registry(&self) -> Arc<RwLock<ObserverRegistry>> {
        Arc::clone(&self.observer_registry)
    }

    /// Get shared type flow tracker
    pub fn type_flow(&self) -> Arc<RwLock<TypeFlowTracker>> {
        Arc::clone(&self.type_flow)
    }

    /// Resolve an imported type to its source module
    ///
    /// Returns a TypeId with the module path where the type is defined
    pub fn resolve_imported_type(&self, import_name: &str, current_file: &Path) -> Option<TypeId> {
        // Check if this name is imported in the current file
        let imports = self.imports.get(current_file)?;

        for import in imports {
            // Check if the import matches
            if import.name == import_name || import.alias.as_ref() == Some(&import_name.to_string())
            {
                // Resolve the module path
                let source_module = self.resolve_import_path(current_file, &import.module)?;

                return Some(TypeId::new(import_name.to_string(), Some(source_module)));
            }
        }

        None
    }

    /// Record that a type flows into a collection across modules
    pub fn record_cross_module_type_flow(&self, collection: &str, type_id: TypeId) {
        let mut flow = self.type_flow.write().unwrap();
        flow.record_collection_add(collection, type_id);
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

    /// Check the resolution cache for a previously resolved function
    fn check_resolution_cache(&self, cache_key: &(PathBuf, String)) -> Option<Option<FunctionId>> {
        let cache = self.resolution_cache.read().unwrap();
        cache.get(cache_key).cloned()
    }

    /// Update the resolution cache with a new result
    fn update_resolution_cache(&self, cache_key: (PathBuf, String), result: Option<FunctionId>) {
        self.resolution_cache
            .write()
            .unwrap()
            .insert(cache_key, result);
    }

    /// Try to resolve using the enhanced resolver
    fn try_enhanced_resolver(&self, module_path: &Path, name: &str) -> Option<FunctionId> {
        let mut resolver = self.enhanced_resolver.write().unwrap();
        if let Some(ref mut resolver) = *resolver {
            if let Some(resolved) = resolver.resolve_symbol(module_path, name) {
                // Look up the FunctionId from the resolved symbol
                return self
                    .symbols
                    .get(&format!(
                        "{}:{}",
                        resolved.module_path.display(),
                        resolved.name
                    ))
                    .or_else(|| self.symbols.get(&resolved.name))
                    .cloned();
            }
        }
        None
    }

    /// Try to resolve using namespace-based resolution
    fn try_namespace_resolution(&self, module_path: &Path, name: &str) -> Option<FunctionId> {
        let namespace = self.namespaces.get(module_path)?;

        // Try direct import resolution
        if let Some((source_module, original_name)) = namespace.resolve_import(name) {
            if let Some(func_id) = self.resolve_function_in_module(&source_module, &original_name) {
                return Some(func_id);
            }
        }

        // Try wildcard imports
        for wildcard_module in &namespace.wildcard_imports {
            if let Some(func_id) = self.resolve_function_in_module(wildcard_module, name) {
                return Some(func_id);
            }
        }

        None
    }

    /// Try to find a function by scanning exports
    fn try_export_scan(&self, resolved: &str) -> Option<FunctionId> {
        for exports in self.exports.values() {
            for export in exports {
                if export.qualified_name == resolved || export.name == resolved {
                    if let Some(func_id) = self
                        .symbols
                        .get(&export.qualified_name)
                        .or_else(|| self.symbols.get(&export.name))
                    {
                        return Some(func_id.clone());
                    }
                }
            }
        }
        None
    }

    /// Try to resolve using tracker-based resolution
    fn try_tracker_resolution(&self, module_path: &Path, name: &str) -> Option<FunctionId> {
        let tracker = self.module_trackers.get(module_path)?;
        let resolved = tracker.resolve_name(name)?;

        // Try direct resolution
        if let Some(func_id) = self.symbols.get(&resolved) {
            return Some(func_id.clone());
        }

        // Try without module qualification if it's a module.function format
        if let Some(dot_pos) = resolved.rfind('.') {
            let func_name = &resolved[dot_pos + 1..];
            if let Some(func_id) = self.symbols.get(func_name) {
                return Some(func_id.clone());
            }
        }

        // Try to find it in any module's exports
        self.try_export_scan(&resolved)
    }

    /// Try direct lookup in symbols
    fn try_direct_lookup(&self, module_path: &Path, name: &str) -> Option<FunctionId> {
        self.symbols
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
            .cloned()
    }

    pub fn resolve_function(&self, module_path: &Path, name: &str) -> Option<FunctionId> {
        let cache_key = (module_path.to_path_buf(), name.to_string());

        // Check cache first
        if let Some(cached) = self.check_resolution_cache(&cache_key) {
            return cached;
        }

        // Try resolution strategies in order
        let result = self
            .try_enhanced_resolver(module_path, name)
            .or_else(|| self.try_namespace_resolution(module_path, name))
            .or_else(|| self.try_tracker_resolution(module_path, name))
            .or_else(|| self.try_direct_lookup(module_path, name));

        // Update cache
        self.update_resolution_cache(cache_key, result.clone());
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

        // Resolve pending observer dispatches now that all implementations are registered
        self.resolve_observer_dispatches(&mut merged);

        merged
    }

    /// Resolve all pending observer dispatches and add them to the call graph
    ///
    /// This must be called after all files have been analyzed and all observer
    /// implementations have been registered in the observer registry.
    fn resolve_observer_dispatches(&self, call_graph: &mut CallGraph) {
        use super::observer_dispatch::ObserverDispatchDetector;

        let pending = self.pending_observer_dispatches.lock().unwrap();
        let detector = ObserverDispatchDetector::new(self.observer_registry.clone());

        for pending_dispatch in pending.iter() {
            let dispatches = detector.detect_in_for_loop(
                &pending_dispatch.for_stmt,
                pending_dispatch.current_class.as_deref(),
                &pending_dispatch.caller,
            );

            // Add call edges for each detected dispatch
            for dispatch in dispatches {
                if let Some(interface) = &dispatch.observer_interface {
                    let impls: Vec<_> = {
                        let registry = self.observer_registry.read().unwrap();
                        registry
                            .get_implementations(interface, &dispatch.method_name)
                            .into_iter()
                            .cloned()
                            .collect()
                    };

                    for impl_func_id in impls {
                        call_graph.add_call(FunctionCall {
                            caller: dispatch.caller_id.clone(),
                            callee: impl_func_id.clone(),
                            call_type: CallType::ObserverDispatch,
                        });
                    }
                }
            }
        }
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
