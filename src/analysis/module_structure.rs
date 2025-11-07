//! Module Structure Analysis
//!
//! Provides detailed analysis of module structure including:
//! - Accurate function counting (module-level, impl methods, trait methods)
//! - Component detection (structs, enums, impl blocks)
//! - Responsibility identification
//! - Coupling analysis for refactoring recommendations
//!
//! ## Line Range Extraction
//!
//! Line ranges are extracted from `syn::Span` information using the `Spanned` trait.
//! All line numbers are 1-based, consistent with syn's conventions and typical editor
//! line numbering.
//!
//! The `extract_line_range()` helper function provides a generic way to extract
//! line ranges from any AST node, ensuring consistency across all component types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use syn::spanned::Spanned;

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

/// Module facade detection information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleFacadeInfo {
    /// Whether this file qualifies as a module facade
    pub is_facade: bool,
    /// Number of submodules (both #\[path\] and inline)
    pub submodule_count: usize,
    /// List of #\[path\] declarations
    pub path_declarations: Vec<PathDeclaration>,
    /// Facade quality score (0.0-1.0)
    pub facade_score: f64,
    /// Organization quality classification
    pub organization_quality: OrganizationQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PathDeclaration {
    pub module_name: String,
    pub file_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrganizationQuality {
    Excellent,  // ≥10 submodules, facade_score ≥0.8
    Good,       // ≥5 submodules, facade_score ≥0.6
    Poor,       // ≥3 submodules, facade_score ≥0.5
    Monolithic, // <3 submodules or facade_score <0.5
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
    /// Facade detection results (Spec 170)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facade_info: Option<ModuleFacadeInfo>,
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

    pub fn new_python() -> Self {
        Self {
            _language: "python".to_string(),
        }
    }

    pub fn new_javascript() -> Self {
        Self {
            _language: "javascript".to_string(),
        }
    }

    pub fn new_typescript() -> Self {
        Self {
            _language: "typescript".to_string(),
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
                    facade_info: None,
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
        let facade_info = Some(self.detect_module_facade(ast));

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
            line_range: extract_line_range(s),
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
            line_range: extract_line_range(e),
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
            line_range: extract_line_range(i),
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

    /// Analyze a Python source file to extract module structure
    pub fn analyze_python_file(
        &self,
        content: &str,
        _file_path: &std::path::Path,
    ) -> ModuleStructure {
        // Basic text-based analysis for Python
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let mut components = Vec::new();
        let mut public_count = 0;
        let mut private_count = 0;

        // Simple pattern matching for Python classes and functions
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Detect class definitions
            if trimmed.starts_with("class ") {
                let name = trimmed
                    .strip_prefix("class ")
                    .and_then(|s| s.split(['(', ':']).next())
                    .unwrap_or("Unknown")
                    .trim()
                    .to_string();
                let public = !name.starts_with('_');
                components.push(ModuleComponent::Struct {
                    name,
                    fields: 0,
                    methods: 0,
                    public,
                    line_range: (idx, idx),
                });
            }

            // Detect function definitions
            if trimmed.starts_with("def ") {
                let name = trimmed
                    .strip_prefix("def ")
                    .and_then(|s| s.split('(').next())
                    .unwrap_or("unknown")
                    .trim()
                    .to_string();
                let public = !name.starts_with('_');

                if public {
                    public_count += 1;
                } else {
                    private_count += 1;
                }

                components.push(ModuleComponent::ModuleLevelFunction {
                    name,
                    public,
                    lines: 5, // Estimate
                    complexity: 1,
                });
            }
        }

        let responsibility_count = (components.len() / 5).max(1);

        ModuleStructure {
            total_lines,
            components,
            function_counts: FunctionCounts {
                module_level_functions: public_count + private_count,
                impl_methods: 0,
                trait_methods: 0,
                nested_module_functions: 0,
                public_functions: public_count,
                private_functions: private_count,
            },
            responsibility_count,
            public_api_surface: public_count,
            dependencies: ComponentDependencyGraph::new(),
            facade_info: None,
        }
    }

    /// Analyze a JavaScript source file to extract module structure
    pub fn analyze_javascript_file(
        &self,
        content: &str,
        _file_path: &std::path::Path,
    ) -> ModuleStructure {
        // Basic text-based analysis for JavaScript
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let mut components = Vec::new();
        let mut public_count = 0;
        let mut private_count = 0;

        // Simple pattern matching for JS classes and functions
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Detect class definitions
            if trimmed.starts_with("class ") || trimmed.contains(" class ") {
                let name = trimmed
                    .split_whitespace()
                    .skip_while(|&s| s != "class")
                    .nth(1)
                    .unwrap_or("Unknown")
                    .split(['{', ' '])
                    .next()
                    .unwrap_or("Unknown")
                    .to_string();
                components.push(ModuleComponent::Struct {
                    name,
                    fields: 0,
                    methods: 0,
                    public: true,
                    line_range: (idx, idx),
                });
            }

            // Detect function definitions (function keyword, arrow functions, methods)
            if trimmed.starts_with("function ")
                || trimmed.starts_with("export function ")
                || trimmed.contains("= function")
                || trimmed.contains("=> ")
                    && (trimmed.starts_with("const ") || trimmed.starts_with("let "))
            {
                let is_export = trimmed.starts_with("export ");
                if is_export {
                    public_count += 1;
                } else {
                    private_count += 1;
                }

                components.push(ModuleComponent::ModuleLevelFunction {
                    name: "function".to_string(),
                    public: is_export,
                    lines: 5,
                    complexity: 1,
                });
            }
        }

        let responsibility_count = (components.len() / 5).max(1);

        ModuleStructure {
            total_lines,
            components,
            function_counts: FunctionCounts {
                module_level_functions: public_count + private_count,
                impl_methods: 0,
                trait_methods: 0,
                nested_module_functions: 0,
                public_functions: public_count,
                private_functions: private_count,
            },
            responsibility_count,
            public_api_surface: public_count,
            dependencies: ComponentDependencyGraph::new(),
            facade_info: None,
        }
    }

    /// Analyze a TypeScript source file to extract module structure
    pub fn analyze_typescript_file(
        &self,
        content: &str,
        _file_path: &std::path::Path,
    ) -> ModuleStructure {
        // TypeScript analysis is similar to JavaScript with type annotations
        // For now, delegate to JavaScript analyzer
        self.analyze_javascript_file(content, _file_path)
    }

    /// Detect if a Rust file is a module facade (Spec 170)
    ///
    /// A module facade is a file that primarily organizes submodules through
    /// #\[path\] declarations and re-exports, with minimal implementation code.
    fn detect_module_facade(&self, ast: &syn::File) -> ModuleFacadeInfo {
        let mut path_declarations = Vec::new();
        let mut inline_modules = 0;
        let mut impl_lines = 0;
        let mut fn_lines = 0;
        let mut total_lines = 0;

        for item in &ast.items {
            let span = item.span();
            total_lines = total_lines.max(span.end().line);

            match item {
                syn::Item::Mod(module) => {
                    if let Some(path) = extract_path_attribute(module) {
                        path_declarations.push(PathDeclaration {
                            module_name: module.ident.to_string(),
                            file_path: path,
                            line: span.start().line,
                        });
                    } else if module.content.is_some() {
                        inline_modules += 1;
                    }
                }
                syn::Item::Impl(_impl_block) => {
                    impl_lines += span.end().line.saturating_sub(span.start().line);
                }
                syn::Item::Fn(_func) => {
                    fn_lines += span.end().line.saturating_sub(span.start().line);
                }
                _ => {}
            }
        }

        let submodule_count = path_declarations.len() + inline_modules;
        let implementation_lines = impl_lines + fn_lines;

        // Calculate facade score
        let declaration_ratio = if total_lines > 0 {
            (total_lines.saturating_sub(implementation_lines)) as f64 / total_lines as f64
        } else {
            0.0
        };

        let submodule_factor = (submodule_count as f64 / 5.0).min(1.0);
        let facade_score = (declaration_ratio * 0.7 + submodule_factor * 0.3).clamp(0.0, 1.0);

        // Classify organization quality
        let organization_quality = classify_organization_quality(submodule_count, facade_score);

        ModuleFacadeInfo {
            is_facade: submodule_count >= 3 && facade_score >= 0.5,
            submodule_count,
            path_declarations,
            facade_score,
            organization_quality,
        }
    }
}

/// Extract line range from any syn AST node that implements Spanned.
///
/// Returns a tuple of (start_line, end_line) using 1-based line numbering
/// consistent with syn's conventions.
///
/// # Examples
///
/// ```no_run
/// use syn::spanned::Spanned;
///
/// let code = "pub struct Foo { x: u32 }";
/// let ast: syn::ItemStruct = syn::parse_str(code).unwrap();
/// // extract_line_range is a private helper function
/// // that extracts (start_line, end_line) from the span
/// ```
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

/// Extract #[path = "..."] attribute from module declaration (Spec 170)
///
/// Parses module attributes to find path declarations that indicate
/// external submodule files.
fn extract_path_attribute(module: &syn::ItemMod) -> Option<String> {
    for attr in &module.attrs {
        if attr.path().is_ident("path") {
            if let syn::Meta::NameValue(meta) = &attr.meta {
                if let syn::Expr::Lit(expr_lit) = &meta.value {
                    if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                        return Some(lit_str.value());
                    }
                }
            }
        }
    }
    None
}

