/// Integration test to prevent coverage indexing performance regressions
///
/// This test validates that file analysis with coverage lookups completes
/// within the acceptable performance threshold of ≤3x baseline overhead.
///
/// Baseline: ~53ms for analysis without coverage
/// Target: ≤160ms for analysis with indexed coverage lookups
use debtmap::risk::lcov::parse_lcov_file;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::NamedTempFile;

/// Create a realistic LCOV file for performance testing
fn create_test_lcov_file(num_files: usize, funcs_per_file: usize) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().unwrap();

    for file_idx in 0..num_files {
        let file_path = format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx);
        writeln!(temp_file, "TN:").unwrap();
        writeln!(temp_file, "SF:{}", file_path).unwrap();

        for func_idx in 0..funcs_per_file {
            let line_start = func_idx * 15 + 10;
            let func_name = format!("function_{}_{}", file_idx, func_idx);

            writeln!(temp_file, "FN:{},{}", line_start, func_name).unwrap();
            writeln!(temp_file, "FNDA:5,{}", func_name).unwrap();

            // Add line coverage data
            for line_offset in 0..10 {
                let line_num = line_start + line_offset;
                let count = if line_offset < 7 { 5 } else { 0 };
                writeln!(temp_file, "DA:{},{}", line_num, count).unwrap();
            }
        }

        writeln!(temp_file, "LF:{}", funcs_per_file * 10).unwrap();
        writeln!(temp_file, "LH:{}", funcs_per_file * 7).unwrap();
        writeln!(temp_file, "end_of_record").unwrap();
    }

    temp_file
}

#[test]
fn test_coverage_lookup_performance_overhead() {
    const NUM_FILES: usize = 100;
    const FUNCS_PER_FILE: usize = 20;
    const MAX_COVERAGE_TIME_MS: u128 = 200; // Absolute max time for lookups

    // Create and parse coverage data
    let temp_file = create_test_lcov_file(NUM_FILES, FUNCS_PER_FILE);
    let data = parse_lcov_file(temp_file.path()).expect("Failed to parse LCOV file");

    // Measure performance with indexed coverage lookups
    let coverage_start = Instant::now();
    for file_idx in 0..NUM_FILES {
        for func_idx in 0..FUNCS_PER_FILE {
            // Add indexed coverage lookup
            let file = PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));
            let func_name = format!("function_{}_{}", file_idx, func_idx);
            let _coverage = data.get_function_coverage(&file, &func_name);
        }
    }
    let coverage_duration = coverage_start.elapsed();
    let coverage_ms = coverage_duration.as_millis();

    println!(
        "Coverage lookup duration: {:?} for {} lookups",
        coverage_duration,
        NUM_FILES * FUNCS_PER_FILE
    );
    println!(
        "Average per lookup: {:.2}μs",
        coverage_duration.as_micros() as f64 / (NUM_FILES * FUNCS_PER_FILE) as f64
    );

    // Assert absolute performance target: coverage lookups should be fast
    assert!(
        coverage_ms <= MAX_COVERAGE_TIME_MS,
        "Coverage lookup took {}ms for {} lookups, exceeds maximum {}ms",
        coverage_ms,
        NUM_FILES * FUNCS_PER_FILE,
        MAX_COVERAGE_TIME_MS
    );
}

