//! Declaration metadata and lexical path transformations.

use super::*;

pub(super) fn make_callable(
    context: &Context,
    prefix: &[String],
    signature: &syn::Signature,
    attrs: &[syn::Attribute],
    body: Option<syn::Block>,
) -> Callable {
    let name = qualified(prefix, &signature.ident.to_string()).join("::");
    let id = FunctionId::with_module_path(
        context.file.clone(),
        name,
        signature.ident.span().start().line,
        ModuleTree::infer_module_from_file(&context.file),
    )
    .with_column(Some(signature.ident.span().start().column));
    Callable {
        id,
        context: context.clone(),
        signature: signature.clone(),
        owner: None,
        trait_path: None,
        trait_type: None,
        kind: CallableKind::FreeFunction,
        requirements_known: true,
        body,
        is_test: attrs.iter().any(|attr| {
            attr.path().segments.last().is_some_and(|segment| {
                matches!(
                    segment.ident.to_string().as_str(),
                    "test" | "rstest" | "test_case"
                )
            })
        }),
        substitutions: generic_substitutions(&signature.generics),
        const_parameters: signature
            .generics
            .const_params()
            .map(|parameter| parameter.ident.to_string())
            .collect(),
    }
}

pub(super) fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect()
}

pub(super) fn resolution_segments(path: &syn::Path) -> Vec<String> {
    path.leading_colon
        .iter()
        .map(|_| "::".to_string())
        .chain(path_segments(path))
        .collect()
}

pub(super) fn generic_names(generics: &syn::Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(ty) => Some(ty.ident.to_string()),
            syn::GenericParam::Const(constant) => Some(constant.ident.to_string()),
            syn::GenericParam::Lifetime(_) => None,
        })
        .collect()
}

pub(super) fn generic_substitutions(generics: &syn::Generics) -> Substitutions {
    generic_names(generics)
        .into_iter()
        .map(|name| {
            let traits = generic_bounds(generics, &name);
            let fact = if traits.is_empty() {
                TypeFact::Generic(name.clone())
            } else {
                TypeFact::BoundedGeneric {
                    name: name.clone(),
                    traits,
                }
            };
            (name, fact)
        })
        .collect()
}

fn generic_bounds(generics: &syn::Generics, name: &str) -> Vec<Vec<String>> {
    let direct = generics
        .type_params()
        .filter(|parameter| parameter.ident == name)
        .flat_map(|parameter| parameter.bounds.iter());
    let predicates = generics.where_clause.iter().flat_map(|clause| clause.predicates.iter())
        .filter_map(|predicate| match predicate { syn::WherePredicate::Type(predicate) => Some(predicate), _ => None })
        .filter(|predicate| matches!(&predicate.bounded_ty, syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident(name)))
        .flat_map(|predicate| predicate.bounds.iter());
    direct
        .chain(predicates)
        .filter_map(trait_bound_path)
        .collect()
}

pub(super) fn bind_generics(names: &[String], arguments: &[TypeFact]) -> Substitutions {
    names
        .iter()
        .cloned()
        .zip(arguments.iter().cloned())
        .collect()
}

pub(super) fn complete_arguments(arguments: Vec<TypeFact>, generics: &[String]) -> Vec<TypeFact> {
    let count = arguments.len();
    arguments
        .into_iter()
        .chain(generics.iter().skip(count).cloned().map(TypeFact::Generic))
        .collect()
}

pub(super) fn requirements_known(generics: &syn::Generics) -> bool {
    generics.where_clause.is_none()
        && generics.params.iter().all(|parameter| match parameter {
            syn::GenericParam::Type(ty) => ty.bounds.is_empty(),
            _ => true,
        })
}

pub(super) fn trait_bound_path(bound: &syn::TypeParamBound) -> Option<Vec<String>> {
    match bound {
        syn::TypeParamBound::Trait(bound)
            if matches!(bound.modifier, syn::TraitBoundModifier::None) =>
        {
            Some(resolution_segments(&bound.path))
        }
        _ => None,
    }
}

pub(super) fn qualified(prefix: &[String], name: &str) -> Vec<String> {
    prefix.iter().cloned().chain([name.to_string()]).collect()
}

pub(super) fn qualified_path(prefix: &[String], suffix: &[String]) -> Vec<String> {
    prefix.iter().chain(suffix).cloned().collect()
}

pub(super) fn child_context(context: &Context, name: &str) -> Context {
    Context {
        file: context.file.clone(),
        module: qualified(&context.module, name),
    }
}

pub(super) fn relative_path(path: &[String], module: &[String]) -> Vec<String> {
    let Some(first) = path.first() else {
        return Vec::new();
    };
    match first.as_str() {
        "::" => path.to_vec(),
        "crate" => path[1..].to_vec(),
        "self" => qualified_path(module, &path[1..]),
        "super" => {
            let count = path
                .iter()
                .take_while(|segment| segment.as_str() == "super")
                .count();
            if count > module.len() {
                return Vec::new();
            }
            qualified_path(&module[..module.len() - count], &path[count..])
        }
        _ => qualified_path(module, path),
    }
}

pub(super) fn has_trait_type_arguments(path: &syn::Path) -> bool {
    path.segments
        .iter()
        .any(|segment| match &segment.arguments {
            syn::PathArguments::None => false,
            syn::PathArguments::Parenthesized(_) => true,
            syn::PathArguments::AngleBracketed(arguments) => arguments
                .args
                .iter()
                .any(|argument| !matches!(argument, syn::GenericArgument::Lifetime(_))),
        })
}
