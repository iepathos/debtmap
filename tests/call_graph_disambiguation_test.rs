use debtmap::analyzers::call_graph::CallGraphExtractor;
use std::path::PathBuf;

fn parse_rust_code(code: &str) -> syn::File {
    syn::parse_str(code).expect("Failed to parse code")
}

#[test]
fn test_context_matcher_any_caller_accuracy() {
    let code = r#"
        struct ContextMatcher;

        impl ContextMatcher {
            fn any() -> Self {
                ContextMatcher
            }
        }

        fn parse_config_rule() {
            let _matcher = ContextMatcher::any();
        }

        fn has_test_attribute() -> bool {
            let attrs = vec![1, 2, 3];
            attrs.iter().any(|x| *x > 0)
        }

        fn detect_gather() -> bool {
            let calls = vec![1, 2, 3];
            calls.iter().any(|call| *call == 1)
        }
    "#;

    let file = parse_rust_code(code);
    let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
    let graph = extractor.extract(&file);

    // Find ContextMatcher::any function
    let any_func = graph
        .get_all_functions()
        .find(|f| f.name == "ContextMatcher::any")
        .expect("ContextMatcher::any not found");

    // Get callers
    let callers = graph.get_callers(any_func);

    // Should have exactly 1 caller: parse_config_rule
    assert_eq!(
        callers.len(),
        1,
        "ContextMatcher::any should have exactly 1 caller, found: {:?}",
        callers
    );

    assert_eq!(callers[0].name, "parse_config_rule");
}

#[test]
fn test_iterator_any_excluded() {
    let code = r#"
        fn has_test_attribute() -> bool {
            let attrs = vec![1, 2, 3];
            attrs.iter().any(|x| *x > 0)
        }
    "#;

    let file = parse_rust_code(code);
    let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
    let graph = extractor.extract(&file);

    // Find has_test_attribute function
    let func = graph
        .get_all_functions()
        .find(|f| f.name == "has_test_attribute")
        .expect("has_test_attribute not found");

    // Get callees
    let callees = graph.get_callees(func);

    // Should NOT include "any" or any function ending with "::any"
    assert!(
        !callees
            .iter()
            .any(|c| c.name == "any" || c.name.ends_with("::any")),
        "has_test_attribute should not call any user function named 'any', found: {:?}",
        callees
    );
}

#[test]
fn test_static_vs_instance_call_disambiguation() {
    let code = r#"
        struct MyType;

        impl MyType {
            fn new() -> Self {
                MyType
            }

            fn process(&self) {
                // Instance method call
                self.helper();
            }

            fn helper(&self) {
                println!("Helper");
            }
        }

        fn create_type() {
            // Static call
            let _t = MyType::new();
        }
    "#;

    let file = parse_rust_code(code);
    let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
    let graph = extractor.extract(&file);

    // Find MyType::new
    let new_func = graph
        .get_all_functions()
        .find(|f| f.name == "MyType::new")
        .expect("MyType::new not found");

    let new_callers = graph.get_callers(new_func);
    assert_eq!(new_callers.len(), 1);
    assert_eq!(new_callers[0].name, "create_type");

    // Find MyType::helper
    let helper_func = graph
        .get_all_functions()
        .find(|f| f.name == "MyType::helper")
        .expect("MyType::helper not found");

    let helper_callers = graph.get_callers(helper_func);
    assert_eq!(helper_callers.len(), 1);
    assert_eq!(helper_callers[0].name, "MyType::process");
}

#[test]
fn test_multiple_methods_same_name() {
    let code = r#"
        struct TypeA;
        struct TypeB;

        impl TypeA {
            fn method(&self) {
                println!("A");
            }
        }

        impl TypeB {
            fn method(&self) {
                println!("B");
            }
        }

        fn call_a(a: &TypeA) {
            a.method();
        }

        fn call_b(b: &TypeB) {
            b.method();
        }
    "#;

    let file = parse_rust_code(code);
    let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
    let graph = extractor.extract(&file);

    // Find TypeA::method
    let method_a = graph
        .get_all_functions()
        .find(|f| f.name == "TypeA::method")
        .expect("TypeA::method not found");

    // Find TypeB::method
    let method_b = graph
        .get_all_functions()
        .find(|f| f.name == "TypeB::method")
        .expect("TypeB::method not found");

    let callers_a = graph.get_callers(method_a);
    let callers_b = graph.get_callers(method_b);

    // Each method should have distinct callers (or possibly none if type inference fails)
    // The key is they should NOT both report the same callers
    assert!(
        !(callers_a.len() == 2 && callers_b.len() == 2),
        "TypeA::method and TypeB::method should not both have all callers"
    );
}

#[test]
fn test_std_trait_methods_excluded() {
    let code = r#"
        fn process_data() {
            let items = vec![1, 2, 3];
            let _filtered: Vec<_> = items.iter().filter(|x| **x > 0).collect();
            let _mapped: Vec<_> = items.iter().map(|x| x * 2).collect();
            let _any = items.iter().any(|x| *x > 1);

            let opt = Some(5);
            let _unwrapped = opt.unwrap();
            let _mapped_opt = opt.map(|x| x * 2);
        }
    "#;

    let file = parse_rust_code(code);
    let extractor = CallGraphExtractor::new(PathBuf::from("test.rs"));
    let graph = extractor.extract(&file);

    let process_func = graph
        .get_all_functions()
        .find(|f| f.name == "process_data")
        .expect("process_data not found");

    let callees = graph.get_callees(process_func);

    // Should not include any std trait methods
    let std_methods = ["filter", "collect", "map", "any", "unwrap", "iter"];
    for method in &std_methods {
        assert!(
            !callees
                .iter()
                .any(|c| c.name == *method || c.name.ends_with(&format!("::{}", method))),
            "process_data should not call std method '{}', found: {:?}",
            method,
            callees
        );
    }
}
