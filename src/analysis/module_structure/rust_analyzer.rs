//! Rust-specific module structure analysis
//!
//! Provides detailed analysis of Rust source files including:
//! - Accurate function counting (module-level, impl methods, trait methods)
//! - Component detection (structs, enums, impl blocks)
//! - Responsibility identification
//!
//! ## Line Range Extraction
//!
//! Line ranges are extracted from `syn::Span` information using the `Spanned` trait.
//! All line numbers are 1-based, consistent with syn's conventions.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::Visit;

use super::facade::detect_module_facade;
use super::types::{
    ComponentDependencyGraph, FunctionCounts, FunctionGroup, ModuleComponent, ModuleStructure,
};

/// Analyze a Rust source file to extract detailed module structure
///
/// Spec 202: Resets SourceMap after analysis (not after parsing) to ensure
/// span lookups in the AST remain valid during analysis.
pub fn analyze_rust_file(content: &str, _file_path: &Path) -> ModuleStructure {
    let result = match syn::parse_file(content) {
        Ok(ast) => {
            // Analyze while AST spans are still valid
            let structure = analyze_rust_ast_with_path(&ast, Some(_file_path));
            // Reset SourceMap after all span lookups are done (spec 202)
            crate::core::parsing::reset_span_locations();
            structure
        }
        Err(_) => ModuleStructure {
            total_lines: content.lines().count(),
            components: vec![],
            function_counts: FunctionCounts::new(),
            responsibility_count: 0,
            public_api_surface: 0,
            dependencies: ComponentDependencyGraph::new(),
            facade_info: None,
        },
    };
    result
}

/// Analyze a parsed Rust AST
pub fn analyze_rust_ast(ast: &syn::File) -> ModuleStructure {
    analyze_rust_ast_with_path(ast, None)
}

fn analyze_rust_ast_with_path(ast: &syn::File, file_path: Option<&Path>) -> ModuleStructure {
    let total_lines = estimate_line_count(ast);
    let components = extract_components(ast);
    let function_counts = count_functions(&components);
    let responsibility_count = detect_responsibilities(&components);
    let public_api_surface = count_public_api(&components);
    let dependencies = build_dependency_graph(ast, &components, file_path);
    let facade_info = Some(detect_module_facade(ast));

    ModuleStructure {
        total_lines,
        components,
        function_counts,
        responsibility_count,
        public_api_surface,
        dependencies,
        facade_info,
    }
}

fn build_dependency_graph(
    ast: &syn::File,
    components: &[ModuleComponent],
    file_path: Option<&Path>,
) -> ComponentDependencyGraph {
    let module_name = file_path
        .and_then(|path| path.file_stem())
        .and_then(|stem| stem.to_str())
        .unwrap_or("module")
        .to_string();
    let local_component_names = component_names(components);
    let import_names = collect_import_roots(ast);
    let mut graph_components = vec![module_name.clone()];
    graph_components.extend(components.iter().map(ModuleComponent::name));
    graph_components.extend(import_names.iter().cloned());

    let mut edges = collect_component_edges(ast, &local_component_names);
    edges.extend(
        import_names
            .iter()
            .map(|dependency| (module_name.clone(), dependency.clone())),
    );

    let edge_set: HashSet<(String, String)> =
        edges.into_iter().filter(|(from, to)| from != to).collect();
    let edges: Vec<(String, String)> = edge_set.into_iter().collect();

    graph_components.sort();
    graph_components.dedup();

    let mut coupling_scores = HashMap::new();
    let outgoing_counts = build_outgoing_counts(&edges);

    for component in &graph_components {
        let fan_out = outgoing_counts.get(component).copied().unwrap_or(0) as f64;
        let score = if fan_out == 0.0 {
            1.0
        } else {
            1.0 / (1.0 + fan_out)
        };
        coupling_scores.insert(component.clone(), score);
    }

    ComponentDependencyGraph {
        components: graph_components,
        edges,
        coupling_scores,
    }
}

fn component_names(components: &[ModuleComponent]) -> HashSet<String> {
    components.iter().map(ModuleComponent::name).collect()
}

fn collect_import_roots(ast: &syn::File) -> HashSet<String> {
    let mut imports = HashSet::new();

    for item in &ast.items {
        if let syn::Item::Use(item_use) = item {
            collect_use_tree_roots(&item_use.tree, &mut imports);
        }
    }

    imports
}

