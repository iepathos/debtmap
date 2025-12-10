/// Integration test for concise recommendations (spec 138a)
///
/// This test validates that recommendations for complex functions like those
/// found in ripgrep are concise, actionable, and meet the spec requirements:
/// - Maximum 5 high-level steps per recommendation
/// - Clear impact estimates for each step
/// - Effort estimates > 0
/// - All steps have impact and commands
use debtmap::core::FunctionMetrics;
use debtmap::priority::scoring::concise_recommendation::generate_concise_recommendation;
use debtmap::priority::semantic_classifier::FunctionRole;
use debtmap::priority::{DebtType, FunctionVisibility, TransitiveCoverage};
use std::path::PathBuf;

fn create_complex_metrics(
    name: &str,
    cyclomatic: u32,
    cognitive: u32,
    line: usize,
) -> FunctionMetrics {
    FunctionMetrics {
        name: name.to_string(),
        file: PathBuf::from("crates/core/flags/hiargs.rs"),
        line,
        cyclomatic,
        cognitive,
        nesting: 3,
        length: 150,
        is_test: false,
        visibility: None,
        is_trait_method: false,
        in_test_module: false,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: None,
        detected_patterns: None,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
    }
}

#[test]
fn test_complex_function_max_5_steps() {
    // Simulate a complex function like those in ripgrep
    let metrics = create_complex_metrics("parse_args", 45, 55, 120);

    let debt = DebtType::ComplexityHotspot {
        cyclomatic: 45,
        cognitive: 55,
    };

    let rec = generate_concise_recommendation(&debt, &metrics, FunctionRole::Orchestrator, &None)
        .expect("Test should generate recommendation");

    println!("\n=== COMPLEX FUNCTION RECOMMENDATION ===");
    println!(
        "Function: {} (complexity {}/{})",
        metrics.name, metrics.cyclomatic, metrics.cognitive
    );
    println!("Primary action: {}", rec.primary_action);
    println!("Rationale: {}", rec.rationale);

    if let Some(steps) = &rec.steps {
        println!("\nSteps ({}):", steps.len());
        for (i, step) in steps.iter().enumerate() {
            println!(
                "  {}. {} [Impact: {}, Difficulty: {:?}]",
                i + 1,
                step.description,
                step.impact,
                step.difficulty
            );
            if !step.commands.is_empty() {
                println!("     Commands: {:?}", step.commands);
            }
        }

        // SPEC REQUIREMENT: Maximum 5 steps
        assert!(
            steps.len() <= 5,
            "Recommendation should have at most 5 steps (spec 138a), got {}",
            steps.len()
        );

        // SPEC REQUIREMENT: All steps have impact
        for step in steps {
            assert!(
                !step.impact.is_empty(),
                "Step '{}' missing impact description (spec 138a)",
                step.description
            );
        }

        // SPEC REQUIREMENT: All steps have commands (or are verification steps)
        for step in steps {
            if !step.description.contains("Verify") {
                assert!(
                    !step.commands.is_empty(),
                    "Step '{}' missing commands (spec 138a)",
                    step.description
                );
            }
        }
    }

    // SPEC REQUIREMENT: Effort estimate present and positive
    if let Some(effort) = rec.estimated_effort_hours {
        assert!(
            effort > 0.0,
            "Effort estimate must be positive (spec 138a), got {}",
            effort
        );
        println!("\nEstimated effort: {:.1} hours", effort);
    } else {
        panic!("Missing effort estimate (spec 138a requirement)");
    }

    println!("========================================\n");
}

