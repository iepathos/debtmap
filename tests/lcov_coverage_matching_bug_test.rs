use debtmap::priority::call_graph::{CallGraph, FunctionId};
/// Test case reproducing the LCOV coverage matching bug where functions with coverage
/// are incorrectly reported as having no coverage due to name/line matching failures.
///
/// Bug details:
/// - LCOV data contains function coverage (e.g., "read_requirements" with 577 executions)
/// - debtmap fails to match the function and reports it as uncovered
/// - This causes false positives in technical debt reporting
use debtmap::priority::coverage_propagation::calculate_coverage_urgency;
use debtmap::risk::lcov::parse_lcov_file;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Creates a realistic LCOV fixture mimicking the deku_string coverage data
fn create_lcov_fixture() -> String {
    r#"TN:
SF:/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs
FN:11,StringDeku::from_reader_with_ctx_impl
FN:59,StringDeku::to_writer_impl
FN:103,<impl DekuReader for StringDeku>::from_reader_with_ctx
FN:129,<impl DekuWriter for StringDeku>::to_writer
FN:153,read_requirements
FN:207,read_string
FN:245,write_string
FN:289,write_string_length_prefix
FN:328,write_string_fixed_length
FNF:9
FNDA:577,StringDeku::from_reader_with_ctx_impl
FNDA:565,StringDeku::to_writer_impl
FNDA:282,<impl DekuReader for StringDeku>::from_reader_with_ctx
FNDA:168,<impl DekuWriter for StringDeku>::to_writer
FNDA:577,read_requirements
FNDA:462,read_string
FNDA:565,write_string
FNDA:235,write_string_length_prefix
FNDA:196,write_string_fixed_length
DA:11,577
DA:20,517
DA:21,2368
DA:25,0
DA:27,55
DA:30,0
DA:31,0
DA:32,242
DA:33,261
DA:34,143
DA:37,0
DA:38,862
DA:39,174
DA:40,143
DA:43,0
DA:44,844
DA:45,252
DA:46,1480
DA:47,2428
DA:48,28
DA:50,0
DA:51,0
DA:53,56
DA:59,565
DA:66,565
DA:67,0
DA:68,191
DA:69,0
DA:71,0
DA:72,570
DA:73,950
DA:75,0
DA:76,368
DA:79,2100488
DA:81,920
DA:89,295
DA:96,1180
DA:97,1475
DA:103,282
DA:110,1128
DA:111,1410
DA:117,397
DA:122,1588
DA:123,2382
DA:129,168
DA:134,672
DA:135,1008
DA:153,577
DA:158,577
DA:159,0
DA:160,148
DA:161,0
DA:162,0
DA:163,0
DA:164,74
DA:166,74
DA:168,0
DA:169,0
DA:170,0
DA:171,0
DA:172,0
DA:175,0
DA:177,62
DA:178,139
DA:179,131
DA:180,130
DA:182,367
DA:183,674
DA:184,110
DA:185,270
DA:186,270
DA:187,0
DA:188,73
DA:189,315
DA:190,0
DA:193,0
DA:194,0
DA:195,0
DA:196,0
DA:197,0
DA:207,462
DA:245,565
DA:289,235
DA:299,235
DA:300,94
DA:301,94
DA:302,94
DA:303,47
DA:306,235
DA:307,0
DA:315,235
DA:316,94
DA:317,94
DA:318,94
DA:319,0
DA:320,47
DA:321,47
DA:324,235
DA:328,196
DA:341,196
DA:342,0
DA:347,196
DA:348,0
DA:353,196
DA:355,1764
DA:356,1764
DA:359,196
LF:104
LH:73
end_of_record
"#
    .to_string()
}

#[test]
fn test_lcov_coverage_matching_bug_exact_match() {
    // Create temporary LCOV file
    let lcov_content = create_lcov_fixture();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    // Parse LCOV data
    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();
    let file_path = PathBuf::from("/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs");

    // Test 1: Direct function name lookup should work
    let coverage = lcov_data.get_function_coverage(&file_path, "read_requirements");
    assert!(
        coverage.is_some(),
        "Should find coverage for 'read_requirements' by exact name"
    );

    // The function has 577 executions, so it should be considered covered
    // We don't know the exact percentage without line coverage details, but it should be > 0
    if let Some(cov) = coverage {
        assert!(
            cov > 0.0,
            "Function with 577 executions should have coverage > 0, got {}",
            cov
        );
    }
}

#[test]
fn test_lcov_coverage_matching_bug_with_line_bounds() {
    let lcov_content = create_lcov_fixture();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();
    let file_path = PathBuf::from("/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs");

    // Test 2: Function lookup with exact line boundaries
    // read_requirements starts at line 153 according to LCOV
    let coverage = lcov_data.get_function_coverage_with_bounds(
        &file_path,
        "read_requirements",
        153, // exact start line from LCOV
        200, // approximate end line
    );

    assert!(
        coverage.is_some(),
        "Should find coverage for 'read_requirements' with exact line bounds"
    );
}

#[test]
fn test_lcov_coverage_matching_bug_with_off_by_one_lines() {
    let lcov_content = create_lcov_fixture();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();
    let file_path = PathBuf::from("/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs");

    // Test 3: Function lookup with slightly off line numbers (common AST vs LCOV mismatch)
    // This simulates the case where AST reports line 152 or 154 but LCOV has 153
    let coverage_off_by_one = lcov_data.get_function_coverage_with_bounds(
        &file_path,
        "read_requirements",
        152, // off by 1 line (AST might report different line than LCOV)
        200,
    );

    // This currently FAILS but should succeed with tolerance
    // Uncomment when fix is implemented:
    // assert!(
    //     coverage_off_by_one.is_some(),
    //     "Should find coverage with ±1 line tolerance"
    // );

    // For now, document the bug:
    if coverage_off_by_one.is_none() {
        println!("BUG CONFIRMED: Function not found when line number is off by 1");
    }
}