fn collect_use_tree_roots(tree: &syn::UseTree, imports: &mut HashSet<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            imports.insert(path.ident.to_string());
        }
        syn::UseTree::Name(name) => {
            imports.insert(name.ident.to_string());
        }
        syn::UseTree::Rename(rename) => {
            imports.insert(rename.ident.to_string());
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree_roots(item, imports);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn collect_component_edges(
    ast: &syn::File,
    local_component_names: &HashSet<String>,
) -> Vec<(String, String)> {
    ast.items
        .iter()
        .flat_map(|item| component_edges_for_item(item, local_component_names))
        .collect()
}

fn component_edges_for_item(
    item: &syn::Item,
    local_component_names: &HashSet<String>,
) -> Vec<(String, String)> {
    let Some(component_name) = component_name_for_item(item) else {
        return Vec::new();
    };

    let mut visitor = RustDependencyVisitor::new(local_component_names);
    visitor.visit_item(item);

    visitor
        .references
        .into_iter()
        .map(|dependency| (component_name.clone(), dependency))
        .collect()
}

fn component_name_for_item(item: &syn::Item) -> Option<String> {
    match item {
        syn::Item::Struct(item_struct) => Some(item_struct.ident.to_string()),
        syn::Item::Enum(item_enum) => Some(item_enum.ident.to_string()),
        syn::Item::Fn(item_fn) => Some(item_fn.sig.ident.to_string()),
        syn::Item::Impl(item_impl) => {
            let target = extract_type_name(&item_impl.self_ty)?;
            Some(
                item_impl
                    .trait_
                    .as_ref()
                    .and_then(|(_, path, _)| {
                        path.segments
                            .last()
                            .map(|segment| segment.ident.to_string())
                    })
                    .map(|trait_name| format!("{} for {}", trait_name, target))
                    .unwrap_or_else(|| format!("{} impl", target)),
            )
        }
        _ => None,
    }
}

fn build_outgoing_counts(edges: &[(String, String)]) -> HashMap<String, usize> {
    let mut outgoing = HashMap::new();

    for (from, _) in edges {
        *outgoing.entry(from.clone()).or_insert(0) += 1;
    }

    outgoing
}

struct RustDependencyVisitor<'a> {
    _local_component_names: &'a HashSet<String>,
    references: HashSet<String>,
}

impl<'a> RustDependencyVisitor<'a> {
    fn new(local_component_names: &'a HashSet<String>) -> Self {
        Self {
            _local_component_names: local_component_names,
            references: HashSet::new(),
        }
    }

    fn record_path(&mut self, path: &syn::Path) {
        let Some(first) = path.segments.first() else {
            return;
        };
        let root = first.ident.to_string();

        if is_ignored_dependency(&root) {
            return;
        }

        self.references.insert(root);
    }
}

impl<'ast> Visit<'ast> for RustDependencyVisitor<'_> {
    fn visit_type_path(&mut self, type_path: &'ast syn::TypePath) {
        self.record_path(&type_path.path);
        syn::visit::visit_type_path(self, type_path);
    }

    fn visit_expr_path(&mut self, expr_path: &'ast syn::ExprPath) {
        self.record_path(&expr_path.path);
        syn::visit::visit_expr_path(self, expr_path);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.record_path(&mac.path);
        syn::visit::visit_macro(self, mac);
    }
}

fn is_ignored_dependency(name: &str) -> bool {
    matches!(
        name,
        "Self"
            | "self"
            | "super"
            | "crate"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "str"
            | "String"
            | "Option"
            | "Result"
            | "Vec"
            | "Box"
    )
}

/// Extract all components from a Rust AST
fn extract_components(ast: &syn::File) -> Vec<ModuleComponent> {
    ast.items
        .iter()
        .filter_map(extract_item_component)
        .collect()
}

/// Extract a component from a single AST item
fn extract_item_component(item: &syn::Item) -> Option<ModuleComponent> {
    match item {
        syn::Item::Struct(s) => Some(extract_struct_component(s)),
        syn::Item::Enum(e) => Some(extract_enum_component(e)),
        syn::Item::Impl(i) => extract_impl_component(i),
        syn::Item::Fn(f) => Some(extract_function_component(f)),
        syn::Item::Mod(_) => None, // Handle nested modules separately if needed
        _ => None,
    }
}

fn extract_struct_component(s: &syn::ItemStruct) -> ModuleComponent {
    let name = s.ident.to_string();
    let fields = match &s.fields {
        syn::Fields::Named(f) => f.named.len(),
        syn::Fields::Unnamed(f) => f.unnamed.len(),
        syn::Fields::Unit => 0,
    };
    let public = matches!(s.vis, syn::Visibility::Public(_));

    ModuleComponent::Struct {
        name,
        fields,
        methods: 0, // Will be counted from impl blocks
        public,
        line_range: extract_line_range(s),
    }
}

