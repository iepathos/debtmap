use debtmap::complexity::cognitive::{
    calculate_cognitive, calculate_cognitive_penalty, combine_cognitive,
};
use syn::{parse_quote, Block};

#[test]
fn test_calculate_cognitive_simple_block() {
    let block: Block = parse_quote! {{
        let x = 5;
        let y = 10;
        x + y
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(
        complexity, 0,
        "Simple block should have 0 cognitive complexity"
    );
}

#[test]
fn test_calculate_cognitive_single_if() {
    let block: Block = parse_quote! {{
        if x > 0 {
            println!("positive");
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(
        complexity, 1,
        "Single if statement should have complexity 1"
    );
}

#[test]
fn test_calculate_cognitive_nested_if() {
    let block: Block = parse_quote! {{
        if x > 0 {
            if y > 0 {
                println!("both positive");
            }
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(
        complexity, 3,
        "Nested if should have complexity 3 (1 for outer if + 2 for inner if with nesting)"
    );
}

#[test]
fn test_calculate_cognitive_match_expression() {
    let block: Block = parse_quote! {{
        match value {
            1 => println!("one"),
            2 => println!("two"),
            3 => println!("three"),
            _ => println!("other"),
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(
        complexity, 5,
        "Match with 4 arms should have complexity 5 (1 for match + 4 for arms)"
    );
}

#[test]
fn test_calculate_cognitive_nested_match() {
    let block: Block = parse_quote! {{
        match outer {
            Some(inner) => {
                match inner {
                    1 => println!("one"),
                    2 => println!("two"),
                    _ => println!("other"),
                }
            }
            None => println!("none"),
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(
        complexity, 8,
        "Nested match should accumulate complexity with nesting penalty"
    );
}

#[test]
fn test_calculate_cognitive_while_loop() {
    let block: Block = parse_quote! {{
        while x < 10 {
            x += 1;
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(complexity, 1, "While loop should have complexity 1");
}

#[test]
fn test_calculate_cognitive_for_loop() {
    let block: Block = parse_quote! {{
        for i in 0..10 {
            println!("{}", i);
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(complexity, 1, "For loop should have complexity 1");
}

#[test]
fn test_calculate_cognitive_loop() {
    let block: Block = parse_quote! {{
        loop {
            if done {
                break;
            }
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(
        complexity, 3,
        "Loop with nested if should have complexity 3"
    );
}

#[test]
fn test_calculate_cognitive_logical_operators() {
    let block: Block = parse_quote! {{
        if x > 0 && y > 0 {
            println!("both positive");
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(
        complexity, 2,
        "If with logical AND should have complexity 2"
    );
}

#[test]
fn test_calculate_cognitive_multiple_logical_operators() {
    let block: Block = parse_quote! {{
        if x > 0 && y > 0 || z < 0 {
            println!("complex condition");
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(
        complexity, 3,
        "If with multiple logical operators should have complexity 3"
    );
}

#[test]
fn test_calculate_cognitive_try_expression() {
    let block: Block = parse_quote! {{
        let result = operation()?;
        result
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(complexity, 1, "Try expression should have complexity 1");
}

#[test]
fn test_calculate_cognitive_nested_try() {
    let block: Block = parse_quote! {{
        if let Some(value) = option {
            let result = operation()?;
            result
        } else {
            None
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(complexity, 3, "Nested try in if should have complexity 3");
}

#[test]
fn test_calculate_cognitive_closure() {
    let block: Block = parse_quote! {{
        let add = |x, y| x + y;
        add(1, 2)
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(complexity, 1, "Closure should have complexity 1");
}

#[test]
fn test_calculate_cognitive_nested_closures() {
    let block: Block = parse_quote! {{
        let outer = |x| {
            let inner = |y| x + y;
            inner(5)
        };
        outer(10)
    }};

    let complexity = calculate_cognitive(&block);
    assert_eq!(complexity, 2, "Nested closures should have complexity 2");
}

#[test]
fn test_calculate_cognitive_complex_nesting() {
    let block: Block = parse_quote! {{
        if x > 0 {
            for i in 0..10 {
                match i {
                    0 => {
                        if special {
                            println!("special case");
                        }
                    }
                    _ => println!("normal"),
                }
            }
        }
    }};

    let complexity = calculate_cognitive(&block);
    // 1 (if) + 2 (for with nesting 1) + 3 (match with nesting 2) + 2 (2 arms) + 4 (nested if with nesting 3) = 12
    assert!(
        complexity >= 10,
        "Complex nesting should have high complexity"
    );
}

#[test]
fn test_calculate_cognitive_penalty_zero_nesting() {
    assert_eq!(calculate_cognitive_penalty(0), 0);
}

#[test]
fn test_calculate_cognitive_penalty_level_one() {
    assert_eq!(calculate_cognitive_penalty(1), 1);
}

#[test]
fn test_calculate_cognitive_penalty_level_two() {
    assert_eq!(calculate_cognitive_penalty(2), 2);
}

#[test]
fn test_calculate_cognitive_penalty_level_three() {
    assert_eq!(calculate_cognitive_penalty(3), 4);
}

#[test]
fn test_calculate_cognitive_penalty_high_nesting() {
    assert_eq!(calculate_cognitive_penalty(4), 8);
    assert_eq!(calculate_cognitive_penalty(5), 8);
    assert_eq!(calculate_cognitive_penalty(10), 8);
}

#[test]
fn test_combine_cognitive_empty() {
    let complexities = vec![];
    assert_eq!(combine_cognitive(complexities), 0);
}

#[test]
fn test_combine_cognitive_single() {
    let complexities = vec![5];
    assert_eq!(combine_cognitive(complexities), 5);
}

#[test]
fn test_combine_cognitive_multiple() {
    let complexities = vec![3, 5, 7, 2];
    assert_eq!(combine_cognitive(complexities), 17);
}

#[test]
fn test_combine_cognitive_with_zeros() {
    let complexities = vec![0, 5, 0, 3, 0];
    assert_eq!(combine_cognitive(complexities), 8);
}

#[test]
fn test_calculate_cognitive_else_if_chain() {
    let block: Block = parse_quote! {{
        if x > 10 {
            println!("greater than 10");
        } else if x > 5 {
            println!("greater than 5");
        } else if x > 0 {
            println!("greater than 0");
        } else {
            println!("non-positive");
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert!(
        complexity >= 3,
        "Else-if chain should have complexity for each branch"
    );
}

#[test]
fn test_calculate_cognitive_mixed_control_flow() {
    let block: Block = parse_quote! {{
        for i in 0..10 {
            if i % 2 == 0 {
                continue;
            }
            match i {
                1 | 3 | 5 => println!("small odd"),
                7 | 9 => println!("large odd"),
                _ => {}
            }
        }
    }};

    let complexity = calculate_cognitive(&block);
    assert!(
        complexity >= 6,
        "Mixed control flow should accumulate complexity"
    );
}
