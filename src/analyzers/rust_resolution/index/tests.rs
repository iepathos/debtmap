use super::*;

fn index(source: &str) -> WorkspaceIndex {
    WorkspaceIndex::build(&[(
        PathBuf::from("src/lib.rs"),
        syn::parse_file(source).expect("valid fixture"),
    )])
}

fn root(index: &WorkspaceIndex) -> Context {
    index.context(Path::new("src/lib.rs"), &[])
}

fn fact(index: &WorkspaceIndex, source: &str) -> TypeFact {
    index.type_from_syn(
        &syn::parse_str(source).expect("valid type"),
        &root(index),
        &Substitutions::new(),
    )
}

#[test]
fn method_lookup_preserves_owner_and_receiver_constraints() {
    let index = index(
        "struct Timeline; struct PyTimeline; impl Timeline { fn bar(&self) {} fn associated() {} } impl PyTimeline { fn bar(&self) {} } fn bar() {}",
    );
    let receiver = fact(&index, "&Timeline");
    let lookup = index.lookup_methods(&receiver, "bar", &root(&index));
    assert!(lookup.justified);
    assert_eq!(lookup.candidates[0].id.name, "Timeline::bar");
    assert!(
        index
            .lookup_methods(&receiver, "associated", &root(&index))
            .candidates
            .is_empty()
    );
    let unknown = index.lookup_methods(&TypeFact::unknown(), "bar", &root(&index));
    assert!(!unknown.justified);
    assert_eq!(unknown.candidates.len(), 2);
}

#[test]
fn singleton_method_without_receiver_knowledge_is_uncertain() {
    let index = index("struct Foo; impl Foo { fn bar(&self) {} }");
    let lookup = index.lookup_methods(&TypeFact::unknown(), "bar", &root(&index));
    assert_eq!(lookup.candidates.len(), 1);
    assert!(!lookup.justified);
}

#[test]
fn same_names_in_inline_modules_have_distinct_identity() {
    let index = index(
        "mod a { pub struct Foo; impl Foo { pub fn bar(&self) {} } } mod b { pub struct Foo; impl Foo { pub fn bar(&self) {} } } use a::Foo as Alias;",
    );
    let receiver = fact(&index, "Alias");
    let lookup = index.lookup_methods(&receiver, "bar", &root(&index));
    assert!(lookup.justified);
    assert_eq!(lookup.candidates[0].id.name, "a::Foo::bar");
}

#[test]
fn generic_alias_fields_and_returns_substitute_without_lifetimes() {
    let index = index(
        "struct Foo; struct Wrapper<'a, T> { field: T, marker: &'a T } type Alias<T> = Wrapper<'static, T>; impl<T> Wrapper<'_, T> { fn get(&self) -> T { todo!() } fn new() -> Foo { Foo } }",
    );
    let receiver = fact(&index, "Alias<Foo>");
    assert_eq!(
        index.field_type(&receiver, &syn::parse_quote!(field)),
        fact(&index, "Foo")
    );
    let lookup = index.lookup_methods(&receiver, "get", &root(&index));
    assert!(lookup.justified);
    assert_eq!(
        index.return_type(lookup.candidates[0], Some(&receiver), &[]),
        fact(&index, "Foo")
    );
    let constructor = index.lookup_associated(&receiver, "new", &root(&index));
    assert_eq!(
        index.return_type(constructor.candidates[0], Some(&receiver), &[]),
        fact(&index, "Foo")
    );
}

#[test]
fn recursive_aliases_stop_at_the_expansion_bound() {
    let index = index("type A = B; type B = A;");
    assert_eq!(
        fact(&index, "A"),
        TypeFact::Unknown(UnknownReason::AnalysisLimit)
    );
    assert_eq!(fact(&index, "Unknown"), TypeFact::unknown());
}