fn extract_enum_component(e: &syn::ItemEnum) -> ModuleComponent {
    let name = e.ident.to_string();
    let variants = e.variants.len();
    let public = matches!(e.vis, syn::Visibility::Public(_));

    ModuleComponent::Enum {
        name,
        variants,
        methods: 0,
        public,
        line_range: extract_line_range(e),
    }
}

fn extract_impl_component(i: &syn::ItemImpl) -> Option<ModuleComponent> {
    let target = if let Some((_, path, _)) = &i.trait_ {
        path.segments.last()?.ident.to_string()
    } else {
        extract_type_name(&i.self_ty)?
    };

    let trait_impl = i.trait_.as_ref().map(|(_, path, _)| {
        path.segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default()
    });

    let methods = i
        .items
        .iter()
        .filter(|item| matches!(item, syn::ImplItem::Fn(_)))
        .count();

    Some(ModuleComponent::ImplBlock {
        target,
        methods,
        trait_impl,
        line_range: extract_line_range(i),
    })
}

fn extract_function_component(f: &syn::ItemFn) -> ModuleComponent {
    let name = f.sig.ident.to_string();
    let public = matches!(f.vis, syn::Visibility::Public(_));

    ModuleComponent::ModuleLevelFunction {
        name,
        public,
        lines: estimate_function_lines(f),
        complexity: 1, // Simplified
    }
}

/// Count functions by category from components
fn count_functions(components: &[ModuleComponent]) -> FunctionCounts {
    components
        .iter()
        .fold(FunctionCounts::new(), |mut counts, component| {
            match component {
                ModuleComponent::ImplBlock {
                    methods,
                    trait_impl,
                    ..
                } => {
                    if trait_impl.is_some() {
                        counts.trait_methods += methods;
                    } else {
                        counts.impl_methods += methods;
                    }
                }
                ModuleComponent::ModuleLevelFunction { public, .. } => {
                    counts.module_level_functions += 1;
                    if *public {
                        counts.public_functions += 1;
                    } else {
                        counts.private_functions += 1;
                    }
                }
                _ => {}
            }
            counts
        })
}

/// Detect distinct responsibilities in a module
fn detect_responsibilities(components: &[ModuleComponent]) -> usize {
    let impl_count = components
        .iter()
        .filter(|c| match c {
            ModuleComponent::ImplBlock { .. } => true,
            ModuleComponent::Struct { methods, .. } => *methods > 0,
            ModuleComponent::Enum { methods, .. } => *methods > 0,
            _ => false,
        })
        .count();

    let function_groups = group_module_functions(components);
    let total = impl_count + function_groups.len();

    total.max(1)
}

/// Group module-level functions by prefix
fn group_module_functions(components: &[ModuleComponent]) -> Vec<FunctionGroup> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for component in components {
        if let ModuleComponent::ModuleLevelFunction { name, .. } = component {
            let prefix = extract_function_prefix(name);
            groups.entry(prefix).or_default().push(name.clone());
        }
    }

    groups
        .into_iter()
        .map(|(prefix, functions)| FunctionGroup { prefix, functions })
        .collect()
}

/// Count public API surface
fn count_public_api(components: &[ModuleComponent]) -> usize {
    components
        .iter()
        .filter(|c| match c {
            ModuleComponent::Struct { public, .. } => *public,
            ModuleComponent::Enum { public, .. } => *public,
            ModuleComponent::ModuleLevelFunction { public, .. } => *public,
            _ => false,
        })
        .count()
}

// Pure helper functions

/// Extract line range from any syn AST node that implements Spanned.
///
/// Returns a tuple of (start_line, end_line) using 1-based line numbering
/// consistent with syn's conventions.
fn extract_line_range<T: Spanned>(node: &T) -> (usize, usize) {
    let span = node.span();
    let start = span.start().line;
    let end = span.end().line;
    (start, end)
}

fn extract_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => type_path.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

fn estimate_line_count(ast: &syn::File) -> usize {
    ast.items.len() * 10 // Rough estimate
}

fn estimate_function_lines(_f: &syn::ItemFn) -> usize {
    10 // Simplified - would need actual span info
}

