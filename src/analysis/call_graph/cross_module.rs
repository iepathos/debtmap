//! Cross-Module Dependency Tracking
//!
//! This module tracks cross-module dependencies and public APIs to reduce
//! false positives in dead code detection for public functions and exports.

use crate::priority::call_graph::FunctionId;
use anyhow::Result;
use im::{HashMap, HashSet, Vector};
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{File, ItemFn, ItemMod, ItemUse, Path as SynPath, UseTree, Visibility};

/// Information about a module boundary
#[derive(Debug, Clone)]
pub struct ModuleBoundary {
    /// Module path (e.g., "crate::utils::helpers")
    pub module_path: String,
    /// File path for this module
    pub file_path: PathBuf,
    /// Parent module (if any)
    pub parent_module: Option<String>,
    /// Submodules
    pub submodules: HashSet<String>,
    /// Public exports from this module
    pub public_exports: Vector<PublicExport>,
}

/// Information about a public export
#[derive(Debug, Clone)]
pub struct PublicExport {
    /// Name of the exported item
    pub name: String,
    /// Type of export (function, type, constant, etc.)
    pub export_type: ExportType,
    /// Function ID (if this is a function export)
    pub function_id: Option<FunctionId>,
    /// Visibility level
    pub visibility: VisibilityLevel,
    /// Line number of the export
    pub line: usize,
}

/// Type of export
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportType {
    Function,
    Type,
    Constant,
    Module,
    Macro,
    Trait,
}

/// Visibility levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisibilityLevel {
    Private,
    Crate,
    Public,
    PublicSuper,
    PublicIn(String),
}

/// Information about public API functions
#[derive(Debug, Clone)]
pub struct PublicApiInfo {
    /// Function ID
    pub function_id: FunctionId,
    /// Module where this function is defined
    pub defining_module: String,
    /// Visibility level
    pub visibility: VisibilityLevel,
    /// Whether this function is re-exported
    pub is_reexported: bool,
    /// Modules that import this function
    pub importing_modules: HashSet<String>,
}

/// Information about a cross-module function call
#[derive(Debug, Clone)]
pub struct CrossModuleCall {
    /// Function making the call
    pub caller: FunctionId,
    /// Module path being called
    pub module_path: String,
    /// Function name being called
    pub function_name: String,
    /// Line number of the call
    pub line: usize,
    /// Whether this call is through a use statement
    pub through_import: bool,
}

/// Information about module imports
#[derive(Debug, Clone)]
pub struct ModuleImport {
    /// Module doing the importing
    pub importing_module: String,
    /// Module being imported from
    pub imported_module: String,
    /// Specific items imported
    pub imported_items: Vector<String>,
    /// Whether this is a glob import (use module::*)
    pub is_glob_import: bool,
    /// Line number of the import
    pub line: usize,
}

/// Tracker for cross-module dependencies and public APIs
#[derive(Debug, Clone)]
pub struct CrossModuleTracker {
    /// All module boundaries discovered
    module_boundaries: HashMap<String, ModuleBoundary>,
    /// Public API functions
    public_apis: HashMap<FunctionId, PublicApiInfo>,
    /// Cross-module calls that need resolution
    cross_module_calls: Vector<CrossModuleCall>,
    /// Module imports
    module_imports: Vector<ModuleImport>,
    /// Mapping from file paths to module paths
    file_to_module: HashMap<PathBuf, String>,
    /// Re-exports mapping
    reexports: HashMap<String, Vector<String>>,
}

impl CrossModuleTracker {
    /// Create a new cross-module tracker
    pub fn new() -> Self {
        Self {
            module_boundaries: HashMap::new(),
            public_apis: HashMap::new(),
            cross_module_calls: Vector::new(),
            module_imports: Vector::new(),
            file_to_module: HashMap::new(),
            reexports: HashMap::new(),
        }
    }