/// Classify organization quality based on submodule count and facade score (Spec 170)
///
/// Returns classification from Excellent to Monolithic based on the
/// degree of modular organization in the file.
fn classify_organization_quality(submodule_count: usize, facade_score: f64) -> OrganizationQuality {
    match (submodule_count, facade_score) {
        (0..=2, _) => OrganizationQuality::Monolithic,
        (n, s) if n >= 10 && s >= 0.8 => OrganizationQuality::Excellent,
        (n, s) if n >= 5 && s >= 0.6 => OrganizationQuality::Good,
        (n, s) if n >= 3 && s >= 0.5 => OrganizationQuality::Poor,
        _ => OrganizationQuality::Monolithic,
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
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let structure = analyzer.analyze_rust_ast(&ast);

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
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let structure = analyzer.analyze_rust_ast(&ast);

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
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let structure = analyzer.analyze_rust_ast(&ast);

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
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let structure = analyzer.analyze_rust_ast(&ast);

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

    // Spec 170: Facade detection tests
    #[test]
    fn test_detect_pure_facade_with_path_attributes() {
        let code = r#"
            #[path = "executor/builder.rs"]
            mod builder;
            #[path = "executor/commands.rs"]
            pub mod commands;
            #[path = "executor/pure.rs"]
            pub(crate) mod pure;

            pub use builder::Builder;
            pub use commands::execute;
        "#;

        let ast = syn::parse_file(code).unwrap();
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let facade_info = analyzer.detect_module_facade(&ast);

        assert!(facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 3);
        assert_eq!(facade_info.path_declarations.len(), 3);
        assert!(facade_info.facade_score > 0.8);
        // With 3 modules and high facade score, it's still only "Poor" quality
        // because the threshold requires >=5 modules for "Good"
        assert_eq!(facade_info.organization_quality, OrganizationQuality::Poor);
    }

    #[test]
    fn test_detect_monolithic_file_no_modules() {
        let code = r#"
            struct Foo { x: u32 }

            impl Foo {
                fn method1(&self) -> u32 { self.x }
                fn method2(&self) -> u32 { self.x * 2 }
                fn method3(&self) -> u32 { self.x * 3 }
            }

            fn standalone1() { println!("test"); }
            fn standalone2() { println!("test"); }
            fn standalone3() { println!("test"); }
        "#;

        let ast = syn::parse_file(code).unwrap();
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let facade_info = analyzer.detect_module_facade(&ast);

        assert!(!facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 0);
        // With no submodules, facade score should be low (< 0.5)
        assert!(
            facade_info.facade_score < 0.5,
            "Expected facade_score < 0.5, got {}",
            facade_info.facade_score
        );
        assert_eq!(
            facade_info.organization_quality,
            OrganizationQuality::Monolithic
        );
    }

    #[test]
    fn test_detect_excellent_facade() {
        let code = r#"
            #[path = "executor/builder.rs"]
            mod builder;
            #[path = "executor/commands.rs"]
            mod commands;
            #[path = "executor/pure.rs"]
            mod pure;
            #[path = "executor/data.rs"]
            mod data;
            #[path = "executor/types.rs"]
            mod types;
            #[path = "executor/errors.rs"]
            mod errors;
            #[path = "executor/config.rs"]
            mod config;
            #[path = "executor/validation.rs"]
            mod validation;
            #[path = "executor/helpers.rs"]
            mod helpers;
            #[path = "executor/hooks.rs"]
            mod hooks;

            pub use builder::Builder;
            pub use commands::*;
            pub use types::{Type1, Type2};
        "#;

        let ast = syn::parse_file(code).unwrap();
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let facade_info = analyzer.detect_module_facade(&ast);

        assert!(facade_info.is_facade);
        assert_eq!(facade_info.submodule_count, 10);
        assert!(facade_info.facade_score > 0.85);
        assert_eq!(
            facade_info.organization_quality,
            OrganizationQuality::Excellent
        );
    }

    #[test]
    fn test_extract_path_attribute() {
        let code = r#"
            #[path = "foo/bar.rs"]
            mod test_module;
        "#;

        let ast = syn::parse_file(code).unwrap();
        if let syn::Item::Mod(module) = &ast.items[0] {
            let path = extract_path_attribute(module);
            assert_eq!(path, Some("foo/bar.rs".to_string()));
        } else {
            panic!("Expected module item");
        }
    }

    #[test]
    fn test_classify_organization_quality_thresholds() {
        assert_eq!(
            classify_organization_quality(13, 0.92),
            OrganizationQuality::Excellent
        );

        assert_eq!(
            classify_organization_quality(6, 0.65),
            OrganizationQuality::Good
        );

        assert_eq!(
            classify_organization_quality(3, 0.55),
            OrganizationQuality::Poor
        );

        assert_eq!(
            classify_organization_quality(1, 0.2),
            OrganizationQuality::Monolithic
        );

        assert_eq!(
            classify_organization_quality(5, 0.45),
            OrganizationQuality::Monolithic
        );
    }

    #[test]
    fn test_mixed_inline_and_path_modules() {
        let code = r#"
            #[path = "external.rs"]
            mod external;

            mod inline {
                pub fn helper() {}
            }

            #[path = "another.rs"]
            mod another;
        "#;

        let ast = syn::parse_file(code).unwrap();
        let analyzer = ModuleStructureAnalyzer::new_rust();
        let facade_info = analyzer.detect_module_facade(&ast);

        assert_eq!(facade_info.submodule_count, 3); // 2 path + 1 inline
        assert_eq!(facade_info.path_declarations.len(), 2);
        assert!(facade_info.is_facade);
    }
}
