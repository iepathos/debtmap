//! Test that unified_visitor doesn't incorrectly flag tokio::process::Command::output() as I/O

use debtmap::performance::collect_performance_data;
use debtmap::performance::collected_data::IOType;
use std::path::Path;
use syn::parse_file;

#[test]
fn test_tokio_command_output_not_flagged() {
    // This is the exact pattern from retry.rs
    let source = r#"
        use tokio::process::Command;
        
        pub async fn execute_with_retry(
            mut command: Command,
            description: &str,
        ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
            // This should NOT be flagged as blocking I/O
            match command.output().await {
                Ok(output) => Ok(output),
                Err(e) => Err(Box::new(e))
            }
        }
    "#;

    let file = parse_file(source).unwrap();

    // Collect performance data
    let data = collect_performance_data(&file, Path::new("test.rs"), source);

    // Check that no I/O operations were detected for the output() call
    // The output() method on tokio::process::Command should not be flagged
    for io_op in &data.io_operations {
        // If we find an I/O operation, check it's not the output() call
        if io_op.operation_type == IOType::ProcessSpawn {
            panic!("tokio::process::Command::output() incorrectly flagged as ProcessSpawn I/O");
        }
    }

    println!("✅ tokio::process::Command::output() not flagged as I/O");
}

#[test]
fn test_std_process_command_new_is_flagged() {
    // std::process::Command::new should still be detected
    let source = r#"
        use std::process::Command;
        
        pub fn run_command() {
            let output = std::process::Command::new("echo")
                .arg("hello")
                .output()
                .unwrap();
        }
    "#;

    let file = parse_file(source).unwrap();

    // Collect performance data
    let data = collect_performance_data(&file, Path::new("test.rs"), source);

    // std::process::Command::new should be detected as ProcessSpawn
    let has_process_spawn = data
        .io_operations
        .iter()
        .any(|io| matches!(io.operation_type, IOType::ProcessSpawn));

    assert!(
        has_process_spawn,
        "std::process::Command::new should be detected as ProcessSpawn"
    );
}
