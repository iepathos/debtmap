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
        
        let ast1 = analyzer1.parse(simple_code, PathBuf::from("test1.rs")).unwrap();
        let ast2 = analyzer2.parse(simple_code, PathBuf::from("test2.rs")).unwrap();
        
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
        
        let ast = analyzer.parse(complex_code, PathBuf::from("complex.rs")).unwrap();
        let metrics = analyzer.analyze(&ast);
        
        // Verify the analyzer works correctly
        assert_eq!(metrics.language, Language::Rust);
        assert!(!metrics.complexity.functions.is_empty());
    }

    #[test]
    fn test_rust_analyzer_default_empty_file() {
        let analyzer = RustAnalyzer::default();
        
        let empty_code = "";
        let ast = analyzer.parse(empty_code, PathBuf::from("empty.rs")).unwrap();
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
        
        let ast = analyzer.parse(struct_code, PathBuf::from("struct.rs")).unwrap();
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
        
        let ast = analyzer.parse(test_code, PathBuf::from("with_tests.rs")).unwrap();
        let metrics = analyzer.analyze(&ast);
        
        assert_eq!(metrics.language, Language::Rust);
        // Should analyze all functions including test functions
        assert!(metrics.complexity.functions.iter().any(|f| f.name == "add"));
        assert!(metrics.complexity.functions.iter().any(|f| f.name == "test_add"));
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
        
        let ast = analyzer.parse(macro_code, PathBuf::from("macros.rs")).unwrap();
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