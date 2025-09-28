use super::import_tracker::{ExportedSymbol, ImportTracker, ImportedSymbol};
use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone)]
pub struct CrossModuleContext {
    pub symbols: HashMap<String, FunctionId>,
    pub dependencies: HashMap<PathBuf, Vec<PathBuf>>,
    pub imports: HashMap<PathBuf, Vec<ImportedSymbol>>,
    pub exports: HashMap<PathBuf, Vec<ExportedSymbol>>,
    pub module_trackers: HashMap<PathBuf, ImportTracker>,
}

impl CrossModuleContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_module(
        &mut self,
        path: PathBuf,
        tracker: ImportTracker,
        exports: Vec<ExportedSymbol>,
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
        self.module_trackers.insert(path, tracker);
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
        if let Some(tracker) = self.module_trackers.get(module_path) {
            if let Some(resolved) = tracker.resolve_name(name) {
                if let Some(func_id) = self.symbols.get(&resolved) {
                    return Some(func_id.clone());
                }
            }
        }

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
                .map_or(false, |name| {
                    name == module_name || module_name.ends_with(name)
                })
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

impl CrossModuleAnalyzer {
    pub fn new() -> Self {
        Self {
            context: CrossModuleContext::new(),
        }
    }

    pub fn analyze_file(&mut self, path: &Path, content: &str) -> anyhow::Result<()> {
        let module = rustpython_parser::parse(
            content,
            rustpython_parser::Mode::Module,
            path.to_str().unwrap_or("unknown"),
        )?;

        let mut tracker = ImportTracker::new(path.to_path_buf());

        if let ast::Mod::Module(module) = &module {
            for stmt in &module.body {
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

        // Register exported functions in the global symbol table
        for export in &exports {
            if export.is_function || export.is_class {
                let func_id = FunctionId {
                    name: export.qualified_name.clone(),
                    file: path.to_path_buf(),
                    line: 0, // We could extract line numbers if needed
                };
                self.context.register_function(path, &export.name, func_id);
            }
        }

        self.context
            .add_module(path.to_path_buf(), tracker, exports);

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

        context.add_module(path.clone(), tracker, exports);
        assert!(context.module_trackers.contains_key(&path));
    }

    #[test]
    fn test_function_registration() {
        let mut context = CrossModuleContext::new();
        let path = Path::new("module.py");
        let func_id = FunctionId {
            name: "test_func".to_string(),
            file: path.to_path_buf(),
            line: 0,
        };

        context.register_function(path, "test_func", func_id.clone());

        let resolved = context.resolve_function(path, "test_func");
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), func_id);
    }
}
