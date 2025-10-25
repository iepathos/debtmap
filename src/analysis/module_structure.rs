//! Module Structure Analysis
//!
//! Provides detailed analysis of module structure including:
//! - Accurate function counting (module-level, impl methods, trait methods)
//! - Component detection (structs, enums, impl blocks)
//! - Responsibility identification
//! - Coupling analysis for refactoring recommendations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Categorized function counts within a module
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionCounts {
    pub module_level_functions: usize,
    pub impl_methods: usize,
    pub trait_methods: usize,
    pub nested_module_functions: usize,
    pub public_functions: usize,
    pub private_functions: usize,
}

impl FunctionCounts {
    pub fn new() -> Self {
        Self {
            module_level_functions: 0,
            impl_methods: 0,
            trait_methods: 0,
            nested_module_functions: 0,
            public_functions: 0,
            private_functions: 0,
        }
    }

    pub fn total(&self) -> usize {
        self.module_level_functions
            + self.impl_methods
            + self.trait_methods
            + self.nested_module_functions
    }
}

impl Default for FunctionCounts {
    fn default() -> Self {
        Self::new()
    }
}

/// A component within a module (struct, enum, impl block, or function)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleComponent {
    Struct {
        name: String,
        fields: usize,
        methods: usize,
        public: bool,
        line_range: (usize, usize),
    },
    Enum {
        name: String,
        variants: usize,
        methods: usize,
        public: bool,
        line_range: (usize, usize),
    },
    ImplBlock {
        target: String,
        methods: usize,
        trait_impl: Option<String>,
        line_range: (usize, usize),
    },
    ModuleLevelFunction {
        name: String,
        public: bool,
        lines: usize,
        complexity: u32,
    },
    NestedModule {
        name: String,
        file_path: Option<PathBuf>,
        functions: usize,
    },
}

impl ModuleComponent {
    pub fn name(&self) -> String {
        match self {
            ModuleComponent::Struct { name, .. } => name.clone(),
            ModuleComponent::Enum { name, .. } => name.clone(),
            ModuleComponent::ImplBlock {
                target, trait_impl, ..
            } => {
                if let Some(trait_name) = trait_impl {
                    format!("{} for {}", trait_name, target)
                } else {
                    format!("{} impl", target)
                }
            }
            ModuleComponent::ModuleLevelFunction { name, .. } => name.clone(),
            ModuleComponent::NestedModule { name, .. } => format!("mod {}", name),
        }
    }

    pub fn method_count(&self) -> usize {
        match self {
            ModuleComponent::Struct { methods, .. } => *methods,
            ModuleComponent::Enum { methods, .. } => *methods,
            ModuleComponent::ImplBlock { methods, .. } => *methods,
            ModuleComponent::ModuleLevelFunction { .. } => 1,
            ModuleComponent::NestedModule { functions, .. } => *functions,
        }
    }

    pub fn line_count(&self) -> usize {
        match self {
            ModuleComponent::Struct { line_range, .. } => line_range.1.saturating_sub(line_range.0),
            ModuleComponent::Enum { line_range, .. } => line_range.1.saturating_sub(line_range.0),
            ModuleComponent::ImplBlock { line_range, .. } => {
                line_range.1.saturating_sub(line_range.0)
            }
            ModuleComponent::ModuleLevelFunction { lines, .. } => *lines,
            ModuleComponent::NestedModule { .. } => 0,
        }
    }
}

/// Complete structure analysis of a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStructure {
    pub total_lines: usize,
    pub components: Vec<ModuleComponent>,
    pub function_counts: FunctionCounts,
    pub responsibility_count: usize,
    pub public_api_surface: usize,
    pub dependencies: ComponentDependencyGraph,
}

/// Dependency graph for coupling analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentDependencyGraph {
    pub components: Vec<String>,
    pub edges: Vec<(String, String)>,
    pub coupling_scores: HashMap<String, f64>,
}

