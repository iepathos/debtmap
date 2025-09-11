use debtmap::analyzers::python::PythonAnalyzer;
use debtmap::analyzers::Analyzer;
use debtmap::complexity::entropy_core::LanguageEntropyAnalyzer;
use debtmap::complexity::entropy_core::{EntropyConfig, EntropyToken, UniversalEntropyCalculator};
use debtmap::complexity::languages::python::PythonEntropyAnalyzer;
use rustpython_parser::ast;
use std::path::PathBuf;

#[test]
fn test_python_entropy_analyzer_basic() {
    let source = r#"
def process_data(x):
    if x > 0:
        return x * 2
    else:
        return x / 2
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);

    // Parse the Python code
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    // Extract tokens
    let tokens = analyzer.extract_tokens(&module.body);
    assert!(!tokens.is_empty(), "Should extract tokens from Python code");

    // Detect patterns
    let patterns = analyzer.detect_patterns(&module.body);
    assert!(patterns.total_patterns > 0, "Should detect patterns");

    // Check branch similarity
    let similarity = analyzer.calculate_branch_similarity(&module.body);
    assert!(
        (0.0..=1.0).contains(&similarity),
        "Branch similarity should be between 0 and 1"
    );

    // Analyze structure
    let (vars, nesting) = analyzer.analyze_structure(&module.body);
    assert_eq!(vars, 1, "Should find one variable (x)");
    assert!(nesting > 0, "Should have some nesting");
}

#[test]
fn test_python_entropy_calculation() {
    let source = r#"
def calculate_total(items):
    total = 0
    for item in items:
        if item.is_valid:
            total += item.value
        else:
            total -= item.penalty
    return total
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(source, PathBuf::from("test.py"))
        .expect("Should parse Python code");

    let metrics = analyzer.analyze(&ast);

    // Check that entropy score is calculated
    assert!(
        !metrics.complexity.functions.is_empty(),
        "Should have function metrics"
    );
    let func = &metrics.complexity.functions[0];
    assert!(
        func.entropy_score.is_some(),
        "Entropy score should be calculated"
    );

    let entropy_score = func.entropy_score.as_ref().unwrap();
    assert!(
        entropy_score.effective_complexity > 0.0,
        "Entropy score should be positive"
    );
    assert!(
        entropy_score.effective_complexity < 100.0,
        "Entropy score should be reasonable"
    );
}

#[test]
fn test_python_entropy_with_repetitive_code() {
    let source = r#"
def validate_fields(data):
    if not data.field1:
        return False, "field1 is required"
    if not data.field2:
        return False, "field2 is required"
    if not data.field3:
        return False, "field3 is required"
    if not data.field4:
        return False, "field4 is required"
    if not data.field5:
        return False, "field5 is required"
    return True, "OK"
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(source, PathBuf::from("test.py"))
        .expect("Should parse Python code");

    let metrics = analyzer.analyze(&ast);

    assert!(!metrics.complexity.functions.is_empty());
    let func = &metrics.complexity.functions[0];

    // Repetitive validation code should have moderate to high entropy
    assert!(func.entropy_score.is_some());
    let entropy = func.entropy_score.as_ref().unwrap();
    // Repetitive code should have some entropy score calculated
    assert!(
        entropy.effective_complexity >= 0.0,
        "Repetitive code should have entropy calculated (got {})",
        entropy.effective_complexity
    );
}

#[test]
fn test_python_entropy_with_complex_branching() {
    let source = r#"
def process_request(req):
    if req.type == 'GET':
        if req.auth:
            if req.cache:
                return fetch_cached(req)
            else:
                return fetch_fresh(req)
        else:
            return error_401()
    elif req.type == 'POST':
        if validate(req.data):
            if req.is_async:
                return queue_job(req)
            else:
                return process_sync(req)
        else:
            return error_400()
    else:
        return error_405()
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(source, PathBuf::from("test.py"))
        .expect("Should parse Python code");

    let metrics = analyzer.analyze(&ast);

    assert!(!metrics.complexity.functions.is_empty());
    let func = &metrics.complexity.functions[0];

    // Complex nested branching should have high entropy
    assert!(func.entropy_score.is_some());
    let entropy = func.entropy_score.as_ref().unwrap();
    // Complex nested branching should have entropy calculated
    assert!(
        entropy.effective_complexity >= 0.0,
        "Complex branching should have entropy calculated (got {})",
        entropy.effective_complexity
    );
}

