//! Effect Pipeline Example - Demonstrating Stillwater Effects Integration (Spec 207)
//!
//! This example demonstrates debtmap's effect system for composing analysis pipelines
//! with pure core / imperative shell architecture.
//!
//! Run with: `cargo run --example effect_pipeline`

use debtmap::analyzers::effects::analyze_file_effect;
use debtmap::config::DebtmapConfig;
use debtmap::core::Language;
use debtmap::effects::{run_effect, AnalysisEffect};
use debtmap::io::effects::read_file_effect;
use std::path::PathBuf;
use stillwater::EffectExt;

fn main() -> anyhow::Result<()> {
    println!("Effect Pipeline Example\n");
    println!("Demonstrating Stillwater effects for pure functional composition\n");

    // Example 1: Simple effect composition
    println!("=== Example 1: Basic Effect Composition ===\n");
    example_basic_composition()?;

    // Example 2: Pure transformations injected between I/O
    println!("\n=== Example 2: Pure Transformations ===\n");
    example_pure_transformations()?;

    // Example 3: Error handling with effects
    println!("\n=== Example 3: Effect Error Handling ===\n");
    example_error_handling()?;

    println!("\n=== All Examples Complete ===");
    Ok(())
}

/// Example 1: Basic effect composition
///
/// Demonstrates chaining I/O operations (read file -> analyze) using `.and_then()`
fn example_basic_composition() -> anyhow::Result<()> {
    // Find a Rust file in the current project
    let config = DebtmapConfig::default();

    // Create a pipeline: read file -> analyze -> extract function count
    let pipeline = create_analysis_pipeline("src/lib.rs".into());

    // Execute the effect
    let function_count = run_effect(pipeline, config)?;

    println!("Found {} functions in src/lib.rs", function_count);
    Ok(())
}

/// Example 2: Pure transformations
///
/// Shows how to inject pure functions between I/O operations using `.map()`
fn example_pure_transformations() -> anyhow::Result<()> {
    let config = DebtmapConfig::default();

    // Pipeline with pure transformations
    let pipeline = read_file_effect("src/lib.rs".into())
        .map(|content| {
            // Pure transformation: count lines
            println!("  [Pure] Counting lines...");
            content.lines().count()
        })
        .map(|line_count| {
            // Another pure transformation: calculate estimated complexity
            println!("  [Pure] Estimating complexity from {} lines", line_count);
            (line_count as f64 / 10.0).ceil() as usize
        })
        .boxed(); // Box the effect

    let estimated_complexity = run_effect(pipeline, config)?;
    println!("Estimated complexity: {}", estimated_complexity);
    Ok(())
}

/// Example 3: Error handling
///
/// Demonstrates effect error handling and recovery
fn example_error_handling() -> anyhow::Result<()> {
    let config = DebtmapConfig::default();

    // Try to read a file that might not exist
    let pipeline = read_file_effect("nonexistent.rs".into())
        .map(|content| content.len())
        .boxed(); // Box the effect

    match run_effect(pipeline, config) {
        Ok(len) => println!("File length: {}", len),
        Err(e) => {
            println!("Expected error occurred: {}", e);
            println!("(This demonstrates effect error handling)");
        }
    }

    Ok(())
}

// Helper function demonstrating effect composition
fn create_analysis_pipeline(path: PathBuf) -> AnalysisEffect<usize> {
    // Step 1: Read file (I/O effect)
    read_file_effect(path.clone())
        // Step 2: Analyze file (I/O effect - uses tree-sitter parser)
        .and_then(move |content| {
            println!("  [I/O] Analyzing file: {}", path.display());
            analyze_file_effect(path, content, Language::Rust)
        })
        // Step 3: Extract function count (pure transformation)
        .map(|metrics| {
            println!("  [Pure] Extracting metrics...");
            metrics.complexity.functions.len()
        })
        .boxed() // Box the effect to match return type
}

// Advanced example: Processing multiple files (commented out - requires walk_dir_effect)
// #[allow(dead_code)]
// fn example_batch_processing() -> anyhow::Result<()> {
//     let config = DebtmapConfig::default();
//
//     // Discover Rust files in src/ directory
//     let discover_effect = walk_dir_effect("src".into()).map(|paths| {
//         paths
//             .into_iter()
//             .filter(|p| {
//                 p.extension()
//                     .and_then(|e| e.to_str())
//                     .map(|e| e == "rs")
//                     .unwrap_or(false)
//             })
//             .take(5) // Limit to first 5 files for demo
//             .collect::<Vec<_>>()
//     });
//
//     let files = run_effect(discover_effect, config)?;
//     println!("Discovered {} Rust files", files.len());
//
//     Ok(())
// }

// Demonstrating the Reader pattern for config access (commented out - type mismatch)
// The Reader pattern is available but requires boxing the effect or using run_effect_with_env
// #[allow(dead_code)]
// fn example_reader_pattern() -> anyhow::Result<()> {
//     use debtmap::effects::asks_config;
//
//     let config = DebtmapConfig::default();
//
//     // Access config through the environment (Reader pattern)
//     let get_patterns = asks_config(|config| config.get_ignore_patterns()).boxed();
//
//     let patterns = run_effect(get_patterns, config)?;
//     println!("Ignore patterns: {:?}", patterns);
//
//     Ok(())
// }
