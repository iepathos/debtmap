//! Enhanced Python Import Resolution
//!
//! Provides robust import resolution for Python that accurately tracks all import patterns,
//! resolves symbols across modules, and builds complete cross-module call graphs with support
//! for complex import scenarios including:
//! - Star imports (from module import *)
//! - Import aliases (import foo as bar, from foo import x as y)
//! - Relative imports (from . import x, from .. import y)
//! - Package imports and __init__.py exports
//! - Re-exports and import forwarding
//! - Circular imports
//! - Dynamic imports (where statically analyzable)

use rustpython_parser::ast;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Type of import statement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImportType {
    /// Direct module import: `import module`
    Direct,
    /// From import: `from module import name`
    From,
    /// Star import: `from module import *`
    Star,
    /// Relative import: `from . import module`
    Relative { level: usize },
    /// Dynamic import: `__import__()`, `importlib.import_module()`
    Dynamic,
}

/// Symbol exported by a module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedSymbol {
    pub name: String,
    pub original_name: String,
    pub is_function: bool,
    pub is_class: bool,
    pub source_module: Option<PathBuf>,
}

/// Module symbols and exports
#[derive(Debug, Clone, Default)]
pub struct ModuleSymbols {
    pub path: PathBuf,
    /// Explicitly exported names (__all__)
    pub explicit_exports: HashSet<String>,
    /// All top-level definitions
    pub implicit_exports: HashSet<String>,
    /// Function definitions: name -> is_async
    pub functions: HashMap<String, bool>,
    /// Class definitions: name -> methods
    pub classes: HashMap<String, Vec<String>>,
    /// Re-exports: local_name -> (source_module, original_name)
    pub re_exports: HashMap<String, (PathBuf, String)>,
}

impl ModuleSymbols {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            ..Default::default()
        }
    }

    /// Get all exportable symbols (respects __all__ if present)
    pub fn get_exports(&self) -> HashSet<String> {
        if !self.explicit_exports.is_empty() {
            self.explicit_exports.clone()
        } else {
            // Export all non-private symbols
            self.implicit_exports
                .iter()
                .filter(|name| !name.starts_with('_'))
                .cloned()
                .collect()
        }
    }

    /// Check if a symbol is exported
    pub fn exports_symbol(&self, name: &str) -> bool {
        if !self.explicit_exports.is_empty() {
            self.explicit_exports.contains(name)
        } else {
            self.implicit_exports.contains(name) && !name.starts_with('_')
        }
    }
}

/// Import edge in the dependency graph
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportEdge {
    pub from_module: PathBuf,
    pub to_module: PathBuf,
    pub import_type: ImportType,
    pub imported_names: Vec<String>,
    pub aliases: HashMap<String, String>,
}

/// Import dependency graph
#[derive(Debug, Clone, Default)]
pub struct ImportGraph {
    /// Edges: from -> list of imports
    pub edges: HashMap<PathBuf, Vec<ImportEdge>>,
    /// Detected circular import chains
    pub cycles: Vec<Vec<PathBuf>>,
}

impl ImportGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an import edge
    pub fn add_edge(&mut self, edge: ImportEdge) {
        self.edges
            .entry(edge.from_module.clone())
            .or_default()
            .push(edge);
    }

    /// Detect circular imports using DFS
    pub fn detect_cycles(&mut self) {
        let mut visited = HashSet::new();
        let mut rec_stack = Vec::new();
        let mut cycles = Vec::new();

        for start in self.edges.keys() {
            if !visited.contains(start) {
                self.dfs_cycles(start, &mut visited, &mut rec_stack, &mut cycles);
            }
        }

        self.cycles = cycles;
    }

    fn dfs_cycles(
        &self,
        node: &PathBuf,
        visited: &mut HashSet<PathBuf>,
        rec_stack: &mut Vec<PathBuf>,
        cycles: &mut Vec<Vec<PathBuf>>,
    ) {
        visited.insert(node.clone());
        rec_stack.push(node.clone());

        if let Some(edges) = self.edges.get(node) {
            for edge in edges {
                if let Some(pos) = rec_stack.iter().position(|p| p == &edge.to_module) {
                    // Found a cycle
                    let cycle = rec_stack[pos..].to_vec();
                    if !cycles.contains(&cycle) {
                        cycles.push(cycle);
                    }
                } else if !visited.contains(&edge.to_module) {
                    self.dfs_cycles(&edge.to_module, visited, rec_stack, cycles);
                }
            }
        }

        rec_stack.pop();
    }

    /// Get topological order for import resolution (ignoring cycles)
    pub fn topological_order(&self) -> Vec<PathBuf> {
        let mut in_degree: HashMap<PathBuf, usize> = HashMap::new();
        let mut result = Vec::new();

        // Calculate in-degrees
        for node in self.edges.keys() {
            in_degree.entry(node.clone()).or_insert(0);
        }
        for edges in self.edges.values() {
            for edge in edges {
                *in_degree.entry(edge.to_module.clone()).or_insert(0) += 1;
            }
        }

        // Process nodes with in-degree 0
        let mut queue: Vec<PathBuf> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(node, _)| node.clone())
            .collect();

        while let Some(node) = queue.pop() {
            result.push(node.clone());

            if let Some(edges) = self.edges.get(&node) {
                for edge in edges {
                    if let Some(degree) = in_degree.get_mut(&edge.to_module) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(edge.to_module.clone());
                        }
                    }
                }
            }
        }

        result
    }
}

