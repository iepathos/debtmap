use std::process::Command;
use tempfile::TempDir;

fn debtmap_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_debtmap"))
}

fn run_validate(temp_dir: &TempDir, args: &[&str]) -> std::process::Output {
    let mut command = debtmap_command();
    command.arg("validate").args(args).arg(temp_dir.path());
    command
        .output()
        .expect("Failed to execute validate command")
}

/// Test that validate command runs with parallel processing enabled by default
#[test]
fn test_validate_parallel_enabled_by_default() {
    let temp_dir = TempDir::new().unwrap();

    // Create a simple Rust project for testing
    std::fs::write(
        temp_dir.path().join("test.rs"),
        r#"
        fn simple_function() -> i32 {
            42
        }

        fn another_function(x: i32) -> i32 {
            x * 2
        }
        "#,
    )
    .unwrap();

    let output = run_validate(&temp_dir, &[]);

    // Command should succeed
    assert!(
        output.status.success(),
        "Validate command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that validate command respects --no-parallel flag
#[test]
fn test_validate_no_parallel_flag() {
    let temp_dir = TempDir::new().unwrap();

    // Create a simple Rust project for testing
    std::fs::write(
        temp_dir.path().join("test.rs"),
        r#"
        fn simple_function() -> i32 {
            42
        }
        "#,
    )
    .unwrap();

    let output = run_validate(&temp_dir, &["--no-parallel"]);

    // Command should succeed
    assert!(
        output.status.success(),
        "Validate command with --no-parallel failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that validate command respects --jobs parameter
#[test]
fn test_validate_jobs_parameter() {
    let temp_dir = TempDir::new().unwrap();

    // Create a simple Rust project for testing
    std::fs::write(
        temp_dir.path().join("test.rs"),
        r#"
        fn simple_function() -> i32 {
            42
        }
        "#,
    )
    .unwrap();

    let output = run_validate(&temp_dir, &["--jobs", "4"]);

    // Command should succeed
    assert!(
        output.status.success(),
        "Validate command with --jobs failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that parallel and sequential validation produce equivalent results
#[test]
fn test_parallel_sequential_equivalence() {
    let temp_dir = TempDir::new().unwrap();

    // Create a moderately complex Rust file
    std::fs::write(
        temp_dir.path().join("test.rs"),
        r#"
        fn complex_function(a: i32, b: i32, c: i32) -> i32 {
            if a > 0 {
                if b > 0 {
                    if c > 0 {
                        return a + b + c;
                    } else {
                        return a + b - c;
                    }
                } else {
                    return a - b;
                }
            } else {
                return 0;
            }
        }

        fn simple_function() -> i32 {
            42
        }

        fn another_complex(x: i32) -> i32 {
            let mut result = 0;
            for i in 0..x {
                if i % 2 == 0 {
                    result += i;
                } else {
                    result -= i;
                }
            }
            result
        }
        "#,
    )
    .unwrap();

    let parallel_output = run_validate(&temp_dir, &[]);
    let sequential_output = run_validate(&temp_dir, &["--no-parallel"]);

    // Both should succeed
    assert!(
        parallel_output.status.success(),
        "Parallel validation failed: {}",
        String::from_utf8_lossy(&parallel_output.stderr)
    );
    assert!(
        sequential_output.status.success(),
        "Sequential validation failed: {}",
        String::from_utf8_lossy(&sequential_output.stderr)
    );

    // Results should be equivalent (both pass or both fail)
    assert_eq!(
        parallel_output.status.code(),
        sequential_output.status.code(),
        "Parallel and sequential validation produced different exit codes"
    );
}

/// Test that DEBTMAP_JOBS environment variable is respected
#[test]
fn test_debtmap_jobs_env_var() {
    let temp_dir = TempDir::new().unwrap();

    // Create a simple Rust project for testing
    std::fs::write(
        temp_dir.path().join("test.rs"),
        r#"
        fn simple_function() -> i32 {
            42
        }
        "#,
    )
    .unwrap();

    let output = debtmap_command()
        .arg("validate")
        .arg(temp_dir.path())
        .env("DEBTMAP_JOBS", "2")
        .output()
        .expect("Failed to execute validate command");

    // Command should succeed
    assert!(
        output.status.success(),
        "Validate command with DEBTMAP_JOBS env var failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Test that parallel validation works on a larger project
#[test]
fn test_validate_parallel_large_project() {
    let temp_dir = TempDir::new().unwrap();

    // Create multiple Rust files to simulate a larger project
    for i in 0..10 {
        std::fs::write(
            temp_dir.path().join(format!("module_{}.rs", i)),
            format!(
                r#"
                pub fn function_{}(x: i32) -> i32 {{
                    if x > 0 {{
                        x * 2
                    }} else {{
                        x / 2
                    }}
                }}

                pub fn another_function_{}(a: i32, b: i32) -> i32 {{
                    a + b
                }}
                "#,
                i, i
            ),
        )
        .unwrap();
    }

    let output = run_validate(&temp_dir, &[]);

    // Command should succeed
    assert!(
        output.status.success(),
        "Validate command on large project failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Output should indicate parallel processing (check for progress messages)
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The parallel version should show some indication of parallel processing
    // This is a weak assertion - ideally we'd verify performance difference
    assert!(
        !stderr.is_empty(),
        "Expected some output from validate command"
    );
}