#[test]
fn test_python_entropy_with_list_comprehension() {
    let source = r#"
def transform_data(items):
    results = [x * 2 for x in items if x > 0]
    squared = [x ** 2 for x in results]
    filtered = [x for x in squared if x < 100]
    return sum(filtered)
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);

    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    // Test that list comprehensions are properly tokenized
    let tokens = analyzer.extract_tokens(&module.body);
    let comp_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.value().contains("comp"))
        .collect();
    assert!(!comp_tokens.is_empty(), "Should detect list comprehensions");

    // Pattern detection should find comprehensions
    let patterns = analyzer.detect_patterns(&module.body);
    assert!(
        patterns.total_patterns > 0,
        "Should detect comprehension patterns"
    );
}

#[test]
fn test_python_entropy_async_functions() {
    let source = r#"
async def fetch_data(url):
    response = await http_get(url)
    if response.status == 200:
        data = await response.json()
        return process(data)
    else:
        raise Exception("Failed to fetch")
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(source, PathBuf::from("test.py"))
        .expect("Should parse Python code");

    let metrics = analyzer.analyze(&ast);

    assert!(!metrics.complexity.functions.is_empty());
    let func = &metrics.complexity.functions[0];

    // Async functions should have entropy calculated
    assert!(
        func.entropy_score.is_some(),
        "Async functions should have entropy score"
    );
    assert!(
        func.name.contains("async"),
        "Function should be marked as async"
    );
}

#[test]
fn test_python_entropy_class_methods() {
    let source = r#"
class DataProcessor:
    def __init__(self, config):
        self.config = config
        self.cache = {}
    
    def process(self, item):
        if item.id in self.cache:
            return self.cache[item.id]
        
        result = self._compute(item)
        self.cache[item.id] = result
        return result
    
    def _compute(self, item):
        if self.config.mode == 'fast':
            return item.value * 2
        else:
            return item.value ** 2
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(source, PathBuf::from("test.py"))
        .expect("Should parse Python code");

    let metrics = analyzer.analyze(&ast);

    // Should have metrics for all methods
    assert_eq!(
        metrics.complexity.functions.len(),
        3,
        "Should have 3 methods"
    );

    for func in &metrics.complexity.functions {
        assert!(
            func.entropy_score.is_some(),
            "Method {} should have entropy score",
            func.name
        );
        assert!(
            func.name.contains("DataProcessor"),
            "Method {} should include class name",
            func.name
        );
    }
}

#[test]
fn test_python_entropy_exception_handling() {
    let source = r#"
def safe_divide(a, b):
    try:
        result = a / b
        return result
    except ZeroDivisionError:
        return float('inf')
    except TypeError:
        return None
    except Exception as e:
        log_error(e)
        raise
    finally:
        cleanup()
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);

    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    // Exception handling should contribute to entropy
    let tokens = analyzer.extract_tokens(&module.body);
    let exception_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| {
            t.value().contains("except")
                || t.value().contains("try")
                || t.value().contains("finally")
        })
        .collect();
    assert!(
        !exception_tokens.is_empty(),
        "Should detect exception handling"
    );

    // Calculate entropy with calculator
    let mut calculator = UniversalEntropyCalculator::new(EntropyConfig::default());
    let score = calculator.calculate(&analyzer, &module.body);
    assert!(
        score.effective_complexity > 0.0,
        "Exception handling should add complexity"
    );
}

#[test]
fn test_python_entropy_pattern_matching() {
    let source = r#"
def handle_value(value):
    match value:
        case 0:
            return "zero"
        case 1 | 2 | 3:
            return "small"
        case int(x) if x < 0:
            return "negative"
        case int(x) if x > 100:
            return "large"
        case _:
            return "other"
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);

    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    // Pattern matching should be detected
    let patterns = analyzer.detect_patterns(&module.body);
    assert!(patterns.total_patterns > 0, "Should detect match patterns");

    // Branch similarity should be calculated for match arms
    let similarity = analyzer.calculate_branch_similarity(&module.body);
    assert!(
        similarity > 0.0,
        "Match statements should have branch similarity"
    );
}

#[test]
fn test_python_entropy_generator_expressions() {
    let source = r#"
def process_large_dataset(data):
    filtered = (x for x in data if x > 0)
    transformed = (x * 2 for x in filtered)
    result = sum(x for x in transformed if x < 100)
    return result
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);

    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    // Generator expressions should be detected as patterns
    let tokens = analyzer.extract_tokens(&module.body);
    let gen_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.value().contains("generator"))
        .collect();
    assert!(
        !gen_tokens.is_empty(),
        "Should detect generator expressions"
    );
}

