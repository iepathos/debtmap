/// Test that reproduces the path mismatch bug where LCOV files with relative paths
/// don't match when queried with absolute paths or different relative path formats.
///
/// This is the root cause of "No coverage data" being shown even when coverage exists.
use debtmap::risk::lcov::parse_lcov_file;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Create LCOV data with relative path (common in real LCOV files)
fn create_lcov_with_relative_path() -> String {
    r#"TN:
SF:src/string/deku_impl.rs
FN:153,read_requirements
FN:289,write_string_length_prefix
FN:328,write_string_fixed_length
FNDA:577,read_requirements
FNDA:235,write_string_length_prefix
FNDA:196,write_string_fixed_length
DA:153,577
DA:154,577
DA:155,577
DA:156,0
DA:157,0
DA:158,577
DA:159,0
DA:160,148
DA:289,235
DA:290,235
DA:291,235
DA:292,0
DA:328,196
DA:329,196
DA:330,0
LF:15
LH:10
end_of_record
"#
    .to_string()
}

#[test]
fn test_lcov_path_mismatch_relative_vs_absolute() {
    let lcov_content = create_lcov_with_relative_path();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();

    // The LCOV file has relative path: "src/string/deku_impl.rs"
    let relative_path = PathBuf::from("src/string/deku_impl.rs");

    // Test 1: Exact relative path should work
    let coverage = lcov_data.get_function_coverage(&relative_path, "read_requirements");
    assert!(
        coverage.is_some(),
        "Should find coverage with exact relative path"
    );
    assert!(
        coverage.unwrap() > 0.6 && coverage.unwrap() < 0.65,
        "read_requirements should have ~62.5% coverage (5/8 lines), got {:?}",
        coverage
    );

    // Test 2: Absolute path should ALSO work (this currently FAILS)
    let absolute_path =
        PathBuf::from("/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs");
    let coverage_abs = lcov_data.get_function_coverage(&absolute_path, "read_requirements");
    assert!(
        coverage_abs.is_some(),
        "Should find coverage with absolute path that ends with the relative path"
    );

    // Test 3: Different relative path format should work
    let relative_with_dot = PathBuf::from("./src/string/deku_impl.rs");
    let coverage_dot = lcov_data.get_function_coverage(&relative_with_dot, "read_requirements");
    assert!(
        coverage_dot.is_some(),
        "Should find coverage with ./relative path"
    );
}

#[test]
fn test_lcov_path_mismatch_with_bounds() {
    let lcov_content = create_lcov_with_relative_path();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();

    // Test with different path formats using get_function_coverage_with_bounds
    let test_cases = vec![
        ("exact relative", PathBuf::from("src/string/deku_impl.rs")),
        ("with ./", PathBuf::from("./src/string/deku_impl.rs")),
        (
            "absolute",
            PathBuf::from("/some/project/src/string/deku_impl.rs"),
        ),
        (
            "different absolute",
            PathBuf::from("/Users/glen/src/string/deku_impl.rs"),
        ),
    ];

    for (desc, path) in test_cases {
        let coverage =
            lcov_data.get_function_coverage_with_bounds(&path, "read_requirements", 153, 200);
        assert!(
            coverage.is_some(),
            "Should find coverage with {} path: {:?}",
            desc,
            path
        );
    }
}

#[test]
fn test_lcov_path_normalization_both_directions() {
    // Test 1: LCOV has absolute path, query with relative
    let lcov_absolute = r#"TN:
SF:/project/src/lib.rs
FN:10,my_function
FNDA:100,my_function
DA:10,100
DA:11,100
LF:2
LH:2
end_of_record
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_absolute.as_bytes()).unwrap();
    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();

    // Query with relative path should find the absolute path entry
    let relative = PathBuf::from("src/lib.rs");
    let coverage = lcov_data.get_function_coverage(&relative, "my_function");
    assert!(
        coverage.is_some(),
        "Should find absolute path entry when querying with relative path"
    );
    assert_eq!(coverage.unwrap(), 1.0, "Should have 100% coverage");

    // Test 2: LCOV has relative path, query with absolute (already tested above)
}

#[test]
fn test_real_world_scenario_deku_string() {
    // This simulates the exact scenario from the deku_string project
    let lcov_content = create_lcov_with_relative_path();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();

    // Simulate what debtmap does: uses the file path from AST analysis
    // which might be absolute or relative depending on how it was invoked
    let ast_file_path = PathBuf::from("src/string/deku_impl.rs");

    // All three functions should have coverage
    let functions = vec![
        ("read_requirements", 153, 0.66), // ~66% coverage (10/15 lines)
        ("write_string_length_prefix", 289, 0.75), // 75% coverage (3/4 lines)
        ("write_string_fixed_length", 328, 0.66), // ~66% coverage (2/3 lines)
    ];

    for (func_name, line, expected_coverage) in functions {
        let coverage =
            lcov_data.get_function_coverage_with_bounds(&ast_file_path, func_name, line, line + 50);

        assert!(
            coverage.is_some(),
            "Function {} should have coverage data",
            func_name
        );

        let actual = coverage.unwrap();
        assert!(
            (actual - expected_coverage).abs() < 0.1,
            "Function {} should have ~{:.0}% coverage, got {:.0}%",
            func_name,
            expected_coverage * 100.0,
            actual * 100.0
        );
    }
}
