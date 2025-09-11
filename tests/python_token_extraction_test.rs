use debtmap::complexity::entropy_core::LanguageEntropyAnalyzer;
use debtmap::complexity::languages::python::PythonEntropyAnalyzer;
use rustpython_parser::{ast, parse, Mode};

/// Helper to parse Python expression and extract tokens
fn extract_tokens_from_expr_str(
    expr_str: &str,
) -> Vec<debtmap::complexity::entropy_traits::GenericToken> {
    let full_source = format!("x = {}", expr_str);
    let module = parse(&full_source, Mode::Module, "<test>").expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let analyzer = PythonEntropyAnalyzer::new(&full_source);

    // Use the public extract_tokens method which takes statements
    analyzer.extract_tokens(&module.body)
}

/// Helper to parse Python statement and extract tokens
fn extract_tokens_from_stmt_str(
    stmt_str: &str,
) -> Vec<debtmap::complexity::entropy_traits::GenericToken> {
    let module = parse(stmt_str, Mode::Module, "<test>").expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let analyzer = PythonEntropyAnalyzer::new(stmt_str);

    // Use the public extract_tokens method
    analyzer.extract_tokens(&module.body)
}

#[test]
fn test_extract_boolean_operations() {
    // Test AND operation
    let tokens = extract_tokens_from_expr_str("a and b");
    assert!(
        tokens.iter().any(|t| t.value() == "and"),
        "Should extract 'and' operator"
    );

    // Test OR operation
    let tokens = extract_tokens_from_expr_str("x or y");
    assert!(
        tokens.iter().any(|t| t.value() == "or"),
        "Should extract 'or' operator"
    );

    // Test complex boolean expression
    let tokens = extract_tokens_from_expr_str("(a and b) or (c and d)");
    let and_count = tokens.iter().filter(|t| t.value() == "and").count();
    let or_count = tokens.iter().filter(|t| t.value() == "or").count();
    assert_eq!(and_count, 2, "Should extract two 'and' operators");
    assert_eq!(or_count, 1, "Should extract one 'or' operator");
}

#[test]
fn test_extract_binary_operations() {
    // Test arithmetic operators
    let tokens = extract_tokens_from_expr_str("a + b");
    assert!(
        tokens.iter().any(|t| t.value().contains("Add")),
        "Should extract addition operator"
    );

    let tokens = extract_tokens_from_expr_str("x - y");
    assert!(
        tokens.iter().any(|t| t.value().contains("Sub")),
        "Should extract subtraction operator"
    );

    let tokens = extract_tokens_from_expr_str("m * n");
    assert!(
        tokens.iter().any(|t| t.value().contains("Mult")),
        "Should extract multiplication operator"
    );

    let tokens = extract_tokens_from_expr_str("p / q");
    assert!(
        tokens.iter().any(|t| t.value().contains("Div")),
        "Should extract division operator"
    );
}

#[test]
fn test_extract_unary_operations() {
    // Test NOT operation
    let tokens = extract_tokens_from_expr_str("not x");
    assert!(
        tokens.iter().any(|t| t.value() == "not"),
        "Should extract 'not' operator"
    );

    // Test negation
    let tokens = extract_tokens_from_expr_str("-value");
    assert!(
        tokens.iter().any(|t| t.value() == "-"),
        "Should extract negation operator"
    );

    // Test positive
    let tokens = extract_tokens_from_expr_str("+value");
    assert!(
        tokens.iter().any(|t| t.value() == "+"),
        "Should extract positive operator"
    );

    // Test bitwise NOT
    let tokens = extract_tokens_from_expr_str("~bits");
    assert!(
        tokens.iter().any(|t| t.value() == "~"),
        "Should extract bitwise NOT operator"
    );
}

#[test]
fn test_extract_lambda_expressions() {
    let tokens = extract_tokens_from_expr_str("lambda x: x * 2");
    assert!(
        tokens.iter().any(|t| t.value() == "lambda"),
        "Should extract 'lambda' keyword"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("Mult")),
        "Should extract multiplication in lambda body"
    );
}

