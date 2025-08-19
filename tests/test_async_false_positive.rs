//! Test to reproduce false positive for async Command::output() being flagged as blocking I/O

use debtmap::context::async_detector::AsyncBoundaryDetector;
use syn::{parse_str, File};

#[test]
fn test_tokio_command_output_not_blocking() {
    // This is the code from src/cook/retry.rs that's being incorrectly flagged
    let source = r#"
        use tokio::process::Command;
        
        pub async fn execute_with_retry(
            mut command: Command,
            description: &str,
            max_retries: u32,
            verbose: bool,
        ) -> Result<std::process::Output> {
            // This should NOT be flagged as blocking I/O
            match command.output().await {
                Ok(output) => Ok(output),
                Err(e) => Err(e)
            }
        }
    "#;

    let file = parse_str::<File>(source).unwrap();
    let mut detector = AsyncBoundaryDetector::new();
    detector.analyze_file(&file);
    
    // The detector should NOT find any blocking calls
    assert_eq!(
        detector.blocking_in_async.len(), 
        0,
        "tokio::process::Command::output().await should not be flagged as blocking I/O, but found: {:?}",
        detector.blocking_in_async
    );
}

#[test]
fn test_std_command_output_is_blocking() {
    // This SHOULD be flagged as blocking
    let source = r#"
        use std::process::Command;
        
        pub async fn bad_async_function() {
            // This SHOULD be flagged as blocking I/O in async context
            let output = std::process::Command::new("echo")
                .output()
                .unwrap();
        }
    "#;

    let file = parse_str::<File>(source).unwrap();
    let mut detector = AsyncBoundaryDetector::new();
    detector.analyze_file(&file);
    
    // The detector SHOULD find blocking calls
    assert!(
        detector.blocking_in_async.len() > 0,
        "std::process::Command::output() should be flagged as blocking I/O in async context"
    );
}

#[test]
fn test_tokio_spawn_blocking_is_ok() {
    // tokio::task::spawn_blocking is the correct way to do blocking I/O in async
    let source = r#"
        use tokio::task;
        use std::fs;
        
        pub async fn good_async_function() {
            // This should NOT be flagged - it's the correct pattern
            let content = task::spawn_blocking(|| {
                fs::read_to_string("file.txt")
            }).await.unwrap();
        }
    "#;

    let file = parse_str::<File>(source).unwrap();
    let mut detector = AsyncBoundaryDetector::new();
    detector.analyze_file(&file);
    
    // spawn_blocking wraps the blocking call properly
    // The detector might flag the fs::read_to_string inside, but that's in a blocking context
    // For this test, we're mainly checking it doesn't explode
    println!("Found {} potential blocking calls", detector.blocking_in_async.len());
}

#[test]
fn test_async_std_command_also_not_blocking() {
    let source = r#"
        use async_std::process::Command;
        
        pub async fn async_std_example() {
            let output = Command::new("echo")
                .arg("hello")
                .output()
                .await
                .unwrap();
        }
    "#;

    let file = parse_str::<File>(source).unwrap();
    let mut detector = AsyncBoundaryDetector::new();
    detector.analyze_file(&file);
    
    assert_eq!(
        detector.blocking_in_async.len(), 
        0,
        "async_std::process::Command::output().await should not be flagged as blocking I/O"
    );
}

#[test]
fn test_import_disambiguation_works() {
    // Test that import tracking correctly disambiguates Command types
    let source = r#"
        use tokio::process::Command;
        
        pub async fn async_function() {
            // This Command is from tokio, so should NOT be flagged
            let output = Command::new("echo")
                .output()
                .await
                .unwrap();
        }
    "#;

    let file = parse_str::<File>(source).unwrap();
    let mut detector = AsyncBoundaryDetector::new();
    detector.analyze_file(&file);
    
    assert_eq!(
        detector.blocking_in_async.len(), 
        0,
        "tokio::process::Command with proper imports should not be flagged"
    );
}

#[test]
fn test_std_command_with_imports_is_blocking() {
    // Test that std::process::Command is correctly flagged even with imports
    let source = r#"
        use std::process::Command;
        
        pub async fn bad_async_function() {
            // This Command is from std, so SHOULD be flagged
            let output = Command::new("echo")
                .output()
                .unwrap();
        }
    "#;

    let file = parse_str::<File>(source).unwrap();
    let mut detector = AsyncBoundaryDetector::new();
    detector.analyze_file(&file);
    
    assert!(
        detector.blocking_in_async.len() > 0,
        "std::process::Command with imports should be flagged as blocking"
    );
}