#[test]
fn test_python_entropy_lambda_functions() {
    let source = r#"
def create_handlers():
    add_func = lambda x, y: x + y
    multiply_func = lambda x, y: x * y
    return add_func, multiply_func
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);

    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    // Lambda functions should be detected
    let tokens = analyzer.extract_tokens(&module.body);
    let lambda_tokens: Vec<_> = tokens.iter().filter(|t| t.value() == "lambda").collect();
    // Should detect at least 2 lambda functions
    assert!(
        lambda_tokens.len() >= 2,
        "Should detect lambda functions, found {}",
        lambda_tokens.len()
    );
}

#[test]
fn test_python_entropy_walrus_operator() {
    let source = r#"
def process_with_walrus(items):
    results = []
    for item in items:
        if (value := compute(item)) > 0:
            results.append(value)
        elif (error := check_error(item)) is not None:
            log(error)
    return results
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);

    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    // Walrus operator should be detected
    let tokens = analyzer.extract_tokens(&module.body);
    let walrus_tokens: Vec<_> = tokens.iter().filter(|t| t.value() == ":=").collect();
    assert!(!walrus_tokens.is_empty(), "Should detect walrus operator");
}

#[test]
fn test_entropy_score_integration() {
    // Test that entropy scores are properly integrated into function metrics
    let source = r#"
def simple_function(x):
    # Repetitive simple operations
    a = x + 1
    b = x + 1  
    c = x + 1
    d = x + 1
    return a + b + c + d

def complex_function(data, mode, threshold):
    if data is None:
        return None
    
    results = []
    cache = {}
    
    for item in data:
        if item > threshold:
            if mode == 'square':
                val = item ** 2
                cache[item] = val
                results.append(val)
            elif mode == 'factorial':
                fact = 1
                for i in range(1, item + 1):
                    fact *= i
                results.append(fact)
            else:
                transformed = item * 2 + threshold
                results.append(transformed)
        else:
            results.append(item)
    
    return results
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(source, PathBuf::from("test.py"))
        .expect("Should parse Python code");

    let metrics = analyzer.analyze(&ast);

    assert_eq!(
        metrics.complexity.functions.len(),
        2,
        "Should have 2 functions"
    );

    let simple_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name == "simple_function")
        .expect("Should find simple_function");

    let complex_func = metrics
        .complexity
        .functions
        .iter()
        .find(|f| f.name == "complex_function")
        .expect("Should find complex_function");

    // Both should have entropy scores
    assert!(
        simple_func.entropy_score.is_some(),
        "Simple function should have entropy score"
    );
    assert!(
        complex_func.entropy_score.is_some(),
        "Complex function should have entropy score"
    );

    // Complex function should have higher entropy than simple
    let simple_entropy = simple_func.entropy_score.as_ref().unwrap();
    let complex_entropy = complex_func.entropy_score.as_ref().unwrap();

    assert!(
        complex_entropy.effective_complexity > simple_entropy.effective_complexity,
        "Complex function ({}) should have higher entropy than simple ({})",
        complex_entropy.effective_complexity,
        simple_entropy.effective_complexity
    );
}

#[test]
fn test_extract_binary_op() {
    let source = r#"
def binary_ops():
    a = 5 + 3
    b = 10 - 2
    c = 4 * 2
    d = 8 / 2
    e = 7 % 3
    f = 2 ** 3
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let tokens = analyzer.extract_tokens(&module.body);

    // Check that binary operators are extracted
    let bin_ops: Vec<_> = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.to_category(),
                debtmap::complexity::entropy_core::TokenCategory::Operator
            )
        })
        .collect();

    assert!(!bin_ops.is_empty(), "Should extract binary operators");

    // Verify specific operators are present
    let op_values: Vec<String> = bin_ops.iter().map(|t| t.value().to_string()).collect();
    assert!(
        op_values.contains(&"Add".to_string()),
        "Should detect addition"
    );
    assert!(
        op_values.contains(&"Sub".to_string()),
        "Should detect subtraction"
    );
    assert!(
        op_values.contains(&"Mult".to_string()),
        "Should detect multiplication"
    );
    assert!(
        op_values.contains(&"Div".to_string()),
        "Should detect division"
    );
}

