use debtmap::analyzers::type_tracker::{ScopeKind, TypeSource, TypeTracker};
use syn::{parse_quote, Expr};

#[test]
fn test_resolve_struct_literal_type() {
    // Test that TypeTracker correctly resolves struct literal types
    let tracker = TypeTracker::new();

    // Create a struct literal expression
    let expr: Expr = parse_quote! {
        Calculator { value: 10 }
    };

    let resolved_type = tracker.resolve_expr_type(&expr);
    assert!(
        resolved_type.is_some(),
        "Should resolve struct literal type"
    );

    let resolved = resolved_type.unwrap();
    assert_eq!(resolved.type_name, "Calculator");
    assert_eq!(resolved.source, TypeSource::StructLiteral);
}

#[test]
fn test_resolve_nested_struct_literal_type() {
    // Test nested struct literal type resolution
    let tracker = TypeTracker::new();

    let expr: Expr = parse_quote! {
        Outer {
            inner: Inner {
                data: 42
            }
        }
    };

    let resolved_type = tracker.resolve_expr_type(&expr);
    assert!(
        resolved_type.is_some(),
        "Should resolve outer struct literal type"
    );

    let resolved = resolved_type.unwrap();
    assert_eq!(resolved.type_name, "Outer");
    assert_eq!(resolved.source, TypeSource::StructLiteral);
}

#[test]
fn test_variable_type_tracking_with_struct_literal() {
    // Test that variables initialized with struct literals are tracked correctly
    let mut tracker = TypeTracker::new();

    // Simulate entering a function scope
    tracker.enter_scope(ScopeKind::Function, None);

    // Create a struct literal expression
    let expr: Expr = parse_quote! {
        MyStruct { field: "value" }
    };

    // Resolve the type of the struct literal
    if let Some(type_info) = tracker.resolve_expr_type(&expr) {
        // Record the variable with its type
        tracker.record_variable("my_var".to_string(), type_info);
    }

    // Now check that the variable type is correctly tracked
    let var_type = tracker.resolve_variable_type("my_var");
    assert!(var_type.is_some(), "Variable type should be tracked");

    let resolved = var_type.unwrap();
    assert_eq!(resolved.type_name, "MyStruct");
    assert_eq!(resolved.source, TypeSource::StructLiteral);

    tracker.exit_scope();
}

#[test]
fn test_qualified_struct_literal_type() {
    // Test that qualified struct paths are handled correctly
    let tracker = TypeTracker::new();

    let expr: Expr = parse_quote! {
        module::SubModule::MyStruct { value: 100 }
    };

    let resolved_type = tracker.resolve_expr_type(&expr);
    assert!(
        resolved_type.is_some(),
        "Should resolve qualified struct literal type"
    );

    let resolved = resolved_type.unwrap();
    // Should extract just the struct name, not the full path
    assert_eq!(resolved.type_name, "MyStruct");
    assert_eq!(resolved.source, TypeSource::StructLiteral);
}

#[test]
fn test_struct_literal_with_generic_type() {
    // Test struct literals with generic type parameters
    let tracker = TypeTracker::new();

    // Note: In real code, generics would be specified differently,
    // but for this test we're just checking the basic name extraction
    let expr: Expr = parse_quote! {
        Container { items: vec![] }
    };

    let resolved_type = tracker.resolve_expr_type(&expr);
    assert!(
        resolved_type.is_some(),
        "Should resolve generic struct literal type"
    );

    let resolved = resolved_type.unwrap();
    assert_eq!(resolved.type_name, "Container");
    assert_eq!(resolved.source, TypeSource::StructLiteral);
}
