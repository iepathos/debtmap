//! Regression cases for unsupported execution and falsely confirmed effects.
use super::extract;
use crate::analysis::effect_evidence::{
    EffectAssessment, EffectClassification, ObservedEffectKind,
};
use std::path::PathBuf;

fn assessment(source: &str, name: &str) -> EffectAssessment {
    let graph = extract(&[(
        PathBuf::from("src/lib.rs"),
        syn::parse_file(source).unwrap(),
    )]);
    let id = graph
        .get_all_functions()
        .find(|id| id.name == name)
        .unwrap();
    graph.effect_assessment(id).unwrap().clone()
}

#[test]
fn unsupported_operations_do_not_establish_complete_purity() {
    for source in [
        "fn f(x: &mut i32) { *x = 1; }",
        "struct S { x: i32 } fn f(x: &mut S) { x.x = 1; }",
        "fn f<T: std::ops::Add<Output=T>>(x: T, y: T) -> T { x + y }",
        "fn f<T: std::ops::Index<usize>>(x: T) { let _ = &x[0]; }",
        "fn f<T: IntoIterator>(x: T) { for _ in x {} }",
        "fn f(x: Result<(), ()>) -> Result<(), ()> { x?; Ok(()) }",
        "static mut VALUE: i32 = 0; fn f() { unsafe { VALUE = 1; } }",
    ] {
        assert_eq!(
            assessment(source, "f").classification(),
            EffectClassification::Unknown,
            "{source}"
        );
    }
}

#[test]
fn unresolved_field_projection_cannot_hide_custom_deref_effects() {
    let source = r#"
        struct Inner { value: i32 }
        struct Smart(Inner);
        impl std::ops::Deref for Smart {
            type Target = Inner;
            fn deref(&self) -> &Inner { println!("access"); &self.0 }
        }
        fn f(value: &Smart) -> i32 { value.value }
        fn direct(value: &Inner) -> i32 { value.value }
    "#;
    assert_eq!(
        assessment(source, "f").classification(),
        EffectClassification::Unknown
    );
    assert_eq!(
        assessment(source, "direct").classification(),
        EffectClassification::StrictlyPure
    );
}

#[test]
fn generic_library_argument_conversions_do_not_inherit_readonly_completeness() {
    let source = r#"
        struct Key;
        impl AsRef<std::ffi::OsStr> for Key {
            fn as_ref(&self) -> &std::ffi::OsStr { println!("convert"); std::ffi::OsStr::new("KEY") }
        }
        fn f(key: &Key) { let _ = std::env::var(key); }
        fn builtin() { let _ = std::env::var("KEY"); }
    "#;
    let result = assessment(source, "f");
    assert_eq!(result.classification(), EffectClassification::Unknown);
    assert!(
        result
            .observed()
            .any(|effect| effect.kind == ObservedEffectKind::ExternalRead)
    );
    assert_eq!(
        assessment(source, "builtin").classification(),
        EffectClassification::ReadOnly
    );
}

#[test]
fn console_io_retains_formatting_dispatch_uncertainty() {
    let result = assessment(
        "struct Value; fn f(value: &Value) { println!(\"{}\", value); }",
        "f",
    );
    assert_eq!(result.classification(), EffectClassification::Impure);
    assert!(
        result
            .observed()
            .any(|effect| effect.kind == ObservedEffectKind::Io)
    );
    assert!(result.unresolved().any(|behavior| behavior.reason
        == crate::analysis::effect_evidence::UnresolvedReason::UnsupportedDispatch));
}

#[test]
fn owned_self_field_assignment_is_local() {
    let result = assessment(
        "struct S { x: i32 } impl S { fn f(mut self) { self.x = 1; } }",
        "S::f",
    );
    assert_eq!(result.classification(), EffectClassification::Unknown);
    assert!(
        result
            .observed()
            .any(|effect| effect.kind == ObservedEffectKind::LocalMutation)
    );
    assert!(
        !result
            .observed()
            .any(|effect| effect.kind == ObservedEffectKind::ExternalWrite)
    );
}

