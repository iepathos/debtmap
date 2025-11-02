//! Type Normalization
//!
//! Converts syn AST types to canonical representation, handling:
//! - Type aliases (anyhow::Result → Result<T, anyhow::Error>)
//! - References and mutability
//! - Generics and nested types
//! - Trait objects and impl Trait

use quote::ToTokens;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Canonical type representation (normalized)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalType {
    pub base: String,
    pub generics: Vec<CanonicalType>,
    pub is_reference: bool,
    pub is_mutable: bool,
}

impl Hash for CanonicalType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.base.hash(state);
        self.generics.hash(state);
        self.is_reference.hash(state);
        self.is_mutable.hash(state);
    }
}

/// Type normalizer for handling aliases
pub struct TypeNormalizer {
    aliases: HashMap<String, String>,
}

impl TypeNormalizer {
    pub fn new() -> Self {
        let mut aliases = HashMap::new();

        // Common Rust type aliases
        aliases.insert("anyhow::Result".into(), "Result".into());
        aliases.insert("std::io::Result".into(), "Result".into());
        aliases.insert("std::result::Result".into(), "Result".into());
        aliases.insert("std::option::Option".into(), "Option".into());
        aliases.insert("std::vec::Vec".into(), "Vec".into());
        aliases.insert("std::collections::HashMap".into(), "HashMap".into());
        aliases.insert("std::collections::HashSet".into(), "HashSet".into());
        aliases.insert("std::collections::BTreeMap".into(), "BTreeMap".into());
        aliases.insert("std::collections::BTreeSet".into(), "BTreeSet".into());

        Self { aliases }
    }

    /// Normalize a syn::Type to canonical form
    pub fn normalize(&self, ty: &syn::Type) -> CanonicalType {
        match ty {
            syn::Type::Path(type_path) => self.normalize_path(type_path),
            syn::Type::Reference(type_ref) => {
                let mut inner = self.normalize(&type_ref.elem);
                inner.is_reference = true;
                inner.is_mutable = type_ref.mutability.is_some();
                inner
            }
            syn::Type::TraitObject(trait_obj) => self.normalize_trait_object(trait_obj),
            syn::Type::ImplTrait(impl_trait) => self.normalize_impl_trait(impl_trait),
            syn::Type::Tuple(tuple) => self.normalize_tuple(tuple),
            syn::Type::Ptr(ptr) => self.normalize_ptr(ptr),
            syn::Type::Paren(paren) => self.normalize(&paren.elem),
            _ => CanonicalType {
                base: "Unknown".into(),
                generics: vec![],
                is_reference: false,
                is_mutable: false,
            },
        }
    }