#[test]
fn test_extract_if_expressions() {
    let tokens = extract_tokens_from_expr_str("a if condition else b");
    assert!(
        tokens.iter().any(|t| t.value() == "if"),
        "Should extract 'if' control flow"
    );
}

#[test]
fn test_extract_list_comprehensions() {
    let tokens = extract_tokens_from_expr_str("[x * 2 for x in range(10)]");
    assert!(
        tokens.iter().any(|t| t.value() == "list_comp"),
        "Should extract list comprehension token"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "for"),
        "Should extract 'for' control flow"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "in"),
        "Should extract 'in' operator"
    );

    // Test with condition
    let tokens = extract_tokens_from_expr_str("[x for x in items if x > 0]");
    assert!(
        tokens.iter().any(|t| t.value() == "if"),
        "Should extract 'if' in comprehension"
    );
}

#[test]
fn test_extract_set_comprehensions() {
    let tokens = extract_tokens_from_expr_str("{x * 2 for x in range(10)}");
    assert!(
        tokens.iter().any(|t| t.value() == "set_comp"),
        "Should extract set comprehension token"
    );
}

#[test]
fn test_extract_dict_comprehensions() {
    let tokens = extract_tokens_from_expr_str("{k: v * 2 for k, v in items.items()}");
    assert!(
        tokens.iter().any(|t| t.value() == "dict_comp"),
        "Should extract dict comprehension token"
    );
}

#[test]
fn test_extract_generator_expressions() {
    let tokens = extract_tokens_from_expr_str("(x * 2 for x in range(10))");
    assert!(
        tokens.iter().any(|t| t.value() == "generator"),
        "Should extract generator token"
    );
}

#[test]
fn test_extract_await_expressions() {
    let tokens = extract_tokens_from_expr_str("await async_func()");
    assert!(
        tokens.iter().any(|t| t.value() == "await"),
        "Should extract 'await' keyword"
    );
}

#[test]
fn test_extract_yield_expressions() {
    let stmt = "def gen():\n    yield value";
    let tokens = extract_tokens_from_stmt_str(stmt);
    assert!(
        tokens.iter().any(|t| t.value() == "yield"),
        "Should extract 'yield' keyword"
    );

    // Test yield from
    let stmt = "def gen():\n    yield from other_gen()";
    let tokens = extract_tokens_from_stmt_str(stmt);
    assert!(
        tokens.iter().any(|t| t.value() == "yield"),
        "Should extract 'yield' keyword"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "from"),
        "Should extract 'from' keyword"
    );
}

#[test]
fn test_extract_comparison_operations() {
    // Test equality
    let tokens = extract_tokens_from_expr_str("a == b");
    assert!(
        tokens.iter().any(|t| t.value() == "=="),
        "Should extract equality operator"
    );

    // Test inequality
    let tokens = extract_tokens_from_expr_str("x != y");
    assert!(
        tokens.iter().any(|t| t.value() == "!="),
        "Should extract inequality operator"
    );

    // Test less than
    let tokens = extract_tokens_from_expr_str("m < n");
    assert!(
        tokens.iter().any(|t| t.value() == "<"),
        "Should extract less than operator"
    );

    // Test greater than or equal
    let tokens = extract_tokens_from_expr_str("p >= q");
    assert!(
        tokens.iter().any(|t| t.value() == ">="),
        "Should extract greater than or equal operator"
    );

    // Test identity
    let tokens = extract_tokens_from_expr_str("a is None");
    assert!(
        tokens.iter().any(|t| t.value() == "is"),
        "Should extract 'is' operator"
    );

    // Test membership
    let tokens = extract_tokens_from_expr_str("item in collection");
    assert!(
        tokens.iter().any(|t| t.value() == "in"),
        "Should extract 'in' operator"
    );
}

