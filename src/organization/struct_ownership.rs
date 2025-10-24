/// Struct ownership analysis for Rust codebases.
///
/// This module analyzes Rust source code to determine which methods belong to which structs,
/// distinguishing between inherent implementations and trait implementations. This information
/// is used to generate more accurate module split recommendations based on struct ownership
/// rather than just method name patterns.
use std::collections::HashMap;
use syn::{self, visit::Visit, File, ImplItem, Item, ItemFn, ItemImpl, ItemStruct};

/// Tracks struct ownership relationships in a Rust file.
///
/// Maps methods to their owner structs and identifies standalone functions.
/// Excludes trait implementations as they cannot be moved independently.
#[derive(Debug, Clone, Default)]
pub struct StructOwnershipAnalyzer {
    /// Maps struct names to their methods
    struct_to_methods: HashMap<String, Vec<String>>,
    /// Maps method names to their struct owner
    method_to_struct: HashMap<String, String>,
    /// Standalone functions (not part of any struct)
    standalone_functions: Vec<String>,
    /// Struct line spans (start_line, end_line)
    struct_locations: HashMap<String, (usize, usize)>,
}

impl StructOwnershipAnalyzer {
    /// Analyze a parsed Rust file to extract struct ownership information.
    ///
    /// # Arguments
    ///
    /// * `parsed` - Parsed AST from syn
    ///
    /// # Returns
    ///
    /// A `StructOwnershipAnalyzer` containing the ownership mappings.
    pub fn analyze_file(parsed: &File) -> Self {
        let mut visitor = StructVisitor::default();
        visitor.visit_file(parsed);
        visitor.analyzer
    }