#[test]
fn explicit_traits_resolve_and_dynamic_dispatch_stays_uncertain() {
    let index = index(
        "trait Work { fn run(&self); } struct A; struct B; impl Work for A { fn run(&self) {} } impl Work for B { fn run(&self) {} }",
    );
    let lookup = index.lookup_methods(&fact(&index, "&A"), "run", &root(&index));
    assert!(lookup.justified);
    let dynamic = index.lookup_methods(&fact(&index, "&dyn Work"), "run", &root(&index));
    assert!(!dynamic.justified);
    assert_eq!(dynamic.candidates.len(), 2);
    let expression: syn::ExprPath = syn::parse_quote!(<A as Work>::run);
    let qualified = index.lookup_call(&expression.path, expression.qself.as_ref(), &root(&index));
    assert!(qualified.justified);
    assert_eq!(qualified.candidates[0].id.name, "A::run");
}

#[test]
fn bounded_trait_requirements_do_not_promote_blanket_impls() {
    let index = index(
        "trait Work { fn run(&self); } struct A; impl<T: Work> Work for T { fn run(&self) {} }",
    );
    let unknown = index.lookup_methods(&TypeFact::unknown(), "run", &root(&index));
    assert_eq!(unknown.candidates.len(), 1);
    assert!(!unknown.justified);
    let concrete = index.lookup_methods(&fact(&index, "A"), "run", &root(&index));
    assert_eq!(concrete.candidates.len(), 1);
    assert!(!concrete.justified);
}

#[test]
fn only_explicit_modules_link_different_files() {
    let files = [
        ("src/lib.rs", "mod a; use a::Foo;"),
        (
            "src/a.rs",
            "pub struct Foo; impl Foo { pub fn bar(&self) {} }",
        ),
        (
            "other/lib.rs",
            "pub struct Foo; impl Foo { pub fn bar(&self) {} }",
        ),
    ]
    .map(|(file, source)| {
        (
            PathBuf::from(file),
            syn::parse_file(source).expect("valid fixture"),
        )
    });
    let index = WorkspaceIndex::build(&files);
    let receiver = fact(&index, "Foo");
    let lookup = index.lookup_methods(&receiver, "bar", &root(&index));
    assert!(lookup.justified);
    assert_eq!(lookup.candidates[0].id.file, PathBuf::from("src/a.rs"));
    let unknown = index.lookup_methods(&TypeFact::unknown(), "bar", &root(&index));
    assert_eq!(unknown.candidates.len(), 1);
}

#[test]
fn explicit_reexports_preserve_original_declaration_identity() {
    let index = index(
        "mod a { pub struct Foo; impl Foo { pub fn run(&self) {} } pub fn factory() -> Foo { Foo } } mod b { pub use super::a::{Foo as Alias, factory}; } use b::Alias;",
    );
    let receiver = fact(&index, "Alias");
    let lookup = index.lookup_methods(&receiver, "run", &root(&index));
    assert!(lookup.justified);
    assert_eq!(lookup.candidates[0].id.name, "a::Foo::run");
    let lookup = index.lookup_call(&syn::parse_quote!(b::factory), None, &root(&index));
    assert!(lookup.justified);
    assert_eq!(lookup.candidates[0].id.name, "a::factory");
    assert_eq!(lookup.provenance, CallEdgeProvenance::ImportResolution);
    let direct = index.lookup_call(&syn::parse_quote!(crate::a::factory), None, &root(&index));
    assert_eq!(direct.provenance, CallEdgeProvenance::AstDirect);
}

#[test]
fn a_function_reexported_through_its_own_module_name_resolves_once() {
    let files = [
        (
            "src/lib.rs",
            "mod evaluate; pub use evaluate::evaluate; fn call() { evaluate(); }",
        ),
        ("src/evaluate.rs", "mod run; pub use run::evaluate;"),
        ("src/evaluate/run.rs", "pub fn evaluate() {}"),
    ]
    .map(|(file, source)| {
        (
            PathBuf::from(file),
            syn::parse_file(source).expect("valid fixture"),
        )
    });
    let index = WorkspaceIndex::build(&files);
    let lookup = index.lookup_call(&syn::parse_quote!(evaluate), None, &root(&index));
    assert!(lookup.justified);
    assert_eq!(lookup.candidates.len(), 1);
    assert_eq!(
        lookup.candidates[0].id.file,
        PathBuf::from("src/evaluate/run.rs")
    );
}

