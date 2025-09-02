use debtmap::complexity::cognitive::{calculate_cognitive_legacy, calculate_cognitive_normalized};
use syn::parse_quote;

#[test]
fn test_multiline_function_signature_normalization() {
    // Function with multiline signature should have same complexity as single-line
    let multiline_block: syn::Block = parse_quote! {
        {
            fn process(
                param1: String,
                param2: Vec<u32>,
                param3: HashMap<String, Value>,
                param4: Option<Box<dyn Future>>
            ) -> Result<ComplexType, Error> {
                if param1.is_empty() {
                    return Err(Error::Empty);
                }
                Ok(ComplexType::new())
            }
        }
    };

    let single_line_block: syn::Block = parse_quote! {
        {
            fn process(param1: String, param2: Vec<u32>, param3: HashMap<String, Value>, param4: Option<Box<dyn Future>>) -> Result<ComplexType, Error> {
                if param1.is_empty() {
                    return Err(Error::Empty);
                }
                Ok(ComplexType::new())
            }
        }
    };

    let multiline_complexity = calculate_cognitive_normalized(&multiline_block);
    let single_line_complexity = calculate_cognitive_normalized(&single_line_block);

    assert_eq!(
        multiline_complexity, single_line_complexity,
        "Multiline function signature should not increase complexity"
    );
}

#[test]
fn test_multiline_tuple_normalization() {
    // Tuple destructuring across multiple lines should not add complexity
    let multiline_block: syn::Block = parse_quote! {
        {
            let (
                first,
                second,
                third,
                fourth,
                fifth
            ) = get_values();

            if first > 0 {
                process(first);
            }
        }
    };

    let single_line_block: syn::Block = parse_quote! {
        {
            let (first, second, third, fourth, fifth) = get_values();

            if first > 0 {
                process(first);
            }
        }
    };

    let multiline_complexity = calculate_cognitive_normalized(&multiline_block);
    let single_line_complexity = calculate_cognitive_normalized(&single_line_block);

    assert_eq!(
        multiline_complexity, single_line_complexity,
        "Multiline tuple destructuring should not increase complexity"
    );
}

#[test]
fn test_method_chain_normalization() {
    // Method chains formatted across lines should not add complexity
    let multiline_block: syn::Block = parse_quote! {
        {
            let result = data
                .iter()
                .filter(|x| x > 0)
                .map(|x| x * 2)
                .collect();

            result
        }
    };

    let single_line_block: syn::Block = parse_quote! {
        {
            let result = data.iter().filter(|x| x > 0).map(|x| x * 2).collect();
            result
        }
    };

    let multiline_complexity = calculate_cognitive_normalized(&multiline_block);
    let single_line_complexity = calculate_cognitive_normalized(&single_line_block);

    // Method chains might have slight differences due to closure detection
    assert!(
        (multiline_complexity as i32 - single_line_complexity as i32).abs() <= 1,
        "Method chain formatting should have minimal impact on complexity"
    );
}

#[test]
fn test_match_expression_normalization() {
    // Match expressions with multiline patterns should normalize correctly
    let multiline_block: syn::Block = parse_quote! {
        {
            match value {
                Pattern::Complex {
                    field1,
                    field2,
                    field3,
                    field4
                } => {
                    process(field1);
                }
                Pattern::Simple(x) => {
                    handle(x);
                }
                _ => {}
            }
        }
    };

    let compact_block: syn::Block = parse_quote! {
        {
            match value {
                Pattern::Complex { field1, field2, field3, field4 } => {
                    process(field1);
                }
                Pattern::Simple(x) => {
                    handle(x);
                }
                _ => {}
            }
        }
    };

    let multiline_complexity = calculate_cognitive_normalized(&multiline_block);
    let compact_complexity = calculate_cognitive_normalized(&compact_block);

    assert_eq!(
        multiline_complexity, compact_complexity,
        "Match pattern formatting should not affect complexity"
    );
}

#[test]
fn test_string_literal_normalization() {
    // Multiline string literals should not add complexity
    let multiline_block: syn::Block = parse_quote! {
        {
            let message = "This is a very long message that \
                          spans multiple lines for readability \
                          but is logically a single string";

            if condition {
                println!("{}", message);
            }
        }
    };

    let single_line_block: syn::Block = parse_quote! {
        {
            let message = "This is a very long message that spans multiple lines for readability but is logically a single string";

            if condition {
                println!("{}", message);
            }
        }
    };

    let multiline_complexity = calculate_cognitive_normalized(&multiline_block);
    let single_line_complexity = calculate_cognitive_normalized(&single_line_block);

    assert_eq!(
        multiline_complexity, single_line_complexity,
        "String literal formatting should not affect complexity"
    );
}

#[test]
fn test_legacy_vs_normalized_real_complexity() {
    // Real complexity increases should still be detected
    let complex_block: syn::Block = parse_quote! {
        {
            if condition1 {
                if condition2 {
                    while condition3 {
                        for item in items {
                            match item {
                                Some(x) if x > 0 => process(x),
                                Some(x) => handle(x),
                                None => break,
                            }
                        }
                    }
                }
            }
        }
    };

    let normalized_complexity = calculate_cognitive_normalized(&complex_block);
    let legacy_complexity = calculate_cognitive_legacy(&complex_block);

    // Both should detect complexity
    // Note: Normalized complexity may be lower as it removes formatting artifacts
    println!(
        "Normalized complexity: {}, Legacy complexity: {}",
        normalized_complexity, legacy_complexity
    );
    assert!(
        normalized_complexity > 0,
        "Normalized should detect some complexity: {}",
        normalized_complexity
    );
    assert!(
        legacy_complexity > 5,
        "Legacy should detect complexity: {}",
        legacy_complexity
    );

    // The key is that normalized doesn't exceed legacy and still detects real complexity
    assert!(
        normalized_complexity <= legacy_complexity,
        "Normalized complexity ({}) should not exceed legacy ({}) for real complexity",
        normalized_complexity,
        legacy_complexity
    );
}

#[test]
fn test_formatting_only_changes() {
    // Test the specific case from commit c257094
    let before: syn::Block = parse_quote! {
        {
            assert_eq!(result, expected, "Failed: {}", description);
        }
    };

    let after: syn::Block = parse_quote! {
        {
            assert_eq!(
                result,
                expected,
                "Failed: {}",
                description
            );
        }
    };

    let before_complexity = calculate_cognitive_normalized(&before);
    let after_complexity = calculate_cognitive_normalized(&after);

    assert_eq!(
        before_complexity, after_complexity,
        "Formatting-only changes should produce identical complexity scores"
    );
}

#[test]
fn test_async_closure_normalization() {
    // Async closures with formatting should normalize correctly
    let multiline_block: syn::Block = parse_quote! {
        {
            let handler = async move |
                request: Request,
                context: Context
            | -> Result<Response> {
                process_async(request, context).await
            };

            handler
        }
    };

    let single_line_block: syn::Block = parse_quote! {
        {
            let handler = async move |request: Request, context: Context| -> Result<Response> {
                process_async(request, context).await
            };

            handler
        }
    };

    let multiline_complexity = calculate_cognitive_normalized(&multiline_block);
    let single_line_complexity = calculate_cognitive_normalized(&single_line_block);

    assert_eq!(
        multiline_complexity, single_line_complexity,
        "Async closure formatting should not affect complexity"
    );
}