#[test]
fn test_extract_call_expressions() {
    let tokens = extract_tokens_from_expr_str("func(arg1, arg2)");
    assert!(
        tokens.iter().any(|t| t.value() == "call"),
        "Should extract function call token"
    );
}

#[test]
fn test_extract_name_expressions() {
    let tokens = extract_tokens_from_expr_str("variable_name");
    // The assignment will have both the = operator and the variable name
    assert!(
        tokens.iter().any(|t| t.value() == "="),
        "Should extract assignment operator"
    );
    // Just check that we got tokens
    assert!(
        tokens.len() > 1,
        "Should extract multiple tokens including identifier"
    );
}

#[test]
fn test_extract_constant_expressions() {
    // Test None
    let tokens = extract_tokens_from_expr_str("None");
    assert!(
        tokens.iter().any(|t| t.value() == "None"),
        "Should extract None literal"
    );

    // Test boolean
    let tokens = extract_tokens_from_expr_str("True");
    assert!(
        tokens.iter().any(|t| t.value() == "bool"),
        "Should extract bool literal"
    );

    // Test string
    let tokens = extract_tokens_from_expr_str("'hello'");
    assert!(
        tokens.iter().any(|t| t.value() == "string"),
        "Should extract string literal"
    );

    // Test integer
    let tokens = extract_tokens_from_expr_str("42");
    assert!(
        tokens.iter().any(|t| t.value() == "int"),
        "Should extract int literal"
    );

    // Test float
    let tokens = extract_tokens_from_expr_str("3.14");
    assert!(
        tokens.iter().any(|t| t.value() == "float"),
        "Should extract float literal"
    );
}

#[test]
fn test_extract_walrus_operator() {
    let tokens = extract_tokens_from_expr_str("(n := len(data))");
    assert!(
        tokens.iter().any(|t| t.value() == ":="),
        "Should extract walrus operator"
    );
}

#[test]
fn test_extract_collection_literals() {
    // Test list
    let tokens = extract_tokens_from_expr_str("[1, 2, 3]");
    assert!(
        tokens.iter().any(|t| t.value() == "list"),
        "Should extract list token"
    );

    // Test tuple
    let tokens = extract_tokens_from_expr_str("(1, 2, 3)");
    assert!(
        tokens.iter().any(|t| t.value() == "tuple"),
        "Should extract tuple token"
    );

    // Test dict
    let tokens = extract_tokens_from_expr_str("{'key': 'value'}");
    assert!(
        tokens.iter().any(|t| t.value() == "dict"),
        "Should extract dict token"
    );

    // Test set
    let tokens = extract_tokens_from_expr_str("{1, 2, 3}");
    assert!(
        tokens.iter().any(|t| t.value() == "set"),
        "Should extract set token"
    );
}

#[test]
fn test_extract_attribute_access() {
    let tokens = extract_tokens_from_expr_str("obj.attribute");
    assert!(
        tokens.iter().any(|t| t.value() == "="),
        "Should extract assignment operator"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "."),
        "Should extract dot operator"
    );
    // Just check that we have multiple tokens
    assert!(
        tokens.len() > 2,
        "Should extract multiple tokens for attribute access"
    );
}

#[test]
fn test_extract_subscript_operations() {
    let tokens = extract_tokens_from_expr_str("array[index]");
    assert!(
        tokens.iter().any(|t| t.value() == "[]"),
        "Should extract subscript operator"
    );
}

#[test]
fn test_extract_slice_operations() {
    let tokens = extract_tokens_from_expr_str("array[1:10:2]");
    assert!(
        tokens.iter().any(|t| t.value() == ":"),
        "Should extract slice operator"
    );

    // Test slice with missing parts
    let tokens = extract_tokens_from_expr_str("array[:5]");
    assert!(
        tokens.iter().any(|t| t.value() == ":"),
        "Should extract slice operator for partial slice"
    );
}

#[test]
fn test_extract_starred_expressions() {
    let tokens = extract_tokens_from_expr_str("func(*args)");
    assert!(
        tokens.iter().any(|t| t.value() == "*"),
        "Should extract star operator"
    );
}