#[test]
fn ambiguous_declared_owners_exclude_unrelated_possible_methods() {
    let index = index(
        "mod a { pub struct Same; impl Same { fn run(&self) {} } } mod b { pub struct Same; impl Same { fn run(&self) {} } } struct Unrelated; impl Unrelated { fn run(&self) {} } use a::Same; use b::Same;",
    );
    let receiver = fact(&index, "Same");
    assert!(matches!(
        receiver,
        TypeFact::Uncertain {
            reason: UnknownReason::AmbiguousDeclaration,
            ..
        }
    ));
    let lookup = index.lookup_methods(&receiver, "run", &root(&index));
    assert!(!lookup.justified);
    assert_eq!(
        lookup
            .candidates
            .iter()
            .map(|call| call.id.name.as_str())
            .collect::<Vec<_>>(),
        ["a::Same::run", "b::Same::run"]
    );
}

#[test]
fn contradictory_declarations_retain_owner_constraints_through_references() {
    let index =
        index("struct A; struct B; impl A { fn run(&self) {} } impl B { fn run(&self) {} }");
    let receiver = TypeFact::Uncertain {
        constraint: Box::new(fact(&index, "A")),
        reason: UnknownReason::UnsupportedTypeOperation,
    };
    let receiver = TypeFact::Reference {
        mutable: false,
        inner: Box::new(receiver),
    };
    let lookup = index.lookup_methods(&receiver, "run", &root(&index));
    assert!(!lookup.justified);
    assert_eq!(
        lookup
            .candidates
            .iter()
            .map(|call| call.id.name.as_str())
            .collect::<Vec<_>>(),
        ["A::run"]
    );
}

#[test]
fn generic_trait_bounds_constrain_possible_implementation_bodies() {
    let index = index(
        "trait First { fn run(&self); } trait Second { fn run(&self); } struct A; struct B; impl First for A { fn run(&self) {} } impl Second for B { fn run(&self) {} } fn caller<T>(value: T) where T: First {} ",
    );
    let caller = index
        .callables()
        .iter()
        .find(|call| call.id.name == "caller")
        .expect("caller");
    let receiver = index.type_from_syn(
        &syn::parse_quote!(T),
        &caller.context,
        &caller.substitutions,
    );
    let lookup = index.lookup_methods(&receiver, "run", &caller.context);
    assert!(!lookup.justified);
    assert_eq!(
        lookup
            .candidates
            .iter()
            .map(|call| call.id.name.as_str())
            .collect::<Vec<_>>(),
        ["A::run"]
    );
}

#[test]
fn unavailable_qualified_or_imported_type_excludes_project_owners() {
    let index = index("use external::Missing as Imported; struct A; impl A { fn run(&self) {} }");
    for source in ["external::Missing", "Imported", "&external::Missing"] {
        let lookup = index.lookup_methods(&fact(&index, source), "run", &root(&index));
        assert!(!lookup.justified);
        assert!(lookup.candidates.is_empty(), "{source}");
    }
    let unknown = index.lookup_methods(&fact(&index, "Missing"), "run", &root(&index));
    assert_eq!(unknown.candidates.len(), 1);
    assert!(!unknown.justified);
}

#[test]
fn mutable_receiver_adjustment_preserves_all_reference_layers() {
    let index = index("struct A; impl A { fn run(&mut self) {} }");
    let shared_inner = index.lookup_methods(&fact(&index, "&mut &A"), "run", &root(&index));
    assert!(!shared_inner.justified);
    assert_eq!(shared_inner.candidates.len(), 1);
    let mutable_inner = index.lookup_methods(&fact(&index, "&mut &mut A"), "run", &root(&index));
    assert!(mutable_inner.justified);
}

