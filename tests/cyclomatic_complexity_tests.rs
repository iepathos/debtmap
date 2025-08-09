use debtmap::complexity::cyclomatic::{
    calculate_cyclomatic, calculate_cyclomatic_for_function, combine_cyclomatic,
};
use syn::{parse_quote, Block};

#[test]
fn test_calculate_cyclomatic_simple_block() {
    let block: Block = parse_quote! {{
        let x = 5;
        let y = 10;
        x + y
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 1,
        "Simple block should have cyclomatic complexity of 1"
    );
}

#[test]
fn test_calculate_cyclomatic_single_if() {
    let block: Block = parse_quote! {{
        if x > 0 {
            println!("positive");
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 1,
        "Single if statement doesn't add complexity in this implementation"
    );
}

#[test]
fn test_calculate_cyclomatic_if_else() {
    let block: Block = parse_quote! {{
        if x > 0 {
            println!("positive");
        } else {
            println!("non-positive");
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 1,
        "If-else doesn't add complexity in this implementation"
    );
}

#[test]
fn test_calculate_cyclomatic_nested_if() {
    let block: Block = parse_quote! {{
        if x > 0 {
            if y > 0 {
                println!("both positive");
            }
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 1,
        "Nested if statements don't add complexity in this implementation"
    );
}

#[test]
fn test_calculate_cyclomatic_match_expression() {
    let block: Block = parse_quote! {{
        match value {
            1 => println!("one"),
            2 => println!("two"),
            3 => println!("three"),
            _ => println!("other"),
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(complexity, 5, "Match with 4 arms adds 4 to complexity");
}

#[test]
fn test_calculate_cyclomatic_match_with_guards() {
    let block: Block = parse_quote! {{
        match value {
            x if x > 10 => println!("large"),
            x if x > 5 => println!("medium"),
            x if x > 0 => println!("small"),
            _ => println!("non-positive"),
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 5,
        "Match arms with guards count the same as regular arms"
    );
}

#[test]
fn test_calculate_cyclomatic_while_loop() {
    let block: Block = parse_quote! {{
        while x < 10 {
            x += 1;
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(complexity, 2, "While loop adds 1 to complexity");
}

#[test]
fn test_calculate_cyclomatic_for_loop() {
    let block: Block = parse_quote! {{
        for i in 0..10 {
            println!("{}", i);
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(complexity, 2, "For loop adds 1 to complexity");
}

#[test]
fn test_calculate_cyclomatic_loop() {
    let block: Block = parse_quote! {{
        loop {
            if done {
                break;
            }
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(complexity, 2, "Loop adds complexity but nested if doesn't");
}

#[test]
fn test_calculate_cyclomatic_logical_and() {
    let block: Block = parse_quote! {{
        if x > 0 && y > 0 {
            println!("both positive");
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 1,
        "Logical AND doesn't add complexity in this block context"
    );
}

#[test]
fn test_calculate_cyclomatic_logical_or() {
    let block: Block = parse_quote! {{
        if x > 0 || y > 0 {
            println!("at least one positive");
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 1,
        "Logical OR doesn't add complexity in this block context"
    );
}

#[test]
fn test_calculate_cyclomatic_multiple_logical_operators() {
    let block: Block = parse_quote! {{
        if (x > 0 && y > 0) || z < 0 {
            println!("complex condition");
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 1,
        "Logical operators don't add complexity in this block context"
    );
}

#[test]
fn test_calculate_cyclomatic_try_expression() {
    let block: Block = parse_quote! {{
        let result = operation()?;
        result
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(complexity, 2, "Try expression adds 1 to complexity");
}

#[test]
fn test_calculate_cyclomatic_multiple_try() {
    let block: Block = parse_quote! {{
        let a = operation1()?;
        let b = operation2()?;
        let c = operation3()?;
        a + b + c
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(complexity, 4, "Each try expression adds 1 to complexity");
}

#[test]
fn test_calculate_cyclomatic_nested_match() {
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

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 6,
        "Nested match statements accumulate complexity"
    );
}

#[test]
fn test_calculate_cyclomatic_complex_control_flow() {
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

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 5,
        "Complex control flow accumulates decision points"
    );
}

#[test]
fn test_calculate_cyclomatic_for_function_no_params() {
    let base_complexity = 5;
    let param_count = 0;
    let result = calculate_cyclomatic_for_function(base_complexity, param_count);
    assert_eq!(
        result, 5,
        "Function with no parameters doesn't add to complexity"
    );
}

#[test]
fn test_calculate_cyclomatic_for_function_single_param() {
    let base_complexity = 5;
    let param_count = 1;
    let result = calculate_cyclomatic_for_function(base_complexity, param_count);
    assert_eq!(
        result, 5,
        "Function with one parameter doesn't add to complexity"
    );
}

#[test]
fn test_calculate_cyclomatic_for_function_multiple_params() {
    let base_complexity = 5;
    let param_count = 3;
    let result = calculate_cyclomatic_for_function(base_complexity, param_count);
    assert_eq!(result, 7, "Function with 3 parameters adds 2 to complexity");
}

#[test]
fn test_calculate_cyclomatic_for_function_many_params() {
    let base_complexity = 5;
    let param_count = 10;
    let result = calculate_cyclomatic_for_function(base_complexity, param_count);
    assert_eq!(
        result, 14,
        "Function with 10 parameters adds 9 to complexity"
    );
}

#[test]
fn test_combine_cyclomatic_empty() {
    let branches = vec![];
    assert_eq!(
        combine_cyclomatic(branches),
        1,
        "Empty branches should return base complexity of 1"
    );
}

#[test]
fn test_combine_cyclomatic_single_branch() {
    let branches = vec![2];
    assert_eq!(
        combine_cyclomatic(branches),
        3,
        "Single branch adds to base complexity"
    );
}

#[test]
fn test_combine_cyclomatic_multiple_branches() {
    let branches = vec![2, 3, 4];
    assert_eq!(
        combine_cyclomatic(branches),
        10,
        "Multiple branches sum up plus 1"
    );
}

#[test]
fn test_combine_cyclomatic_with_ones() {
    let branches = vec![1, 1, 1, 1];
    assert_eq!(
        combine_cyclomatic(branches),
        5,
        "Four branches of complexity 1 sum to 5"
    );
}

#[test]
fn test_calculate_cyclomatic_else_if_chain() {
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

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 1,
        "Else-if chain doesn't add complexity in this implementation"
    );
}

#[test]
fn test_calculate_cyclomatic_early_return() {
    let block: Block = parse_quote! {{
        if error {
            return Err("error");
        }
        if warning {
            return Ok(0);
        }
        Ok(42)
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 1,
        "Early returns don't add complexity in this implementation"
    );
}

#[test]
fn test_calculate_cyclomatic_break_continue() {
    let block: Block = parse_quote! {{
        for i in 0..10 {
            if i == 5 {
                break;
            }
            if i % 2 == 0 {
                continue;
            }
            println!("{}", i);
        }
    }};

    let complexity = calculate_cyclomatic(&block);
    assert_eq!(
        complexity, 2,
        "For loop adds complexity but if statements don't"
    );
}
