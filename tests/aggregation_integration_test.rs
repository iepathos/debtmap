use debtmap::builders::unified_analysis;
use debtmap::core::Language;
use debtmap::utils::analyze_project;
use tempfile::TempDir;

#[test]
fn test_file_aggregation_with_real_codebase() {
    // Create a test project structure
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Create test files with varying complexity
    let src_dir = project_path.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    // File 1: High complexity functions with more branches
    let file1_content = r#"
fn complex_function_1(x: i32, y: i32, z: i32) -> i32 {
    let mut result = 0;
    if x > 0 {
        if y > 0 {
            for i in 0..10 {
                if i > 5 {
                    if z > 0 {
                        result += 1;
                    } else if z < 0 {
                        result -= 1;
                    } else {
                        result += 2;
                    }
                }
                if i % 2 == 0 {
                    match i {
                        0 => result *= 2,
                        2 => result *= 3,
                        4 => result *= 4,
                        6 => result *= 5,
                        8 => result *= 6,
                        _ => result *= 7,
                    }
                }
            }
        } else if y < 0 {
            while result < 100 {
                result += x;
                if result % 3 == 0 {
                    break;
                }
            }
        }
    } else if x < 0 {
        for j in 0..20 {
            if j % 3 == 0 {
                result += j;
            } else if j % 3 == 1 {
                result -= j;
            } else {
                result *= 2;
            }
        }
    }
    result
}

fn complex_function_2(input: &str) -> Result<Vec<i32>, String> {
    let mut values = Vec::new();
    let parts: Vec<&str> = input.split(',').collect();
    
    for part in parts {
        match part.trim() {
            "one" => {
                if values.len() > 0 {
                    values.push(1);
                } else {
                    for _ in 0..5 {
                        values.push(1);
                    }
                }
            },
            "two" => {
                for i in 0..2 {
                    if i == 0 {
                        values.push(2);
                    } else {
                        values.push(22);
                    }
                }
            },
            "three" => {
                let mut count = 0;
                while count < 3 {
                    values.push(3);
                    count += 1;
                    if count == 2 {
                        values.push(33);
                    }
                }
            },
            _ => {
                if let Ok(num) = part.trim().parse::<i32>() {
                    if num > 0 && num < 100 {
                        values.push(num);
                    } else if num >= 100 {
                        return Err("Number too large".to_string());
                    } else {
                        return Err("Negative number".to_string());
                    }
                } else {
                    return Err(format!("Invalid input: {}", part));
                }
            }
        }
    }
    
    if values.is_empty() {
        Err("No valid values".to_string())
    } else {
        Ok(values)
    }
}
"#;
    std::fs::write(src_dir.join("complex.rs"), file1_content).unwrap();

    // File 2: Simple functions
    let file2_content = r#"
fn simple_function_1() {
    println!("Simple");
}

fn simple_function_2() -> i32 {
    42
}
"#;
    std::fs::write(src_dir.join("simple.rs"), file2_content).unwrap();

    // Analyze the project
    let languages = vec![Language::Rust];
    let results = analyze_project(
        project_path.to_path_buf(),
        languages,
        2,  // Lower complexity threshold to catch our test functions
        50, // duplication threshold
    )
    .unwrap();

    // Perform unified analysis with aggregation enabled
    let unified = unified_analysis::perform_unified_analysis_with_options(
        unified_analysis::UnifiedAnalysisOptions {
            results: &results,
            coverage_file: None,
            semantic_off: false,
            project_path,
            verbose_macro_warnings: false,
            show_macro_stats: false,
            parallel: false,
            jobs: 0,
            use_cache: false,
            multi_pass: false,
            show_attribution: false,
            aggregate_only: false,
            no_aggregation: false,
            aggregation_method: Some("weighted_sum".to_string()),
            min_problematic: None,
            no_god_object: false,
        },
    )
    .unwrap();

    // Verify file aggregates were created
    assert!(
        !unified.file_aggregates.is_empty(),
        "File aggregates should be created"
    );

    // Check that complex.rs has higher aggregate score than simple.rs
    let complex_aggregate = unified
        .file_aggregates
        .iter()
        .find(|a| a.file_path.to_string_lossy().contains("complex.rs"));
    let simple_aggregate = unified
        .file_aggregates
        .iter()
        .find(|a| a.file_path.to_string_lossy().contains("simple.rs"));

    if let (Some(complex), Some(simple)) = (complex_aggregate, simple_aggregate) {
        assert!(
            complex.aggregate_score > simple.aggregate_score,
            "Complex file should have higher aggregate score than simple file"
        );
    }
}

