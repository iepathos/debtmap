//! Rust Type Extraction
//!
//! Extracts type signatures from Rust AST (using syn):
//! - Parameters with types
//! - Return types
//! - Generic bounds (including where clauses)
//! - Error types from Result<T, E>

use crate::analysis::type_signatures::analyzer::{GenericBound, Parameter, TypeSignature};
use crate::analysis::type_signatures::normalizer::{CanonicalType, TypeNormalizer};
use quote::ToTokens;

/// Extract type signature from Rust function
///
/// Composes pure extraction functions to build a complete type signature.
pub fn extract_rust_signature(
    function: &syn::ItemFn,
    normalizer: &TypeNormalizer,
) -> TypeSignature {
    let parameters = extract_parameters(&function.sig.inputs, normalizer);
    let return_type = extract_return_type(&function.sig.output, normalizer);
    let error_type = extract_error_type(return_type.as_ref());
    let generic_bounds = extract_generic_bounds(&function.sig.generics);

    TypeSignature {
        parameters,
        return_type,
        generic_bounds,
        error_type,
    }
}

/// Extract parameters from function inputs
fn extract_parameters(
    inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
    normalizer: &TypeNormalizer,
) -> Vec<Parameter> {
    inputs
        .iter()
        .filter_map(|arg| extract_typed_param(arg, normalizer))
        .collect()
}

/// Extract a single typed parameter, ignoring self parameters
fn extract_typed_param(arg: &syn::FnArg, normalizer: &TypeNormalizer) -> Option<Parameter> {
    let syn::FnArg::Typed(pat_type) = arg else {
        return None;
    };

    let canonical = normalizer.normalize(&pat_type.ty);
    Some(Parameter {
        name: extract_param_name(&pat_type.pat),
        type_annotation: canonical.clone(),
        is_reference: canonical.is_reference,
        is_mutable: canonical.is_mutable,
    })
}

/// Extract return type from function output
fn extract_return_type(
    output: &syn::ReturnType,
    normalizer: &TypeNormalizer,
) -> Option<CanonicalType> {
    match output {
        syn::ReturnType::Type(_, ty) => Some(normalizer.normalize(ty)),
        syn::ReturnType::Default => None,
    }
}

/// Extract error type from Result<T, E> return type
fn extract_error_type(return_type: Option<&CanonicalType>) -> Option<CanonicalType> {
    return_type
        .filter(|rt| rt.base == "Result" && rt.generics.len() == 2)
        .map(|rt| rt.generics[1].clone())
}

/// Extract all generic bounds from generics (type params + where clause)
fn extract_generic_bounds(generics: &syn::Generics) -> Vec<GenericBound> {
    let type_param_bounds = extract_type_param_bounds(&generics.params);
    let where_clause_bounds = extract_where_clause_bounds(generics.where_clause.as_ref());

    type_param_bounds
        .into_iter()
        .chain(where_clause_bounds)
        .collect()
}

/// Extract bounds from type parameters (e.g., `<T: Read + Write>`)
fn extract_type_param_bounds(
    params: &syn::punctuated::Punctuated<syn::GenericParam, syn::token::Comma>,
) -> Vec<GenericBound> {
    params.iter().filter_map(extract_type_param_bound).collect()
}

/// Extract bound from a single generic parameter
fn extract_type_param_bound(param: &syn::GenericParam) -> Option<GenericBound> {
    let syn::GenericParam::Type(type_param) = param else {
        return None;
    };

    let bounds = collect_trait_bounds(&type_param.bounds);
    if bounds.is_empty() {
        return None;
    }

    Some(GenericBound {
        type_param: type_param.ident.to_string(),
        trait_bounds: bounds,
    })
}

/// Extract bounds from where clause
fn extract_where_clause_bounds(where_clause: Option<&syn::WhereClause>) -> Vec<GenericBound> {
    where_clause
        .map(|wc| {
            wc.predicates
                .iter()
                .filter_map(extract_where_predicate_bound)
                .collect()
        })
        .unwrap_or_default()
}

/// Extract bound from a single where predicate
fn extract_where_predicate_bound(predicate: &syn::WherePredicate) -> Option<GenericBound> {
    let syn::WherePredicate::Type(type_pred) = predicate else {
        return None;
    };

    let bounds = collect_trait_bounds(&type_pred.bounds);
    if bounds.is_empty() {
        return None;
    }

    Some(GenericBound {
        type_param: type_pred.bounded_ty.to_token_stream().to_string(),
        trait_bounds: bounds,
    })
}

/// Collect trait bounds as strings from a bounds iterator
fn collect_trait_bounds<'a>(
    bounds: impl IntoIterator<Item = &'a syn::TypeParamBound>,
) -> Vec<String> {
    bounds
        .into_iter()
        .map(|b| b.to_token_stream().to_string())
        .collect()
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