    /// Analyze workspace files for cross-module dependencies
    pub fn analyze_workspace(&mut self, workspace_files: &[(PathBuf, File)]) -> Result<()> {
        // First pass: Build module structure
        for (file_path, ast) in workspace_files {
            let module_path = self.infer_module_path(file_path);
            self.file_to_module
                .insert(file_path.clone(), module_path.clone());

            let mut visitor = ModuleVisitor::new(file_path.clone(), module_path.clone());
            visitor.visit_file(ast);

            // Add module boundary
            let boundary = ModuleBoundary {
                module_path: module_path.clone(),
                file_path: file_path.clone(),
                parent_module: self.infer_parent_module(&module_path),
                submodules: visitor.submodules.into_iter().collect(),
                public_exports: visitor.public_exports.into_iter().collect(),
            };

            self.module_boundaries.insert(module_path, boundary);
        }

        // Second pass: Analyze imports and cross-module calls
        for (file_path, ast) in workspace_files {
            let module_path = self.file_to_module.get(file_path).unwrap().clone();

            let mut call_visitor = CrossModuleCallVisitor::new(module_path.clone());
            call_visitor.visit_file(ast);

            // Add cross-module calls
            for call in call_visitor.cross_module_calls {
                self.cross_module_calls.push_back(call);
            }

            // Add module imports
            for import in call_visitor.module_imports {
                self.module_imports.push_back(import);
            }
        }

        // Third pass: Build public API mappings
        self.build_public_api_mappings();

        Ok(())
    }

    /// Get all cross-module calls
    pub fn get_cross_module_calls(&self) -> Vector<CrossModuleCall> {
        self.cross_module_calls.clone()
    }

    /// Get all public APIs
    pub fn get_public_apis(&self) -> Vec<PublicApiInfo> {
        self.public_apis.values().cloned().collect()
    }

    /// Check if a function is a public API
    pub fn is_public_api(&self, func_id: &FunctionId) -> bool {
        self.public_apis.contains_key(func_id)
    }

    /// Resolve a cross-module call to a specific function
    pub fn resolve_module_call(
        &self,
        module_path: &str,
        function_name: &str,
    ) -> Option<FunctionId> {
        // Look for the function in the target module
        if let Some(boundary) = self.module_boundaries.get(module_path) {
            for export in &boundary.public_exports {
                if export.name == function_name && export.export_type == ExportType::Function {
                    return export.function_id.clone();
                }
            }
        }

        // Check re-exports
        if let Some(reexported_modules) = self.reexports.get(module_path) {
            for reexported_module in reexported_modules {
                if let Some(func_id) = self.resolve_module_call(reexported_module, function_name) {
                    return Some(func_id);
                }
            }
        }

        None
    }

    /// Get statistics about cross-module usage
    pub fn get_statistics(&self) -> CrossModuleStatistics {
        let total_modules = self.module_boundaries.len();
        let total_public_apis = self.public_apis.len();
        let total_cross_module_calls = self.cross_module_calls.len();
        let total_imports = self.module_imports.len();

        let public_functions = self
            .public_apis
            .values()
            .filter(|api| matches!(api.visibility, VisibilityLevel::Public))
            .count();

        let crate_functions = self
            .public_apis
            .values()
            .filter(|api| matches!(api.visibility, VisibilityLevel::Crate))
            .count();

        CrossModuleStatistics {
            total_modules,
            total_public_apis,
            total_cross_module_calls,
            total_imports,
            public_functions,
            crate_functions,
        }
    }

    /// Get functions that should be excluded from dead code analysis
    pub fn get_public_exclusions(&self) -> HashSet<FunctionId> {
        self.public_apis
            .iter()
            .filter(|(_, api)| matches!(api.visibility, VisibilityLevel::Public))
            .map(|(func_id, _)| func_id.clone())
            .collect()
    }

    /// Get crate-visible functions that might be used by other modules
    pub fn get_crate_visible_functions(&self) -> HashSet<FunctionId> {
        self.public_apis
            .iter()
            .filter(|(_, api)| !matches!(api.visibility, VisibilityLevel::Private))
            .map(|(func_id, _)| func_id.clone())
            .collect()
    }