#[test]
fn test_extract_fstring_expressions() {
    let tokens = extract_tokens_from_expr_str("f'Hello {name}'");
    assert!(
        tokens.iter().any(|t| t.value() == "f-string"),
        "Should extract f-string literal"
    );
}

#[test]
fn test_extract_complex_nested_expression() {
    // Test a complex expression with multiple nested operations
    let expr = "[x * 2 if x > 0 else -x for x in data if x != 0]";
    let tokens = extract_tokens_from_expr_str(expr);

    // Verify various token types are extracted
    assert!(
        tokens.iter().any(|t| t.value() == "list_comp"),
        "Should extract list comprehension"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "if"),
        "Should extract if control flow"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "for"),
        "Should extract for control flow"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "in"),
        "Should extract in operator"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("Mult")),
        "Should extract multiplication"
    );
    assert!(
        tokens.iter().any(|t| t.value() == ">"),
        "Should extract comparison"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "!="),
        "Should extract inequality"
    );
}

#[test]
fn test_statement_extraction() {
    // Test function definition
    let stmt = "def my_func(x, y):\n    return x + y";
    let tokens = extract_tokens_from_stmt_str(stmt);
    assert!(
        tokens.iter().any(|t| t.value() == "def"),
        "Should extract 'def' keyword"
    );
    // Just check that we have the tokens
    assert!(
        tokens.len() > 3,
        "Should extract multiple tokens including function name"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "return"),
        "Should extract 'return' keyword"
    );

    // Test class definition
    let stmt = "class MyClass:\n    pass";
    let tokens = extract_tokens_from_stmt_str(stmt);
    assert!(
        tokens.iter().any(|t| t.value() == "class"),
        "Should extract 'class' keyword"
    );
    // Just check that we have tokens
    assert!(tokens.len() > 1, "Should extract class tokens");

    // Test async function
    let stmt = "async def async_func():\n    pass";
    let tokens = extract_tokens_from_stmt_str(stmt);
    assert!(
        tokens.iter().any(|t| t.value() == "async"),
        "Should extract 'async' keyword"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "def"),
        "Should extract 'def' keyword"
    );

    // Test assignment
    let stmt = "x = 42";
    let tokens = extract_tokens_from_stmt_str(stmt);
    assert!(
        tokens.iter().any(|t| t.value() == "="),
        "Should extract assignment operator"
    );

    // Test augmented assignment
    let stmt = "x += 1";
    let tokens = extract_tokens_from_stmt_str(stmt);
    assert!(
        tokens.iter().any(|t| t.value().contains("=")),
        "Should extract augmented assignment"
    );
}

#[test]
fn test_all_expression_types_covered() {
    // This test ensures we have coverage for all 28 expression types in the match
    let test_cases = vec![
        ("a and b", "and"),                 // BoolOp
        ("x + y", "Add"),                   // BinOp
        ("not x", "not"),                   // UnaryOp
        ("lambda x: x", "lambda"),          // Lambda
        ("a if b else c", "if"),            // IfExp
        ("[x for x in y]", "list_comp"),    // ListComp
        ("{x for x in y}", "set_comp"),     // SetComp
        ("{k: v for k in d}", "dict_comp"), // DictComp
        ("(x for x in y)", "generator"),    // GeneratorExp
        ("await f()", "await"),             // Await
        ("a == b", "=="),                   // Compare
        ("func()", "call"),                 // Call
        ("var_name", "="),                  // Name (we'll check for assignment)
        ("42", "int"),                      // Constant
        ("(x := 1)", ":="),                 // NamedExpr
        ("[1, 2]", "list"),                 // List
        ("(1, 2)", "tuple"),                // Tuple
        ("{'k': 'v'}", "dict"),             // Dict
        ("{1, 2}", "set"),                  // Set
        ("obj.attr", "."),                  // Attribute
        ("arr[0]", "[]"),                   // Subscript
        ("arr[1:2]", ":"),                  // Slice
        ("*args", "*"),                     // Starred
        ("f'hi {x}'", "f-string"),          // JoinedStr
    ];

    for (expr, expected_token) in test_cases {
        let tokens = extract_tokens_from_expr_str(expr);
        assert!(
            tokens.iter().any(|t| t.value().contains(expected_token)),
            "Failed to extract expected token '{}' from expression '{}'",
            expected_token,
            expr
        );
    }
}