#[test]
fn compound_assignment_retains_mutation() {
    let result = assessment("fn f(mut x: i32) { x += 1; }", "f");
    assert_eq!(result.classification(), EffectClassification::LocallyPure);
}

#[test]
fn constructing_async_blocks_does_not_execute_effects() {
    let result = assessment(
        "fn f() { let _future = async { println!(\"later\"); }; }",
        "f",
    );
    assert_eq!(result.classification(), EffectClassification::StrictlyPure);
    assert_eq!(result.observed().count(), 0);
}

#[test]
fn calling_async_function_only_constructs_its_future() {
    let result = assessment(
        "async fn later() { println!(\"later\"); } fn f() { let _ = later(); }",
        "f",
    );
    assert_eq!(result.dependencies().count(), 0);
    assert_eq!(result.classification(), EffectClassification::StrictlyPure);
}

#[test]
fn shadowed_and_qualified_nonstandard_console_macros_do_not_confirm_io() {
    for source in [
        "macro_rules! println { ($($x:tt)*) => {} } fn f() { println!(\"ignored\"); }",
        "fn f() { macro_rules! println { ($($x:tt)*) => {} } println!(\"ignored\"); }",
        "fn f() { custom::println!(\"ignored\"); }",
        "use custom::println; fn f() { println!(\"ignored\"); }",
        "mod std {} fn f() { std::println!(\"ignored\"); }",
        "macro_rules! println { ($($x:tt)*) => {} } fn f() { println!(std::fs::write(\"a\", \"b\")); }",
    ] {
        let result = assessment(source, "f");
        assert_eq!(
            result.classification(),
            EffectClassification::Unknown,
            "{source}"
        );
        assert_eq!(result.observed().count(), 0, "{source}");
    }
}

#[test]
fn unshadowed_and_aliased_standard_console_macros_retain_io() {
    for source in [
        "fn f() { println!(\"hello\"); }",
        "fn f() { std::println!(\"hello\"); }",
        "use std::println as output; fn f() { output!(\"hello\"); }",
    ] {
        let result = assessment(source, "f");
        assert_eq!(
            result.classification(),
            EffectClassification::Impure,
            "{source}"
        );
        assert!(
            result
                .observed()
                .any(|effect| effect.kind == ObservedEffectKind::Io)
        );
    }
}

#[test]
fn generic_destruction_and_model_dispatch_remain_uncertain() {
    for source in [
        "struct Owned; fn f(value: Owned) {}",
        "struct Owned; fn f() { let value = Owned {}; }",
        "fn f<T>(value: T) {}",
        "fn f<T>(mut value: Vec<T>) { value.clear(); }",
        "fn f(mut value: std::io::Cursor<Vec<u8>>) { value.write(&[]); }",
        "fn f(value: String) { value.contains(|c| true); }",
    ] {
        let result = assessment(source, "f");
        assert_eq!(
            result.classification(),
            EffectClassification::Unknown,
            "{source}"
        );
        assert!(
            !result
                .observed()
                .any(|effect| effect.kind == ObservedEffectKind::Io)
        );
    }
}

#[test]
fn static_access_is_not_mistaken_for_a_callable_reference() {
    let result = assessment("static VALUE: i32 = 1; fn f() -> i32 { VALUE }", "f");
    assert_eq!(result.classification(), EffectClassification::Unknown);
    let local = assessment(
        "static VALUE: i32 = 1; fn f() { let VALUE = 2; let _ = VALUE; }",
        "f",
    );
    assert_eq!(local.classification(), EffectClassification::StrictlyPure);
    let constant = assessment("const VALUE: i32 = 1; fn f() -> i32 { VALUE }", "f");
    assert_eq!(
        constant.classification(),
        EffectClassification::StrictlyPure
    );
}
