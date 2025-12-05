//! Demonstration of the composable pipeline architecture (Spec 209).
//!
//! This example shows how to use the type-safe pipeline builder to compose
//! analysis stages into reusable workflows.
//!
//! Run with: `cargo run --example pipeline_demo`

use debtmap::pipeline::{
    stage::{FallibleStage, PureStage},
    PipelineBuilder,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Composable Pipeline Demo (Spec 209) ===\n");

    // Example 1: Simple pure pipeline
    println!("Example 1: Simple Pure Pipeline");
    let pipeline1 = PipelineBuilder::new()
        .stage(PureStage::new("Generate Numbers", |()| vec![1, 2, 3, 4, 5]))
        .stage(PureStage::new("Double Each", |nums: Vec<i32>| {
            nums.into_iter().map(|n| n * 2).collect::<Vec<_>>()
        }))
        .stage(PureStage::new("Sum", |nums: Vec<i32>| {
            nums.into_iter().sum::<i32>()
        }))
        .with_progress()
        .build();

    let (result1, timings1) = pipeline1.execute_with_timing()?;
    println!("Result: {} (expected: 30)", result1);
    println!("Stages executed:");
    for timing in timings1 {
        println!("  - {}", timing.format());
    }
    println!();

    // Example 2: Pipeline with conditional stages
    println!("Example 2: Pipeline with Conditional Stages");
    let enable_extra_processing = true;

    let pipeline2 = PipelineBuilder::new()
        .stage(PureStage::new("Start", |()| 10))
        .when(enable_extra_processing, |p| {
            p.stage(PureStage::new("Extra Processing", |x: i32| x + 5))
        })
        .stage(PureStage::new("Final", |x: i32| x * 2))
        .build();

    let result2 = pipeline2.execute()?;
    println!("Result with extra processing: {} (expected: 30)", result2);
    println!();

    // Example 3: Pipeline with fallible stages
    println!("Example 3: Pipeline with Fallible Stages");
    use debtmap::errors::AnalysisError;

    let pipeline3 = PipelineBuilder::new()
        .stage(PureStage::new("Create Input", |()| "42".to_string()))
        .stage(FallibleStage::new("Parse", |s: String| {
            s.parse::<i32>()
                .map_err(|e| AnalysisError::parse(format!("Parse error: {}", e)))
        }))
        .stage(PureStage::new("Square", |x: i32| x * x))
        .build();

    match pipeline3.execute() {
        Ok(result3) => {
            println!("Result: {} (expected: 1764)", result3);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    println!();

    // Example 4: Multiple pipeline configurations
    println!("Example 4: Different Pipeline Configurations");

    fn create_pipeline(steps: usize) -> impl Fn() -> Result<i32, String> {
        move || {
            let mut builder = PipelineBuilder::new().stage(PureStage::new("Init", |()| 1));

            for i in 0..steps {
                let stage = PureStage::new(format!("Step {}", i + 1), |x: i32| x + 1);
                builder = builder.stage(stage);
            }

            let pipeline = builder.build();
            pipeline.execute().map_err(|e| e.to_string())
        }
    }

    let short_pipeline = create_pipeline(3);
    let long_pipeline = create_pipeline(10);

    println!("Short pipeline (3 steps): {}", short_pipeline()?);
    println!("Long pipeline (10 steps): {}", long_pipeline()?);
    println!();

    // Example 5: Standard example pipeline from configs
    println!("Example 5: Standard Example Pipeline");
    use debtmap::pipeline::configs::example_pipeline;

    let pipeline5 = example_pipeline();
    let (result5, _timings5) = pipeline5.execute_with_timing()?;
    println!("Result: {}", result5);
    println!();

    println!("=== Demo Complete ===");
    println!("\nKey Benefits:");
    println!("  ✓ Type-safe composition (incompatible stages won't compile)");
    println!("  ✓ Conditional stages (enable/disable features at runtime)");
    println!("  ✓ Progress reporting (automatic from stage names)");
    println!("  ✓ Timing information (performance analysis per stage)");
    println!("  ✓ Reusable configurations (share pipelines across codebase)");

    Ok(())
}