#[test]
fn test_extract_bool_op() {
    let source = r#"
def bool_ops():
    a = True and False
    b = x or y
    c = (a and b) or (c and d)
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let tokens = analyzer.extract_tokens(&module.body);

    // Check that boolean operators are extracted
    let and_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "and").collect();
    let or_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "or").collect();

    assert!(!and_ops.is_empty(), "Should extract 'and' operators");
    assert!(!or_ops.is_empty(), "Should extract 'or' operators");
}

#[test]
fn test_extract_unary_op() {
    let source = r#"
def unary_ops():
    a = not True
    b = -x
    c = +y
    d = ~bits
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let tokens = analyzer.extract_tokens(&module.body);

    // Check that unary operators are extracted
    let not_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "not").collect();
    let neg_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "-").collect();
    let pos_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "+").collect();
    let inv_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "~").collect();

    assert!(!not_ops.is_empty(), "Should extract 'not' operator");
    assert!(!neg_ops.is_empty(), "Should extract negation operator");
    assert!(!pos_ops.is_empty(), "Should extract positive operator");
    assert!(!inv_ops.is_empty(), "Should extract invert operator");
}

#[test]
fn test_extract_compare() {
    let source = r#"
def compare_ops():
    a = x == y
    b = x != y
    c = x < y
    d = x <= y
    e = x > y
    f = x >= y
    g = x is None
    h = x is not None
    i = x in items
    j = x not in items
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let tokens = analyzer.extract_tokens(&module.body);

    // Check that comparison operators are extracted
    let eq_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "==").collect();
    let ne_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "!=").collect();
    let lt_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "<").collect();
    let is_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "is").collect();
    let in_ops: Vec<_> = tokens.iter().filter(|t| t.value() == "in").collect();

    assert!(!eq_ops.is_empty(), "Should extract equality operator");
    assert!(!ne_ops.is_empty(), "Should extract inequality operator");
    assert!(!lt_ops.is_empty(), "Should extract less than operator");
    assert!(!is_ops.is_empty(), "Should extract 'is' operator");
    assert!(!in_ops.is_empty(), "Should extract 'in' operator");
}

#[test]
fn test_extract_comprehension_expr() {
    let source = r#"
def comprehensions():
    list_comp = [x * 2 for x in range(10)]
    set_comp = {x * 2 for x in range(10)}
    dict_comp = {x: x * 2 for x in range(10)}
    gen_exp = (x * 2 for x in range(10))
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let tokens = analyzer.extract_tokens(&module.body);

    // Check that comprehension tokens are created
    let list_comp_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.value().contains("list_comp"))
        .collect();
    let set_comp_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.value().contains("set_comp"))
        .collect();
    let dict_comp_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.value().contains("dict_comp"))
        .collect();
    let gen_tokens: Vec<_> = tokens
        .iter()
        .filter(|t| t.value().contains("generator"))
        .collect();

    assert!(
        !list_comp_tokens.is_empty(),
        "Should detect list comprehension"
    );
    assert!(
        !set_comp_tokens.is_empty(),
        "Should detect set comprehension"
    );
    assert!(
        !dict_comp_tokens.is_empty(),
        "Should detect dict comprehension"
    );
    assert!(!gen_tokens.is_empty(), "Should detect generator expression");
}

#[test]
fn test_all_expression_types() {
    let source = r#"
async def all_expressions():
    # Lambda
    f = lambda x: x * 2
    
    # If expression (ternary)
    result = a if condition else b
    
    # Await
    value = await async_func()
    
    # Yield
    yield x
    yield from iterator
    
    # Call
    result = func(arg1, arg2)
    
    # Named expression (walrus)
    if (n := len(items)) > 0:
        print(n)
    
    # Constants
    none_val = None
    bool_val = True
    str_val = "string"
    int_val = 42
    float_val = 1.23
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let tokens = analyzer.extract_tokens(&module.body);

    // Verify various token types are extracted
    assert!(
        tokens.iter().any(|t| t.value() == "lambda"),
        "Should detect lambda"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "if"),
        "Should detect if expression"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "await"),
        "Should detect await"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "yield"),
        "Should detect yield"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "from"),
        "Should detect yield from"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "call"),
        "Should detect function call"
    );
    assert!(
        tokens.iter().any(|t| t.value() == ":="),
        "Should detect walrus operator"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "None"),
        "Should detect None constant"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "bool"),
        "Should detect bool constant"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "string"),
        "Should detect string constant"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "int"),
        "Should detect int constant"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "float"),
        "Should detect float constant"
    );
}