fn extract_function_prefix(name: &str) -> String {
    name.find('_')
        .map(|idx| name[..idx].to_string())
        .unwrap_or_else(|| "other".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_function_prefix() {
        assert_eq!(extract_function_prefix("format_output"), "format");
        assert_eq!(extract_function_prefix("parse_input"), "parse");
        assert_eq!(extract_function_prefix("simple"), "other");
    }

    #[test]
    fn test_analyze_simple_rust_file() {
        let code = r#"
            pub struct Foo {
                field: u32,
            }

            impl Foo {
                pub fn new() -> Self {
                    Self { field: 0 }
                }
            }

            pub fn helper() {}
        "#;

        let structure = analyze_rust_file(code, Path::new("test.rs"));

        assert_eq!(structure.function_counts.impl_methods, 1);
        assert_eq!(structure.function_counts.module_level_functions, 1);
        assert!(structure.responsibility_count >= 1);
    }

    #[test]
    fn test_extract_struct_line_range() {
        let code = r#"
pub struct Foo {
    field1: u32,
    field2: String,
    field3: bool,
}
        "#;

        let ast = syn::parse_file(code).unwrap();
        let structure = analyze_rust_ast(&ast);

        let struct_comp = structure
            .components
            .iter()
            .find(|c| matches!(c, ModuleComponent::Struct { name, .. } if name == "Foo"))
            .expect("Foo struct should exist");

        let line_count = struct_comp.line_count();
        assert!(
            line_count >= 4,
            "Struct should span 4+ lines, got {}",
            line_count
        );
        assert!(
            line_count <= 6,
            "Struct should span at most 6 lines, got {}",
            line_count
        );
    }

    #[test]
    fn test_extract_enum_line_range() {
        let code = r#"
pub enum Status {
    Active,
    Inactive,
    Pending,
}
        "#;

        let ast = syn::parse_file(code).unwrap();
        let structure = analyze_rust_ast(&ast);

        let enum_comp = structure
            .components
            .iter()
            .find(|c| matches!(c, ModuleComponent::Enum { name, .. } if name == "Status"))
            .expect("Status enum should exist");

        let line_count = enum_comp.line_count();
        assert!(
            line_count >= 4,
            "Enum should span 4+ lines, got {}",
            line_count
        );
    }

    #[test]
    fn test_extract_impl_line_range() {
        let code = r#"
impl Foo {
    pub fn new() -> Self {
        Self { field1: 0, field2: String::new(), field3: false }
    }

    pub fn process(&self) -> u32 {
        self.field1 * 2
    }

    fn helper(&self) -> String {
        self.field2.clone()
    }
}
        "#;

        let ast = syn::parse_file(code).unwrap();
        let structure = analyze_rust_ast(&ast);

        let impl_comp = structure
            .components
            .iter()
            .find(|c| matches!(c, ModuleComponent::ImplBlock { target, .. } if target == "Foo"))
            .expect("Foo impl should exist");

        let line_count = impl_comp.line_count();
        assert!(
            line_count >= 12,
            "Impl block should span 12+ lines, got {}",
            line_count
        );
    }

    #[test]
    fn test_component_sorting_by_line_count() {
        let code = r#"
pub struct Small { x: u32 }

pub struct Large {
    field1: u32,
    field2: String,
    field3: bool,
    field4: Vec<u8>,
    field5: Option<usize>,
}

impl Large {
    pub fn new() -> Self {
        Self {
            field1: 0,
            field2: String::new(),
            field3: false,
            field4: Vec::new(),
            field5: None,
        }
    }
}
        "#;

        let ast = syn::parse_file(code).unwrap();
        let structure = analyze_rust_ast(&ast);

        let mut sorted = structure.components.clone();
        sorted.sort_by_key(|c| std::cmp::Reverse(c.line_count()));

        // Verify Large impl is first (longest)
        if let ModuleComponent::ImplBlock { target, .. } = &sorted[0] {
            assert_eq!(target, "Large", "Largest component should be Large impl");
            assert!(sorted[0].line_count() > sorted[1].line_count());
        } else {
            panic!("First component should be impl block");
        }
    }

    #[test]
    fn test_analyze_rust_ast_populates_dependency_graph() {
        let code = r#"
use serde::Serialize;

pub struct Config {
    helper: Helper,
}

pub struct Helper;

impl Config {
    pub fn build(helper: Helper) -> Self {
        log::info!("building");
        Self { helper }
    }
}

pub fn render(config: Config) -> Helper {
    Helper
}
"#;

        let ast = syn::parse_file(code).unwrap();
        let structure = analyze_rust_ast(&ast);

        assert!(structure
            .dependencies
            .edges
            .iter()
            .any(|(from, to)| from == "Config" && to == "Helper"));
        assert!(structure
            .dependencies
            .edges
            .iter()
            .any(|(from, to)| from == "Config impl" && to == "log"));
        assert!(structure
            .dependencies
            .edges
            .iter()
            .any(|(from, to)| from == "module" && to == "serde"));
        assert!(structure
            .dependencies
            .coupling_scores
            .contains_key("Config impl"));
    }
}
