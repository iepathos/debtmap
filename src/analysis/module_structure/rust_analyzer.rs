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
use std::path::Path;
use syn::spanned::Spanned;

use super::facade::detect_module_facade;
use super::types::{
    ComponentDependencyGraph, FunctionCounts, FunctionGroup, ModuleComponent, ModuleStructure,
};

/// Analyze a Rust source file to extract detailed module structure
pub fn analyze_rust_file(content: &str, _file_path: &Path) -> ModuleStructure {
    match syn::parse_file(content) {
        Ok(ast) => analyze_rust_ast(&ast),
        Err(_) => ModuleStructure {
            total_lines: content.lines().count(),
            components: vec![],
            function_counts: FunctionCounts::new(),
            responsibility_count: 0,
            public_api_surface: 0,
            dependencies: ComponentDependencyGraph::new(),
            facade_info: None,
        },
    }
}

/// Analyze a parsed Rust AST
pub fn analyze_rust_ast(ast: &syn::File) -> ModuleStructure {
    let total_lines = estimate_line_count(ast);
    let components = extract_components(ast);
    let function_counts = count_functions(&components);
    let responsibility_count = detect_responsibilities(&components);
    let public_api_surface = count_public_api(&components);
    let dependencies = ComponentDependencyGraph::new();
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
}
