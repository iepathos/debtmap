use debtmap::complexity::cognitive::calculate_cognitive;
use debtmap::complexity::pattern_adjustments::{
    calculate_cognitive_adjusted, PatternMatchRecognizer, PatternRecognizer, PatternType,
    SimpleDelegationRecognizer,
};

#[test]
fn test_pattern_matching_reduces_complexity() {
    // Test with a typical file type detection function
    let code = r#"
        fn detect_file_type(path: &str) -> FileType {
            if path.ends_with(".rs") {
                return FileType::Rust;
            }
            if path.ends_with(".py") {
                return FileType::Python;
            }
            if path.ends_with(".js") {
                return FileType::JavaScript;
            }
            if path.ends_with(".ts") {
                return FileType::TypeScript;
            }
            if path.ends_with(".go") {
                return FileType::Go;
            }
            if path.ends_with(".java") {
                return FileType::Java;
            }
            if path.ends_with(".cpp") || path.ends_with(".cc") {
                return FileType::Cpp;
            }
            if path.ends_with(".c") {
                return FileType::C;
            }
            FileType::Unknown
        }
    "#;

    let file = syn::parse_file(code).unwrap();
    if let syn::Item::Fn(func) = &file.items[0] {
        // calculate_cognitive already includes pattern adjustments now
        let complexity = calculate_cognitive(&func.block);

        // With pattern adjustments, complexity should be logarithmic (log2(8) = 3 + 1 for no default = 4)
        assert!(
            complexity <= 5,
            "Adjusted complexity should be low (log scale): got {}",
            complexity
        );
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_simple_delegation_low_complexity() {
    let code = r#"
        fn process_data(input: Data) -> Result<Output> {
            let validated = validate(input)?;
            let transformed = transform(validated);
            finalize(transformed)
        }
    "#;

    let file = syn::parse_file(code).unwrap();
    if let syn::Item::Fn(func) = &file.items[0] {
        let recognizer = SimpleDelegationRecognizer::new();
        let info = recognizer.detect(&func.block);

        assert!(info.is_some(), "Should detect simple delegation");

        // Simple delegation should have minimal complexity
        let adjusted = recognizer.adjust_complexity(&info.unwrap(), 10);
        assert_eq!(adjusted, 1, "Simple delegation should have complexity of 1");
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_non_pattern_matching_unchanged() {
    let code = r#"
        fn complex_logic(x: i32, y: i32) -> i32 {
            if x > 0 {
                if y > 0 {
                    return x + y;
                } else {
                    return x - y;
                }
            } else {
                if y > 0 {
                    return y - x;
                } else {
                    return -(x + y);
                }
            }
        }
    "#;

    let file = syn::parse_file(code).unwrap();
    if let syn::Item::Fn(func) = &file.items[0] {
        let base_complexity = 10; // Example base
        let adjusted = calculate_cognitive_adjusted(&func.block, base_complexity);

        // Non-pattern matching should not be adjusted
        assert_eq!(
            adjusted, base_complexity,
            "Non-pattern complexity should be unchanged"
        );
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_pattern_with_else_default() {
    let code = r#"
        fn classify_number(n: i32) -> Classification {
            if n == 0 {
                return Classification::Zero;
            } else if n == 1 {
                return Classification::One;
            } else if n == 2 {
                return Classification::Two;
            } else if n == 3 {
                return Classification::Three;
            } else {
                return Classification::Other;
            }
        }
    "#;

    let file = syn::parse_file(code).unwrap();
    if let syn::Item::Fn(func) = &file.items[0] {
        let recognizer = PatternMatchRecognizer::new();
        let info = recognizer.detect(&func.block);

        assert!(info.is_some(), "Should detect pattern matching");
        let info = info.unwrap();
        assert!(info.has_default, "Should detect else clause as default");
        assert_eq!(info.condition_count, 4, "Should count 4 conditions");
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_mixed_conditions_not_pattern() {
    // Different variables being tested - not pattern matching
    let code = r#"
        fn check_conditions(x: i32, y: i32, z: i32) -> bool {
            if x > 0 {
                return true;
            }
            if y < 0 {
                return false;
            }
            if z == 0 {
                return true;
            }
            false
        }
    "#;

    let file = syn::parse_file(code).unwrap();
    if let syn::Item::Fn(func) = &file.items[0] {
        let recognizer = PatternMatchRecognizer::new();
        let info = recognizer.detect(&func.block);

        assert!(
            info.is_none(),
            "Should not detect pattern matching for different variables"
        );
    } else {
        panic!("Expected function");
    }
}

#[test]
fn test_logarithmic_scaling_for_many_conditions() {
    use syn::parse_quote;
    use syn::Block;

    // Test with 16 conditions
    let block: Block = parse_quote! {{
        if x == 1 { return A; }
        if x == 2 { return B; }
        if x == 3 { return C; }
        if x == 4 { return D; }
        if x == 5 { return E; }
        if x == 6 { return F; }
        if x == 7 { return G; }
        if x == 8 { return H; }
        if x == 9 { return I; }
        if x == 10 { return J; }
        if x == 11 { return K; }
        if x == 12 { return L; }
        if x == 13 { return M; }
        if x == 14 { return N; }
        if x == 15 { return O; }
        if x == 16 { return P; }
    }};

    let recognizer = PatternMatchRecognizer::new();
    let info = recognizer.detect(&block);

    assert!(info.is_some());
    let info = info.unwrap();
    assert_eq!(info.condition_count, 16);

    // log2(16) = 4, plus 1 for no default = 5
    let adjusted = recognizer.adjust_complexity(&info, 16);
    assert_eq!(
        adjusted, 5,
        "16 conditions should have log2(16) + 1 = 5 complexity"
    );
}

#[test]
fn test_analyze_if_chain_simple_pattern() {
    use syn::parse_quote;
    use syn::ExprIf;

    // Test simple if-else chain with same variable
    let if_expr: ExprIf = parse_quote! {
        if name.starts_with("test_") {
            return ItemType::Test;
        } else if name.contains("_impl") {
            return ItemType::Implementation;
        } else {
            return ItemType::Regular;
        }
    };

    let recognizer = PatternMatchRecognizer::new();
    let mut tracked_var = None;
    let result = recognizer.analyze_if_chain(&if_expr, &mut tracked_var);

    assert!(result.is_some(), "Should recognize if-else chain pattern");
    let (conditions, _pattern_types, has_else) = result.unwrap();
    assert_eq!(conditions.len(), 2, "Should detect 2 conditions");
    assert!(has_else, "Should detect else branch");
    assert_eq!(
        tracked_var,
        Some("name".to_string()),
        "Should track variable name"
    );
}

#[test]
fn test_analyze_if_chain_no_immediate_return() {
    use syn::parse_quote;
    use syn::ExprIf;

    // Test if-else chain without immediate returns
    let if_expr: ExprIf = parse_quote! {
        if x == 1 {
            let y = 2;
            println!("Processing");
            return y;
        } else if x == 2 {
            return 3;
        }
    };

    let recognizer = PatternMatchRecognizer::new();
    let mut tracked_var = None;
    let result = recognizer.analyze_if_chain(&if_expr, &mut tracked_var);

    assert!(
        result.is_none(),
        "Should not recognize pattern without immediate returns"
    );
}

#[test]
fn test_analyze_if_chain_different_variables() {
    use syn::parse_quote;
    use syn::ExprIf;

    // Test if-else chain with different variables (not pattern matching)
    let if_expr: ExprIf = parse_quote! {
        if x > 0 {
            return true;
        } else if y < 0 {
            return false;
        }
    };

    let recognizer = PatternMatchRecognizer::new();
    let mut tracked_var = None;
    let result = recognizer.analyze_if_chain(&if_expr, &mut tracked_var);

    assert!(
        result.is_none(),
        "Should not recognize pattern with different variables"
    );
}

#[test]
fn test_analyze_if_chain_no_else() {
    use syn::parse_quote;
    use syn::ExprIf;

    // Test if-else chain without final else
    let if_expr: ExprIf = parse_quote! {
        if status == "pending" {
            return Status::Pending;
        } else if status == "active" {
            return Status::Active;
        } else if status == "done" {
            return Status::Done;
        }
    };

    let recognizer = PatternMatchRecognizer::new();
    let mut tracked_var = None;
    let result = recognizer.analyze_if_chain(&if_expr, &mut tracked_var);

    assert!(result.is_some(), "Should recognize pattern without else");
    let (_conditions, _pattern_types, has_else) = result.unwrap();
    assert!(!has_else, "Should detect no else branch");
}

#[test]
fn test_analyze_if_chain_method_calls() {
    use syn::parse_quote;
    use syn::ExprIf;

    // Test with method calls on same variable
    let if_expr: ExprIf = parse_quote! {
        if path.ends_with(".rs") {
            return FileType::Rust;
        } else if path.ends_with(".py") {
            return FileType::Python;
        } else if path.starts_with("/tmp") {
            return FileType::Temp;
        }
    };

    let recognizer = PatternMatchRecognizer::new();
    let mut tracked_var = None;
    let result = recognizer.analyze_if_chain(&if_expr, &mut tracked_var);

    assert!(result.is_some(), "Should recognize method call patterns");
    let (conditions, pattern_types, _) = result.unwrap();
    assert_eq!(conditions.len(), 3, "Should detect 3 conditions");
    assert!(pattern_types
        .iter()
        .any(|pt| matches!(pt, PatternType::StringMatching)));
}

#[test]
fn test_analyze_if_chain_field_access() {
    use syn::parse_quote;
    use syn::ExprIf;

    // Test with field access patterns
    let if_expr: ExprIf = parse_quote! {
        if self.state == State::Init {
            return Action::Start;
        } else if self.state == State::Running {
            return Action::Continue;
        } else {
            return Action::Stop;
        }
    };

    let recognizer = PatternMatchRecognizer::new();
    let mut tracked_var = None;
    let result = recognizer.analyze_if_chain(&if_expr, &mut tracked_var);

    // Current implementation doesn't handle field access in binary expressions
    assert!(
        result.is_none(),
        "Field access in binary expressions not currently supported"
    );
}

#[test]
fn test_analyze_if_chain_single_expression() {
    use syn::parse_quote;
    use syn::ExprIf;

    // Test with single expressions in branches (not returns)
    let if_expr: ExprIf = parse_quote! {
        if mode == 1 {
            Mode::Fast
        } else if mode == 2 {
            Mode::Normal
        } else {
            Mode::Slow
        }
    };

    let recognizer = PatternMatchRecognizer::new();
    let mut tracked_var = None;
    let result = recognizer.analyze_if_chain(&if_expr, &mut tracked_var);

    // This should be recognized as it has single expressions
    assert!(
        result.is_some(),
        "Should recognize single expression patterns"
    );
}

#[test]
fn test_extract_tested_variable_various_forms() {
    use syn::parse_quote;
    use syn::Expr;

    let recognizer = PatternMatchRecognizer::new();

    // Test method call extraction
    let expr: Expr = parse_quote! { path.ends_with(".rs") };
    assert_eq!(
        recognizer.extract_tested_variable(&expr),
        Some("path".to_string())
    );

    // Test binary comparison
    let expr: Expr = parse_quote! { x == 5 };
    assert_eq!(
        recognizer.extract_tested_variable(&expr),
        Some("x".to_string())
    );

    // Test field access in method call - extracts base only
    let expr: Expr = parse_quote! { self.field.method() };
    assert_eq!(
        recognizer.extract_tested_variable(&expr),
        Some("self".to_string())
    );

    // Test unary expression
    let expr: Expr = parse_quote! { !is_valid };
    assert_eq!(
        recognizer.extract_tested_variable(&expr),
        Some("is_valid".to_string())
    );

    // Test parenthesized expression
    let expr: Expr = parse_quote! { (value) };
    assert_eq!(
        recognizer.extract_tested_variable(&expr),
        Some("value".to_string())
    );

    // Test direct path
    let expr: Expr = parse_quote! { flag };
    assert_eq!(
        recognizer.extract_tested_variable(&expr),
        Some("flag".to_string())
    );

    // Test field in binary expression - current behavior
    let expr: Expr = parse_quote! { self.state == 5 };
    // Binary expressions with field access don't extract the field properly
    assert_eq!(recognizer.extract_tested_variable(&expr), None);
}

#[test]
fn test_detect_pattern_type_classification() {
    use syn::parse_quote;
    use syn::Expr;

    let recognizer = PatternMatchRecognizer::new();

    // Test string matching methods
    let expr: Expr = parse_quote! { name.ends_with("_test") };
    assert!(matches!(
        recognizer.detect_pattern_type(&expr),
        PatternType::StringMatching
    ));

    let expr: Expr = parse_quote! { path.starts_with("/usr") };
    assert!(matches!(
        recognizer.detect_pattern_type(&expr),
        PatternType::StringMatching
    ));

    let expr: Expr = parse_quote! { text.contains("error") };
    assert!(matches!(
        recognizer.detect_pattern_type(&expr),
        PatternType::StringMatching
    ));

    // Test simple comparisons
    let expr: Expr = parse_quote! { x == 10 };
    assert!(matches!(
        recognizer.detect_pattern_type(&expr),
        PatternType::SimpleComparison
    ));

    let expr: Expr = parse_quote! { y != 0 };
    assert!(matches!(
        recognizer.detect_pattern_type(&expr),
        PatternType::SimpleComparison
    ));

    // Test range matching
    let expr: Expr = parse_quote! { value > 100 };
    assert!(matches!(
        recognizer.detect_pattern_type(&expr),
        PatternType::RangeMatching
    ));

    let expr: Expr = parse_quote! { count <= 5 };
    assert!(matches!(
        recognizer.detect_pattern_type(&expr),
        PatternType::RangeMatching
    ));
}

#[test]
fn test_has_immediate_return() {
    use syn::parse_quote;
    use syn::Block;

    let recognizer = PatternMatchRecognizer::new();

    // Test block with immediate return
    let block: Block = parse_quote! {{
        return value;
    }};
    assert!(recognizer.has_immediate_return(&block));

    // Test block with two statements (second is return)
    let block: Block = parse_quote! {{
        let x = 5;
        return x;
    }};
    assert!(recognizer.has_immediate_return(&block));

    // Test block without return
    let block: Block = parse_quote! {{
        let x = 5;
        let y = 10;
        x + y
    }};
    assert!(!recognizer.has_immediate_return(&block));

    // Test empty block
    let block: Block = parse_quote! {{}};
    assert!(!recognizer.has_immediate_return(&block));

    // Test block with too many statements
    let block: Block = parse_quote! {{
        let x = 1;
        let y = 2;
        let z = 3;
        return x + y + z;
    }};
    assert!(!recognizer.has_immediate_return(&block));
}

#[test]
fn test_extract_field_path() {
    use syn::parse_quote;
    use syn::Expr;

    let recognizer = PatternMatchRecognizer::new();

    // Test simple field access
    let expr: Expr = parse_quote! { self.state };
    assert_eq!(
        recognizer.extract_field_path(&expr),
        Some("self.state".to_string())
    );

    // Test nested field access
    let expr: Expr = parse_quote! { self.inner.value };
    assert_eq!(
        recognizer.extract_field_path(&expr),
        Some("self.inner.value".to_string())
    );

    // Test object field access
    let expr: Expr = parse_quote! { obj.field };
    assert_eq!(
        recognizer.extract_field_path(&expr),
        Some("obj.field".to_string())
    );

    // Test simple path without fields
    let expr: Expr = parse_quote! { variable };
    assert_eq!(
        recognizer.extract_field_path(&expr),
        Some("variable".to_string())
    );

    // Test expression that's not a field access
    let expr: Expr = parse_quote! { 42 };
    assert_eq!(recognizer.extract_field_path(&expr), None);
}