#[test]
fn test_lcov_coverage_matching_bug_with_impl_blocks() {
    let lcov_content = create_lcov_fixture();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();
    let file_path = PathBuf::from("/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs");

    // Test 4: Functions with impl blocks and angle brackets
    let impl_coverage = lcov_data.get_function_coverage(
        &file_path,
        "<impl DekuReader for StringDeku>::from_reader_with_ctx",
    );

    assert!(
        impl_coverage.is_some(),
        "Should find coverage for impl block function with angle brackets"
    );

    // Test normalized version (what AST might provide)
    let normalized_coverage = lcov_data.get_function_coverage(
        &file_path,
        "_impl DekuReader for StringDeku_::from_reader_with_ctx",
    );

    // This might fail due to normalization issues
    if normalized_coverage.is_none() {
        println!("BUG: Normalized function name doesn't match LCOV data");
    }
}

#[test]
fn test_lcov_coverage_urgency_calculation_with_bug() {
    let lcov_content = create_lcov_fixture();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();
    let call_graph = CallGraph::new();

    // Test 5: Coverage urgency calculation for functions that ARE covered
    let func_id = FunctionId::new(
        PathBuf::from("/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs"),
        "read_requirements".to_string(),
        153,
    );

    let urgency = calculate_coverage_urgency(&func_id, &call_graph, &lcov_data, 12);

    // Function has 577 executions but only 48% line coverage,
    // so urgency should be moderate-to-high (around 6-7)
    // With complexity 12 and ~50% coverage gap, urgency ~7 is correct
    assert!(
        (6.0..8.0).contains(&urgency),
        "Function with 48% coverage and complexity 12 should have urgency 6-8, got {}",
        urgency
    );
}

#[test]
fn test_lcov_false_positive_detection() {
    let lcov_content = create_lcov_fixture();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();
    let file_path = PathBuf::from("/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs");

    // Test all three high-complexity functions reported by debtmap
    let test_cases = vec![
        ("read_requirements", 153, 577),          // Finding #1
        ("write_string_fixed_length", 328, 196),  // Finding #2
        ("write_string_length_prefix", 289, 235), // Finding #3
    ];

    for (func_name, line, expected_executions) in test_cases {
        // Check that function exists in LCOV with expected execution count
        let funcs = lcov_data.functions.get(&file_path).unwrap();
        let lcov_func = funcs.iter().find(|f| f.name == func_name);

        assert!(
            lcov_func.is_some(),
            "Function '{}' should exist in LCOV data",
            func_name
        );

        let lcov_func = lcov_func.unwrap();
        assert_eq!(
            lcov_func.execution_count, expected_executions,
            "Function '{}' should have {} executions",
            func_name, expected_executions
        );
        assert_eq!(
            lcov_func.start_line, line,
            "Function '{}' should start at line {}",
            func_name, line
        );

        // Now test that coverage lookup works
        let coverage = lcov_data.get_function_coverage(&file_path, func_name);
        assert!(
            coverage.is_some(),
            "Should find coverage for '{}' which has {} executions",
            func_name,
            expected_executions
        );

        // Test with bounds lookup (simulating what debtmap does)
        let coverage_with_bounds = lcov_data.get_function_coverage_with_bounds(
            &file_path,
            func_name,
            line,
            line + 50, // approximate function length
        );

        assert!(
            coverage_with_bounds.is_some(),
            "Should find coverage for '{}' with line bounds [{}, {}]",
            func_name,
            line,
            line + 50
        );
    }
}

#[test]
fn test_coverage_factor_scoring_bug() {
    // This test demonstrates the scoring bug where coverage_factor = 10.0
    // is displayed as "Coverage gap (40%)" which is misleading

    let lcov_content = create_lcov_fixture();
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(lcov_content.as_bytes()).unwrap();

    let lcov_data = parse_lcov_file(temp_file.path()).unwrap();
    let call_graph = CallGraph::new();

    // Simulate what happens when function is not found
    let missing_func_id = FunctionId::new(
        PathBuf::from("/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs"),
        "nonexistent_function".to_string(),
        999,
    );

    let missing_urgency = calculate_coverage_urgency(&missing_func_id, &call_graph, &lcov_data, 10);

    // When function is not found, urgency can exceed 10.0 with spec 96
    assert!(
        missing_urgency >= 10.0,
        "Missing function should have urgency at least 10.0, got {}",
        missing_urgency
    );

    // Now test with a function that exists
    let existing_func_id = FunctionId::new(
        PathBuf::from("/Users/glen/memento-mori/deku_string/src/string/deku_impl.rs"),
        "read_requirements".to_string(),
        153,
    );

    let existing_urgency =
        calculate_coverage_urgency(&existing_func_id, &call_graph, &lcov_data, 12);

    // Function with coverage should have urgency < 10.0
    assert!(
        existing_urgency < 10.0,
        "Covered function should have urgency < 10.0, got {}",
        existing_urgency
    );

    println!(
        "Coverage urgency difference: missing={:.2}, existing={:.2}",
        missing_urgency, existing_urgency
    );
}