/// Resolved symbol information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSymbol {
    pub name: String,
    pub module_path: PathBuf,
    pub original_name: String,
    pub is_function: bool,
    pub is_class: bool,
}

/// Enhanced import resolver
#[derive(Debug, Default, Clone)]
pub struct EnhancedImportResolver {
    /// Module symbols indexed by path
    module_symbols: HashMap<PathBuf, ModuleSymbols>,
    /// Import dependency graph
    import_graph: ImportGraph,
    /// Resolution cache: (module_path, name) -> resolved symbol
    resolution_cache: HashMap<(PathBuf, String), Option<ResolvedSymbol>>,
    /// Alias mapping: (module_path, alias) -> original_name
    alias_map: HashMap<(PathBuf, String), String>,
}

impl EnhancedImportResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze imports in a module
    pub fn analyze_imports(&mut self, module: &ast::Mod, path: &Path) {
        let mut symbols = ModuleSymbols::new(path.to_path_buf());

        if let ast::Mod::Module(module) = module {
            // First pass: collect all definitions and __all__
            for stmt in &module.body {
                self.collect_module_symbols(stmt, &mut symbols, path);
            }

            // Second pass: process imports
            for stmt in &module.body {
                self.process_import_statement(stmt, path, &symbols);
            }
        }

        self.module_symbols.insert(path.to_path_buf(), symbols);
    }

    /// Collect symbols defined in a module
    fn collect_module_symbols(
        &mut self,
        stmt: &ast::Stmt,
        symbols: &mut ModuleSymbols,
        path: &Path,
    ) {
        match stmt {
            ast::Stmt::FunctionDef(f) => {
                let name = f.name.as_str().to_string();
                symbols.functions.insert(name.clone(), false);
                symbols.implicit_exports.insert(name);
                // Check for dynamic imports in function body
                for body_stmt in &f.body {
                    self.detect_dynamic_imports(body_stmt, path);
                }
            }
            ast::Stmt::AsyncFunctionDef(f) => {
                let name = f.name.as_str().to_string();
                symbols.functions.insert(name.clone(), true);
                symbols.implicit_exports.insert(name);
                // Check for dynamic imports in function body
                for body_stmt in &f.body {
                    self.detect_dynamic_imports(body_stmt, path);
                }
            }
            ast::Stmt::ClassDef(c) => {
                let name = c.name.as_str().to_string();
                let mut methods = Vec::new();

                for item in &c.body {
                    if let ast::Stmt::FunctionDef(method) = item {
                        methods.push(method.name.as_str().to_string());
                        // Check for dynamic imports in method body
                        for body_stmt in &method.body {
                            self.detect_dynamic_imports(body_stmt, path);
                        }
                    } else if let ast::Stmt::AsyncFunctionDef(method) = item {
                        methods.push(method.name.as_str().to_string());
                        // Check for dynamic imports in method body
                        for body_stmt in &method.body {
                            self.detect_dynamic_imports(body_stmt, path);
                        }
                    }
                }

                symbols.classes.insert(name.clone(), methods);
                symbols.implicit_exports.insert(name);
            }
            ast::Stmt::Assign(assign) => {
                // Check for __all__ definition
                for target in &assign.targets {
                    if let ast::Expr::Name(name) = target {
                        if name.id.as_str() == "__all__" {
                            // Extract __all__ list
                            if let ast::Expr::List(list) = assign.value.as_ref() {
                                for elt in &list.elts {
                                    // Handle string literals in __all__
                                    if let ast::Expr::Constant(constant) = elt {
                                        if let ast::Constant::Str(s) = &constant.value {
                                            symbols.explicit_exports.insert(s.to_string());
                                        }
                                    }
                                }
                            }
                        } else {
                            // Regular assignment
                            symbols
                                .implicit_exports
                                .insert(name.id.as_str().to_string());
                        }
                    }
                }
                // Check for dynamic imports in assignment expressions
                self.detect_dynamic_imports_in_expr(&assign.value, path);
            }
            ast::Stmt::ImportFrom(import_from) => {
                // Track re-exports
                let level = import_from.level.map(|l| l.to_usize()).unwrap_or(0);
                if let Some(module) = &import_from.module {
                    let source_path = self.resolve_relative_import(path, module.as_str(), level);
                    for alias in &import_from.names {
                        let name = alias.name.as_str();
                        let local_name = alias.asname.as_ref().map(|a| a.as_str()).unwrap_or(name);
                        symbols.re_exports.insert(
                            local_name.to_string(),
                            (source_path.clone(), name.to_string()),
                        );
                    }
                }
            }
            ast::Stmt::Expr(expr) => {
                // Check for dynamic imports in expressions
                self.detect_dynamic_imports_in_expr(&expr.value, path);
            }
            _ => {}
        }
    }

    /// Detect dynamic import calls: __import__() and importlib.import_module()
    fn detect_dynamic_imports(&mut self, stmt: &ast::Stmt, path: &Path) {
        match stmt {
            ast::Stmt::Expr(expr) => {
                self.detect_dynamic_imports_in_expr(&expr.value, path);
            }
            ast::Stmt::Assign(assign) => {
                self.detect_dynamic_imports_in_expr(&assign.value, path);
            }
            ast::Stmt::AugAssign(aug) => {
                self.detect_dynamic_imports_in_expr(&aug.value, path);
            }
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.detect_dynamic_imports_in_expr(value, path);
                }
            }
            ast::Stmt::If(if_stmt) => {
                self.detect_dynamic_imports_in_expr(&if_stmt.test, path);
                for body_stmt in &if_stmt.body {
                    self.detect_dynamic_imports(body_stmt, path);
                }
                for orelse_stmt in &if_stmt.orelse {
                    self.detect_dynamic_imports(orelse_stmt, path);
                }
            }
            ast::Stmt::While(while_stmt) => {
                self.detect_dynamic_imports_in_expr(&while_stmt.test, path);
                for body_stmt in &while_stmt.body {
                    self.detect_dynamic_imports(body_stmt, path);
                }
            }
            ast::Stmt::For(for_stmt) => {
                self.detect_dynamic_imports_in_expr(&for_stmt.iter, path);
                for body_stmt in &for_stmt.body {
                    self.detect_dynamic_imports(body_stmt, path);
                }
            }
            ast::Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    self.detect_dynamic_imports_in_expr(&item.context_expr, path);
                }
                for body_stmt in &with_stmt.body {
                    self.detect_dynamic_imports(body_stmt, path);
                }
            }
            ast::Stmt::Try(try_stmt) => {
                for body_stmt in &try_stmt.body {
                    self.detect_dynamic_imports(body_stmt, path);
                }
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = handler;
                    for handler_stmt in &h.body {
                        self.detect_dynamic_imports(handler_stmt, path);
                    }
                }
                for orelse_stmt in &try_stmt.orelse {
                    self.detect_dynamic_imports(orelse_stmt, path);
                }
                for finalbody_stmt in &try_stmt.finalbody {
                    self.detect_dynamic_imports(finalbody_stmt, path);
                }
            }
            _ => {}
        }
    }

    /// Detect dynamic imports in expressions
    fn detect_dynamic_imports_in_expr(&mut self, expr: &ast::Expr, path: &Path) {
        match expr {
            ast::Expr::Call(call) => {
                // Check for __import__() call
                if let ast::Expr::Name(name) = call.func.as_ref() {
                    if name.id.as_str() == "__import__" {
                        // Extract module name from first argument if it's a string literal
                        if let Some(first_arg) = call.args.first() {
                            if let Some(module_name) = self.extract_string_literal(first_arg) {
                                self.add_dynamic_import(path, &module_name);
                            }
                        }
                    }
                }
                // Check for importlib.import_module() call
                else if let ast::Expr::Attribute(attr) = call.func.as_ref() {
                    if attr.attr.as_str() == "import_module" {
                        if let ast::Expr::Name(name) = attr.value.as_ref() {
                            if name.id.as_str() == "importlib" {
                                // Extract module name from first argument if it's a string literal
                                if let Some(first_arg) = call.args.first() {
                                    if let Some(module_name) = self.extract_string_literal(first_arg) {
                                        self.add_dynamic_import(path, &module_name);
                                    }
                                }
                            }
                        }
                    }
                }
                // Recursively check call arguments
                for arg in &call.args {
                    self.detect_dynamic_imports_in_expr(arg, path);
                }
            }
            ast::Expr::BinOp(binop) => {
                self.detect_dynamic_imports_in_expr(&binop.left, path);
                self.detect_dynamic_imports_in_expr(&binop.right, path);
            }
            ast::Expr::UnaryOp(unaryop) => {
                self.detect_dynamic_imports_in_expr(&unaryop.operand, path);
            }
            ast::Expr::Lambda(lambda) => {
                self.detect_dynamic_imports_in_expr(&lambda.body, path);
            }
            ast::Expr::IfExp(ifexp) => {
                self.detect_dynamic_imports_in_expr(&ifexp.test, path);
                self.detect_dynamic_imports_in_expr(&ifexp.body, path);
                self.detect_dynamic_imports_in_expr(&ifexp.orelse, path);
            }
            ast::Expr::ListComp(comp) => {
                self.detect_dynamic_imports_in_expr(&comp.elt, path);
                for generator in &comp.generators {
                    self.detect_dynamic_imports_in_expr(&generator.iter, path);
                }
            }
            ast::Expr::DictComp(comp) => {
                self.detect_dynamic_imports_in_expr(&comp.key, path);
                self.detect_dynamic_imports_in_expr(&comp.value, path);
                for generator in &comp.generators {
                    self.detect_dynamic_imports_in_expr(&generator.iter, path);
                }
            }
            ast::Expr::List(list) => {
                for elt in &list.elts {
                    self.detect_dynamic_imports_in_expr(elt, path);
                }
            }
            ast::Expr::Tuple(tuple) => {
                for elt in &tuple.elts {
                    self.detect_dynamic_imports_in_expr(elt, path);
                }
            }
            ast::Expr::Dict(dict) => {
                for key in &dict.keys {
                    if let Some(k) = key {
                        self.detect_dynamic_imports_in_expr(k, path);
                    }
                }
                for value in &dict.values {
                    self.detect_dynamic_imports_in_expr(value, path);
                }
            }
            _ => {}
        }
    }

    /// Extract string literal from expression
    fn extract_string_literal(&self, expr: &ast::Expr) -> Option<String> {
        if let ast::Expr::Constant(constant) = expr {
            if let ast::Constant::Str(s) = &constant.value {
                return Some(s.to_string());
            }
        }
        None
    }

    /// Add a dynamic import edge to the graph
    fn add_dynamic_import(&mut self, from_path: &Path, module_name: &str) {
        let target_path = self.resolve_absolute_import(from_path, module_name);

        self.import_graph.add_edge(ImportEdge {
            from_module: from_path.to_path_buf(),
            to_module: target_path,
            import_type: ImportType::Dynamic,
            imported_names: vec![module_name.to_string()],
            aliases: HashMap::new(),
        });
    }

    /// Process import statements and build graph
    fn process_import_statement(
        &mut self,
        stmt: &ast::Stmt,
        path: &Path,
        _symbols: &ModuleSymbols,
    ) {
        match stmt {
            ast::Stmt::Import(import) => {
                for alias in &import.names {
                    let module_name = alias.name.as_str();
                    let alias_name = alias.asname.as_ref().map(|a| a.as_str());
                    let target_path = self.resolve_import_path(path, module_name, 0);

                    let mut aliases = HashMap::new();
                    if let Some(alias) = alias_name {
                        aliases.insert(alias.to_string(), module_name.to_string());
                        self.alias_map.insert(
                            (path.to_path_buf(), alias.to_string()),
                            module_name.to_string(),
                        );
                    }

                    self.import_graph.add_edge(ImportEdge {
                        from_module: path.to_path_buf(),
                        to_module: target_path,
                        import_type: ImportType::Direct,
                        imported_names: vec![module_name.to_string()],
                        aliases,
                    });
                }
            }
            ast::Stmt::ImportFrom(import_from) => {
                let level = import_from.level.map(|l| l.to_usize()).unwrap_or(0);
                let module_name = import_from
                    .module
                    .as_ref()
                    .map(|m| m.as_str())
                    .unwrap_or("");

                let target_path = self.resolve_import_path(path, module_name, level);
                let import_type = if level > 0 {
                    ImportType::Relative { level }
                } else {
                    ImportType::From
                };

                let mut imported_names = Vec::new();
                let mut aliases = HashMap::new();
                let mut is_star = false;

                for alias in &import_from.names {
                    let name = alias.name.as_str();
                    if name == "*" {
                        is_star = true;
                        break;
                    }

                    imported_names.push(name.to_string());
                    if let Some(alias_name) = &alias.asname {
                        let alias_str = alias_name.as_str();
                        aliases.insert(alias_str.to_string(), name.to_string());
                        self.alias_map.insert(
                            (path.to_path_buf(), alias_str.to_string()),
                            name.to_string(),
                        );
                    }
                }

                let final_type = if is_star {
                    ImportType::Star
                } else {
                    import_type
                };

                self.import_graph.add_edge(ImportEdge {
                    from_module: path.to_path_buf(),
                    to_module: target_path,
                    import_type: final_type,
                    imported_names,
                    aliases,
                });
            }
            _ => {}
        }
    }

    /// Resolve import path (handles relative imports)
    fn resolve_import_path(&self, from_path: &Path, module_name: &str, level: usize) -> PathBuf {
        if level > 0 {
            self.resolve_relative_import(from_path, module_name, level)
        } else {
            self.resolve_absolute_import(from_path, module_name)
        }
    }

    /// Resolve relative import (from . import x, from .. import y)
    fn resolve_relative_import(
        &self,
        from_path: &Path,
        module_name: &str,
        level: usize,
    ) -> PathBuf {
        let mut current = from_path.parent().unwrap_or(from_path).to_path_buf();

        // Go up 'level' directories
        for _ in 1..level {
            current = current.parent().unwrap_or(&current).to_path_buf();
        }

        if module_name.is_empty() {
            // from . import something
            return current;
        }

        // Append module path
        for part in module_name.split('.') {
            current.push(part);
        }

        // Try as file first, then as package
        let mut file_path = current.clone();
        file_path.set_extension("py");

        if file_path.exists() {
            file_path
        } else {
            current.push("__init__.py");
            current
        }
    }

    /// Resolve absolute import
    ///
    /// # Limitations
    ///
    /// This implementation handles standard Python packages (with `__init__.py`) and modules.
    /// PEP 420 namespace packages (packages without `__init__.py` that span multiple directories)
    /// are not explicitly supported. Namespace packages are less common in practice, and full
    /// support would require checking multiple directories in sys.path to aggregate namespace
    /// package portions, which adds significant complexity for minimal benefit in most codebases.
    ///
    /// For regular packages and modules, this resolver correctly:
    /// - Resolves module files (e.g., `module.py`)
    /// - Resolves package directories (e.g., `package/__init__.py`)
    /// - Handles dotted module paths (e.g., `package.submodule.module`)
    fn resolve_absolute_import(&self, from_path: &Path, module_name: &str) -> PathBuf {
        if module_name.is_empty() {
            return from_path.to_path_buf();
        }

        let parent = from_path.parent().unwrap_or(from_path);
        let mut path = parent.to_path_buf();

        for part in module_name.split('.') {
            path.push(part);
        }

        let mut file_path = path.clone();
        file_path.set_extension("py");

        if file_path.exists() {
            file_path
        } else {
            // Note: If no __init__.py exists, this could be a namespace package (PEP 420),
            // but we treat it as a regular package directory for simplicity
            path.push("__init__.py");
            path
        }
    }

    /// Build import graph for multiple modules
    pub fn build_import_graph(&mut self, modules: &[(ast::Mod, PathBuf)]) {
        // Analyze all modules
        for (module, path) in modules {
            self.analyze_imports(module, path);
        }

        // Detect circular imports
        self.import_graph.detect_cycles();
    }

    /// Resolve a symbol in a module's namespace
    pub fn resolve_symbol(&mut self, module: &Path, name: &str) -> Option<ResolvedSymbol> {
        // Check cache first
        let cache_key = (module.to_path_buf(), name.to_string());
        if let Some(cached) = self.resolution_cache.get(&cache_key) {
            return cached.clone();
        }

        // Perform resolution
        let result = self.resolve_symbol_uncached(module, name);

        // Cache result
        self.resolution_cache.insert(cache_key, result.clone());

        result
    }

    fn resolve_symbol_uncached(&self, module: &Path, name: &str) -> Option<ResolvedSymbol> {
        // Check if it's defined locally
        if let Some(symbols) = self.module_symbols.get(module) {
            if symbols.implicit_exports.contains(name) {
                let is_function = symbols.functions.contains_key(name);
                let is_class = symbols.classes.contains_key(name);
                return Some(ResolvedSymbol {
                    name: name.to_string(),
                    module_path: module.to_path_buf(),
                    original_name: name.to_string(),
                    is_function,
                    is_class,
                });
            }

            // Check re-exports
            if let Some((source_module, original_name)) = symbols.re_exports.get(name) {
                return self.resolve_symbol_uncached(source_module, original_name);
            }
        }

        // Check imports
        if let Some(edges) = self.import_graph.edges.get(module) {
            for edge in edges {
                // Check aliases
                if let Some(original) = edge.aliases.get(name) {
                    return self.resolve_symbol_uncached(&edge.to_module, original);
                }

                // Check imported names
                if edge.imported_names.contains(&name.to_string()) {
                    return self.resolve_symbol_uncached(&edge.to_module, name);
                }

                // Check star imports
                if matches!(edge.import_type, ImportType::Star) {
                    if let Some(result) = self.resolve_star_import(&edge.to_module, name) {
                        return Some(result);
                    }
                }
            }
        }

        None
    }

    /// Resolve symbol from a star import
    fn resolve_star_import(&self, module: &Path, name: &str) -> Option<ResolvedSymbol> {
        let symbols = self.module_symbols.get(module)?;

        if symbols.exports_symbol(name) {
            let is_function = symbols.functions.contains_key(name);
            let is_class = symbols.classes.contains_key(name);
            Some(ResolvedSymbol {
                name: name.to_string(),
                module_path: module.to_path_buf(),
                original_name: name.to_string(),
                is_function,
                is_class,
            })
        } else {
            None
        }
    }

    /// Get all symbols exported by a module (for star imports)
    pub fn resolve_star_imports(&self, module: &Path) -> Vec<ResolvedSymbol> {
        let symbols = match self.module_symbols.get(module) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let exports = symbols.get_exports();
        exports
            .into_iter()
            .map(|name| {
                let is_function = symbols.functions.contains_key(&name);
                let is_class = symbols.classes.contains_key(&name);
                ResolvedSymbol {
                    name: name.clone(),
                    module_path: module.to_path_buf(),
                    original_name: name,
                    is_function,
                    is_class,
                }
            })
            .collect()
    }

    /// Get module exports
    pub fn get_module_exports(&self, module: &Path) -> HashSet<String> {
        self.module_symbols
            .get(module)
            .map(|s| s.get_exports())
            .unwrap_or_default()
    }

    /// Get import graph
    pub fn import_graph(&self) -> &ImportGraph {
        &self.import_graph
    }

    /// Get detected circular imports
    pub fn circular_imports(&self) -> &[Vec<PathBuf>] {
        &self.import_graph.cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_python(source: &str) -> ast::Mod {
        rustpython_parser::parse(source, rustpython_parser::Mode::Module, "test.py")
            .expect("Failed to parse Python code")
    }

    #[test]
    fn test_direct_import() {
        let source = "import os\nimport sys as system\n";
        let module = parse_python(source);
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&module, Path::new("test.py"));

        assert_eq!(resolver.import_graph.edges.len(), 1);
    }

    #[test]
    fn test_from_import() {
        let source = "from typing import List, Dict as D\n";
        let module = parse_python(source);
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&module, Path::new("test.py"));

        assert_eq!(resolver.import_graph.edges.len(), 1);
    }

    #[test]
    fn test_star_import() {
        let source = "from collections import *\n";
        let module = parse_python(source);
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&module, Path::new("test.py"));

        let edges = &resolver.import_graph.edges[&PathBuf::from("test.py")];
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].import_type, ImportType::Star);
    }

    #[test]
    fn test_relative_import() {
        let source = "from . import helper\nfrom .. import utils\n";
        let module = parse_python(source);
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&module, Path::new("package/submodule/test.py"));

        assert_eq!(resolver.import_graph.edges.len(), 1);
    }

    #[test]
    fn test_module_symbols() {
        let source = r#"
def my_function():
    pass

class MyClass:
    def method(self):
        pass

async def async_function():
    pass

__all__ = ['my_function', 'MyClass']
"#;
        let module = parse_python(source);
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&module, Path::new("test.py"));

        let symbols = &resolver.module_symbols[&PathBuf::from("test.py")];
        assert!(symbols.functions.contains_key("my_function"));
        assert!(symbols.functions.contains_key("async_function"));
        assert!(symbols.classes.contains_key("MyClass"));
        assert_eq!(symbols.explicit_exports.len(), 2);
        assert!(symbols.exports_symbol("my_function"));
        assert!(symbols.exports_symbol("MyClass"));
        assert!(!symbols.exports_symbol("async_function")); // Not in __all__
    }

    #[test]
    fn test_symbol_resolution() {
        let source = r#"
def helper():
    pass
"#;
        let module = parse_python(source);
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&module, Path::new("test.py"));

        let resolved = resolver.resolve_symbol(Path::new("test.py"), "helper");
        assert!(resolved.is_some());
        let symbol = resolved.unwrap();
        assert_eq!(symbol.name, "helper");
        assert!(symbol.is_function);
    }

    #[test]
    fn test_dynamic_import_builtin() {
        let source = r#"
def load_module(name):
    return __import__(name)

# Direct call at module level
module = __import__("os")
"#;
        let module = parse_python(source);
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&module, Path::new("test.py"));

        let edges = &resolver.import_graph.edges[&PathBuf::from("test.py")];
        let dynamic_imports: Vec<_> = edges
            .iter()
            .filter(|e| e.import_type == ImportType::Dynamic)
            .collect();

        // Should detect the __import__() call with string literal "os"
        // The call with variable "name" cannot be statically analyzed
        assert_eq!(dynamic_imports.len(), 1);
        assert!(dynamic_imports
            .iter()
            .any(|e| e.imported_names.contains(&"os".to_string())));
    }

    #[test]
    fn test_dynamic_import_importlib() {
        let source = r#"
import importlib

def load_module(name):
    return importlib.import_module(name)

# Direct call at module level
plugin = importlib.import_module("plugins.core")
"#;
        let module = parse_python(source);
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&module, Path::new("test.py"));

        let edges = &resolver.import_graph.edges[&PathBuf::from("test.py")];
        let dynamic_imports: Vec<_> = edges
            .iter()
            .filter(|e| e.import_type == ImportType::Dynamic)
            .collect();

        // Should detect the importlib.import_module() call with string literal
        // The call with variable "name" cannot be statically analyzed
        assert_eq!(dynamic_imports.len(), 1);
        assert!(dynamic_imports
            .iter()
            .any(|e| e.imported_names.contains(&"plugins.core".to_string())));
    }

    #[test]
    fn test_dynamic_import_in_conditionals() {
        let source = r#"
def conditional_import(use_new):
    if use_new:
        return __import__("new_impl")
    else:
        return __import__("old_impl")
"#;
        let module = parse_python(source);
        let mut resolver = EnhancedImportResolver::new();
        resolver.analyze_imports(&module, Path::new("test.py"));

        let edges = &resolver.import_graph.edges[&PathBuf::from("test.py")];
        let dynamic_imports: Vec<_> = edges
            .iter()
            .filter(|e| e.import_type == ImportType::Dynamic)
            .collect();

        // Should detect both dynamic imports in if/else branches
        assert_eq!(dynamic_imports.len(), 2);
        assert!(dynamic_imports
            .iter()
            .any(|e| e.imported_names.contains(&"new_impl".to_string())));
        assert!(dynamic_imports
            .iter()
            .any(|e| e.imported_names.contains(&"old_impl".to_string())));
    }
}
