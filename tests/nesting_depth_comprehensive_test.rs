//! Comprehensive tests for nesting depth calculation.
//!
//! This test file ensures consistent and correct nesting depth calculations
//! across all implementations. It covers:
//! - Else-if chain nesting (should be 1, not N)
//! - `else { if }` equivalence to `else if`
//! - Nested control flow structures
//! - Match arm nesting
//! - Complex real-world patterns
//! - Consistency across all nesting implementations

use debtmap::analyzers::rust::RustAnalyzer;
use debtmap::complexity::pure::calculate_max_nesting_depth;
use debtmap::Analyzer;
use std::path::PathBuf;

// =============================================================================
// Helper Functions
// =============================================================================

/// Calculate nesting depth for code using the pure function directly.
fn calculate_nesting_for_code(code: &str) -> u32 {
    let file: syn::File = syn::parse_str(code).expect("Failed to parse test code");
    for item in &file.items {
        if let syn::Item::Fn(func) = item {
            return calculate_max_nesting_depth(&func.block);
        }
    }
    panic!("No function found in test code");
}

/// Calculate nesting depth via the RustAnalyzer (extractor path).
fn calculate_via_analyzer(code: &str) -> u32 {
    let analyzer = RustAnalyzer::new();
    let path = PathBuf::from("test.rs");
    let ast = analyzer.parse(code, path).expect("Failed to parse");
    let metrics = analyzer.analyze(&ast);

    metrics
        .complexity
        .functions
        .first()
        .map(|f| f.nesting)
        .expect("No function metrics found")
}

// =============================================================================
// 1. Else-If Chain Tests
// =============================================================================