#[test]
fn test_aggregation_methods() {
    // Test different aggregation methods produce different results
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Create test file
    let src_dir = project_path.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    let file_content = r#"
fn func1(data: Vec<i32>) -> i32 {
    let mut sum = 0;
    for i in 0..data.len() {
        if data[i] > 5 {
            for j in 0..10 {
                if j > i {
                    sum += data[i] * j;
                    if sum > 100 {
                        break;
                    }
                } else if j < i {
                    sum -= j;
                } else {
                    sum *= 2;
                }
            }
        } else if data[i] < 0 {
            match data[i] {
                -1 => sum += 10,
                -2 => sum += 20,
                -3 => sum += 30,
                _ => sum = 0,
            }
        }
    }
    sum
}

fn func2(input: String) -> Result<i32, String> {
    let x = input.len();
    let result = match x {
        0 => return Err("Empty input".to_string()),
        1..=3 => {
            if input.chars().all(|c| c.is_ascii_lowercase()) {
                10
            } else if input.chars().all(|c| c.is_ascii_uppercase()) {
                20
            } else {
                30
            }
        },
        4..=6 => {
            let mut val = 0;
            for c in input.chars() {
                if c.is_numeric() {
                    val += c.to_digit(10).unwrap() as i32;
                } else if c.is_alphabetic() {
                    val *= 2;
                } else {
                    val -= 1;
                }
            }
            val
        },
        _ => {
            let mut counter = 0;
            while counter < x {
                counter += 1;
                if counter % 2 == 0 {
                    counter *= 2;
                }
            }
            counter as i32
        }
    };
    Ok(result)
}

fn func3(a: bool, b: bool, c: bool, d: i32) -> i32 {
    let mut result = 0;
    if a && b || c {
        for i in 0..d {
            if i % 3 == 0 {
                result += i;
            } else if i % 3 == 1 {
                result -= i;
                if result < 0 {
                    result = result.abs();
                }
            } else {
                match i {
                    0..=10 => result *= 2,
                    11..=20 => result *= 3,
                    _ => result = result / 2,
                }
            }
        }
    } else if !a && !b {
        while result < 100 {
            result += d;
            if result % 7 == 0 {
                break;
            }
        }
    }
    result
}
"#;
    std::fs::write(src_dir.join("test.rs"), file_content).unwrap();

    let languages = vec![Language::Rust];
    let results = analyze_project(
        project_path.to_path_buf(),
        languages,
        2, // lower threshold to catch more functions
        50,
    )
    .unwrap();

    // Test each aggregation method
    let methods = vec!["sum", "weighted_sum", "logarithmic_sum", "max_plus_average"];

    let mut scores = Vec::new();

    for method in methods {
        let unified = unified_analysis::perform_unified_analysis_with_options(
            unified_analysis::UnifiedAnalysisOptions {
                results: &results,
                coverage_file: None,
                semantic_off: false,
                project_path,
                verbose_macro_warnings: false,
                show_macro_stats: false,
                parallel: false,
                jobs: 0,
                use_cache: false,
                multi_pass: false,
                show_attribution: false,
                aggregate_only: false,
                no_aggregation: false,
                aggregation_method: Some(method.to_string()),
                min_problematic: None,
            no_god_object: false,
            },
        )
        .unwrap();

        if let Some(aggregate) = unified.file_aggregates.iter().next() {
            scores.push((method, aggregate.aggregate_score));
        }
    }

    // Verify that different methods produce different scores
    assert!(scores.len() == 4, "Should have scores for all 4 methods");

    // Check that at least some methods produce different scores
    let unique_scores: std::collections::HashSet<_> = scores
        .iter()
        .map(|(_, score)| (score * 1000.0) as i64) // Convert to int for comparison
        .collect();

    assert!(
        unique_scores.len() > 1,
        "Different aggregation methods should produce different scores"
    );
}