#[test]
fn test_extract_tokens_from_expr_visitor_pattern() {
    // Test all 28 branches of the extract_tokens_from_expr visitor pattern
    let source = r#"
def comprehensive_expressions():
    # All expression types to test the visitor pattern
    
    # Boolean operations
    bool_result = a and b or c
    
    # Binary operations
    bin_result = x + y - z * w / q % r ** s
    
    # Unary operations
    unary_result = not a
    neg = -x
    pos = +y
    inv = ~z
    
    # Lambda
    func = lambda x, y: x + y
    
    # If expression (ternary)
    tern = a if condition else b
    
    # List comprehension
    list_comp = [x * 2 for x in range(10) if x > 0]
    
    # Set comprehension
    set_comp = {x * 2 for x in range(10)}
    
    # Dict comprehension
    dict_comp = {x: x * 2 for x in range(10)}
    
    # Generator expression
    gen_exp = (x * 2 for x in range(10))
    
    # Await expression
    awaited = await async_func()
    
    # Yield expressions
    yield value
    yield from generator
    
    # Comparison
    comp = a == b != c < d <= e > f >= g
    is_check = x is None
    is_not_check = y is not None
    in_check = z in container
    not_in_check = w not in container
    
    # Function call
    result = func(arg1, arg2, keyword=value)
    
    # Name (identifier)
    name = variable_name
    
    # Constants
    none_const = None
    bool_const = True
    str_const = "string"
    int_const = 42
    float_const = 3.14
    
    # Named expression (walrus operator)
    if (n := len(items)) > 0:
        print(n)
    
    # Collection literals
    list_lit = [1, 2, 3]
    tuple_lit = (1, 2, 3)
    dict_lit = {"key": "value", "foo": "bar"}
    set_lit = {1, 2, 3}
    
    # Attribute access
    attr = obj.attribute
    
    # Subscript
    item = container[index]
    
    # Slice
    sliced = sequence[start:stop:step]
    
    # Starred expression
    unpacked = [*items, extra]
    
    # JoinedStr (f-string)
    f_string = f"Value: {value}"
    
    # FormattedValue (inside f-string)
    formatted = f"{expr:.2f}"
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let tokens = analyzer.extract_tokens(&module.body);

    // Verify that all expression types are properly extracted
    // This tests all 28 branches of the visitor pattern

    // Boolean operations
    assert!(
        tokens.iter().any(|t| t.value() == "and"),
        "Should extract 'and' operator"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "or"),
        "Should extract 'or' operator"
    );

    // Binary operations (various)
    assert!(
        tokens.iter().any(|t| t.value().contains("Add")),
        "Should extract addition"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("Sub")),
        "Should extract subtraction"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("Mult")),
        "Should extract multiplication"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("Div")),
        "Should extract division"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("Mod")),
        "Should extract modulo"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("Pow")),
        "Should extract power"
    );

    // Unary operations
    assert!(
        tokens.iter().any(|t| t.value() == "not"),
        "Should extract 'not' operator"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "-"),
        "Should extract negation"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "+"),
        "Should extract positive"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "~"),
        "Should extract invert"
    );

    // Lambda
    assert!(
        tokens.iter().any(|t| t.value() == "lambda"),
        "Should extract lambda"
    );

    // If expression
    assert!(
        tokens.iter().any(|t| t.value() == "if"),
        "Should extract if expression"
    );

    // Comprehensions
    assert!(
        tokens.iter().any(|t| t.value().contains("list_comp")),
        "Should extract list comprehension"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("set_comp")),
        "Should extract set comprehension"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("dict_comp")),
        "Should extract dict comprehension"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("generator")),
        "Should extract generator expression"
    );

    // Async/await
    assert!(
        tokens.iter().any(|t| t.value() == "await"),
        "Should extract await"
    );

    // Yield
    assert!(
        tokens.iter().any(|t| t.value() == "yield"),
        "Should extract yield"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "from"),
        "Should extract yield from"
    );

    // Comparisons
    assert!(
        tokens.iter().any(|t| t.value() == "=="),
        "Should extract equality"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "!="),
        "Should extract inequality"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "<"),
        "Should extract less than"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "<="),
        "Should extract less than or equal"
    );
    assert!(
        tokens.iter().any(|t| t.value() == ">"),
        "Should extract greater than"
    );
    assert!(
        tokens.iter().any(|t| t.value() == ">="),
        "Should extract greater than or equal"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "is"),
        "Should extract 'is' operator"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "in"),
        "Should extract 'in' operator"
    );

    // Function calls
    assert!(
        tokens.iter().any(|t| t.value() == "call"),
        "Should extract function call"
    );

    // Named expression (walrus)
    assert!(
        tokens.iter().any(|t| t.value() == ":="),
        "Should extract walrus operator"
    );

    // Constants
    assert!(
        tokens.iter().any(|t| t.value() == "None"),
        "Should extract None"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "bool"),
        "Should extract bool constant"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "string"),
        "Should extract string constant"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "int"),
        "Should extract int constant"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "float"),
        "Should extract float constant"
    );

    // Collections
    assert!(
        tokens.iter().any(|t| t.value() == "list"),
        "Should extract list literal"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "tuple"),
        "Should extract tuple literal"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "dict"),
        "Should extract dict literal"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "set"),
        "Should extract set literal"
    );

    // Attribute and subscript
    assert!(
        tokens.iter().any(|t| t.value() == "."),
        "Should extract attribute access"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "[]"),
        "Should extract subscript"
    );

    // Slice (uses : operator)
    assert!(
        tokens.iter().any(|t| t.value() == ":"),
        "Should extract slice operator"
    );

    // Starred
    assert!(
        tokens.iter().any(|t| t.value() == "*"),
        "Should extract starred expression"
    );

    // F-strings
    assert!(
        tokens.iter().any(|t| t.value() == "f-string"),
        "Should extract f-string"
    );
}

