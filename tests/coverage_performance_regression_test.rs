//! Coverage lookup correctness at scale, with opt-in wall-clock regression checks.
//!
//! Timing checks are ignored during ordinary and instrumented runs because
//! instrumentation and concurrent tests invalidate their absolute thresholds.
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
#[ignore = "stress: wall-clock coverage lookup threshold requires an uninstrumented run"]
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
#[ignore = "stress: wall-clock indexed lookup threshold requires an uninstrumented run"]
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
#[ignore = "stress: wall-clock bounded lookup threshold requires an uninstrumented run"]
fn bounded_lookup_is_indexed_for_absolute_source_paths() {
    let fixture = create_test_lcov_file(100, 20);
    let data = parse_lcov_file(fixture.path()).unwrap();
    let start = Instant::now();
    for file in 0..100 {
        let path = PathBuf::from(format!(
            "/workspace/src/module_{}/file_{}.rs",
            file / 10,
            file
        ));
        for function in 0..20 {
            let line = function * 15 + 10;
            assert_eq!(
                data.get_function_coverage_with_bounds(&path, "f", line, line + 9),
                Some(0.7)
            );
        }
    }
    assert!(
        start.elapsed().as_millis() < 200,
        "bounded queries should use the path/line index"
    );
}

#[test]
#[ignore = "stress: wall-clock line lookup threshold requires an uninstrumented run"]
fn test_line_based_lookup_with_tolerance() {
    const NUM_FILES: usize = 100;
    const FUNCS_PER_FILE: usize = 20;
    const MAX_LOOKUP_TIME_MS: u128 = 3500; // Line-based fallback with BTreeMap range query and tolerance strategies (increased for CI environment variability)

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
#[ignore = "stress: wall-clock batch lookup threshold requires an uninstrumented run"]
fn test_batch_parallel_lookup_performance() {
    const NUM_FILES: usize = 100;
    const FUNCS_PER_FILE: usize = 20;
    const MAX_BATCH_TIME_MS: u128 = 150;

    let temp_file = create_test_lcov_file(NUM_FILES, FUNCS_PER_FILE);
    let data = parse_lcov_file(temp_file.path()).expect("Failed to parse LCOV file");

    // Create batch queries
    let queries = lookup_queries(NUM_FILES, FUNCS_PER_FILE);

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

fn lookup_queries(num_files: usize, funcs_per_file: usize) -> Vec<(PathBuf, String, usize)> {
    (0..num_files)
        .flat_map(|file_idx| {
            (0..funcs_per_file).map(move |func_idx| {
                (
                    PathBuf::from(format!("src/module_{}/file_{}.rs", file_idx / 10, file_idx)),
                    format!("function_{}_{}", file_idx, func_idx),
                    func_idx * 15 + 10,
                )
            })
        })
        .collect()
}

#[test]
fn named_lookups_return_exact_coverage_at_scale() {
    let fixture = create_test_lcov_file(100, 20);
    let data = parse_lcov_file(fixture.path()).unwrap();
    for (path, name, _) in lookup_queries(100, 20) {
        assert_eq!(data.get_function_coverage(&path, &name), Some(0.7));
    }
}

#[test]
fn line_lookups_return_exact_coverage_at_scale() {
    let fixture = create_test_lcov_file(100, 20);
    let data = parse_lcov_file(fixture.path()).unwrap();
    for (path, _, line) in lookup_queries(100, 20) {
        assert_eq!(
            data.get_function_coverage_with_line(&path, "unknown_function", line),
            Some(0.7)
        );
    }
}

#[test]
fn bounded_lookups_return_exact_coverage_for_absolute_source_paths() {
    let fixture = create_test_lcov_file(100, 20);
    let data = parse_lcov_file(fixture.path()).unwrap();
    for (path, _, line) in lookup_queries(100, 20) {
        assert_eq!(
            data.get_function_coverage_with_bounds(
                &PathBuf::from("/workspace").join(path),
                "unknown_function",
                line,
                line + 9,
            ),
            Some(0.7)
        );
    }
}

#[test]
fn batch_lookups_return_exact_coverage_at_scale() {
    let fixture = create_test_lcov_file(100, 20);
    let data = parse_lcov_file(fixture.path()).unwrap();
    let queries = lookup_queries(100, 20);
    assert_eq!(
        data.batch_get_function_coverage(&queries),
        vec![Some(0.7); queries.len()]
    );
}