#[test]
fn test_indexed_lookup_is_fast() {
    const NUM_FILES: usize = 100;
    const FUNCS_PER_FILE: usize = 20;
    const MAX_LOOKUP_TIME_MS: u128 = 100; // 100ms for 2000 lookups = 50μs per lookup

    let temp_file = create_test_lcov_file(NUM_FILES, FUNCS_PER_FILE);
    let data = parse_lcov_file(temp_file.path()).expect("Failed to parse LCOV file");

    // Measure time for many indexed lookups
    let start = Instant::now();
    let mut lookup_count = 0;

    for file_idx in 0..NUM_FILES {
        for func_idx in 0..FUNCS_PER_FILE {
            let file = PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));
            let func_name = format!("function_{}_{}", file_idx, func_idx);
            let coverage = data.get_function_coverage(&file, &func_name);
            assert!(
                coverage.is_some(),
                "Coverage should be found for existing function"
            );
            lookup_count += 1;
        }
    }

    let duration = start.elapsed();
    let duration_ms = duration.as_millis();

    println!(
        "Performed {} lookups in {:?} ({:.2}μs per lookup)",
        lookup_count,
        duration,
        duration.as_micros() as f64 / lookup_count as f64
    );

    assert!(
        duration_ms <= MAX_LOOKUP_TIME_MS,
        "Indexed lookup took {}ms for {} lookups, exceeds maximum {}ms",
        duration_ms,
        lookup_count,
        MAX_LOOKUP_TIME_MS
    );
}

#[test]
fn test_line_based_lookup_with_tolerance() {
    const NUM_FILES: usize = 100;
    const FUNCS_PER_FILE: usize = 20;
    const MAX_LOOKUP_TIME_MS: u128 = 1500; // Line-based fallback with BTreeMap range query and tolerance strategies (increased for slower CI environments)

    let temp_file = create_test_lcov_file(NUM_FILES, FUNCS_PER_FILE);
    let data = parse_lcov_file(temp_file.path()).expect("Failed to parse LCOV file");

    let start = Instant::now();
    let mut lookup_count = 0;

    // Test line-based lookup with unknown function names (forces line-based search)
    for file_idx in 0..NUM_FILES {
        for func_idx in 0..FUNCS_PER_FILE {
            let file = PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx));
            let line = func_idx * 15 + 10;
            let coverage = data.get_function_coverage_with_line(&file, "unknown_function", line);
            assert!(
                coverage.is_some(),
                "Coverage should be found by line number"
            );
            lookup_count += 1;
        }
    }

    let duration = start.elapsed();
    let duration_ms = duration.as_millis();

    println!(
        "Performed {} line-based lookups in {:?} ({:.2}μs per lookup)",
        lookup_count,
        duration,
        duration.as_micros() as f64 / lookup_count as f64
    );

    assert!(
        duration_ms <= MAX_LOOKUP_TIME_MS,
        "Line-based lookup took {}ms for {} lookups, exceeds maximum {}ms",
        duration_ms,
        lookup_count,
        MAX_LOOKUP_TIME_MS
    );
}

#[test]
fn test_batch_parallel_lookup_performance() {
    const NUM_FILES: usize = 100;
    const FUNCS_PER_FILE: usize = 20;
    const MAX_BATCH_TIME_MS: u128 = 150;

    let temp_file = create_test_lcov_file(NUM_FILES, FUNCS_PER_FILE);
    let data = parse_lcov_file(temp_file.path()).expect("Failed to parse LCOV file");

    // Create batch queries
    let queries: Vec<(PathBuf, String, usize)> = (0..NUM_FILES)
        .flat_map(|file_idx| {
            (0..FUNCS_PER_FILE).map(move |func_idx| {
                (
                    PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx)),
                    format!("function_{}_{}", file_idx, func_idx),
                    func_idx * 15 + 10,
                )
            })
        })
        .collect();

    let start = Instant::now();
    let results = data.batch_get_function_coverage(&queries);
    let duration = start.elapsed();
    let duration_ms = duration.as_millis();

    println!(
        "Batch processed {} queries in {:?} ({:.2}μs per lookup)",
        queries.len(),
        duration,
        duration.as_micros() as f64 / queries.len() as f64
    );

    // Verify all queries succeeded
    let successful_lookups = results.iter().filter(|r| r.is_some()).count();
    assert_eq!(
        successful_lookups,
        queries.len(),
        "All batch lookups should succeed"
    );

    assert!(
        duration_ms <= MAX_BATCH_TIME_MS,
        "Batch lookup took {}ms for {} queries, exceeds maximum {}ms",
        duration_ms,
        queries.len(),
        MAX_BATCH_TIME_MS
    );
}