    /// Get all methods for a specific struct.
    ///
    /// Returns an empty slice if the struct has no methods or doesn't exist.
    pub fn get_struct_methods(&self, struct_name: &str) -> &[String] {
        self.struct_to_methods
            .get(struct_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get the struct that owns a specific method.
    ///
    /// Returns None if the method is standalone or doesn't exist.
    pub fn get_method_struct(&self, method_name: &str) -> Option<&str> {
        self.method_to_struct.get(method_name).map(|s| s.as_str())
    }

    /// Get all standalone functions (not part of any struct).
    pub fn get_standalone_functions(&self) -> &[String] {
        &self.standalone_functions
    }

    /// Get all struct names tracked by this analyzer.
    pub fn get_struct_names(&self) -> Vec<&str> {
        self.struct_to_methods.keys().map(|s| s.as_str()).collect()
    }

    /// Get the line span for a struct.
    pub fn get_struct_location(&self, struct_name: &str) -> Option<(usize, usize)> {
        self.struct_locations.get(struct_name).copied()
    }

    /// Get total method count across all structs.
    pub fn total_method_count(&self) -> usize {
        self.struct_to_methods.values().map(|v| v.len()).sum()
    }
}

/// Visitor for collecting struct ownership information from AST.
#[derive(Default)]
struct StructVisitor {
    analyzer: StructOwnershipAnalyzer,
}

impl StructVisitor {
    /// Extract the base type name from a syn::Type, handling generics.
    ///
    /// For example: `Container<T>` becomes `Container`
    fn extract_type_name(ty: &syn::Type) -> Option<String> {
        match ty {
            syn::Type::Path(type_path) => type_path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string()),
            _ => None,
        }
    }

    /// Track a struct definition.
    fn track_struct(&mut self, item_struct: &ItemStruct) {
        let struct_name = item_struct.ident.to_string();

        // Initialize empty method list for this struct
        self.analyzer
            .struct_to_methods
            .entry(struct_name.clone())
            .or_default();

        // Track location (approximation - syn doesn't give us exact line numbers without source)
        // We'll use 0,0 as placeholder - can be improved with location_extractor if needed
        self.analyzer.struct_locations.insert(struct_name, (0, 0));
    }

    /// Track an impl block (only inherent impls, not trait impls).
    fn track_impl(&mut self, item_impl: &ItemImpl) {
        // Skip trait implementations
        if item_impl.trait_.is_some() {
            return;
        }

        // Extract struct name from self type
        let struct_name = match Self::extract_type_name(&item_impl.self_ty) {
            Some(name) => name,
            None => return,
        };

        // Ensure struct entry exists
        let methods = self
            .analyzer
            .struct_to_methods
            .entry(struct_name.clone())
            .or_default();

        // Extract method names from impl block
        for item in &item_impl.items {
            if let ImplItem::Fn(method) = item {
                let method_name = method.sig.ident.to_string();
                methods.push(method_name.clone());
                self.analyzer
                    .method_to_struct
                    .insert(method_name, struct_name.clone());
            }
        }
    }

    /// Track a standalone function.
    fn track_standalone_function(&mut self, item_fn: &ItemFn) {
        let fn_name = item_fn.sig.ident.to_string();
        self.analyzer.standalone_functions.push(fn_name);
    }
}

impl<'ast> Visit<'ast> for StructVisitor {
    fn visit_item(&mut self, item: &'ast Item) {
        match item {
            Item::Struct(item_struct) => {
                self.track_struct(item_struct);
            }
            Item::Impl(item_impl) => {
                self.track_impl(item_impl);
            }
            Item::Fn(item_fn) => {
                self.track_standalone_function(item_fn);
            }
            _ => {}
        }

        // Continue visiting nested items
        syn::visit::visit_item(self, item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_struct_ownership() {
        let code = r#"
            struct Config {
                value: String,
            }

            impl Config {
                fn new() -> Self {
                    Config { value: String::new() }
                }
                fn get_value(&self) -> &str {
                    &self.value
                }
            }

            fn standalone_helper() { }
        "#;

        let parsed = syn::parse_file(code).unwrap();
        let analyzer = StructOwnershipAnalyzer::analyze_file(&parsed);

        assert_eq!(analyzer.get_struct_methods("Config"), &["new", "get_value"]);
        assert_eq!(analyzer.get_standalone_functions(), &["standalone_helper"]);
        assert_eq!(analyzer.get_method_struct("new"), Some("Config"));
        assert_eq!(analyzer.get_method_struct("get_value"), Some("Config"));
    }

    #[test]
    fn test_multiple_impl_blocks() {
        let code = r#"
            struct Config { }

            impl Config {
                fn new() -> Self { Config {} }
            }

            impl Config {
                fn get_value(&self) -> i32 { 42 }
            }
        "#;

        let parsed = syn::parse_file(code).unwrap();
        let analyzer = StructOwnershipAnalyzer::analyze_file(&parsed);

        let methods = analyzer.get_struct_methods("Config");
        assert_eq!(methods.len(), 2);
        assert!(methods.contains(&"new".to_string()));
        assert!(methods.contains(&"get_value".to_string()));
    }

    #[test]
    fn test_trait_impl_exclusion() {
        let code = r#"
            use std::fmt::{Display, Formatter, Result};

            struct Config { }

            impl Config {
                fn new() -> Self { Config {} }
            }

            impl Display for Config {
                fn fmt(&self, f: &mut Formatter) -> Result {
                    Ok(())
                }
            }
        "#;

        let parsed = syn::parse_file(code).unwrap();
        let analyzer = StructOwnershipAnalyzer::analyze_file(&parsed);

        // Should only include inherent impl methods, not trait methods
        assert_eq!(analyzer.get_struct_methods("Config"), &["new"]);
        assert_eq!(analyzer.get_method_struct("fmt"), None);
    }

    #[test]
    fn test_generic_impl_blocks() {
        let code = r#"
            struct Container<T> {
                value: T,
            }

            impl<T> Container<T> {
                fn new(value: T) -> Self { Container { value } }
                fn get(&self) -> &T { &self.value }
            }
        "#;

        let parsed = syn::parse_file(code).unwrap();
        let analyzer = StructOwnershipAnalyzer::analyze_file(&parsed);

        // Should handle generic impl blocks
        assert_eq!(analyzer.get_struct_methods("Container"), &["new", "get"]);
    }

    #[test]
    fn test_empty_impl_block() {
        let code = r#"
            struct Config { }
            impl Config { }
        "#;

        let parsed = syn::parse_file(code).unwrap();
        let analyzer = StructOwnershipAnalyzer::analyze_file(&parsed);

        // Should handle empty impl blocks gracefully
        assert_eq!(analyzer.get_struct_methods("Config"), &[] as &[String]);
    }

    #[test]
    fn test_multiple_structs() {
        let code = r#"
            struct ConfigA { }
            struct ConfigB { }

            impl ConfigA {
                fn method_a(&self) { }
            }

            impl ConfigB {
                fn method_b(&self) { }
            }
        "#;

        let parsed = syn::parse_file(code).unwrap();
        let analyzer = StructOwnershipAnalyzer::analyze_file(&parsed);

        assert_eq!(analyzer.get_struct_methods("ConfigA"), &["method_a"]);
        assert_eq!(analyzer.get_struct_methods("ConfigB"), &["method_b"]);
        assert_eq!(analyzer.get_method_struct("method_a"), Some("ConfigA"));
        assert_eq!(analyzer.get_method_struct("method_b"), Some("ConfigB"));

        let struct_names = analyzer.get_struct_names();
        assert_eq!(struct_names.len(), 2);
        assert!(struct_names.contains(&"ConfigA"));
        assert!(struct_names.contains(&"ConfigB"));
    }
}
