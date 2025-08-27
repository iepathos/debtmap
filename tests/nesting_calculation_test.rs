use debtmap::analyzers::rust::RustAnalyzer;
use debtmap::Analyzer;
use std::path::PathBuf;

#[test]
fn test_rust_nesting_calculation() {
    // Create a test file with nested control structures
    let code = r#"
fn test_function() {
    // No nesting here
    let x = 1;
    
    // Nesting level 1: if statement
    if x > 0 {
        // Nesting level 2: for loop
        for i in 0..10 {
            // Nesting level 3: while loop
            let mut j = 0;
            while j < 5 {
                // Nesting level 4: match statement
                match j {
                    0 => {
                        // Nesting level 5: if inside match arm
                        if i > 5 {
                            println!("Deep nesting!");
                        }
                    },
                    _ => {}
                }
                j += 1;
            }
        }
    }
}

fn simple_function() {
    // No nesting
    let x = 1;
    let y = 2;
    let z = x + y;
}

fn single_if_function() {
    let x = 1;
    // Nesting level 1
    if x > 0 {
        println!("x is positive");
    }
}

fn nested_loops() {
    // Nesting level 1
    for i in 0..10 {
        // Nesting level 2
        for j in 0..10 {
            // Nesting level 3
            for k in 0..10 {
                println!("{} {} {}", i, j, k);
            }
        }
    }
}
"#;

    let analyzer = RustAnalyzer::new();
    let path = PathBuf::from("test.rs");
    let ast = analyzer.parse(code, path.clone()).unwrap();
    let file_metrics = analyzer.analyze(&ast);

    // Find each function and check its nesting depth
    let functions = &file_metrics.complexity.functions;

    // Debug print all functions and their nesting values
    for func in functions {
        println!("Function {} has nesting depth {}", func.name, func.nesting);
    }

    // Find test_function - should have nesting depth of 5
    let test_fn = functions
        .iter()
        .find(|f| f.name == "test_function")
        .unwrap();
    assert_eq!(
        test_fn.nesting, 5,
        "test_function should have nesting depth of 5, got {}",
        test_fn.nesting
    );

    // Find simple_function - should have nesting depth of 0
    let simple_fn = functions
        .iter()
        .find(|f| f.name == "simple_function")
        .unwrap();
    assert_eq!(
        simple_fn.nesting, 0,
        "simple_function should have nesting depth of 0, got {}",
        simple_fn.nesting
    );

    // Find single_if_function - should have nesting depth of 1
    let single_if_fn = functions
        .iter()
        .find(|f| f.name == "single_if_function")
        .unwrap();
    assert_eq!(
        single_if_fn.nesting, 1,
        "single_if_function should have nesting depth of 1, got {}",
        single_if_fn.nesting
    );

    // Find nested_loops - should have nesting depth of 3
    let nested_loops_fn = functions.iter().find(|f| f.name == "nested_loops").unwrap();
    assert_eq!(
        nested_loops_fn.nesting, 3,
        "nested_loops should have nesting depth of 3, got {}",
        nested_loops_fn.nesting
    );
}
