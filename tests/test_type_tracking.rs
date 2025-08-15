use debtmap::analyzers::type_registry::{extract_type_definitions, GlobalTypeRegistry};
use debtmap::analyzers::type_tracker::TypeTracker;
use std::sync::Arc;
use syn;

#[test]
fn test_struct_field_type_tracking() {
    let code = r#"
        struct DependencyGraph {
            nodes: Vec<String>,
            edges: HashMap<String, Vec<String>>,
        }

        struct RustCallGraph {
            base_graph: CallGraph,
            framework_patterns: FrameworkPatternDetector,
        }

        impl RustCallGraph {
            fn analyze(&self) {
                let patterns = self.framework_patterns;
            }
        }
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    
    // Create and populate the type registry
    let mut registry = GlobalTypeRegistry::new();
    extract_type_definitions(&syntax, vec![], &mut registry);

    // Verify that structs were registered
    assert!(registry.get_type("DependencyGraph").is_some());
    assert!(registry.get_type("RustCallGraph").is_some());

    // Verify field types can be resolved
    let field_type = registry.resolve_field("RustCallGraph", "framework_patterns");
    assert!(field_type.is_some());
    assert_eq!(field_type.unwrap().type_name, "FrameworkPatternDetector");

    let field_type = registry.resolve_field("DependencyGraph", "nodes");
    assert!(field_type.is_some());
    assert_eq!(field_type.unwrap().type_name, "Vec");
}

#[test]
fn test_self_reference_tracking() {
    let code = r#"
        struct MyStruct {
            value: i32,
        }

        impl MyStruct {
            fn get_value(&self) -> i32 {
                self.value
            }
        }
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    
    // Create and populate the type registry
    let mut registry = GlobalTypeRegistry::new();
    extract_type_definitions(&syntax, vec![], &mut registry);

    // Create a type tracker with the registry
    let arc_registry = Arc::new(registry);
    let mut tracker = TypeTracker::with_registry(arc_registry.clone());
    
    // Verify that struct was registered
    assert!(arc_registry.get_type("MyStruct").is_some());

    // Verify field can be resolved
    let field_type = arc_registry.resolve_field("MyStruct", "value");
    assert!(field_type.is_some());
    assert_eq!(field_type.unwrap().type_name, "i32");
}

#[test]
fn test_field_access_chain() {
    let code = r#"
        struct Inner {
            data: String,
        }

        struct Middle {
            inner: Inner,
        }

        struct Outer {
            middle: Middle,
        }

        impl Outer {
            fn access_nested(&self) {
                // This would require full AST traversal to test properly
                // Just testing the registry structure here
            }
        }
    "#;

    let syntax = syn::parse_file(code).expect("Failed to parse code");
    
    // Create and populate the type registry
    let mut registry = GlobalTypeRegistry::new();
    extract_type_definitions(&syntax, vec![], &mut registry);

    // Verify nested field resolution
    let outer_field = registry.resolve_field("Outer", "middle");
    assert!(outer_field.is_some());
    assert_eq!(outer_field.unwrap().type_name, "Middle");

    let middle_field = registry.resolve_field("Middle", "inner");
    assert!(middle_field.is_some());
    assert_eq!(middle_field.unwrap().type_name, "Inner");

    let inner_field = registry.resolve_field("Inner", "data");
    assert!(inner_field.is_some());
    assert_eq!(inner_field.unwrap().type_name, "String");
}