#[test]
fn test_else_if_chain_nesting_depth_1() {
    let code = r#"
    fn test() {
        if a {
            x
        } else if b {
            y
        } else if c {
            z
        } else {
            w
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(
        nesting, 1,
        "else-if chain should have nesting 1, not {}",
        nesting
    );
}

#[test]
fn test_simple_else_if() {
    let code = r#"
    fn test() {
        if a {
            x
        } else if b {
            y
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(nesting, 1, "simple else-if should have nesting 1");
}

#[test]
fn test_long_else_if_chain() {
    let code = r#"
    fn test() {
        if a { 1 }
        else if b { 2 }
        else if c { 3 }
        else if d { 4 }
        else if e { 5 }
        else if f { 6 }
        else if g { 7 }
        else if h { 8 }
        else { 9 }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(
        nesting, 1,
        "8-branch else-if chain should still have nesting 1, got {}",
        nesting
    );
}

#[test]
fn test_else_if_with_let() {
    let code = r#"
    fn test() {
        if let Some(x) = opt {
            x
        } else if let Some(y) = opt2 {
            y
        } else {
            0
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(nesting, 1, "if-let else-if chain should have nesting 1");
}

// =============================================================================
// 2. Else Block Equivalence Tests
// =============================================================================

#[test]
fn test_else_block_with_if_equals_else_if() {
    let else_if_code = r#"
    fn test() {
        if a {
            x
        } else if b {
            y
        }
    }
    "#;

    let else_block_code = r#"
    fn test() {
        if a {
            x
        } else {
            if b {
                y
            }
        }
    }
    "#;

    let nesting1 = calculate_nesting_for_code(else_if_code);
    let nesting2 = calculate_nesting_for_code(else_block_code);

    assert_eq!(
        nesting1, nesting2,
        "else if (nesting {}) and else {{ if }} (nesting {}) should have same nesting",
        nesting1, nesting2
    );
    assert_eq!(nesting1, 1, "Both should have nesting 1");
}

#[test]
fn test_else_block_with_multiple_statements_before_if() {
    // When there are statements before the if in the else block,
    // the if is truly inside a deeper scope
    let code = r#"
    fn test() {
        if a {
            x
        } else {
            let y = 1;
            if b {
                y
            }
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    // The inner if is inside a block that isn't just an else-if continuation
    // This case is ambiguous - document whatever the expected behavior is
    // For now, we accept either 1 or 2 as valid (depends on implementation)
    assert!(
        (1..=2).contains(&nesting),
        "if after statement in else block should have nesting 1 or 2, got {}",
        nesting
    );
}

#[test]
fn test_deeply_chained_else_block_ifs() {
    // else { if ... else { if ... else { if } } }
    let code = r#"
    fn test() {
        if a {
            1
        } else {
            if b {
                2
            } else {
                if c {
                    3
                } else {
                    4
                }
            }
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    // This is structurally equivalent to else if chain
    assert_eq!(
        nesting, 1,
        "deeply chained else {{ if }} should have nesting 1, got {}",
        nesting
    );
}

// =============================================================================
// 3. Nested Control Flow Tests
// =============================================================================

#[test]
fn test_if_inside_if() {
    let code = r#"
    fn test() {
        if a {
            if b {
                x
            }
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        2,
        "if inside if should have nesting 2"
    );
}

#[test]
fn test_if_inside_for() {
    let code = r#"
    fn test() {
        for i in items {
            if i > 0 {
                x
            }
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        2,
        "if inside for should have nesting 2"
    );
}

#[test]
fn test_for_inside_if() {
    let code = r#"
    fn test() {
        if condition {
            for i in items {
                x
            }
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        2,
        "for inside if should have nesting 2"
    );
}

#[test]
fn test_while_inside_for() {
    let code = r#"
    fn test() {
        for i in items {
            while condition {
                x
            }
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        2,
        "while inside for should have nesting 2"
    );
}

#[test]
fn test_loop_inside_if() {
    let code = r#"
    fn test() {
        if condition {
            loop {
                break;
            }
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        2,
        "loop inside if should have nesting 2"
    );
}

#[test]
fn test_match_inside_while() {
    let code = r#"
    fn test() {
        while condition {
            match x {
                A => {}
                B => {}
            }
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        2,
        "match inside while should have nesting 2"
    );
}

#[test]
fn test_triple_nesting() {
    let code = r#"
    fn test() {
        if a {
            for i in items {
                while condition {
                    x
                }
            }
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        3,
        "if -> for -> while should have nesting 3"
    );
}

#[test]
fn test_five_levels_of_nesting() {
    let code = r#"
    fn test() {
        if a {
            for i in items {
                while condition {
                    match x {
                        A => {
                            if y {
                                z
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        5,
        "5 levels of nesting should be 5"
    );
}

// =============================================================================
// 4. Match Arm Tests
// =============================================================================

#[test]
fn test_simple_match() {
    let code = r#"
    fn test() {
        match x {
            A => 1,
            B => 2,
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        1,
        "simple match should have nesting 1"
    );
}

#[test]
fn test_match_with_if_in_arm() {
    let code = r#"
    fn test() {
        match x {
            A => {
                if y {
                    z
                }
            }
            B => {}
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        2,
        "match with if in arm should have nesting 2"
    );
}

#[test]
fn test_match_with_else_if_in_arm() {
    let code = r#"
    fn test() {
        match x {
            A => {
                if a { 1 }
                else if b { 2 }
                else { 3 }
            }
            B => {}
        }
    }
    "#;

    // match is nesting 1, else-if chain inside doesn't add depth
    assert_eq!(
        calculate_nesting_for_code(code),
        2,
        "match with else-if chain in arm should have nesting 2"
    );
}

#[test]
fn test_match_with_nested_match_in_arm() {
    let code = r#"
    fn test() {
        match x {
            A => {
                match y {
                    C => 1,
                    D => 2,
                }
            }
            B => 0,
        }
    }
    "#;

    assert_eq!(
        calculate_nesting_for_code(code),
        2,
        "nested match should have nesting 2"
    );
}

#[test]
fn test_match_with_if_guard() {
    let code = r#"
    fn test() {
        match x {
            A if condition => 1,
            B => 2,
            _ => 3,
        }
    }
    "#;

    // Guard doesn't add nesting, it's part of the match arm
    assert_eq!(
        calculate_nesting_for_code(code),
        1,
        "match with guard should have nesting 1"
    );
}

// =============================================================================
// 5. Complex Real-World Pattern Tests
// =============================================================================

#[test]
fn test_url_parser_pattern() {
    // Pattern inspired by Zed's mention.rs
    let code = r#"
    fn parse(input: &str) -> Result<Self, ()> {
        match url.scheme() {
            "file" => {
                if let Some(fragment) = url.fragment() {
                    if let Some(name) = get_param(&url, "symbol") {
                        Ok(Symbol { name })
                    } else {
                        Ok(Selection { })
                    }
                } else {
                    Ok(File { })
                }
            }
            "zed" => {
                if let Some(id) = path.strip_prefix("/thread/") {
                    Ok(Thread { id })
                } else if let Some(path) = path.strip_prefix("/text-thread/") {
                    Ok(TextThread { path })
                } else if let Some(id) = path.strip_prefix("/rule/") {
                    Ok(Rule { id })
                } else if path.starts_with("/pasted-image") {
                    Ok(PastedImage)
                } else if path.starts_with("/untitled-buffer") {
                    Ok(Selection { })
                } else if let Some(name) = path.strip_prefix("/symbol/") {
                    Ok(Symbol { name })
                } else if path.starts_with("/file") {
                    Ok(File { })
                } else if path.starts_with("/directory") {
                    Ok(Directory { })
                } else {
                    Err(())
                }
            }
            _ => Err(())
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);

    // match (1) -> if let in file arm (2) -> nested if let (3)
    // The zed arm has else-if chain at level 2, doesn't go deeper
    assert!(
        nesting <= 4,
        "Complex parser should have nesting <= 4, got {}",
        nesting
    );
}

#[test]
fn test_validation_chain_pattern() {
    let code = r#"
    fn validate(input: &Input) -> Result<(), Error> {
        if input.name.is_empty() {
            Err(Error::EmptyName)
        } else if input.name.len() > 100 {
            Err(Error::NameTooLong)
        } else if !input.name.chars().all(|c| c.is_alphanumeric()) {
            Err(Error::InvalidCharacters)
        } else if input.email.is_empty() {
            Err(Error::EmptyEmail)
        } else if !input.email.contains('@') {
            Err(Error::InvalidEmail)
        } else {
            Ok(())
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(
        nesting, 1,
        "Validation chain should have nesting 1, got {}",
        nesting
    );
}

#[test]
fn test_event_handler_pattern() {
    let code = r#"
    fn handle_event(event: Event) {
        match event {
            Event::Click(point) => {
                if point.x > 0 && point.y > 0 {
                    for handler in handlers {
                        handler.on_click(point);
                    }
                }
            }
            Event::KeyPress(key) => {
                match key {
                    Key::Enter => submit(),
                    Key::Escape => cancel(),
                    Key::Tab => focus_next(),
                    _ => {}
                }
            }
            Event::Resize(width, height) => {
                if width > 0 && height > 0 {
                    resize(width, height);
                }
            }
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    // Click arm: match (1) -> if (2) -> for (3) = max 3
    // KeyPress arm: match (1) -> match (2) = max 2
    // Resize arm: match (1) -> if (2) = max 2
    assert_eq!(
        nesting, 3,
        "Event handler should have nesting 3, got {}",
        nesting
    );
}

#[test]
fn test_option_chain_pattern() {
    let code = r#"
    fn process(value: Option<Data>) -> Option<Result> {
        if let Some(data) = value {
            if let Some(inner) = data.inner {
                if let Some(processed) = transform(inner) {
                    Some(processed)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(
        nesting, 3,
        "Option chain should have nesting 3, got {}",
        nesting
    );
}

// =============================================================================
// 6. Consistency Tests
// =============================================================================

#[test]
fn test_nesting_consistency_simple_cases() {
    let test_cases = vec![
        ("simple", "fn f() { let x = 1; }", 0),
        ("if", "fn f() { if a { x } }", 1),
        ("if_else", "fn f() { if a { x } else { y } }", 1),
        (
            "else_if",
            "fn f() { if a { x } else if b { y } else { z } }",
            1,
        ),
        ("nested_if", "fn f() { if a { if b { x } } }", 2),
        ("for_if", "fn f() { for i in x { if a { y } } }", 2),
        (
            "match_if",
            "fn f() { match x { A => { if a { y } }, B => {} } }",
            2,
        ),
    ];

    for (name, code, expected) in test_cases {
        let pure = calculate_nesting_for_code(code);
        let analyzer = calculate_via_analyzer(code);

        assert_eq!(
            pure, expected,
            "Case '{}': pure nesting {} != expected {}",
            name, pure, expected
        );
        assert_eq!(
            analyzer, expected,
            "Case '{}': analyzer nesting {} != expected {}",
            name, analyzer, expected
        );
        assert_eq!(
            pure, analyzer,
            "Case '{}': pure ({}) != analyzer ({})",
            name, pure, analyzer
        );
    }
}

#[test]
fn test_nesting_consistency_else_if_chains() {
    let test_cases = vec![
        ("2_branch", "fn f() { if a { 1 } else if b { 2 } }", 1),
        (
            "3_branch",
            "fn f() { if a { 1 } else if b { 2 } else { 3 } }",
            1,
        ),
        (
            "4_branch",
            "fn f() { if a { 1 } else if b { 2 } else if c { 3 } else { 4 } }",
            1,
        ),
        (
            "5_branch",
            "fn f() { if a { 1 } else if b { 2 } else if c { 3 } else if d { 4 } else { 5 } }",
            1,
        ),
    ];

    for (name, code, expected) in test_cases {
        let pure = calculate_nesting_for_code(code);
        let analyzer = calculate_via_analyzer(code);

        assert_eq!(
            pure, expected,
            "Case '{}': pure nesting {} != expected {}",
            name, pure, expected
        );
        assert_eq!(
            analyzer, expected,
            "Case '{}': analyzer nesting {} != expected {}",
            name, analyzer, expected
        );
    }
}

#[test]
fn test_nesting_consistency_loops() {
    let test_cases = vec![
        ("for", "fn f() { for i in x { y } }", 1),
        ("while", "fn f() { while a { y } }", 1),
        ("loop", "fn f() { loop { break; } }", 1),
        ("for_for", "fn f() { for i in x { for j in y { z } } }", 2),
        ("while_for", "fn f() { while a { for i in x { y } } }", 2),
    ];

    for (name, code, expected) in test_cases {
        let pure = calculate_nesting_for_code(code);
        let analyzer = calculate_via_analyzer(code);

        assert_eq!(
            pure, expected,
            "Case '{}': pure nesting {} != expected {}",
            name, pure, expected
        );
        assert_eq!(
            analyzer, expected,
            "Case '{}': analyzer nesting {} != expected {}",
            name, analyzer, expected
        );
    }
}

#[test]
fn test_nesting_consistency_match() {
    let test_cases = vec![
        ("match", "fn f() { match x { A => 1, B => 2 } }", 1),
        (
            "match_match",
            "fn f() { match x { A => match y { C => 1, D => 2 }, B => 3 } }",
            2,
        ),
        (
            "match_if",
            "fn f() { match x { A => if a { 1 } else { 2 }, B => 3 } }",
            2,
        ),
    ];

    for (name, code, expected) in test_cases {
        let pure = calculate_nesting_for_code(code);
        let analyzer = calculate_via_analyzer(code);

        assert_eq!(
            pure, expected,
            "Case '{}': pure nesting {} != expected {}",
            name, pure, expected
        );
        assert_eq!(
            analyzer, expected,
            "Case '{}': analyzer nesting {} != expected {}",
            name, analyzer, expected
        );
    }
}

// =============================================================================
// 7. Edge Cases
// =============================================================================

#[test]
fn test_empty_function() {
    let code = "fn f() {}";
    assert_eq!(calculate_nesting_for_code(code), 0);
}

#[test]
fn test_function_with_only_expression() {
    let code = "fn f() { 42 }";
    assert_eq!(calculate_nesting_for_code(code), 0);
}

#[test]
fn test_closure_inside_function() {
    // Closures don't add to function nesting, they have their own
    let code = r#"
    fn f() {
        let closure = |x| {
            if x > 0 {
                x
            } else {
                -x
            }
        };
    }
    "#;

    // The function itself doesn't have nesting, the closure does
    // But we're measuring the function's nesting, not the closure's
    let nesting = calculate_nesting_for_code(code);
    // The closure's body isn't counted as part of the function's control flow nesting
    // This may vary based on implementation - document the expected behavior
    assert!(
        nesting <= 1,
        "Function with closure should have nesting 0 or 1, got {}",
        nesting
    );
}

#[test]
fn test_block_expression_doesnt_add_nesting() {
    let code = r#"
    fn f() {
        let x = {
            let y = 1;
            let z = 2;
            y + z
        };
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(
        nesting, 0,
        "Block expression should not add nesting, got {}",
        nesting
    );
}

#[test]
fn test_unsafe_block_doesnt_add_nesting() {
    let code = r#"
    fn f() {
        unsafe {
            ptr::read(addr)
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(
        nesting, 0,
        "Unsafe block should not add nesting, got {}",
        nesting
    );
}

#[test]
fn test_async_block_doesnt_add_nesting() {
    let code = r#"
    fn f() {
        let future = async {
            do_something().await
        };
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(
        nesting, 0,
        "Async block should not add nesting, got {}",
        nesting
    );
}

#[test]
fn test_parallel_control_flow_takes_max() {
    // Two parallel control flow paths - nesting should be max of them
    let code = r#"
    fn f() {
        if a {
            x
        }
        if b {
            if c {
                y
            }
        }
    }
    "#;

    let nesting = calculate_nesting_for_code(code);
    assert_eq!(
        nesting, 2,
        "Parallel control flow should take max nesting, got {}",
        nesting
    );
}
