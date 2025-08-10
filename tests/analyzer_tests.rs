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
