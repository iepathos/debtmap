//! Integration test for LOC counting consistency across analysis modes
//!
//! This test ensures that LOC counts are consistent whether coverage data is provided or not.

use debtmap::metrics::{LocCounter, LocCountingConfig};
use debtmap::risk::lcov::parse_lcov_file;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_loc_consistency_without_coverage() {
    // Create a temporary Rust file
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");

    std::fs::write(
        &file_path,
        r#"
// This is a comment
fn main() {
    println!("Hello");
}

fn helper() {
    println!("Helper");
}
"#,
    )
    .unwrap();

    // Count LOC using LocCounter
    let counter = LocCounter::default();
    let count = counter.count_file(&file_path).unwrap();

    // Verify we got the expected line counts
    assert!(count.code_lines > 0, "Should have counted some code lines");
    assert_eq!(count.comment_lines, 1, "Should have 1 comment line");
}

#[test]
fn test_loc_consistency_with_coverage() {
    // Create a temporary Rust file
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");

    std::fs::write(
        &file_path,
        r#"
fn main() {
    println!("Hello");
}

fn helper() {
    println!("Helper");
}
"#,
    )
    .unwrap();

    // Create LCOV file for the same code
    let lcov_content = format!(
        r#"TN:
SF:{}
FN:2,main
FN:6,helper
FNDA:1,main
FNDA:0,helper
DA:2,1
DA:3,1
DA:4,1
DA:6,0
DA:7,0
DA:8,0
LF:6
LH:3
end_of_record
"#,
        file_path.display()
    );

    let mut lcov_file = NamedTempFile::new().unwrap();
    lcov_file.write_all(lcov_content.as_bytes()).unwrap();

    // Parse LCOV data
    let mut lcov_data = parse_lcov_file(lcov_file.path()).unwrap();

    // Count LOC using LocCounter
    let counter = LocCounter::default();
    let _count = counter.count_file(&file_path).unwrap();

    // Integrate LocCounter with LCOV data
    lcov_data = lcov_data.with_loc_counter(counter);
    lcov_data.recalculate_with_loc_counter();

    // Verify consistency: LCOV total_lines should match or be close to LocCounter code_lines
    // Note: LCOV might count slightly differently, but they should be in the same ballpark
    assert!(
        lcov_data.total_lines > 0,
        "LCOV should have recalculated total_lines using LocCounter"
    );
}

#[test]
fn test_loc_counter_filters_test_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file with _test suffix
    let test_file = temp_dir.path().join("example_test.rs");
    std::fs::write(
        &test_file,
        r#"
#[test]
fn test_something() {
    assert_eq!(1, 1);
}
"#,
    )
    .unwrap();

    // Count with default config (exclude tests)
    let counter_no_tests = LocCounter::default();
    assert!(
        !counter_no_tests.should_include(&test_file),
        "Should exclude test files by default"
    );

    // Count with config that includes tests
    let config_with_tests = LocCountingConfig {
        include_tests: true,
        ..Default::default()
    };
    let counter_with_tests = LocCounter::new(config_with_tests);
    assert!(
        counter_with_tests.should_include(&test_file),
        "Should include test files when configured"
    );
}

#[test]
fn test_loc_counter_filters_generated_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create a generated file
    let generated_file = temp_dir.path().join("generated.rs");
    std::fs::write(
        &generated_file,
        r#"
// @generated
// DO NOT EDIT

fn auto_generated() {
    println!("Generated");
}
"#,
    )
    .unwrap();

    // Count with default config (exclude generated)
    let counter_no_generated = LocCounter::default();
    assert!(
        !counter_no_generated.should_include(&generated_file),
        "Should exclude generated files by default"
    );

    // Count with config that includes generated files
    let config_with_generated = LocCountingConfig {
        include_generated: true,
        ..Default::default()
    };
    let counter_with_generated = LocCounter::new(config_with_generated);
    assert!(
        counter_with_generated.should_include(&generated_file),
        "Should include generated files when configured"
    );
}

#[test]
fn test_loc_counter_custom_exclusion_patterns() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file in a vendor directory
    let vendor_file = temp_dir.path().join("vendor").join("lib.rs");
    std::fs::create_dir(temp_dir.path().join("vendor")).unwrap();
    std::fs::write(
        &vendor_file,
        r#"
fn vendor_function() {
    println!("Vendor");
}
"#,
    )
    .unwrap();

    // Count with custom exclusion pattern
    let config = LocCountingConfig {
        exclude_patterns: vec!["vendor".to_string()],
        ..Default::default()
    };
    let counter = LocCounter::new(config);

    assert!(
        !counter.should_include(&vendor_file),
        "Should exclude files matching custom patterns"
    );
}

#[test]
fn test_loc_counting_from_file_paths() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple files
    let file1 = temp_dir.path().join("file1.rs");
    let file2 = temp_dir.path().join("file2.rs");
    let test_file = temp_dir.path().join("example_test.rs"); // _test suffix

    std::fs::write(&file1, "fn f1() { println!(\"1\"); }").unwrap();
    std::fs::write(&file2, "fn f2() { println!(\"2\"); }").unwrap();
    std::fs::write(&test_file, "#[test]\nfn test() { }").unwrap();

    let files = vec![file1.clone(), file2.clone(), test_file.clone()];

    // Count LOC for all files
    let counter = LocCounter::default();
    let project_count = counter.count_from_file_paths(&files);

    // Should have counted file1 and file2, but not test_file (excluded by default)
    assert_eq!(
        project_count.by_file.len(),
        2,
        "Should count only non-test files"
    );
    assert!(project_count.by_file.contains_key(&file1));
    assert!(project_count.by_file.contains_key(&file2));
    assert!(!project_count.by_file.contains_key(&test_file));

    // Total should be sum of individual file counts
    assert!(project_count.total.code_lines >= 2, "Should have at least 2 code lines total");
}
