use debtmap::analyzers::python::PythonAnalyzer;
use debtmap::analyzers::rust::RustAnalyzer;
use debtmap::analyzers::Analyzer;
use debtmap::core::{ast::Ast, Language};
use std::path::PathBuf;

#[cfg(test)]
mod rust_analyzer_tests {
    use super::*;

    #[test]
    fn test_rust_analyzer_default() {
        let analyzer1 = RustAnalyzer::default();
        let analyzer2 = RustAnalyzer::new();

        // Both should create identical analyzers
        // Test by parsing simple code
        let simple_code = r#"
            fn hello() {
                println!("Hello, world!");
            }
        "#;

        let ast1 = analyzer1
            .parse(simple_code, PathBuf::from("test1.rs"))
            .unwrap();
        let ast2 = analyzer2
            .parse(simple_code, PathBuf::from("test2.rs"))
            .unwrap();

        // Both should successfully parse
        assert!(matches!(ast1, Ast::Rust(_)));
        assert!(matches!(ast2, Ast::Rust(_)));
    }

    #[test]
    fn test_rust_analyzer_default_complexity_threshold() {
        let analyzer = RustAnalyzer::default();

        // Test with a moderately complex function
        let complex_code = r#"
            fn complex_function(x: i32) -> i32 {
                if x > 10 {
                    if x > 20 {
                        return x * 2;
                    } else {
                        return x + 10;
                    }
                } else if x > 5 {
                    return x - 5;
                } else {
                    return x;
                }
            }
        "#;

        let ast = analyzer
            .parse(complex_code, PathBuf::from("complex.rs"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        // Verify the analyzer works correctly
        assert_eq!(metrics.language, Language::Rust);
        assert!(!metrics.complexity.functions.is_empty());
    }

    #[test]
    fn test_rust_analyzer_default_empty_file() {
        let analyzer = RustAnalyzer::default();

        let empty_code = "";
        let ast = analyzer
            .parse(empty_code, PathBuf::from("empty.rs"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Rust);
        assert!(metrics.complexity.functions.is_empty());
        assert_eq!(metrics.complexity.cyclomatic_complexity, 0);
        assert_eq!(metrics.complexity.cognitive_complexity, 0);
    }

    #[test]
    fn test_rust_analyzer_default_with_structs() {
        let analyzer = RustAnalyzer::default();

        let struct_code = r#"
            struct Person {
                name: String,
                age: u32,
            }
            
            impl Person {
                fn new(name: String, age: u32) -> Self {
                    Self { name, age }
                }
                
                fn get_age(&self) -> u32 {
                    self.age
                }
            }
        "#;

        let ast = analyzer
            .parse(struct_code, PathBuf::from("struct.rs"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Rust);
        // Should detect the impl methods
        assert_eq!(metrics.complexity.functions.len(), 2);
        assert_eq!(metrics.complexity.functions[0].name, "new");
        assert_eq!(metrics.complexity.functions[1].name, "get_age");
    }

    #[test]
    fn test_rust_analyzer_default_with_tests() {
        let analyzer = RustAnalyzer::default();

        let test_code = r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }
            
            #[cfg(test)]
            mod tests {
                use super::*;
                
                #[test]
                fn test_add() {
                    assert_eq!(add(2, 2), 4);
                }
            }
        "#;

        let ast = analyzer
            .parse(test_code, PathBuf::from("with_tests.rs"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Rust);
        // Should analyze all functions including test functions
        assert!(metrics.complexity.functions.iter().any(|f| f.name == "add"));
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "test_add"));

        // Verify test function is marked as test
        let test_fn = metrics
            .complexity
            .functions
            .iter()
            .find(|f| f.name == "test_add")
            .unwrap();
        assert!(test_fn.is_test, "test_add should be marked as a test");

        // Verify non-test function is not marked as test
        let add_fn = metrics
            .complexity
            .functions
            .iter()
            .find(|f| f.name == "add")
            .unwrap();
        assert!(!add_fn.is_test, "add should not be marked as a test");
    }

    #[test]
    fn test_rust_analyzer_tokio_test_detection() {
        let analyzer = RustAnalyzer::default();

        let test_code = r#"
            #[tokio::test]
            async fn test_async_function() {
                assert!(true);
            }
            
            #[test]
            fn test_regular_test() {
                assert!(true);
            }
            
            fn test_prefixed_function() {
                // Should be detected as test by name
            }
            
            fn it_should_work() {
                // Should be detected as test by name pattern
            }
            
            fn regular_function() {
                // Should NOT be detected as test
            }
        "#;

        let ast = analyzer
            .parse(test_code, PathBuf::from("tokio_tests.rs"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        // Check that tokio::test is detected
        let async_test = metrics
            .complexity
            .functions
            .iter()
            .find(|f| f.name == "test_async_function")
            .unwrap();
        assert!(
            async_test.is_test,
            "tokio::test function should be marked as test"
        );

        // Check that regular #[test] is detected
        let regular_test = metrics
            .complexity
            .functions
            .iter()
            .find(|f| f.name == "test_regular_test")
            .unwrap();
        assert!(
            regular_test.is_test,
            "regular test function should be marked as test"
        );

        // Check that test_ prefix is detected
        let prefixed = metrics
            .complexity
            .functions
            .iter()
            .find(|f| f.name == "test_prefixed_function")
            .unwrap();
        assert!(
            prefixed.is_test,
            "test_ prefixed function should be marked as test"
        );

        // Check that it_ prefix is detected
        let it_fn = metrics
            .complexity
            .functions
            .iter()
            .find(|f| f.name == "it_should_work")
            .unwrap();
        assert!(
            it_fn.is_test,
            "it_ prefixed function should be marked as test"
        );

        // Check that regular function is NOT detected as test
        let regular = metrics
            .complexity
            .functions
            .iter()
            .find(|f| f.name == "regular_function")
            .unwrap();
        assert!(
            !regular.is_test,
            "regular function should not be marked as test"
        );
    }

    #[test]
    fn test_rust_analyzer_test_file_detection() {
        let analyzer = RustAnalyzer::default();

        let code = r#"
            fn setup_test_data() {
                // Helper function in a test file
            }
            
            fn validate_state() {
                // Another helper in test file
            }
            
            fn test_something() {
                setup_test_data();
                validate_state();
            }
        "#;

        // Test with various test file paths
        let test_paths = vec![
            "src/tests/mod.rs",
            "src/test/helpers.rs",
            "src/module_test.rs",
            "tests/integration_tests.rs",
            "src/test_utils.rs",
        ];

        for path in test_paths {
            let ast = analyzer.parse(code, PathBuf::from(path)).unwrap();
            let metrics = analyzer.analyze(&ast);

            // In test files, only functions with test names/attributes should be marked as tests
            for func in &metrics.complexity.functions {
                if func.name.starts_with("test_") {
                    assert!(
                        func.is_test,
                        "test_ prefixed function '{}' in test file '{}' should be marked as test",
                        func.name, path
                    );
                } else {
                    // Helper functions in test files are not tests themselves
                    assert!(
                        !func.is_test,
                        "Helper function '{}' in test file '{}' should not be marked as test",
                        func.name, path
                    );
                }
            }
        }

        // Test with non-test file path
        let ast = analyzer.parse(code, PathBuf::from("src/lib.rs")).unwrap();
        let metrics = analyzer.analyze(&ast);

        // Only test_ prefixed function should be marked as test in non-test files
        for func in &metrics.complexity.functions {
            if func.name.starts_with("test_") {
                assert!(
                    func.is_test,
                    "test_ prefixed function should be marked as test"
                );
            } else {
                assert!(
                    !func.is_test,
                    "Function '{}' should not be marked as test",
                    func.name
                );
            }
        }
    }

    #[test]
    fn test_rust_analyzer_default_with_macros() {
        let analyzer = RustAnalyzer::default();

        let macro_code = r#"
            macro_rules! say_hello {
                () => {
                    println!("Hello!");
                };
            }
            
            fn use_macro() {
                say_hello!();
            }
        "#;

        let ast = analyzer
            .parse(macro_code, PathBuf::from("macros.rs"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Rust);
        assert_eq!(metrics.complexity.functions.len(), 1);
        assert_eq!(metrics.complexity.functions[0].name, "use_macro");
    }

    #[test]
    fn test_rust_analyzer_default_error_handling() {
        let analyzer = RustAnalyzer::default();

        let invalid_code = "fn broken(";
        let result = analyzer.parse(invalid_code, PathBuf::from("invalid.rs"));

        // Should return an error for invalid syntax
        assert!(result.is_err());
    }

    #[test]
    fn test_rust_analyzer_default_multiple_instances() {
        // Test that multiple default instances work independently
        let analyzer1 = RustAnalyzer::default();
        let analyzer2 = RustAnalyzer::default();

        let code1 = "fn func1() { }";
        let code2 = "fn func2() { let x = 5; }";

        let ast1 = analyzer1.parse(code1, PathBuf::from("file1.rs")).unwrap();
        let ast2 = analyzer2.parse(code2, PathBuf::from("file2.rs")).unwrap();

        let metrics1 = analyzer1.analyze(&ast1);
        let metrics2 = analyzer2.analyze(&ast2);

        assert_eq!(metrics1.complexity.functions[0].name, "func1");
        assert_eq!(metrics2.complexity.functions[0].name, "func2");
    }
}

#[cfg(test)]
mod python_analyzer_tests {
    use super::*;

    #[test]
    fn test_python_analyzer_default() {
        let analyzer1 = PythonAnalyzer::default();
        let analyzer2 = PythonAnalyzer::new();

        // Both should create identical analyzers
        // Test by parsing simple code
        let simple_code = r#"
def hello():
    print("Hello, world!")
        "#;

        let ast1 = analyzer1
            .parse(simple_code, PathBuf::from("test1.py"))
            .unwrap();
        let ast2 = analyzer2
            .parse(simple_code, PathBuf::from("test2.py"))
            .unwrap();

        // Both should successfully parse
        assert!(matches!(ast1, Ast::Python(_)));
        assert!(matches!(ast2, Ast::Python(_)));
    }

    #[test]
    fn test_python_analyzer_default_complexity_threshold() {
        let analyzer = PythonAnalyzer::default();

        // Test with a moderately complex function
        let complex_code = r#"
def complex_function(x):
    if x > 10:
        if x > 20:
            return x * 2
        else:
            return x + 10
    elif x > 5:
        return x - 5
    else:
        return x
        "#;

        let ast = analyzer
            .parse(complex_code, PathBuf::from("complex.py"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        // Verify the analyzer works correctly
        assert_eq!(metrics.language, Language::Python);
        assert!(!metrics.complexity.functions.is_empty());
    }

    #[test]
    fn test_python_analyzer_default_empty_file() {
        let analyzer = PythonAnalyzer::default();

        let empty_code = "";
        let ast = analyzer
            .parse(empty_code, PathBuf::from("empty.py"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Python);
        assert!(metrics.complexity.functions.is_empty());
        assert_eq!(metrics.complexity.cyclomatic_complexity, 0);
        assert_eq!(metrics.complexity.cognitive_complexity, 0);
    }

    #[test]
    fn test_python_analyzer_default_with_classes() {
        let analyzer = PythonAnalyzer::default();

        let class_code = r#"
class Person:
    def __init__(self, name, age):
        self.name = name
        self.age = age
    
    def get_age(self):
        return self.age
    
    def set_age(self, age):
        if age >= 0:
            self.age = age
        "#;

        let ast = analyzer
            .parse(class_code, PathBuf::from("class.py"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Python);
        // Should detect the class methods
        assert_eq!(metrics.complexity.functions.len(), 3);
        assert_eq!(metrics.complexity.functions[0].name, "__init__");
        assert_eq!(metrics.complexity.functions[1].name, "get_age");
        assert_eq!(metrics.complexity.functions[2].name, "set_age");
    }

    #[test]
    fn test_python_analyzer_default_with_tests() {
        let analyzer = PythonAnalyzer::default();

        let test_code = r#"
def add(a, b):
    return a + b

def test_add():
    assert add(2, 2) == 4
    assert add(-1, 1) == 0

def test_add_negative():
    result = add(-5, -3)
    assert result == -8
        "#;

        let ast = analyzer
            .parse(test_code, PathBuf::from("with_tests.py"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Python);
        // Should analyze all functions including test functions
        assert!(metrics.complexity.functions.iter().any(|f| f.name == "add"));
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "test_add"));
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "test_add_negative"));
    }

    #[test]
    fn test_python_analyzer_default_with_decorators() {
        let analyzer = PythonAnalyzer::default();

        let decorator_code = r#"
def decorator(func):
    def wrapper(*args, **kwargs):
        print("Before")
        result = func(*args, **kwargs)
        print("After")
        return result
    return wrapper

@decorator
def greet(name):
    return f"Hello, {name}!"
        "#;

        let ast = analyzer
            .parse(decorator_code, PathBuf::from("decorators.py"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Python);
        assert_eq!(metrics.complexity.functions.len(), 3);
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "decorator"));
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "wrapper"));
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "greet"));
    }

    #[test]
    fn test_python_analyzer_default_async_functions() {
        let analyzer = PythonAnalyzer::default();

        let async_code = r#"
async def fetch_data(url):
    response = await get(url)
    return response

async def process_data():
    data = await fetch_data("https://api.example.com")
    if data:
        return data.upper()
    return ""
        "#;

        let ast = analyzer
            .parse(async_code, PathBuf::from("async.py"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Python);
        assert_eq!(metrics.complexity.functions.len(), 2);
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "async fetch_data"));
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "async process_data"));
    }

    #[test]
    fn test_python_analyzer_default_error_handling() {
        let analyzer = PythonAnalyzer::default();

        let invalid_code = "def broken(";
        let result = analyzer.parse(invalid_code, PathBuf::from("invalid.py"));

        // Should return an error for invalid syntax
        assert!(result.is_err());
    }

    #[test]
    fn test_python_analyzer_default_multiple_instances() {
        // Test that multiple default instances work independently
        let analyzer1 = PythonAnalyzer::default();
        let analyzer2 = PythonAnalyzer::default();

        let code1 = "def func1():\n    pass";
        let code2 = "def func2():\n    x = 5\n    return x";

        let ast1 = analyzer1.parse(code1, PathBuf::from("file1.py")).unwrap();
        let ast2 = analyzer2.parse(code2, PathBuf::from("file2.py")).unwrap();

        let metrics1 = analyzer1.analyze(&ast1);
        let metrics2 = analyzer2.analyze(&ast2);

        assert_eq!(metrics1.complexity.functions[0].name, "func1");
        assert_eq!(metrics2.complexity.functions[0].name, "func2");
    }

    #[test]
    fn test_python_analyzer_default_comprehensions() {
        let analyzer = PythonAnalyzer::default();

        let comprehension_code = r#"
def process_list(items):
    squares = [x**2 for x in items if x > 0]
    evens = {x for x in items if x % 2 == 0}
    pairs = {x: x**2 for x in items}
    return squares, evens, pairs
        "#;

        let ast = analyzer
            .parse(comprehension_code, PathBuf::from("comprehensions.py"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Python);
        assert_eq!(metrics.complexity.functions.len(), 1);
        assert_eq!(metrics.complexity.functions[0].name, "process_list");
        // Function with comprehensions should be detected
        assert_eq!(metrics.complexity.functions[0].cyclomatic, 1);
    }

    #[test]
    fn test_python_analyzer_default_nested_functions() {
        let analyzer = PythonAnalyzer::default();

        let nested_code = r#"
def outer_function(x):
    def inner_function(y):
        def innermost(z):
            return x + y + z
        return innermost(10)
    return inner_function(5)
        "#;

        let ast = analyzer
            .parse(nested_code, PathBuf::from("nested.py"))
            .unwrap();
        let metrics = analyzer.analyze(&ast);

        assert_eq!(metrics.language, Language::Python);
        // Should detect all nested functions
        assert_eq!(metrics.complexity.functions.len(), 3);
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "outer_function"));
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "inner_function"));
        assert!(metrics
            .complexity
            .functions
            .iter()
            .any(|f| f.name == "innermost"));
    }
}
