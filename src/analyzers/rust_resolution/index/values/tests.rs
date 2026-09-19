use super::*;

fn fixture(source: &str) -> (WorkspaceIndex, Context) {
    let index = WorkspaceIndex::build(&[(
        PathBuf::from("src/lib.rs"),
        syn::parse_file(source).expect("valid syntax"),
    )]);
    let context = index.context(Path::new("src/lib.rs"), &[]);
    (index, context)
}

#[test]
fn repeated_imports_deduplicate_declarations_in_every_value_kind() {
    for declaration in [
        "pub fn Item() {}",
        "pub struct Item;",
        "pub const Item: u32 = 0;",
    ] {
        let (index, context) = fixture(&format!(
            "mod left {{ {declaration} }} use left::*; use left::*;"
        ));
        let bindings = index.value_bindings(&syn::parse_quote!(Item), &context);
        assert!(!bindings.ambiguous(), "{declaration}");
        assert_eq!(
            bindings.functions.len() + bindings.constructors.len() + bindings.values.len(),
            1
        );
    }
}

#[test]
fn equal_result_types_do_not_merge_distinct_value_declarations() {
    let (index, context) = fixture(
        "struct Shared; mod left { pub const Item: crate::Shared = crate::Shared; }
         mod right { pub const Item: crate::Shared = crate::Shared; }
         use left::*; use right::*;",
    );
    let path = syn::parse_quote!(Item);
    let bindings = index.value_bindings(&path, &context);
    assert_eq!(bindings.values.len(), 2);
    assert!(bindings.ambiguous());
    let fact = index
        .value_from_path(&path, &context, &[])
        .expect("value fact");
    assert_eq!(
        fact.uncertainty_reason(),
        Some(UnknownReason::AmbiguousDeclaration)
    );
    let TypeFact::Uncertain { constraint, .. } = fact else {
        panic!("ambiguity retained")
    };
    let TypeFact::Ambiguous(alternatives) = *constraint else {
        panic!("both declarations retained")
    };
    assert_eq!(alternatives.len(), 2);
    assert_eq!(alternatives[0], alternatives[1]);
}

#[test]
fn unresolved_explicit_import_competes_with_known_value() {
    for declaration in [
        "pub fn Item() {}",
        "pub struct Item;",
        "pub const Item: u32 = 0;",
    ] {
        let (index, context) = fixture(&format!(
            "mod left {{ {declaration} }} use left::Item; use missing::Item;"
        ));
        let path = syn::parse_quote!(Item);
        let bindings = index.value_bindings(&path, &context);
        assert!(bindings.ambiguous(), "{declaration}");
        let lookup = index.lookup_free_value(&path, &context);
        assert!(!lookup.justified);
        assert_eq!(lookup.reason, Some(UncertaintyReason::AmbiguousDeclaration));
    }
}

#[test]
fn invocation_shape_does_not_erase_competing_bindings() {
    let (index, context) = fixture(
        "mod left { pub struct Item(pub u32); } mod right { pub struct Item(pub u32, pub u32); }
         use left::*; use right::*;",
    );
    let path = syn::parse_quote!(Item);
    assert!(index.value_call_lookup(&path, &context).1);
    let fact = index
        .value_call_result(&path, 1, &context, &[])
        .expect("matching constructor");
    assert_eq!(
        fact.uncertainty_reason(),
        Some(UnknownReason::AmbiguousDeclaration)
    );
    assert_eq!(
        fact.nominal().expect("known surviving owner").0.module,
        ["left"]
    );
}

#[test]
fn renamed_free_function_keeps_its_declaration_identity() {
    let (index, context) =
        fixture("mod left { pub fn original() {} } use left::original as alias;");
    let lookup = index.lookup_free_value(&syn::parse_quote!(alias), &context);
    assert!(lookup.justified);
    assert_eq!(lookup.candidates[0].id.name, "left::original");
}

#[test]
fn constructor_suppression_requires_all_explicit_competitors_to_be_known() {
    let declarations = "mod left { pub struct Item(pub u32); }
        mod right { pub struct Item(pub u32); } mod empty {}
        mod types { pub type Item = u32; }
        mod type_export { pub use crate::types::Item; }
        mod incomplete { pub use crate::left::Item; pub use missing::Item; }";
    for (imports, query, constructor_only) in [
        ("use left::Item; use missing::Item;", "Item", false),
        ("use left::Item; use right::Item;", "Item", true),
        ("use left::Item; use left::Item;", "Item", true),
        ("use left::*; use empty::*;", "Item", true),
        ("use left::*; use right::*;", "Item", true),
        ("use left::*; use Item as Alias;", "Alias", true),
        ("use incomplete::Item;", "Item", false),
        ("use left::Item; use type_export::Item;", "Item", true),
    ] {
        let (index, context) = fixture(&format!("{declarations} {imports}"));
        let path = syn::parse_str(query).expect("valid path");
        assert_eq!(
            index.value_call_lookup(&path, &context).1,
            constructor_only,
            "{imports}"
        );
    }
}