    /// Infer module path from file path
    fn infer_module_path(&self, file_path: &Path) -> String {
        // This is a simplified heuristic - in a real implementation,
        // we'd need to parse mod.rs files and follow the module tree

        let path_str = file_path.to_string_lossy();

        // Remove src/ prefix and .rs suffix
        let relative_path = path_str
            .strip_prefix("src/")
            .unwrap_or(&path_str)
            .strip_suffix(".rs")
            .unwrap_or(&path_str);

        // Replace path separators with module separators
        let module_path = relative_path.replace('/', "::");

        // Handle lib.rs and main.rs
        match module_path.as_str() {
            "lib" => "crate".to_string(),
            "main" => "crate".to_string(),
            _ => format!("crate::{module_path}"),
        }
    }

    /// Infer parent module from module path
    fn infer_parent_module(&self, module_path: &str) -> Option<String> {
        if module_path == "crate" {
            None
        } else {
            let parts: Vec<&str> = module_path.split("::").collect();
            if parts.len() > 1 {
                Some(parts[..parts.len() - 1].join("::"))
            } else {
                Some("crate".to_string())
            }
        }
    }

    /// Build public API mappings from discovered exports
    fn build_public_api_mappings(&mut self) {
        for (module_path, boundary) in &self.module_boundaries {
            for export in &boundary.public_exports {
                if export.export_type == ExportType::Function {
                    if let Some(func_id) = &export.function_id {
                        let api_info = PublicApiInfo {
                            function_id: func_id.clone(),
                            defining_module: module_path.clone(),
                            visibility: export.visibility.clone(),
                            is_reexported: false, // Would need more analysis
                            importing_modules: HashSet::new(), // Would be filled by usage analysis
                        };

                        self.public_apis.insert(func_id.clone(), api_info);
                    }
                }
            }
        }
    }
}

/// Statistics about cross-module usage
#[derive(Debug, Clone)]
pub struct CrossModuleStatistics {
    pub total_modules: usize,
    pub total_public_apis: usize,
    pub total_cross_module_calls: usize,
    pub total_imports: usize,
    pub public_functions: usize,
    pub crate_functions: usize,
}

/// Visitor for analyzing module structure and exports
struct ModuleVisitor {
    file_path: PathBuf,
    module_path: String,
    submodules: Vec<String>,
    public_exports: Vec<PublicExport>,
}

impl ModuleVisitor {
    fn new(file_path: PathBuf, module_path: String) -> Self {
        Self {
            file_path,
            module_path,
            submodules: Vec::new(),
            public_exports: Vec::new(),
        }
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn extract_visibility(&self, vis: &Visibility) -> VisibilityLevel {
        match vis {
            Visibility::Public(_) => VisibilityLevel::Public,
            // In newer syn versions, handle Restricted visibility
            Visibility::Restricted(restricted) => {
                if restricted.in_token.is_some() {
                    // pub(in path)
                    if let Some(path) = restricted.path.get_ident() {
                        VisibilityLevel::PublicIn(path.to_string())
                    } else {
                        VisibilityLevel::Crate
                    }
                } else {
                    // pub(super) or pub(crate)
                    if let Some(ident) = restricted.path.get_ident() {
                        match ident.to_string().as_str() {
                            "super" => VisibilityLevel::PublicSuper,
                            "crate" => VisibilityLevel::Crate,
                            _ => VisibilityLevel::Crate,
                        }
                    } else {
                        VisibilityLevel::Crate
                    }
                }
            }
            Visibility::Inherited => VisibilityLevel::Private,
        }
    }
}

impl<'ast> Visit<'ast> for ModuleVisitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let visibility = self.extract_visibility(&item.vis);

        // Only track public or crate-visible functions
        if !matches!(visibility, VisibilityLevel::Private) {
            let func_name = item.sig.ident.to_string();
            let line = self.get_line_number(item.sig.ident.span());

            let func_id = FunctionId {
                file: self.file_path.clone(),
                name: func_name.clone(),
                line,
            };

            let export = PublicExport {
                name: func_name,
                export_type: ExportType::Function,
                function_id: Some(func_id),
                visibility,
                line,
            };

            self.public_exports.push(export);
        }