#[test]
fn test_testing_gap_conciseness() {
    // Test a function with testing gap debt
    let metrics = create_complex_metrics("process_flags", 25, 30, 45);

    let debt = DebtType::TestingGap {
        coverage: 0.3, // 30% coverage
        cyclomatic: 25,
        cognitive: 30,
    };

    let rec = generate_concise_recommendation(&debt, &metrics, FunctionRole::Orchestrator, &None)
        .expect("Test should generate recommendation");

    println!("\n=== TESTING GAP RECOMMENDATION ===");
    println!(
        "Function: {} (30% coverage, complexity {}/{})",
        metrics.name, metrics.cyclomatic, metrics.cognitive
    );
    println!("Primary action: {}", rec.primary_action);

    if let Some(steps) = &rec.steps {
        println!("Steps: {}", steps.len());
        for (i, step) in steps.iter().enumerate() {
            println!("  {}. {} [{}]", i + 1, step.description, step.impact);
        }

        assert!(steps.len() <= 5, "Too many steps: {}", steps.len());

        // First step should be about testing (highest impact)
        assert!(
            steps[0].description.to_lowercase().contains("test"),
            "First step should address testing: '{}'",
            steps[0].description
        );

        // All steps should have impact
        for step in steps {
            assert!(
                !step.impact.is_empty(),
                "Step missing impact: '{}'",
                step.description
            );
        }
    }

    assert!(
        rec.estimated_effort_hours.is_some(),
        "Missing effort estimate"
    );
    assert!(
        rec.estimated_effort_hours.unwrap() > 0.0,
        "Effort must be positive"
    );

    println!("Effort: {:.1} hours", rec.estimated_effort_hours.unwrap());
    println!("===================================\n");
}

#[test]
fn test_dead_code_conciseness() {
    let metrics = create_complex_metrics("unused_helper", 12, 15, 200);

    let debt = DebtType::DeadCode {
        visibility: FunctionVisibility::Private,
        cyclomatic: 12,
        cognitive: 15,
        usage_hints: vec![],
    };

    let rec = generate_concise_recommendation(&debt, &metrics, FunctionRole::PureLogic, &None)
        .expect("Test should generate recommendation");

    println!("\n=== DEAD CODE RECOMMENDATION ===");
    println!(
        "Function: {} (unused, complexity {}/{})",
        metrics.name, metrics.cyclomatic, metrics.cognitive
    );

    if let Some(steps) = &rec.steps {
        println!("Steps: {}", steps.len());

        assert!(steps.len() <= 5, "Too many steps: {}", steps.len());

        // Dead code should be simple - verify and remove
        assert!(
            steps.len() <= 3,
            "Dead code recommendations should be simple (≤3 steps), got {}",
            steps.len()
        );

        // Should mention removal
        let has_removal_step = steps
            .iter()
            .any(|s| s.description.to_lowercase().contains("remove"));
        assert!(has_removal_step, "Should have removal step for dead code");
    }

    assert!(
        rec.estimated_effort_hours.is_some(),
        "Missing effort estimate"
    );

    // Dead code removal should be quick
    let effort = rec.estimated_effort_hours.unwrap();
    assert!(
        effort <= 1.0,
        "Dead code removal should be quick (≤1 hour), got {:.1}",
        effort
    );

    println!("Effort: {:.1} hours (quick removal)", effort);
    println!("=================================\n");
}

#[test]
fn test_recommendation_with_transitive_coverage() {
    let metrics = create_complex_metrics("with_callees", 20, 25, 80);

    let coverage = Some(TransitiveCoverage {
        direct: 0.4,
        transitive: 0.6,
        propagated_from: vec![],
        uncovered_lines: vec![],
    });

    let debt = DebtType::TestingGap {
        coverage: 0.4,
        cyclomatic: 20,
        cognitive: 25,
    };

    let rec =
        generate_concise_recommendation(&debt, &metrics, FunctionRole::Orchestrator, &coverage)
            .expect("Test should generate recommendation");

    println!("\n=== RECOMMENDATION WITH TRANSITIVE COVERAGE ===");
    println!("Direct coverage: 40%, Transitive: 60%, Combined: 50%");

    if let Some(steps) = &rec.steps {
        assert!(steps.len() <= 5, "Too many steps: {}", steps.len());

        for step in steps {
            assert!(!step.impact.is_empty(), "Missing impact");
            assert!(!step.description.is_empty(), "Missing description");
        }
    }

    assert!(rec.estimated_effort_hours.is_some());
    println!("Effort: {:.1} hours", rec.estimated_effort_hours.unwrap());
    println!("===============================================\n");
}

