use debtmap::risk::lcov::parse_lcov_file;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_parse_lcov_empty_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("empty.lcov");
    fs::write(&file_path, "").unwrap();

    let result = parse_lcov_file(&file_path).unwrap();
    assert!(result.functions.is_empty());
}

#[test]
fn test_parse_lcov_single_file() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.lcov");

    let lcov_content = r#"SF:src/main.rs
FN:10,main
FN:20,helper_function
FNDA:1,main
FNDA:5,helper_function
end_of_record
"#;

    fs::write(&file_path, lcov_content).unwrap();

    let result = parse_lcov_file(&file_path).unwrap();
    assert_eq!(result.functions.len(), 1);

    let functions = result.functions.get(Path::new("src/main.rs")).unwrap();
    assert_eq!(functions.len(), 2);

    let main_fn = functions.iter().find(|f| f.name == "main").unwrap();
    assert_eq!(main_fn.start_line, 10);
    assert_eq!(main_fn.execution_count, 1);
    assert_eq!(main_fn.coverage_percentage, 100.0);

    let helper_fn = functions
        .iter()
        .find(|f| f.name == "helper_function")
        .unwrap();
    assert_eq!(helper_fn.start_line, 20);
    assert_eq!(helper_fn.execution_count, 5);
    assert_eq!(helper_fn.coverage_percentage, 100.0);
}

#[test]
fn test_parse_lcov_uncovered_functions() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.lcov");

    let lcov_content = r#"SF:src/lib.rs
FN:15,uncovered_function
FN:25,covered_function
FNDA:0,uncovered_function
FNDA:10,covered_function
end_of_record
"#;

    fs::write(&file_path, lcov_content).unwrap();

    let result = parse_lcov_file(&file_path).unwrap();
    let functions = result.functions.get(Path::new("src/lib.rs")).unwrap();

    let uncovered = functions
        .iter()
        .find(|f| f.name == "uncovered_function")
        .unwrap();
    assert_eq!(uncovered.execution_count, 0);
    assert_eq!(uncovered.coverage_percentage, 0.0);

    let covered = functions
        .iter()
        .find(|f| f.name == "covered_function")
        .unwrap();
    assert_eq!(covered.execution_count, 10);
    assert_eq!(covered.coverage_percentage, 100.0);
}

#[test]
fn test_parse_lcov_multiple_files() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.lcov");

    let lcov_content = r#"SF:src/main.rs
FN:10,main
FNDA:1,main
end_of_record
SF:src/lib.rs
FN:5,lib_function
FNDA:3,lib_function
end_of_record
"#;

    fs::write(&file_path, lcov_content).unwrap();

    let result = parse_lcov_file(&file_path).unwrap();
    assert_eq!(result.functions.len(), 2);

    assert!(result.functions.contains_key(Path::new("src/main.rs")));
    assert!(result.functions.contains_key(Path::new("src/lib.rs")));
}

#[test]
fn test_get_function_coverage() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.lcov");

    let lcov_content = r#"SF:src/main.rs
FN:10,main
FN:20,test_function
FNDA:1,main
FNDA:0,test_function
end_of_record
"#;

    fs::write(&file_path, lcov_content).unwrap();

    let result = parse_lcov_file(&file_path).unwrap();

    let coverage = result.get_function_coverage(Path::new("src/main.rs"), "main");
    assert_eq!(coverage, Some(1.0)); // Coverage is returned as fraction (0-1)

    let coverage = result.get_function_coverage(Path::new("src/main.rs"), "test_function");
    assert_eq!(coverage, Some(0.0)); // 0% coverage

    let coverage = result.get_function_coverage(Path::new("src/main.rs"), "nonexistent");
    assert_eq!(coverage, None);
}

#[test]
fn test_get_file_coverage() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.lcov");

    let lcov_content = r#"SF:src/main.rs
FN:10,function1
FN:20,function2
FN:30,function3
FNDA:1,function1
FNDA:0,function2
FNDA:5,function3
end_of_record
"#;

    fs::write(&file_path, lcov_content).unwrap();

    let result = parse_lcov_file(&file_path).unwrap();

    let coverage = result.get_file_coverage(Path::new("src/main.rs"));
    // 2 out of 3 functions are covered
    // Use approximate comparison for floating point
    assert!(coverage.is_some());
    assert!((coverage.unwrap() - 0.6666666666666666).abs() < 0.0000001);

    let coverage = result.get_file_coverage(Path::new("nonexistent.rs"));
    assert_eq!(coverage, None);
}

#[test]
fn test_parse_lcov_malformed_lines() {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.lcov");

    // The lcov crate is strict and will reject malformed LCOV data
    let lcov_content = r#"SF:src/main.rs
FN:not_a_number,function1
FN:10,valid_function
FNDA:also_not_a_number,function1
FNDA:5,valid_function
Some random text
FN:20,another_function
FNDA:3,another_function
end_of_record
"#;

    fs::write(&file_path, lcov_content).unwrap();

    // The lcov crate will fail on malformed input
    let result = parse_lcov_file(&file_path);
    assert!(result.is_err(), "Should fail on malformed LCOV data");
    
    // Test with valid LCOV instead
    let valid_lcov = r#"SF:src/main.rs
FN:10,valid_function
FN:20,another_function
FNDA:5,valid_function
FNDA:3,another_function
end_of_record
"#;
    
    fs::write(&file_path, valid_lcov).unwrap();
    let result = parse_lcov_file(&file_path).unwrap();
    let functions = result.functions.get(Path::new("src/main.rs")).unwrap();
    
    assert_eq!(functions.len(), 2);
    assert!(functions.iter().any(|f| f.name == "valid_function"));
    assert!(functions.iter().any(|f| f.name == "another_function"));
}
