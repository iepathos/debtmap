use super::*;
use crate::analyzers::rust_resolution::index::Context;
use std::path::PathBuf;

fn with_body(check: impl FnOnce(&Body<'_>)) {
    let index = WorkspaceIndex::build(&[(
        PathBuf::from("src/lib.rs"),
        syn::parse_quote! {
            struct A; struct W<T>(T); trait T {} trait Q<X> { fn hit(&self); }
            impl A { fn hit(&self) {} }
            impl Q<A> for W<A> { fn hit(&self) {} }
            mod provider {
                pub struct A;
                pub struct Holder<X> { pub own: A, pub input: X }
                pub type Alias<X> = Holder<X>;
                pub fn returned() -> A { A }
            }
            fn caller() {}
        },
    )]);
    let callable = index
        .callables()
        .iter()
        .find(|call| call.id.name == "caller")
        .unwrap();
    let mut graph = CallGraph::new();
    let mut body = Body::new(&index, callable, &mut graph);
    body.bindings.shadow_item("A".into(), true, false);
    body.bindings.shadow_item("T".into(), true, false);
    check(&body);
}

#[test]
fn nested_shadow_preserves_every_enclosing_type_constructor() {
    with_body(|body| {
        let fact = body.declared_type(&syn::parse_quote!(&W<(A, &crate::A)>));
        let TypeFact::Nominal { arguments, .. } = fact.dereferenced() else {
            panic!("owner lost")
        };
        let TypeFact::Tuple(fields) = &arguments[0] else {
            panic!("tuple lost")
        };
        assert!(matches!(&fields[0], TypeFact::Uncertain { constraint, .. }
            if matches!(&**constraint, TypeFact::UnavailablePath(path) if path == &["A"])));
        assert!(fields[1].is_known());
    });
}

#[test]
fn unavailable_dynamic_bound_keeps_origin_through_reference_projection() {
    with_body(|body| {
        let fact = body
            .declared_type(&syn::parse_quote!(&&dyn T))
            .dereferenced()
            .dereferenced();
        let TypeFact::Dynamic(bounds) = fact else {
            panic!("bound lost")
        };
        assert_eq!(
            bounds,
            vec![TraitBoundFact::Unresolved {
                path: vec!["T".into()],
                file: PathBuf::from("src/lib.rs"),
                module: vec![],
                candidates: vec![],
                reason: UnknownReason::UnsupportedTypeOperation,
            }]
        );
    });
}

#[test]
fn aliases_fields_and_returns_expand_in_declaration_scope() {
    with_body(|body| {
        let owner = body.declared_type(&syn::parse_quote!(provider::Alias<A>));
        let own = body.index.field_type(&owner, &syn::parse_quote!(own));
        let input = body.index.field_type(&owner, &syn::parse_quote!(input));
        assert_eq!(own.nominal().unwrap().0.module, ["provider"]);
        assert!(matches!(input, TypeFact::Uncertain { .. }));
        let context = Context {
            file: PathBuf::from("src/lib.rs"),
            module: vec![],
        };
        let lookup = body
            .index
            .lookup_call(&syn::parse_quote!(provider::returned), None, &context);
        let output = body.index.return_type(lookup.candidates[0], None, &[]);
        assert_eq!(output, own);
    });
}

#[test]
fn equivalent_associated_syntax_has_identical_scoped_arguments() {
    with_body(|body| {
        let first = body
            .associated_query(&syn::parse_quote!(W::<A>::get::<&A>))
            .unwrap();
        let second = body
            .associated_query(&syn::parse_quote!(<W<A>>::get::<&A>))
            .unwrap();
        assert_eq!(first.owner, second.owner);
        assert_eq!(first.arguments, second.arguments);
        assert!(
            matches!(&first.arguments[0], TypeFact::Reference { inner, .. }
            if matches!(&**inner, TypeFact::Uncertain { .. }))
        );
    });
}

#[test]
fn nested_trait_arguments_keep_trait_identity_and_caller_scope() {
    with_body(|body| {
        let query = body
            .associated_query(&syn::parse_quote!(<W<crate::A> as crate::Q<(&A,)>>::get))
            .unwrap();
        let TypeFact::Nominal {
            declaration,
            arguments,
        } = query.trait_type.unwrap()
        else {
            panic!("trait identity lost")
        };
        assert_eq!(declaration.name, "Q");
        let TypeFact::Tuple(fields) = &arguments[0] else {
            panic!("tuple lost")
        };
        assert!(matches!(&fields[0], TypeFact::Reference { inner, .. }
            if matches!(&**inner, TypeFact::Uncertain { .. })));
    });
}

#[test]
fn unsupported_types_retain_shadow_constraints_at_the_unsupported_leaf() {
    with_body(|body| {
        for ty in [
            syn::parse_quote!(&<A as Has>::Out),
            syn::parse_quote!(&[A; 1]),
            syn::parse_quote!(&*const A),
            syn::parse_quote!(&fn(A)),
        ] {
            let projected = body.declared_type(&ty).dereferenced();
            assert!(
                body.index
                    .lookup_methods(&projected, "hit", &body.callable.context)
                    .candidates
                    .is_empty()
            );
            assert!(matches!(projected, TypeFact::Uncertain { constraint, .. }
                if matches!(*constraint, TypeFact::UnavailablePath(_))));
        }
        let qualified = body.declared_type(&syn::parse_quote!(&<crate::A as Has>::Out));
        assert!(matches!(qualified.dereferenced(), TypeFact::Unknown(_)));
    });
}

#[test]
fn dynamic_bound_arguments_cannot_reinterpret_a_local_shadow() {
    with_body(|body| {
        for ty in [
            syn::parse_quote!(&dyn Q<A>),
            syn::parse_quote!(&dyn Q<(&A,)>),
            syn::parse_quote!(&dyn Q<<A as Has>::Out>),
        ] {
            let projected = body.declared_type(&ty).dereferenced();
            assert!(
                body.index
                    .lookup_methods(&projected, "hit", &body.callable.context)
                    .candidates
                    .is_empty()
            );
            let TypeFact::Dynamic(bounds) = projected else {
                panic!("dynamic bound lost")
            };
            assert!(matches!(&bounds[0], TraitBoundFact::Unresolved {
                candidates, reason: UnknownReason::UnsupportedTypeOperation, ..
            } if candidates.is_empty()));
        }
        let TypeFact::Dynamic(bounds) = body
            .declared_type(&syn::parse_quote!(&dyn Q<crate::A>))
            .dereferenced()
        else {
            panic!("qualified bound lost")
        };
        assert!(matches!(&bounds[0], TraitBoundFact::Resolved(id) if id.name == "Q"));
    });
}