#[test]
fn uncertain_fields_and_return_types_preserve_constraints_and_reference_shape() {
    let index = index(
        "struct A; struct B; impl A { fn run(&self) {} } impl B { fn run(&self) {} } struct Wrapper<T> { field: T, borrowed: &'static T } impl<T> Wrapper<T> { fn get(&self) -> &T { todo!() } async fn ready(&self) -> T { todo!() } }",
    );
    let receiver = with_uncertainty(
        fact(&index, "Wrapper<A>"),
        UnknownReason::UnsupportedTypeOperation,
    );
    let field = index.field_type(&receiver, &syn::parse_quote!(field));
    let borrowed = index.field_type(&receiver, &syn::parse_quote!(borrowed));
    assert!(matches!(borrowed, TypeFact::Reference { .. }));
    for derived in [field, borrowed] {
        let lookup = index.lookup_methods(&derived, "run", &root(&index));
        assert!(!lookup.justified);
        assert_eq!(
            lookup
                .candidates
                .iter()
                .map(|call| call.id.name.as_str())
                .collect::<Vec<_>>(),
            ["A::run"]
        );
    }
    let getter = index.lookup_methods(&receiver, "get", &root(&index));
    let returned = index.return_type(getter.candidates[0], Some(&receiver), &[]);
    assert!(matches!(returned, TypeFact::Reference { .. }));
    assert!(returned.is_uncertain());
    let ready = index.lookup_methods(&receiver, "ready", &root(&index));
    let output = index.return_type(ready.candidates[0], Some(&receiver), &[]);
    assert!(matches!(output, TypeFact::Future(inner) if inner.is_uncertain()));
}

#[test]
fn same_line_declarations_have_distinct_internal_identity() {
    let index = index("#[cfg(a)] struct A; #[cfg(b)] struct A;");
    let declarations = index.type_candidates(&["A".into()], &root(&index));
    assert_eq!(declarations.len(), 2);
    assert_eq!(declarations[0].id.line, declarations[1].id.line);
    assert_ne!(declarations[0].id.column, declarations[1].id.column);
    assert_ne!(declarations[0].id, declarations[1].id);
}

#[test]
fn generic_arguments_preserve_reference_shape_during_matching_and_substitution() {
    let index = index(
        "struct A; struct Wrapper<T>(T); impl Wrapper<A> { fn specialized(&self) {} } impl<T> Wrapper<T> { fn get(&self) -> T { todo!() } }",
    );
    let reference = fact(&index, "Wrapper<&A>");
    assert!(
        index
            .lookup_methods(&reference, "specialized", &root(&index))
            .candidates
            .is_empty()
    );
    let getter = index.lookup_methods(&reference, "get", &root(&index));
    assert!(getter.justified);
    assert_eq!(
        index.return_type(getter.candidates[0], Some(&reference), &[]),
        fact(&index, "&A")
    );
    let specialized =
        index.lookup_methods(&fact(&index, "Wrapper<A>"), "specialized", &root(&index));
    assert!(specialized.justified);
}

#[test]
fn repeated_generic_impl_arguments_require_consistent_substitution() {
    let index = index(
        "struct A; struct B; struct Pair<T, U>(T, U); impl<T> Pair<T, T> { fn same(&self) {} }",
    );
    let different = index.lookup_methods(&fact(&index, "Pair<A, B>"), "same", &root(&index));
    assert!(different.candidates.is_empty());
    let same = index.lookup_methods(&fact(&index, "Pair<A, A>"), "same", &root(&index));
    assert!(same.justified);
}

#[test]
fn generic_trait_arguments_exclude_incompatible_explicit_impls_without_guessing() {
    let index = index(
        "trait Work<T> { fn run(&self); } struct A; struct B; struct C; impl Work<B> for A { fn run(&self) {} }",
    );
    let incompatible: syn::ExprPath = syn::parse_quote!(<A as Work<C>>::run);
    let lookup = index.lookup_call(
        &incompatible.path,
        incompatible.qself.as_ref(),
        &root(&index),
    );
    assert!(!lookup.justified);
    assert!(lookup.candidates.is_empty());
    let matching: syn::ExprPath = syn::parse_quote!(<A as Work<B>>::run);
    let lookup = index.lookup_call(&matching.path, matching.qself.as_ref(), &root(&index));
    assert!(!lookup.justified);
    assert_eq!(lookup.candidates.len(), 1);
    let dotted = index.lookup_methods(&fact(&index, "A"), "run", &root(&index));
    assert!(!dotted.justified);
    assert_eq!(dotted.candidates.len(), 1);
}