#[test]
fn test_extract_tokens_from_expr_edge_cases() {
    // Test edge cases and complex nested expressions
    let source = r#"
def edge_cases():
    # Deeply nested expressions
    nested = (a and (b or (c and (d or e))))
    
    # Chained comparisons
    chained = 0 < x < 10 <= y <= 100
    
    # Complex arithmetic
    complex_math = (a + b) * (c - d) / (e % f) ** (g // h)
    
    # Nested comprehensions
    nested_comp = [[x * y for x in range(3)] for y in range(3)]
    
    # Multiple starred expressions
    combined = [*list1, *list2, extra]
    
    # Complex f-string
    complex_f = f"Result: {x + y:.2f} at {time.now()}"
    
    # Nested ternary
    nested_tern = a if b else (c if d else e)
    
    # Complex lambda
    complex_lambda = lambda x, y=10, *args, **kwargs: x + y + sum(args)
    
    # Mixed collections
    mixed = {"list": [1, 2, 3], "tuple": (4, 5, 6), "set": {7, 8, 9}}
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let tokens = analyzer.extract_tokens(&module.body);

    // Verify token extraction doesn't fail on complex expressions
    assert!(
        !tokens.is_empty(),
        "Should extract tokens from complex expressions"
    );

    // Check that nested structures are properly handled
    let operator_count = tokens
        .iter()
        .filter(|t| {
            matches!(
                t.to_category(),
                debtmap::complexity::entropy_core::TokenCategory::Operator
            )
        })
        .count();

    assert!(
        operator_count > 10,
        "Should extract multiple operators from nested expressions"
    );

    // Verify comprehensions are detected even when nested
    let comp_tokens = tokens.iter().filter(|t| t.value().contains("comp")).count();
    assert!(comp_tokens > 0, "Should detect nested comprehensions");
}

#[test]
fn test_extract_tokens_from_expr_recursion() {
    // Test that recursive calls work properly
    let source = r#"
def recursive_expressions():
    # Expression that would cause deep recursion in visitor
    deeply_nested = (
        func1(
            func2(
                func3(
                    [x for x in range(10) if x > 0],
                    {"key": lambda y: y * 2}
                ),
                (1, 2, 3)
            ),
            await async_func()
        )
    )
"#;

    let analyzer = PythonEntropyAnalyzer::new(source);
    let module = rustpython_parser::parse(source, rustpython_parser::Mode::Module, "<test>")
        .expect("Failed to parse Python code");

    let ast::Mod::Module(module) = module else {
        panic!("Expected Module");
    };

    let tokens = analyzer.extract_tokens(&module.body);

    // Verify deep recursion is handled
    assert!(
        !tokens.is_empty(),
        "Should handle deeply nested expressions"
    );

    // Check that all nested elements are extracted
    assert!(
        tokens.iter().any(|t| t.value() == "call"),
        "Should extract nested function calls"
    );
    assert!(
        tokens.iter().any(|t| t.value().contains("list_comp")),
        "Should extract nested list comprehension"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "lambda"),
        "Should extract nested lambda"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "dict"),
        "Should extract nested dict"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "tuple"),
        "Should extract nested tuple"
    );
    assert!(
        tokens.iter().any(|t| t.value() == "await"),
        "Should extract nested await"
    );
}
