use debtmap::complexity::cognitive::calculate_cognitive;
use debtmap::complexity::pattern_adjustments::{
    calculate_cognitive_adjusted, PatternMatchRecognizer, PatternRecognizer,
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