#[test]
fn test_cli_flag_no_aggregation() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Create simple test file
    let src_dir = project_path.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(
        src_dir.join("test.rs"),
        "fn main() { println!(\"Hello\"); }",
    )
    .unwrap();

    let languages = vec![Language::Rust];
    let results = analyze_project(project_path.to_path_buf(), languages, 10, 50).unwrap();

    // Test with aggregation disabled
    let unified = unified_analysis::perform_unified_analysis_with_options(
        unified_analysis::UnifiedAnalysisOptions {
            results: &results,
            coverage_file: None,
            semantic_off: false,
            project_path,
            verbose_macro_warnings: false,
            show_macro_stats: false,
            parallel: false,
            jobs: 0,
            use_cache: false,
            multi_pass: false,
            show_attribution: false,
            aggregate_only: false,
            no_aggregation: true, // Disable aggregation
            aggregation_method: None,
            min_problematic: None,
            no_god_object: false,
        },
    )
    .unwrap();

    // Verify no file aggregates were created
    assert!(
        unified.file_aggregates.is_empty(),
        "File aggregates should not be created when no_aggregation is true"
    );
}

#[test]
fn test_aggregation_with_multiple_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path();

    // Create multiple files with different complexity levels
    let src_dir = project_path.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();

    // Create 5 files with increasing complexity
    for i in 1..=5 {
        let mut content = String::from("use std::collections::HashMap;\n\n");

        // Add functions with increasing complexity
        for j in 1..=i {
            content.push_str(&format!("fn func_{}_{}_() {{\n", i, j));

            // Add nested conditions based on file number
            for k in 0..i {
                content.push_str(&"    ".repeat(k + 1));
                content.push_str(&format!("if condition_{} {{\n", k));
            }

            content.push_str(&"    ".repeat(i + 1));
            content.push_str("println!(\"Complex logic\");\n");

            for k in (0..i).rev() {
                content.push_str(&"    ".repeat(k + 1));
                content.push_str("}\n");
            }

            content.push_str("}\n\n");
        }

        std::fs::write(src_dir.join(format!("file_{}.rs", i)), content).unwrap();
    }

    let languages = vec![Language::Rust];
    let results = analyze_project(
        project_path.to_path_buf(),
        languages,
        2, // Low threshold to catch nested conditions
        50,
    )
    .unwrap();

    let unified = unified_analysis::perform_unified_analysis_with_options(
        unified_analysis::UnifiedAnalysisOptions {
            results: &results,
            coverage_file: None,
            semantic_off: false,
            project_path,
            verbose_macro_warnings: false,
            show_macro_stats: false,
            parallel: false,
            jobs: 0,
            use_cache: false,
            multi_pass: false,
            show_attribution: false,
            aggregate_only: false,
            no_aggregation: false,
            aggregation_method: Some("weighted_sum".to_string()),
            min_problematic: None,
            no_god_object: false,
        },
    )
    .unwrap();

    // Verify we have aggregates for multiple files
    assert!(
        unified.file_aggregates.len() >= 3,
        "Should have aggregates for at least 3 files"
    );

    // Verify aggregates are sorted by score (highest first)
    let scores: Vec<f64> = unified
        .file_aggregates
        .iter()
        .map(|a| a.aggregate_score)
        .collect();

    for i in 1..scores.len() {
        assert!(
            scores[i - 1] >= scores[i] || (scores[i - 1] - scores[i]).abs() < 0.001,
            "Aggregates should be sorted by score in descending order"
        );
    }
}

// Removed test_aggregation_pipeline_direct as it requires internal types
// The other integration tests adequately cover the aggregation functionality