    fn normalize_path(&self, type_path: &syn::TypePath) -> CanonicalType {
        let path_str = type_path.to_token_stream().to_string();
        // Remove spaces from path for consistent matching
        let normalized_path = path_str.replace(' ', "");

        // Check for known aliases
        let base = self
            .aliases
            .get(&normalized_path)
            .cloned()
            .unwrap_or_else(|| {
                // For Error types, preserve module path for better classification
                if normalized_path.ends_with("Error") && normalized_path.contains("::") {
                    normalized_path.clone()
                } else {
                    type_path
                        .path
                        .segments
                        .last()
                        .map(|seg| seg.ident.to_string())
                        .unwrap_or_else(|| "Unknown".into())
                }
            });

        // Extract generic arguments
        let generics = if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                args.args
                    .iter()
                    .filter_map(|arg| {
                        if let syn::GenericArgument::Type(ty) = arg {
                            Some(self.normalize(ty))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        CanonicalType {
            base,
            generics,
            is_reference: false,
            is_mutable: false,
        }
    }

    fn normalize_trait_object(&self, trait_obj: &syn::TypeTraitObject) -> CanonicalType {
        let traits: Vec<String> = trait_obj
            .bounds
            .iter()
            .filter_map(|bound| {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    Some(trait_bound.to_token_stream().to_string())
                } else {
                    None
                }
            })
            .collect();

        CanonicalType {
            base: format!("dyn {}", traits.join(" + ")),
            generics: vec![],
            is_reference: false,
            is_mutable: false,
        }
    }

    fn normalize_impl_trait(&self, impl_trait: &syn::TypeImplTrait) -> CanonicalType {
        let traits: Vec<String> = impl_trait
            .bounds
            .iter()
            .filter_map(|bound| {
                if let syn::TypeParamBound::Trait(trait_bound) = bound {
                    Some(trait_bound.to_token_stream().to_string())
                } else {
                    None
                }
            })
            .collect();

        CanonicalType {
            base: format!("impl {}", traits.join(" + ")),
            generics: vec![],
            is_reference: false,
            is_mutable: false,
        }
    }

    fn normalize_tuple(&self, tuple: &syn::TypeTuple) -> CanonicalType {
        if tuple.elems.is_empty() {
            CanonicalType {
                base: "()".into(),
                generics: vec![],
                is_reference: false,
                is_mutable: false,
            }
        } else {
            let generics: Vec<CanonicalType> = tuple
                .elems
                .iter()
                .map(|elem| self.normalize(elem))
                .collect();

            CanonicalType {
                base: "Tuple".into(),
                generics,
                is_reference: false,
                is_mutable: false,
            }
        }
    }

    fn normalize_ptr(&self, ptr: &syn::TypePtr) -> CanonicalType {
        let inner = self.normalize(&ptr.elem);
        let prefix = if ptr.mutability.is_some() {
            "*mut "
        } else {
            "*const "
        };

        CanonicalType {
            base: format!("{}{}", prefix, inner.base),
            generics: inner.generics,
            is_reference: false,
            is_mutable: false,
        }
    }

    /// Convert canonical type to string representation
    #[allow(clippy::only_used_in_recursion)]
    pub fn canonical_to_string(&self, ty: &CanonicalType) -> String {
        let mut result = if ty.is_reference {
            if ty.is_mutable {
                "&mut ".to_string()
            } else {
                "&".to_string()
            }
        } else {
            String::new()
        };

        result.push_str(&ty.base);

        if !ty.generics.is_empty() {
            result.push('<');
            for (i, gen) in ty.generics.iter().enumerate() {
                if i > 0 {
                    result.push_str(", ");
                }
                result.push_str(&self.canonical_to_string(gen));
            }
            result.push('>');
        }

        result
    }
}

impl Default for TypeNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_simple_type() {
        let normalizer = TypeNormalizer::new();
        let ty: syn::Type = syn::parse_str("String").unwrap();
        let canonical = normalizer.normalize(&ty);

        assert_eq!(canonical.base, "String");
        assert!(canonical.generics.is_empty());
        assert!(!canonical.is_reference);
    }

    #[test]
    fn normalize_reference() {
        let normalizer = TypeNormalizer::new();
        let ty: syn::Type = syn::parse_str("&str").unwrap();
        let canonical = normalizer.normalize(&ty);

        assert_eq!(canonical.base, "str");
        assert!(canonical.is_reference);
        assert!(!canonical.is_mutable);
    }

    #[test]
    fn normalize_mutable_reference() {
        let normalizer = TypeNormalizer::new();
        let ty: syn::Type = syn::parse_str("&mut String").unwrap();
        let canonical = normalizer.normalize(&ty);

        assert_eq!(canonical.base, "String");
        assert!(canonical.is_reference);
        assert!(canonical.is_mutable);
    }

    #[test]
    fn normalize_result() {
        let normalizer = TypeNormalizer::new();
        let ty: syn::Type = syn::parse_str("Result<String, io::Error>").unwrap();
        let canonical = normalizer.normalize(&ty);

        assert_eq!(canonical.base, "Result");
        assert_eq!(canonical.generics.len(), 2);
        assert_eq!(canonical.generics[0].base, "String");
        assert_eq!(canonical.generics[1].base, "io::Error");
    }

    #[test]
    fn normalize_nested_generics() {
        let normalizer = TypeNormalizer::new();
        let ty: syn::Type = syn::parse_str("Result<Option<Vec<User>>, io::Error>").unwrap();
        let canonical = normalizer.normalize(&ty);

        assert_eq!(canonical.base, "Result");
        assert_eq!(canonical.generics.len(), 2);
        assert_eq!(canonical.generics[0].base, "Option");
        assert_eq!(canonical.generics[0].generics[0].base, "Vec");
        assert_eq!(canonical.generics[0].generics[0].generics[0].base, "User");
    }

    #[test]
    fn normalize_tuple() {
        let normalizer = TypeNormalizer::new();
        let ty: syn::Type = syn::parse_str("()").unwrap();
        let canonical = normalizer.normalize(&ty);

        assert_eq!(canonical.base, "()");
    }
}
