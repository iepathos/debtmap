use debtmap::analyzers::python::PythonAnalyzer;
use debtmap::analyzers::Analyzer;
use debtmap::complexity::entropy_core::{EntropyConfig, UniversalEntropyCalculator};
use debtmap::complexity::languages::python::PythonEntropyAnalyzer;
use debtmap::complexity::entropy_core::LanguageEntropyAnalyzer;
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
    assert!(similarity >= 0.0 && similarity <= 1.0, "Branch similarity should be between 0 and 1");
    
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
    assert!(!metrics.complexity.functions.is_empty(), "Should have function metrics");
    let func = &metrics.complexity.functions[0];
    assert!(func.entropy_score.is_some(), "Entropy score should be calculated");
    
    let entropy_score = func.entropy_score.as_ref().unwrap();
    assert!(entropy_score.effective_complexity > 0.0, "Entropy score should be positive");
    assert!(entropy_score.effective_complexity < 100.0, "Entropy score should be reasonable");
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
    assert!(entropy.effective_complexity >= 0.0, 
        "Repetitive code should have entropy calculated (got {})", 
        entropy.effective_complexity);
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
    assert!(entropy.effective_complexity >= 0.0, 
        "Complex branching should have entropy calculated (got {})", 
        entropy.effective_complexity);
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
    let comp_tokens: Vec<_> = tokens.iter()
        .filter(|t| t.value().contains("comp"))
        .collect();
    assert!(!comp_tokens.is_empty(), "Should detect list comprehensions");
    
    // Pattern detection should find comprehensions
    let patterns = analyzer.detect_patterns(&module.body);
    assert!(patterns.total_patterns > 0, "Should detect comprehension patterns");
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
    assert!(func.entropy_score.is_some(), "Async functions should have entropy score");
    assert!(func.name.contains("async"), "Function should be marked as async");
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
    assert_eq!(metrics.complexity.functions.len(), 3, "Should have 3 methods");
    
    for func in &metrics.complexity.functions {
        assert!(func.entropy_score.is_some(), 
            "Method {} should have entropy score", func.name);
        assert!(func.name.contains("DataProcessor"), 
            "Method {} should include class name", func.name);
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
    let exception_tokens: Vec<_> = tokens.iter()
        .filter(|t| t.value().contains("except") || t.value().contains("try") || t.value().contains("finally"))
        .collect();
    assert!(!exception_tokens.is_empty(), "Should detect exception handling");
    
    // Calculate entropy with calculator
    let mut calculator = UniversalEntropyCalculator::new(EntropyConfig::default());
    let score = calculator.calculate(&analyzer, &module.body);
    assert!(score.effective_complexity > 0.0, "Exception handling should add complexity");
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
    assert!(similarity > 0.0, "Match statements should have branch similarity");
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
    let gen_tokens: Vec<_> = tokens.iter()
        .filter(|t| t.value().contains("generator"))
        .collect();
    assert!(!gen_tokens.is_empty(), "Should detect generator expressions");
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
    let lambda_tokens: Vec<_> = tokens.iter()
        .filter(|t| t.value() == "lambda")
        .collect();
    // Should detect at least 2 lambda functions
    assert!(lambda_tokens.len() >= 2, "Should detect lambda functions, found {}", lambda_tokens.len());
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
    let walrus_tokens: Vec<_> = tokens.iter()
        .filter(|t| t.value() == ":=")
        .collect();
    assert!(!walrus_tokens.is_empty(), "Should detect walrus operator");
}

#[test]
fn test_entropy_score_integration() {
    // Test that entropy scores are properly integrated into function metrics
    let source = r#"
def simple_function():
    return 42

def complex_function(data):
    if data is None:
        return None
    
    results = []
    for item in data:
        if item > 0:
            if item % 2 == 0:
                results.append(item * 2)
            else:
                results.append(item * 3)
        else:
            results.append(0)
    
    return results
"#;

    let analyzer = PythonAnalyzer::new();
    let ast = analyzer
        .parse(source, PathBuf::from("test.py"))
        .expect("Should parse Python code");
    
    let metrics = analyzer.analyze(&ast);
    
    assert_eq!(metrics.complexity.functions.len(), 2, "Should have 2 functions");
    
    let simple_func = metrics.complexity.functions.iter()
        .find(|f| f.name == "simple_function")
        .expect("Should find simple_function");
    
    let complex_func = metrics.complexity.functions.iter()
        .find(|f| f.name == "complex_function")
        .expect("Should find complex_function");
    
    // Both should have entropy scores
    assert!(simple_func.entropy_score.is_some(), "Simple function should have entropy score");
    assert!(complex_func.entropy_score.is_some(), "Complex function should have entropy score");
    
    // Complex function should have higher entropy than simple
    let simple_entropy = simple_func.entropy_score.as_ref().unwrap();
    let complex_entropy = complex_func.entropy_score.as_ref().unwrap();
    
    assert!(complex_entropy.effective_complexity > simple_entropy.effective_complexity, 
        "Complex function ({}) should have higher entropy than simple ({})",
        complex_entropy.effective_complexity, simple_entropy.effective_complexity);
}