        // Continue visiting
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast ItemMod) {
        let mod_name = item.ident.to_string();
        let full_module_path = format!("{}::{}", self.module_path, mod_name);

        self.submodules.push(full_module_path);

        // Continue visiting
        syn::visit::visit_item_mod(self, item);
    }
}

/// Visitor for analyzing cross-module calls and imports
struct CrossModuleCallVisitor {
    current_module: String,
    cross_module_calls: Vec<CrossModuleCall>,
    module_imports: Vec<ModuleImport>,
    current_function: Option<FunctionId>,
}

impl CrossModuleCallVisitor {
    fn new(current_module: String) -> Self {
        Self {
            current_module,
            cross_module_calls: Vec::new(),
            module_imports: Vec::new(),
            current_function: None,
        }
    }

    fn get_line_number(&self, span: proc_macro2::Span) -> usize {
        span.start().line
    }

    fn extract_path_string(&self, path: &SynPath) -> String {
        path.segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    }

    fn analyze_use_tree(&mut self, use_tree: &UseTree, line: usize) {
        match use_tree {
            UseTree::Path(use_path) => {
                let _path_segment = use_path.ident.to_string();
                // Recursively analyze the rest of the path
                self.analyze_use_tree(&use_path.tree, line);
            }
            UseTree::Name(use_name) => {
                let imported_item = use_name.ident.to_string();

                let import = ModuleImport {
                    importing_module: self.current_module.clone(),
                    imported_module: "unknown".to_string(), // Would need full path resolution
                    imported_items: vec![imported_item].into_iter().collect(),
                    is_glob_import: false,
                    line,
                };

                self.module_imports.push(import);
            }
            UseTree::Glob(_) => {
                let import = ModuleImport {
                    importing_module: self.current_module.clone(),
                    imported_module: "unknown".to_string(),
                    imported_items: Vector::new(),
                    is_glob_import: true,
                    line,
                };

                self.module_imports.push(import);
            }
            UseTree::Group(use_group) => {
                // Analyze each item in the group
                for item in &use_group.items {
                    self.analyze_use_tree(item, line);
                }
            }
            UseTree::Rename(use_rename) => {
                let imported_item = use_rename.ident.to_string();

                let import = ModuleImport {
                    importing_module: self.current_module.clone(),
                    imported_module: "unknown".to_string(),
                    imported_items: vec![imported_item].into_iter().collect(),
                    is_glob_import: false,
                    line,
                };

                self.module_imports.push(import);
            }
        }
    }
}

impl<'ast> Visit<'ast> for CrossModuleCallVisitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        let func_name = item.sig.ident.to_string();
        let line = self.get_line_number(item.sig.ident.span());

        self.current_function = Some(FunctionId {
            file: PathBuf::new(), // Will be filled in by parent
            name: func_name,
            line,
        });

        // Continue visiting the function body
        syn::visit::visit_item_fn(self, item);

        self.current_function = None;
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        let line = self.get_line_number(item.use_token.span);
        self.analyze_use_tree(&item.tree, line);

        // Continue visiting
        syn::visit::visit_item_use(self, item);
    }

    fn visit_expr_call(&mut self, expr: &'ast syn::ExprCall) {
        if let Some(caller) = &self.current_function {
            if let syn::Expr::Path(path_expr) = &*expr.func {
                let path_string = self.extract_path_string(&path_expr.path);
                let line = self.get_line_number(path_expr.path.span());

                // Check if this is a cross-module call (contains ::)
                if path_string.contains("::") {
                    let parts: Vec<&str> = path_string.rsplitn(2, "::").collect();
                    if parts.len() == 2 {
                        let function_name = parts[0].to_string();
                        let module_path = parts[1].to_string();

                        let cross_call = CrossModuleCall {
                            caller: caller.clone(),
                            module_path,
                            function_name,
                            line,
                            through_import: false, // Would need import analysis
                        };

                        self.cross_module_calls.push(cross_call);
                    }
                }
            }
        }

        // Continue visiting
        syn::visit::visit_expr_call(self, expr);
    }
}

impl Default for CrossModuleTracker {
    fn default() -> Self {
        Self::new()
    }
}