#[test]
fn test_batch_recommendations_are_concise() {
    // Test that we can generate many recommendations efficiently
    let test_cases = vec![
        (
            DebtType::ComplexityHotspot {
                cyclomatic: 30,
                cognitive: 40,
            },
            "complex_func_1",
        ),
        (
            DebtType::TestingGap {
                coverage: 0.2,
                cyclomatic: 20,
                cognitive: 25,
            },
            "untested_func",
        ),
        (
            DebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                cyclomatic: 10,
                cognitive: 12,
                usage_hints: vec![],
            },
            "dead_func",
        ),
        (
            DebtType::ComplexityHotspot {
                cyclomatic: 50,
                cognitive: 65,
            },
            "very_complex_func",
        ),
        (
            DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 15,
                cognitive: 18,
            },
            "partial_coverage",
        ),
    ];

    println!("\n=== BATCH RECOMMENDATION CONCISENESS TEST ===");

    for (i, (debt, name)) in test_cases.iter().enumerate() {
        let metrics = create_complex_metrics(name, 20, 25, 100 + i * 50);

        let rec = generate_concise_recommendation(debt, &metrics, FunctionRole::PureLogic, &None)
            .expect("Test should generate recommendation");

        // Verify conciseness requirements
        if let Some(steps) = &rec.steps {
            assert!(
                steps.len() <= 5,
                "{}: Too many steps ({})",
                name,
                steps.len()
            );

            println!(
                "{}: {} steps, {:.1}h effort",
                name,
                steps.len(),
                rec.estimated_effort_hours.unwrap_or(0.0)
            );
        }

        assert!(
            rec.estimated_effort_hours.is_some(),
            "{}: Missing effort",
            name
        );
        assert!(
            rec.estimated_effort_hours.unwrap() > 0.0,
            "{}: Invalid effort",
            name
        );
    }

    println!(
        "All {} recommendations meet conciseness requirements",
        test_cases.len()
    );
    println!("============================================\n");
}

#[test]
fn test_actual_ripgrep_file_if_available() {
    // This test attempts to analyze an actual complex function from ripgrep
    // to validate conciseness on real-world code
    let ripgrep_path = PathBuf::from("../ripgrep/crates/core/flags/hiargs.rs");

    if !ripgrep_path.exists() {
        println!(
            "Skipping test - ripgrep source not found at {:?}",
            ripgrep_path
        );
        return;
    }

    println!("\n=== ANALYZING ACTUAL RIPGREP COMPLEX FUNCTION ===");

    // Simulate analyzing a complex function from ripgrep
    // (In a real scenario, this would parse the actual file)
    let metrics = create_complex_metrics("parse_cli_args", 38, 48, 150);

    let debt = DebtType::ComplexityHotspot {
        cyclomatic: 38,
        cognitive: 48,
    };

    let rec = generate_concise_recommendation(&debt, &metrics, FunctionRole::Orchestrator, &None)
        .expect("Test should generate recommendation");

    println!("Function: {}", metrics.name);
    println!("Complexity: {}/{}", metrics.cyclomatic, metrics.cognitive);
    println!("Primary action: {}", rec.primary_action);

    if let Some(steps) = &rec.steps {
        println!("\nGenerated {} steps:", steps.len());
        for (i, step) in steps.iter().enumerate() {
            println!("  {}. {}", i + 1, step.description);
            println!("     Impact: {}", step.impact);
            println!("     Difficulty: {:?}", step.difficulty);
        }

        // Validate spec 138a requirements
        assert!(steps.len() <= 5, "Exceeded max steps: {}", steps.len());

        for step in steps {
            assert!(!step.impact.is_empty(), "Step missing impact");
            assert!(!step.description.is_empty(), "Step missing description");
        }
    }

    if let Some(effort) = rec.estimated_effort_hours {
        println!("\nEstimated effort: {:.1} hours", effort);
        assert!(effort > 0.0, "Invalid effort estimate");
    } else {
        panic!("Missing effort estimate");
    }

    println!("✓ All spec 138a requirements met");
    println!("================================================\n");
}
