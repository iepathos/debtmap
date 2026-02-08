//! Dependency extraction
//!
//! Functions for extracting import dependencies from Rust code.

use crate::core::{Dependency, DependencyKind};
use syn::Item;

/// Extract dependencies from a Rust file
pub fn extract_dependencies(file: &syn::File) -> Vec<Dependency> {
    file.items
        .iter()
        .filter_map(|item| match item {
            Item::Use(use_item) => extract_use_name(&use_item.tree).map(|name| Dependency {
                name,
                kind: DependencyKind::Import,
            }),
            _ => None,
        })
        .collect()
}

/// Extract the first segment name from a use tree
pub fn extract_use_name(tree: &syn::UseTree) -> Option<String> {
    match tree {
        syn::UseTree::Path(path) => Some(path.ident.to_string()),
        syn::UseTree::Name(name) => Some(name.ident.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_extract_dependencies() {
        let file: syn::File = parse_quote! {
            use std::io;
            use serde::{Deserialize, Serialize};
            use crate::core::Config;

            fn main() {}
        };

        let deps = extract_dependencies(&file);
        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.name == "std"));
        assert!(deps.iter().any(|d| d.name == "serde"));
        assert!(deps.iter().any(|d| d.name == "crate"));
        assert!(deps.iter().all(|d| d.kind == DependencyKind::Import));
    }

    #[test]
    fn test_extract_use_name() {
        let tree: syn::UseTree = parse_quote!(std);
        assert_eq!(extract_use_name(&tree), Some("std".to_string()));

        let tree: syn::UseTree = parse_quote!(std::io);
        assert_eq!(extract_use_name(&tree), Some("std".to_string()));

        let tree: syn::UseTree = parse_quote!(serde);
        assert_eq!(extract_use_name(&tree), Some("serde".to_string()));
    }
}
