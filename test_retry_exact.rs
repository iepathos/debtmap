// Test program to check if our detector works with the exact retry.rs pattern
use debtmap::context::async_detector::AsyncBoundaryDetector;
use syn::{parse_file, File};

fn main() {
    // Exact code from retry.rs
    let source = r#"
use crate::abstractions::{ClaudeClient, RealClaudeClient};
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::sleep;

pub async fn execute_with_retry(
    mut command: Command,
    description: &str,
    max_retries: u32,
    verbose: bool,
) -> Result<std::process::Output> {
    let mut attempt = 0;
    let mut last_error = None;

    while attempt <= max_retries {
        if attempt > 0 {
            await_retry_delay(attempt, description, max_retries, verbose).await;
        }

        match command.output().await {
            Ok(output) => {
                return Ok(output);
            }
            Err(e) => {
                let retry_result =
                    handle_command_error(e, description, attempt, max_retries, verbose)?;
                if let Some(error_msg) = retry_result {
                    last_error = Some(error_msg);
                    attempt += 1;
                    continue;
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "Failed {} after {} retries. Last error: {}",
        description,
        max_retries,
        last_error.unwrap_or_else(|| "Unknown error".to_string())
    ))
}
    "#;

    let file = parse_file(source).unwrap();
    let mut detector = AsyncBoundaryDetector::new();
    detector.analyze_file(&file);
    
    println!("Detected {} blocking calls", detector.blocking_in_async.len());
    for call in &detector.blocking_in_async {
        println!("  - {}", call.function_name);
    }
    
    if detector.blocking_in_async.is_empty() {
        println!("✅ No blocking I/O detected (correct!)");
    } else {
        println!("❌ False positive: tokio::process::Command flagged as blocking!");
    }
}