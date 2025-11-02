//! Rust Type Extraction
//!
//! Extracts type signatures from Rust AST (using syn):
//! - Parameters with types
//! - Return types
//! - Generic bounds (including where clauses)
//! - Error types from Result<T, E>

use crate::analysis::type_signatures::analyzer::{GenericBound, Parameter, TypeSignature};
use crate::analysis::type_signatures::normalizer::TypeNormalizer;
use quote::ToTokens;

/// Extract type signature from Rust function
pub fn extract_rust_signature(
    function: &syn::ItemFn,
    normalizer: &TypeNormalizer,
) -> TypeSignature {
    let parameters: Vec<Parameter> = function
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let syn::FnArg::Typed(pat_type) = arg {
                // Use normalizer to get CanonicalType
                let canonical = normalizer.normalize(&pat_type.ty);

                Some(Parameter {
                    name: extract_param_name(&pat_type.pat),
                    type_annotation: canonical.clone(),
                    is_reference: canonical.is_reference,
                    is_mutable: canonical.is_mutable,
                })
            } else {
                None
            }
        })
        .collect();

    // Use normalizer for return type
    let return_type = match &function.sig.output {
        syn::ReturnType::Type(_, ty) => Some(normalizer.normalize(ty)),
        syn::ReturnType::Default => None,
    };

    // Extract error type from Result's second generic argument
    let error_type = return_type.as_ref().and_then(|rt| {
        if rt.base == "Result" && rt.generics.len() == 2 {
            Some(rt.generics[1].clone())
        } else {
            None
        }
    });

    // Extract generic bounds with where clause support
    let mut generic_bounds = Vec::new();

    // Process type parameters
    for param in &function.sig.generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            let bounds: Vec<String> = type_param
                .bounds
                .iter()
                .map(|bound| bound.to_token_stream().to_string())
                .collect();

            if !bounds.is_empty() {
                generic_bounds.push(GenericBound {
                    type_param: type_param.ident.to_string(),
                    trait_bounds: bounds,
                });
            }
        }
    }

    // Process where clause
    if let Some(where_clause) = &function.sig.generics.where_clause {
        for predicate in &where_clause.predicates {
            if let syn::WherePredicate::Type(type_pred) = predicate {
                let type_str = type_pred.bounded_ty.to_token_stream().to_string();
                let bounds: Vec<String> = type_pred
                    .bounds
                    .iter()
                    .map(|bound| bound.to_token_stream().to_string())
                    .collect();

                if !bounds.is_empty() {
                    generic_bounds.push(GenericBound {
                        type_param: type_str,
                        trait_bounds: bounds,
                    });
                }
            }
        }
    }

    TypeSignature {
        parameters,
        return_type,
        generic_bounds,
        error_type,
    }
}

fn extract_param_name(pat: &syn::Pat) -> String {
    match pat {
        syn::Pat::Ident(ident) => ident.ident.to_string(),
        syn::Pat::Type(pat_type) => extract_param_name(&pat_type.pat),
        syn::Pat::Reference(pat_ref) => extract_param_name(&pat_ref.pat),
        syn::Pat::Wild(_) => "_".into(),
        _ => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_simple_function() {
        let code = r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
        "#;

        let item: syn::ItemFn = syn::parse_str(code).unwrap();
        let normalizer = TypeNormalizer::new();
        let signature = extract_rust_signature(&item, &normalizer);

        assert_eq!(signature.parameters.len(), 2);
        assert_eq!(signature.parameters[0].name, "a");
        assert_eq!(signature.parameters[0].type_annotation.base, "i32");
        assert_eq!(signature.return_type.as_ref().unwrap().base, "i32");
    }

    #[test]
    fn extract_parser_function() {
        let code = r#"
            fn parse_config(content: &str) -> Result<Config, ParseError> {
                todo!()
            }
        "#;

        let item: syn::ItemFn = syn::parse_str(code).unwrap();
        let normalizer = TypeNormalizer::new();
        let signature = extract_rust_signature(&item, &normalizer);

        assert_eq!(signature.parameters.len(), 1);
        assert_eq!(signature.parameters[0].type_annotation.base, "str");
        assert!(signature.parameters[0].is_reference);

        let return_type = signature.return_type.as_ref().unwrap();
        assert_eq!(return_type.base, "Result");
        assert_eq!(return_type.generics.len(), 2);
        assert_eq!(return_type.generics[0].base, "Config");
        assert_eq!(return_type.generics[1].base, "ParseError");

        let error_type = signature.error_type.as_ref().unwrap();
        assert_eq!(error_type.base, "ParseError");
    }

    #[test]
    fn extract_io_function() {
        let code = r#"
            fn read_file(path: &Path) -> Result<String, io::Error> {
                todo!()
            }
        "#;

        let item: syn::ItemFn = syn::parse_str(code).unwrap();
        let normalizer = TypeNormalizer::new();
        let signature = extract_rust_signature(&item, &normalizer);

        assert_eq!(signature.parameters.len(), 1);
        assert_eq!(signature.parameters[0].type_annotation.base, "Path");

        let error_type = signature.error_type.as_ref().unwrap();
        assert_eq!(error_type.base, "io::Error");
    }

    #[test]
    fn extract_generic_bounds() {
        let code = r#"
            fn process<T: Read + Write>(reader: T) -> Result<(), io::Error> {
                todo!()
            }
        "#;

        let item: syn::ItemFn = syn::parse_str(code).unwrap();
        let normalizer = TypeNormalizer::new();
        let signature = extract_rust_signature(&item, &normalizer);

        assert_eq!(signature.generic_bounds.len(), 1);
        assert_eq!(signature.generic_bounds[0].type_param, "T");
        assert!(signature.generic_bounds[0]
            .trait_bounds
            .iter()
            .any(|b| b.contains("Read")));
    }

    #[test]
    fn extract_where_clause() {
        let code = r#"
            fn process<T>(data: T) -> Result<(), Error>
            where
                T: Iterator<Item = String>,
            {
                todo!()
            }
        "#;

        let item: syn::ItemFn = syn::parse_str(code).unwrap();
        let normalizer = TypeNormalizer::new();
        let signature = extract_rust_signature(&item, &normalizer);

        assert_eq!(signature.generic_bounds.len(), 1);
        assert_eq!(signature.generic_bounds[0].type_param, "T");
        assert!(signature.generic_bounds[0]
            .trait_bounds
            .iter()
            .any(|b| b.contains("Iterator")));
    }
}
