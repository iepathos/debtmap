use debtmap::analyzers::purity_detector::{ImpurityReason, PurityDetector};
use debtmap::analyzers::rust::RustAnalyzer;
use debtmap::analyzers::Analyzer;
use debtmap::core::ast::Ast;
use std::path::PathBuf;

#[test]
fn test_pure_function_detection() {
    let code = r#"
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }
        
        fn multiply(x: f64, y: f64) -> f64 {
            x * y
        }
        
        fn calculate_area(width: f64, height: f64) -> f64 {
            width * height
        }
    "#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // All functions should be detected as pure
    for func in &metrics.complexity.functions {
        assert_eq!(
            func.is_pure,
            Some(true),
            "Function {} should be pure",
            func.name
        );
        assert!(
            func.purity_confidence.unwrap() > 0.9,
            "Function {} should have high purity confidence",
            func.name
        );
    }
}

#[test]
fn test_impure_function_with_print() {
    let code = r#"
        fn debug_add(a: i32, b: i32) -> i32 {
            println!("Adding {} + {}", a, b);
            a + b
        }
        
        fn log_and_multiply(x: f64, y: f64) -> f64 {
            eprintln!("Multiplying {} * {}", x, y);
            x * y
        }
    "#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // All functions should be detected as impure due to I/O
    for func in &metrics.complexity.functions {
        assert_eq!(
            func.is_pure,
            Some(false),
            "Function {} should be impure",
            func.name
        );
    }
}

#[test]
fn test_impure_function_with_mutable_params() {
    let code = r#"
        fn increment(x: &mut i32) {
            *x += 1;
        }
        
        fn swap(a: &mut i32, b: &mut i32) {
            let temp = *a;
            *a = *b;
            *b = temp;
        }
    "#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // All functions should be detected as impure due to mutable parameters
    for func in &metrics.complexity.functions {
        assert_eq!(
            func.is_pure,
            Some(false),
            "Function {} should be impure",
            func.name
        );
    }
}

#[test]
fn test_impure_function_with_file_io() {
    let code = r#"
        use std::fs::File;
        use std::io::Write;
        
        fn write_to_file(content: &str) -> std::io::Result<()> {
            let mut file = File::create("output.txt")?;
            file.write_all(content.as_bytes())?;
            Ok(())
        }
    "#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Function should be impure due to file I/O
    assert_eq!(metrics.complexity.functions[0].is_pure, Some(false));
}

#[test]
fn test_impure_function_with_unsafe() {
    let code = r#"
        fn dangerous() -> i32 {
            unsafe {
                std::ptr::null::<i32>().read()
            }
        }
    "#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Function should be impure due to unsafe block
    assert_eq!(metrics.complexity.functions[0].is_pure, Some(false));
}

#[test]
fn test_pure_function_with_complex_logic() {
    let code = r#"
        fn fibonacci(n: u32) -> u32 {
            match n {
                0 => 0,
                1 => 1,
                n => fibonacci(n - 1) + fibonacci(n - 2),
            }
        }
        
        fn factorial(n: u32) -> u32 {
            if n <= 1 {
                1
            } else {
                n * factorial(n - 1)
            }
        }
    "#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Recursive functions should still be pure if they don't have side effects
    for func in &metrics.complexity.functions {
        assert_eq!(
            func.is_pure,
            Some(true),
            "Function {} should be pure",
            func.name
        );
    }
}

#[test]
fn test_function_with_vec_mutations() {
    let code = r#"
        fn process_data(data: Vec<i32>) -> Vec<i32> {
            let mut result = data;
            result.sort();
            result.dedup();
            result
        }
    "#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Function mutates local data but doesn't have external side effects
    // This could be considered pure from a functional perspective (takes input, returns output)
    // but our detector might flag it as impure due to mutations
    let func = &metrics.complexity.functions[0];
    println!("Function {} purity: {:?}", func.name, func.is_pure);
}

#[test]
fn test_closure_purity() {
    let code = r#"
        fn create_adder(x: i32) -> impl Fn(i32) -> i32 {
            move |y| x + y
        }
        
        fn apply_twice<F>(f: F, x: i32) -> i32
        where
            F: Fn(i32) -> i32,
        {
            f(f(x))
        }
    "#;

    let analyzer = RustAnalyzer::new();
    let ast = analyzer.parse(code, PathBuf::from("test.rs")).unwrap();
    let metrics = analyzer.analyze(&ast);

    // Higher-order functions should be pure if they don't have side effects
    for func in &metrics.complexity.functions {
        assert_eq!(
            func.is_pure,
            Some(true),
            "Function {} should be pure",
            func.name
        );
    }
}