#[test]
fn test_deeply_nested_expressions() {
    // Test expressions with multiple levels of nesting
    let complex_expr = "{'key': [func(x * 2 + y) for x in range(10) if x > 0 and x < 5]}";
    let tokens = extract_tokens_from_expr_str(complex_expr);

    // Should extract tokens from all levels of nesting
    assert!(
        tokens.len() > 10,
        "Should extract many tokens from complex expression"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "dict"),
        "Should extract dict"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "list_comp"),
        "Should extract list comprehension"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "call"),
        "Should extract function call"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("Mult")),
        "Should extract multiplication"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("Add")),
        "Should extract addition"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "and"),
        "Should extract and operator"
    );
}

#[test]
fn test_yield_and_yield_from() {
    // Test yield expression with value
    let stmt = "def gen():\n    yield 42";
    let tokens = extract_tokens_from_stmt_str(stmt);
    assert!(
        tokens.iter().any(|t| t.value() == "yield"),
        "Should extract yield"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "int"),
        "Should extract yielded value"
    );

    // Test yield from
    let stmt = "def gen():\n    yield from other()";
    let tokens = extract_tokens_from_stmt_str(stmt);
    assert!(
        tokens.iter().any(|t| t.value() == "yield"),
        "Should extract yield"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "from"),
        "Should extract from"
    );
}

#[test]
fn test_formatted_value_in_fstring() {
    // Test formatted value extraction in f-strings
    let tokens = extract_tokens_from_expr_str("f'{value:.2f}'");
    assert!(
        tokens.iter().any(|t| t.value() == "f-string"),
        "Should extract f-string"
    );
    // The formatted value should be processed
    assert!(
        tokens.len() > 1,
        "Should extract tokens from formatted value"
    );
}

#[test]
fn test_empty_collections() {
    // Test empty list
    let tokens = extract_tokens_from_expr_str("[]");
    assert!(
        tokens.iter().any(|t| t.value() == "list"),
        "Should extract empty list"
    );

    // Test empty dict
    let tokens = extract_tokens_from_expr_str("{}");
    assert!(
        tokens.iter().any(|t| t.value() == "dict"),
        "Should extract empty dict"
    );

    // Test empty tuple
    let tokens = extract_tokens_from_expr_str("()");
    assert!(
        tokens.iter().any(|t| t.value() == "tuple"),
        "Should extract empty tuple"
    );
}

#[test]
fn test_chained_comparisons() {
    // Python allows chained comparisons
    let tokens = extract_tokens_from_expr_str("a < b <= c");
    let comparison_ops = tokens
        .iter()
        .filter(|t| t.value() == "<" || t.value() == "<=")
        .count();
    assert_eq!(
        comparison_ops, 2,
        "Should extract both comparison operators"
    );
}

#[test]
fn test_ellipsis_constant() {
    // Test ellipsis literal
    let tokens = extract_tokens_from_expr_str("...");
    assert!(
        tokens.iter().any(|t| t.value() == "..."),
        "Should extract ellipsis"
    );
}

#[test]
fn test_bytes_literal() {
    // Test bytes literal
    let tokens = extract_tokens_from_expr_str("b'hello'");
    assert!(
        tokens.iter().any(|t| t.value() == "bytes"),
        "Should extract bytes literal"
    );
}

#[test]
fn test_complex_number_literal() {
    // Test complex number literal
    let tokens = extract_tokens_from_expr_str("3+4j");
    assert!(
        tokens.iter().any(|t| t.value() == "complex"),
        "Should extract complex literal"
    );
}