impl ComponentDependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Identify components that are good candidates for extraction
    pub fn identify_split_candidates(&self) -> Vec<SplitRecommendation> {
        let mut candidates: Vec<_> = self
            .coupling_scores
            .iter()
            .filter(|(_, score)| **score < 0.3)
            .map(|(component, score)| SplitRecommendation {
                component: component.clone(),
                coupling_score: *score,
                suggested_module_name: suggest_module_name(component),
                estimated_lines: 200, // Placeholder - would need actual calculation
                difficulty: difficulty_from_coupling(*score),
            })
            .collect();

        candidates.sort_by(|a, b| {
            a.coupling_score
                .partial_cmp(&b.coupling_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates
    }

    pub fn analyze_coupling(&self) -> ComponentCouplingAnalysis {
        let mut afferent: HashMap<String, usize> = HashMap::new();
        let mut efferent: HashMap<String, usize> = HashMap::new();

        for (from, to) in &self.edges {
            *efferent.entry(from.clone()).or_insert(0) += 1;
            *afferent.entry(to.clone()).or_insert(0) += 1;
        }

        ComponentCouplingAnalysis {
            afferent,
            efferent,
            total_edges: self.edges.len(),
        }
    }
}

/// Recommendation for splitting out a component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitRecommendation {
    pub component: String,
    pub coupling_score: f64,
    pub suggested_module_name: String,
    pub estimated_lines: usize,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Difficulty {
    Easy,   // Coupling < 0.2
    Medium, // Coupling 0.2-0.5
    Hard,   // Coupling > 0.5
}

fn difficulty_from_coupling(score: f64) -> Difficulty {
    if score < 0.2 {
        Difficulty::Easy
    } else if score < 0.5 {
        Difficulty::Medium
    } else {
        Difficulty::Hard
    }
}

fn suggest_module_name(component: &str) -> String {
    let lower = component.to_lowercase().replace(' ', "_");
    if lower.ends_with("_impl") {
        lower.trim_end_matches("_impl").to_string()
    } else {
        lower
    }
}

/// Coupling analysis results
#[derive(Debug, Clone)]
pub struct ComponentCouplingAnalysis {
    pub afferent: HashMap<String, usize>, // Incoming dependencies
    pub efferent: HashMap<String, usize>, // Outgoing dependencies
    pub total_edges: usize,
}

/// Grouped functions by domain/responsibility
#[derive(Debug, Clone)]
pub struct FunctionGroup {
    pub prefix: String,
    pub functions: Vec<String>,
}

/// Module structure analyzer for Rust code
pub struct ModuleStructureAnalyzer {
    _language: String, // For future multi-language support
}

impl ModuleStructureAnalyzer {
    pub fn new_rust() -> Self {
        Self {
            _language: "rust".to_string(),
        }
    }

    /// Analyze a Rust source file to extract detailed module structure
    pub fn analyze_rust_file(
        &self,
        content: &str,
        _file_path: &std::path::Path,
    ) -> ModuleStructure {
        let parse_result = syn::parse_file(content);

        match parse_result {
            Ok(ast) => self.analyze_rust_ast(&ast),
            Err(_) => {
                // Return empty structure if parsing fails
                ModuleStructure {
                    total_lines: content.lines().count(),
                    components: vec![],
                    function_counts: FunctionCounts::new(),
                    responsibility_count: 0,
                    public_api_surface: 0,
                    dependencies: ComponentDependencyGraph::new(),
                }
            }
        }
    }

    fn analyze_rust_ast(&self, ast: &syn::File) -> ModuleStructure {
        let total_lines = estimate_line_count(ast);
        let components = self.extract_components(ast);
        let function_counts = self.count_functions(&components);
        let responsibility_count = self.detect_responsibilities(&components);
        let public_api_surface = self.count_public_api(&components);
        let dependencies = ComponentDependencyGraph::new(); // Simplified for now

        ModuleStructure {
            total_lines,
            components,
            function_counts,
            responsibility_count,
            public_api_surface,
            dependencies,
        }
    }

    fn extract_components(&self, ast: &syn::File) -> Vec<ModuleComponent> {
        let mut components = Vec::new();

        for item in &ast.items {
            match item {
                syn::Item::Struct(s) => {
                    components.push(self.extract_struct_component(s));
                }
                syn::Item::Enum(e) => {
                    components.push(self.extract_enum_component(e));
                }
                syn::Item::Impl(i) => {
                    if let Some(comp) = self.extract_impl_component(i) {
                        components.push(comp);
                    }
                }
                syn::Item::Fn(f) => {
                    components.push(self.extract_function_component(f));
                }
                syn::Item::Mod(_) => {
                    // Handle nested modules if needed
                }
                _ => {}
            }
        }

        components
    }

    fn extract_struct_component(&self, s: &syn::ItemStruct) -> ModuleComponent {
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
            line_range: (0, 0), // Simplified - would need span info
        }
    }

    fn extract_enum_component(&self, e: &syn::ItemEnum) -> ModuleComponent {
        let name = e.ident.to_string();
        let variants = e.variants.len();
        let public = matches!(e.vis, syn::Visibility::Public(_));

        ModuleComponent::Enum {
            name,
            variants,
            methods: 0,
            public,
            line_range: (0, 0),
        }
    }

    fn extract_impl_component(&self, i: &syn::ItemImpl) -> Option<ModuleComponent> {
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
            line_range: (0, 0),
        })
    }

    fn extract_function_component(&self, f: &syn::ItemFn) -> ModuleComponent {
        let name = f.sig.ident.to_string();
        let public = matches!(f.vis, syn::Visibility::Public(_));

        ModuleComponent::ModuleLevelFunction {
            name,
            public,
            lines: estimate_function_lines(f),
            complexity: 1, // Simplified
        }
    }

    fn count_functions(&self, components: &[ModuleComponent]) -> FunctionCounts {
        let mut counts = FunctionCounts::new();

        for component in components {
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
        }

        counts
    }

    fn detect_responsibilities(&self, components: &[ModuleComponent]) -> usize {
        let mut count = 0;

        for component in components {
            match component {
                ModuleComponent::ImplBlock { .. } => count += 1,
                ModuleComponent::Struct { methods, .. } if *methods > 0 => count += 1,
                ModuleComponent::Enum { methods, .. } if *methods > 0 => count += 1,
                _ => {}
            }
        }

        // Group module-level functions
        let function_groups = self.group_module_functions(components);
        count += function_groups.len();

        count.max(1)
    }

    fn group_module_functions(&self, components: &[ModuleComponent]) -> Vec<FunctionGroup> {
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

    fn count_public_api(&self, components: &[ModuleComponent]) -> usize {
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
}

fn extract_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => type_path.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

fn estimate_line_count(ast: &syn::File) -> usize {
    // Simple estimation - count items
    ast.items.len() * 10 // Rough estimate
}

fn estimate_function_lines(_f: &syn::ItemFn) -> usize {
    // Simplified - would need actual span info
    10
}

fn extract_function_prefix(name: &str) -> String {
    if let Some(idx) = name.find('_') {
        name[..idx].to_string()
    } else {
        "other".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_counts_total() {
        let counts = FunctionCounts {
            module_level_functions: 10,
            impl_methods: 20,
            trait_methods: 5,
            nested_module_functions: 3,
            public_functions: 15,
            private_functions: 8,
        };
        assert_eq!(counts.total(), 38);
    }

    #[test]
    fn test_difficulty_from_coupling() {
        assert_eq!(difficulty_from_coupling(0.1), Difficulty::Easy);
        assert_eq!(difficulty_from_coupling(0.3), Difficulty::Medium);
        assert_eq!(difficulty_from_coupling(0.6), Difficulty::Hard);
    }

    #[test]
    fn test_extract_function_prefix() {
        assert_eq!(extract_function_prefix("format_output"), "format");
        assert_eq!(extract_function_prefix("parse_input"), "parse");
        assert_eq!(extract_function_prefix("simple"), "other");
    }

    #[test]
    fn test_module_component_name() {
        let comp = ModuleComponent::Struct {
            name: "TestStruct".to_string(),
            fields: 5,
            methods: 10,
            public: true,
            line_range: (1, 50),
        };
        assert_eq!(comp.name(), "TestStruct");
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

        let analyzer = ModuleStructureAnalyzer::new_rust();
        let structure = analyzer.analyze_rust_file(code, std::path::Path::new("test.rs"));

        assert_eq!(structure.function_counts.impl_methods, 1);
        assert_eq!(structure.function_counts.module_level_functions, 1);
        assert!(structure.responsibility_count >= 1);
    }
}
