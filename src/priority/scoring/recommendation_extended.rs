// Extended recommendation generation functions for debt items
// This file contains the additional generate_* functions extracted from unified_scorer.rs

use crate::core::FunctionMetrics;
use crate::priority::call_graph::{CallGraph, FunctionId};
use crate::priority::{DebtType, FunctionVisibility, TransitiveCoverage};

/// Enum for complexity classification
#[derive(Debug, Clone)]
enum ComplexityLevel {
    Low,         // 1-4
    LowModerate, // 5-6
    Moderate,    // 7-10
    High,        // 11+
}

/// Classify complexity level based on cyclomatic complexity
fn classify_complexity_level(cyclo: u32) -> ComplexityLevel {
    match cyclo {
        1..=4 => ComplexityLevel::Low,
        5..=6 => ComplexityLevel::LowModerate,
        7..=10 => ComplexityLevel::Moderate,
        _ => ComplexityLevel::High,
    }
}

/// Helper to determine visibility from function
fn determine_visibility(func: &FunctionMetrics) -> FunctionVisibility {
    // Try to extract visibility from metrics if available
    let vis_str = func.visibility.as_deref();
    match vis_str {
        Some("pub") => FunctionVisibility::Public,
        Some("pub(crate)") => FunctionVisibility::Crate,
        Some(vis) if vis.starts_with("pub(") => FunctionVisibility::Crate, // pub(super), pub(in ...), etc.
        _ => FunctionVisibility::Private,
    }
}

/// Generate enhanced dead code hints
fn generate_enhanced_dead_code_hints(
    func: &FunctionMetrics,
    visibility: &FunctionVisibility,
) -> Vec<String> {
    let mut hints = Vec::new();

    // Add visibility-based hints
    match visibility {
        FunctionVisibility::Public => {
            hints.push("Public function - verify not used by external crates".to_string());
        }
        FunctionVisibility::Private => {
            hints.push("Private function - safe to remove if no local callers".to_string());
        }
        FunctionVisibility::Crate => {
            hints.push("Crate-visible function - check for usage within crate".to_string());
        }
    }

    // Add file-based hints
    let file_str = func.file.to_string_lossy();
    if file_str.contains("test") {
        hints.push("Test-related function - may be test helper".to_string());
    }

    if func.name.starts_with("test_") {
        hints.push("Test function - verify no test dependencies".to_string());
    }

    hints
}

/// Generate usage hints for dead code analysis
pub fn generate_usage_hints(
    func: &FunctionMetrics,
    call_graph: &CallGraph,
    func_id: &FunctionId,
) -> Vec<String> {
    let visibility = determine_visibility(func);

    // Use enhanced dead code hints
    let mut hints = generate_enhanced_dead_code_hints(func, &visibility);

    // Add call graph information
    let callees = call_graph.get_callees(func_id);
    if callees.is_empty() {
        hints.push("Function has no dependencies - safe to remove".to_string());
    } else {
        hints.push(format!("Function calls {} other functions", callees.len()));
    }

    hints
}

/// Generate complexity-based recommendation for risk debt
pub fn generate_complexity_risk_recommendation(
    cyclo: u32,
    coverage: &Option<TransitiveCoverage>,
    factors: &[String],
) -> (String, String, Vec<String>) {
    let complexity_level = classify_complexity_level(cyclo);
    let has_good_coverage = coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false);
    let has_coverage_issue = factors
        .iter()
        .any(|f| f.contains("coverage") || f.contains("Coverage") || f.contains("uncovered"));

    match complexity_level {
        ComplexityLevel::Low => generate_low_complexity_recommendation(cyclo, has_coverage_issue),
        ComplexityLevel::LowModerate => {
            generate_low_moderate_complexity_recommendation(cyclo, has_good_coverage)
        }
        ComplexityLevel::Moderate => {
            generate_moderate_complexity_recommendation(cyclo, has_good_coverage)
        }
        ComplexityLevel::High => {
            generate_high_complexity_recommendation(cyclo, has_good_coverage, has_coverage_issue)
        }
    }
}

/// Generate recommendation for low complexity functions
fn generate_low_complexity_recommendation(
    cyclo: u32,
    has_coverage_issue: bool,
) -> (String, String, Vec<String>) {
    let action = if has_coverage_issue || cyclo > 3 {
        format!(
            "Extract helper functions for clarity, then add {} unit tests",
            cyclo.max(3)
        )
    } else {
        "Simplify function structure and improve testability".to_string()
    };

    (
        action,
        "Low complexity but flagged for improvement".to_string(),
        vec![
            "Extract helper functions for clarity".to_string(),
            "Remove unnecessary branching".to_string(),
            "Consolidate similar code paths".to_string(),
            format!(
                "Add {} unit tests for edge cases and main paths",
                cyclo.max(3)
            ),
        ],
    )
}

/// Generate recommendation for low-moderate complexity functions (5-6)
fn generate_low_moderate_complexity_recommendation(
    cyclo: u32,
    has_good_coverage: bool,
) -> (String, String, Vec<String>) {
    // For cyclomatic 5-6, extract 2 functions
    let functions_to_extract = 2;
    let target_complexity = 3;

    let action = if has_good_coverage {
        format!(
            "Extract {} pure functions (complexity {} → {})",
            functions_to_extract, cyclo, target_complexity
        )
    } else {
        format!(
            "Extract {} pure functions (complexity {} → {}) and add comprehensive tests",
            functions_to_extract, cyclo, target_complexity
        )
    };

    let mut steps = vec![
        format!(
            "Identify {} logical sections from {} branches:",
            functions_to_extract, cyclo
        ),
        format!(
            "  • Look for groups of ~{} related conditions",
            cyclo / functions_to_extract.max(1)
        ),
        format!(
            "  • Each extracted function should have complexity ≤{}",
            target_complexity
        ),
        "Extraction candidates:".to_string(),
        "  • Validation logic → validate_preconditions()".to_string(),
        "  • Main logic → process_core()".to_string(),
        "Move all I/O operations to a single orchestrator function".to_string(),
    ];

    if !has_good_coverage {
        steps.push(format!(
            "Write {} unit tests for the extracted pure functions",
            functions_to_extract * 3
        ));
        steps.push("Achieve 80%+ test coverage for all functions".to_string());
    } else {
        steps.push(format!(
            "Goal: Reduce cyclomatic complexity from {} to ≤{}",
            cyclo, target_complexity
        ));
    }

    (
        action,
        "Low-moderate complexity requiring refactoring".to_string(),
        steps,
    )
}

/// Generate recommendation for moderate complexity functions (7-10)
fn generate_moderate_complexity_recommendation(
    cyclo: u32,
    has_good_coverage: bool,
) -> (String, String, Vec<String>) {
    let functions_to_extract = (cyclo / 3).max(2);
    let target_complexity = 3;

    let action = if has_good_coverage {
        format!(
            "Extract {} pure functions (complexity {} → {})",
            functions_to_extract, cyclo, target_complexity
        )
    } else {
        format!(
            "Extract {} pure functions (complexity {} → {}) and add comprehensive tests",
            functions_to_extract, cyclo, target_complexity
        )
    };

    let mut steps = vec![
        format!(
            "Identify {} logical sections from {} branches:",
            functions_to_extract, cyclo
        ),
        format!(
            "  • Look for groups of ~{} related conditions",
            cyclo / functions_to_extract.max(1)
        ),
        format!(
            "  • Each extracted function should have complexity ≤{}",
            target_complexity
        ),
        "Extraction candidates:".to_string(),
        "  • Validation logic → validate_preconditions()".to_string(),
        "  • Complex calculations → calculate_specific()".to_string(),
        "  • Loop processing → process_items()".to_string(),
        "Move all I/O operations to a single orchestrator function".to_string(),
    ];

    if !has_good_coverage {
        steps.push(format!(
            "Write {} unit tests for the extracted pure functions",
            functions_to_extract * 3
        ));
        steps.push("Achieve 80%+ test coverage for all functions".to_string());
    } else {
        steps.push(format!(
            "Goal: Reduce cyclomatic complexity from {} to ≤{}",
            cyclo, target_complexity
        ));
    }

    (
        action,
        "Moderate complexity requiring refactoring".to_string(),
        steps,
    )
}

/// Generate recommendation for high complexity functions (11+)
fn generate_high_complexity_recommendation(
    cyclo: u32,
    has_good_coverage: bool,
    has_coverage_issue: bool,
) -> (String, String, Vec<String>) {
    let functions_to_extract = (cyclo / 4).max(3);
    let target_complexity = 5;

    let action = if has_good_coverage {
        format!(
            "Decompose into {} pure functions (complexity {} → {})",
            functions_to_extract, cyclo, target_complexity
        )
    } else if has_coverage_issue {
        format!(
            "Add {} tests, then decompose into {} pure functions (complexity {} → {})",
            cyclo, functions_to_extract, cyclo, target_complexity
        )
    } else {
        format!(
            "Decompose into {} pure functions (complexity {} → {}) with comprehensive tests",
            functions_to_extract, cyclo, target_complexity
        )
    };

    let mut steps = vec![
        format!(
            "This high-complexity function needs to be broken down into {} logical units:",
            functions_to_extract
        ),
        format!("1. Map {} execution paths into logical groupings:", cyclo),
        format!("  • ~{} paths for input validation", cyclo / 4),
        format!("  • ~{} paths for core logic", cyclo / 2),
        format!("  • ~{} paths for output/error handling", cyclo / 4),
    ];

    if has_coverage_issue && !has_good_coverage {
        steps.extend(vec![
            format!(
                "2. Add {} unit tests before refactoring to prevent regressions:",
                cyclo
            ),
            "  • Focus on happy path and major error conditions first".to_string(),
            "  • Cover all branches for confidence in refactoring".to_string(),
        ]);
    }

    steps.extend(vec![
        format!(
            "{}. Extract functions with single responsibilities:",
            if has_coverage_issue && !has_good_coverage {
                3
            } else {
                2
            }
        ),
        "  • validate_inputs() for precondition checks".to_string(),
        "  • process_core_logic() for main algorithm".to_string(),
        "  • handle_results() for output formatting".to_string(),
        "  • handle_errors() for error cases".to_string(),
        format!(
            "{}. Each function should have cyclomatic complexity ≤{}",
            if has_coverage_issue && !has_good_coverage {
                4
            } else {
                3
            },
            target_complexity
        ),
        format!(
            "{}. Add unit tests for each extracted function (~3-5 tests per function)",
            if has_coverage_issue && !has_good_coverage {
                5
            } else {
                4
            }
        ),
    ]);

    (
        action,
        "High complexity requiring decomposition".to_string(),
        steps,
    )
}

/// Generate recommendation for infrastructure debt types (duplication, risk)
pub fn generate_infrastructure_recommendation_with_coverage(
    debt_type: &DebtType,
    coverage: &Option<TransitiveCoverage>,
) -> (String, String, Vec<String>) {
    match debt_type {
        DebtType::Duplication {
            instances,
            total_lines,
        } => (
            "Extract common logic into shared module".to_string(),
            format!("Duplicated across {instances} locations ({total_lines} lines total)"),
            vec![
                "Create shared utility module".to_string(),
                "Replace duplicated code with calls to shared module".to_string(),
                "Add comprehensive tests to shared module".to_string(),
            ],
        ),
        DebtType::Risk {
            risk_score,
            factors,
        } => {
            // Check if any factor mentions complexity to provide more specific recommendations
            let has_complexity_issue = factors.iter().any(|f| {
                f.contains("complexity") || f.contains("cyclomatic") || f.contains("cognitive")
            });

            if has_complexity_issue {
                // Extract complexity values from factors string if present
                let cyclo = extract_cyclomatic_from_factors(factors).unwrap_or(0);
                let (action, _, steps) =
                    generate_complexity_risk_recommendation(cyclo, coverage, factors);
                (
                    action,
                    format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                    steps,
                )
            } else {
                // Non-complexity related risk
                (
                    "Address identified risk factors".to_string(),
                    format!("Risk score {:.1}: {}", risk_score, factors.join(", ")),
                    vec![
                        "Review and refactor problematic areas".to_string(),
                        "Add missing tests if coverage is low".to_string(),
                        "Update documentation".to_string(),
                    ],
                )
            }
        }
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        } => generate_complexity_hotspot_recommendation(*cyclomatic, *cognitive),
        _ => unreachable!("Not an infrastructure debt type"),
    }
}

/// Extract cyclomatic complexity value from factors strings
fn extract_cyclomatic_from_factors(factors: &[String]) -> Option<u32> {
    factors
        .iter()
        .find(|f| f.contains("cyclomatic"))
        .and_then(|f| {
            f.split(':')
                .nth(1)?
                .trim()
                .strip_suffix(')')?
                .parse::<u32>()
                .ok()
        })
}

/// Generate recommendation for complexity hotspots
pub fn generate_complexity_hotspot_recommendation(
    cyclomatic: u32,
    cognitive: u32,
) -> (String, String, Vec<String>) {
    use crate::priority::scoring::recommendation::calculate_functions_to_extract;

    // Calculate extraction based on complexity distribution
    let functions_to_extract = calculate_functions_to_extract(cyclomatic, cognitive);
    let target_per_function = (cyclomatic / functions_to_extract).max(3);
    (
        format!(
            "Extract {} pure functions, each handling ~{} branches (complexity {} → ~{})",
            functions_to_extract,
            cyclomatic / functions_to_extract.max(1),
            cyclomatic,
            target_per_function
        ),
        format!(
            "High complexity function (cyclo={}, cog={}) likely with low coverage - needs testing and refactoring",
            cyclomatic, cognitive
        ),
        vec![
            format!("Identify {} branch clusters from {} total branches:", functions_to_extract, cyclomatic),
            format!("  • Each cluster should handle ~{} related conditions", cyclomatic / functions_to_extract.max(1)),
            "Common extraction patterns:".to_string(),
            "  • Early validation checks → validate_preconditions()".to_string(),
            "  • Complex calculations in branches → calculate_[specific]()".to_string(),  
            "  • Data processing in loops → process_[item_type]()".to_string(),
            "  • Error handling branches → handle_[error_case]()".to_string(),
            format!("Each extracted function should have cyclomatic complexity ≤{}", target_per_function),
            format!("Write ~{} tests per extracted function for full branch coverage", target_per_function),
            "Use property-based testing for complex logic validation".to_string(),
        ],
    )
}

/// Detect programming language from file path
fn detect_file_language(file_path: &std::path::Path) -> crate::core::Language {
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("rs");

    match extension {
        "rs" => crate::core::Language::Rust,
        "py" => crate::core::Language::Python,
        "js" | "jsx" | "ts" | "tsx" => crate::core::Language::JavaScript,
        _ => crate::core::Language::Rust, // Default to Rust
    }
}

/// Get pattern type name for display
fn pattern_type_name(
    pattern_type: &crate::extraction_patterns::ExtractablePattern,
) -> &'static str {
    use crate::extraction_patterns::ExtractablePattern;

    match pattern_type {
        ExtractablePattern::AccumulationLoop { .. } => "accumulation loop",
        ExtractablePattern::GuardChainSequence { .. } => "guard chain",
        ExtractablePattern::TransformationPipeline { .. } => "transformation pipeline",
        ExtractablePattern::SimilarBranches { .. } => "similar branches",
        ExtractablePattern::NestedExtraction { .. } => "nested extraction",
    }
}

/// Generate complexity recommendation using pattern analysis when available
pub fn generate_complexity_recommendation_with_patterns_and_coverage(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    use crate::extraction_patterns::{ExtractionAnalyzer, UnifiedExtractionAnalyzer};

    // Try to analyze extraction patterns
    let analyzer = UnifiedExtractionAnalyzer::new();

    // Create a minimal FileMetrics for the analyzer
    let file_metrics = crate::core::FileMetrics {
        path: func.file.clone(),
        language: detect_file_language(&func.file),
        complexity: crate::core::ComplexityMetrics::default(),
        debt_items: vec![],
        dependencies: vec![],
        duplications: vec![],
    };

    let suggestions = analyzer.analyze_function(func, &file_metrics, data_flow);

    // If we have intelligent suggestions from AST analysis, use them
    if !suggestions.is_empty() {
        // Generate pattern-based recommendation
        let mut action_parts = vec![];
        let mut steps = vec![];
        let mut total_complexity_reduction = 0u32;

        for (i, suggestion) in suggestions.iter().enumerate().take(3) {
            // Include top 3 patterns
            action_parts.push(format!(
                "{} (confidence: {:.0}%)",
                suggestion.suggested_name,
                suggestion.confidence * 100.0
            ));

            steps.push(format!(
                "{}. Extract {} pattern at lines {}-{} as '{}' (complexity {} → {})",
                i + 1,
                pattern_type_name(&suggestion.pattern_type),
                suggestion.start_line,
                suggestion.end_line,
                suggestion.suggested_name,
                suggestion.complexity_reduction.current_cyclomatic,
                suggestion.complexity_reduction.predicted_cyclomatic
            ));

            total_complexity_reduction += suggestion
                .complexity_reduction
                .current_cyclomatic
                .saturating_sub(suggestion.complexity_reduction.predicted_cyclomatic);
        }

        let predicted_complexity = cyclomatic.saturating_sub(total_complexity_reduction);

        // Create action with specific pattern names
        let action = if !action_parts.is_empty() {
            format!(
                "Extract {} to reduce complexity from {} to ~{}",
                action_parts.join(", "),
                cyclomatic,
                predicted_complexity
            )
        } else {
            format!(
                "Extract {} identified patterns to reduce complexity from {} to {}",
                suggestions.len(),
                cyclomatic,
                predicted_complexity
            )
        };

        // Provide detailed explanation of why these extractions are recommended
        let pattern_benefits = match suggestions.len() {
            1 => "This extraction will create a focused, testable unit".to_string(),
            2 => "These extractions will separate distinct concerns into testable units".to_string(),
            _ => format!("These {} extractions will decompose the function into smaller, focused units that are easier to test and understand", suggestions.len()),
        };

        let complexity_explanation = if cyclomatic > 15 {
            format!("Cyclomatic complexity of {} indicates {} independent execution paths, requiring at least {} test cases for full path coverage", 
                    cyclomatic, cyclomatic, cyclomatic)
        } else if cyclomatic > 10 {
            format!("Cyclomatic complexity of {} indicates {} independent paths through the code, making thorough testing difficult", 
                    cyclomatic, cyclomatic)
        } else if cyclomatic > 5 {
            format!("Cyclomatic complexity of {} indicates {} independent paths requiring {} test cases minimum - extraction will reduce this to 3-5 tests per function",
                    cyclomatic, cyclomatic, cyclomatic)
        } else {
            format!("Cyclomatic complexity of {} indicates moderate complexity that can be improved through extraction", cyclomatic)
        };

        let rationale = format!(
            "{}. Function has {} extractable patterns that can be isolated. {}. Target complexity per function is 5 or less for optimal maintainability.",
            complexity_explanation,
            suggestions.len(),
            pattern_benefits
        );

        // Add testing steps only if coverage is low
        let has_good_coverage = coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false);

        if !has_good_coverage {
            // Add uncovered lines information if available
            if let Some(cov) = coverage {
                if !cov.uncovered_lines.is_empty() {
                    use crate::priority::scoring::recommendation::analyze_uncovered_lines;
                    let uncovered_recommendations =
                        analyze_uncovered_lines(func, &cov.uncovered_lines);
                    for (i, rec) in uncovered_recommendations.into_iter().enumerate() {
                        steps.insert(i, rec);
                    }
                }
            }

            steps.push(format!(
                "{}. Write unit tests for each extracted pure function",
                suggestions.len() + 2
            ));
            steps.push(format!(
                "{}. Add property-based tests for complex transformations",
                suggestions.len() + 3
            ));
        }

        steps.push(format!(
            "Expected complexity reduction: {}%",
            (total_complexity_reduction as f32 / cyclomatic as f32 * 100.0) as u32
        ));

        (action, rationale, steps)
    } else {
        // Fall back to heuristic recommendations with estimated line ranges
        generate_heuristic_recommendations_with_line_estimates(
            func, cyclomatic, cognitive, coverage, data_flow,
        )
    }
}

/// Generate recommendations based on data flow analysis when AST is unavailable
pub fn generate_heuristic_recommendations_with_line_estimates(
    func: &FunctionMetrics,
    cyclomatic: u32,
    cognitive: u32,
    coverage: &Option<TransitiveCoverage>,
    data_flow: Option<&crate::data_flow::DataFlowGraph>,
) -> (String, String, Vec<String>) {
    // Analyze function characteristics from available metrics
    let has_high_branching = cyclomatic > 7;
    let has_deep_nesting = func.nesting > 3;
    let is_pure = func.is_pure.unwrap_or(false);
    let purity_confidence = func.purity_confidence.unwrap_or(0.0);

    // Get variable dependencies if data flow is available
    let num_dependencies = if let Some(df) = data_flow {
        let func_id = crate::priority::call_graph::FunctionId {
            file: func.file.clone(),
            name: func.name.clone(),
            line: func.line,
        };
        df.get_variable_dependencies(&func_id)
            .map(|d| d.len())
            .unwrap_or(0)
    } else {
        0
    };

    // Generate targeted recommendations based on patterns
    let mut steps = Vec::new();
    let mut suggested_extractions = Vec::new();
    let mut complexity_reduction = 0;

    if has_high_branching {
        suggested_extractions.push("validation logic");
        steps.push(format!(
            "Identify validation checks from {} branches → extract as validate_*()",
            cyclomatic / 4
        ));
        complexity_reduction += cyclomatic / 4;
    }

    if has_deep_nesting {
        suggested_extractions.push("nested processing");
        steps.push(format!(
            "Extract nested logic (depth {}) → process_*() functions",
            func.nesting
        ));
        complexity_reduction += 2;
    }

    if cognitive > cyclomatic * 2 {
        suggested_extractions.push("complex calculations");
        steps.push(format!(
            "Extract calculations from {} cognitive complexity → calculate_*()",
            cognitive / 5
        ));
        complexity_reduction += cognitive / 5;
    }

    if num_dependencies > 5 {
        suggested_extractions.push("data transformation pipeline");
        steps.push(format!(
            "Create data transformation pipeline to manage {} dependencies",
            num_dependencies
        ));
        complexity_reduction += 1;
    }

    if is_pure && purity_confidence > 0.8 {
        steps.push(
            "Function is likely pure - focus on breaking down into smaller pure functions"
                .to_string(),
        );
    } else if purity_confidence < 0.3 {
        steps.push("Isolate side effects at function boundaries before extraction".to_string());
    }

    // Add testing recommendation only if coverage is low
    let has_good_coverage = coverage.as_ref().map(|c| c.direct >= 0.8).unwrap_or(false);

    // Add uncovered lines info if available
    if let Some(cov) = coverage {
        if !cov.uncovered_lines.is_empty() && !has_good_coverage {
            use crate::priority::scoring::recommendation::analyze_uncovered_lines;
            let uncovered_recommendations = analyze_uncovered_lines(func, &cov.uncovered_lines);
            // Add uncovered lines info at the beginning
            for rec in uncovered_recommendations.into_iter().rev() {
                steps.insert(0, rec);
            }
        }
    }

    if !has_good_coverage {
        let test_count = if suggested_extractions.is_empty() {
            // If no specific extractions suggested, base on complexity
            (cyclomatic / 2).max(3)
        } else {
            // Test count based on extraction suggestions (3-5 tests per function)
            suggested_extractions.len() as u32 * 4
        };

        steps.push(format!(
            "Add {} unit tests (3-5 per extracted function)",
            test_count
        ));
    }

    // Generate action and rationale
    let action = if suggested_extractions.is_empty() {
        format!(
            "Refactor to reduce complexity from {} → ~{}",
            cyclomatic,
            cyclomatic.saturating_sub(complexity_reduction)
        )
    } else {
        format!(
            "Extract {} to reduce complexity {} → ~{}",
            suggested_extractions.join(", "),
            cyclomatic,
            cyclomatic.saturating_sub(complexity_reduction)
        )
    };

    let rationale = format!(
        "Complex function (cyclo={}, cog={}, nesting={}) with {} suggested extraction patterns. Predicted complexity reduction: {}%",
        cyclomatic,
        cognitive,
        func.nesting,
        suggested_extractions.len(),
        if cyclomatic > 0 {
            (complexity_reduction as f32 / cyclomatic as f32 * 100.0) as u32
        } else {
            0
        }
    );

    (action, rationale, steps)
}

/// Generate recommendation for resource management debt
pub fn generate_resource_management_recommendation(
    resource_type: &str,
    detail1: &str,
    detail2: &str,
) -> (String, String, Vec<String>) {
    match resource_type {
        "allocation" => (
            format!("Optimize allocation pattern: {}", detail1),
            format!("Resource impact: {}", detail2),
            vec![
                "Use object pooling where appropriate".to_string(),
                "Consider pre-allocation strategies".to_string(),
                "Profile memory usage patterns".to_string(),
                "Review data structure choices".to_string(),
            ],
        ),
        "blocking_io" => (
            format!("Optimize {} operation", detail1),
            format!("Context: {}", detail2),
            vec![
                "Consider async/await pattern".to_string(),
                "Use appropriate I/O libraries".to_string(),
                "Consider background processing".to_string(),
                "Add proper error handling".to_string(),
            ],
        ),
        "basic" => (
            format!("Optimize {} resource issue", detail1),
            format!("Resource impact ({}): {}", detail2, detail1),
            vec![
                "Profile and identify resource bottlenecks".to_string(),
                "Apply resource optimization techniques".to_string(),
                "Monitor resource usage before and after changes".to_string(),
                "Consider algorithmic improvements".to_string(),
            ],
        ),
        _ => (
            "Optimize resource usage".to_string(),
            "Resource issue detected".to_string(),
            vec!["Monitor and profile resource usage".to_string()],
        ),
    }
}

/// Generate recommendation for string concatenation in loops
pub fn generate_string_concat_recommendation(
    loop_type: &str,
    iterations: &Option<u32>,
) -> (String, String, Vec<String>) {
    let iter_info = iterations.map_or("unknown".to_string(), |i| i.to_string());
    (
        format!("Use StringBuilder for {} loop concatenation", loop_type),
        format!(
            "String concatenation in {} (≈{} iterations)",
            loop_type, iter_info
        ),
        vec![
            "Replace += with StringBuilder/StringBuffer".to_string(),
            "Pre-allocate capacity if known".to_string(),
            "Consider string formatting alternatives".to_string(),
            "Benchmark performance improvement".to_string(),
        ],
    )
}

/// Generate recommendation for nested loops
pub fn generate_nested_loops_recommendation(
    depth: u32,
    complexity_estimate: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Reduce {}-level nested loop complexity", depth),
        format!("Complexity estimate: {}", complexity_estimate),
        vec![
            "Extract inner loops into functions".to_string(),
            "Consider algorithmic improvements".to_string(),
            "Use iterators for cleaner code".to_string(),
            "Profile actual performance impact".to_string(),
        ],
    )
}

/// Generate recommendation for data structure improvements
pub fn generate_data_structure_recommendation(
    current: &str,
    recommended: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Replace {} with {}", current, recommended),
        format!(
            "Data structure {} is suboptimal for access patterns",
            current
        ),
        vec![
            format!("Refactor to use {}", recommended),
            "Update related algorithms".to_string(),
            "Test performance before/after".to_string(),
            "Update documentation".to_string(),
        ],
    )
}

/// Generate recommendation for god object pattern
pub fn generate_god_object_recommendation(
    responsibility_count: u32,
    complexity_score: f64,
) -> (String, String, Vec<String>) {
    (
        format!(
            "Split {} responsibilities into focused classes",
            responsibility_count
        ),
        format!("God object with complexity {:.1}", complexity_score),
        vec![
            "Identify single responsibility principle violations".to_string(),
            "Extract cohesive functionality into separate classes".to_string(),
            "Use composition over inheritance".to_string(),
            "Refactor incrementally with tests".to_string(),
        ],
    )
}

/// Generate recommendation for feature envy pattern
pub fn generate_feature_envy_recommendation(
    external_class: &str,
    usage_ratio: f64,
) -> (String, String, Vec<String>) {
    (
        format!("Move method closer to {} class", external_class),
        format!(
            "Method uses {}% external data",
            (usage_ratio * 100.0) as u32
        ),
        vec![
            format!("Consider moving method to {}", external_class),
            "Extract shared functionality".to_string(),
            "Review class responsibilities".to_string(),
            "Maintain cohesion after refactoring".to_string(),
        ],
    )
}

/// Generate recommendation for primitive obsession pattern
pub fn generate_primitive_obsession_recommendation(
    primitive_type: &str,
    domain_concept: &str,
) -> (String, String, Vec<String>) {
    (
        format!(
            "Create {} domain type instead of {}",
            domain_concept, primitive_type
        ),
        format!(
            "Primitive obsession: {} used for {}",
            primitive_type, domain_concept
        ),
        vec![
            format!("Create {} value object", domain_concept),
            "Add validation and behavior to type".to_string(),
            "Replace primitive usage throughout codebase".to_string(),
            "Add type safety and domain logic".to_string(),
        ],
    )
}

/// Generate recommendation for magic values
pub fn generate_magic_values_recommendation(
    value: &str,
    occurrences: u32,
) -> (String, String, Vec<String>) {
    (
        format!("Extract '{}' into named constant", value),
        format!("Magic value '{}' appears {} times", value, occurrences),
        vec![
            format!(
                "Define const {} = '{}'",
                value.to_uppercase().replace(' ', "_"),
                value
            ),
            "Replace all occurrences with named constant".to_string(),
            "Add documentation explaining value meaning".to_string(),
            "Group related constants in module".to_string(),
        ],
    )
}

/// Generate recommendation for complex assertions in tests
pub fn generate_assertion_complexity_recommendation(
    assertion_count: u32,
    complexity_score: f64,
) -> (String, String, Vec<String>) {
    (
        format!("Simplify {} complex assertions", assertion_count),
        format!("Test assertion complexity: {:.1}", complexity_score),
        vec![
            "Split complex assertions into multiple simple ones".to_string(),
            "Use custom assertion helpers".to_string(),
            "Add descriptive assertion messages".to_string(),
            "Consider table-driven test patterns".to_string(),
        ],
    )
}

/// Generate recommendation for flaky test patterns
pub fn generate_flaky_test_recommendation(
    pattern_type: &str,
    reliability_impact: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Fix {} flaky test pattern", pattern_type),
        format!("Reliability impact: {}", reliability_impact),
        vec![
            "Identify and eliminate non-deterministic behavior".to_string(),
            "Use test doubles to isolate dependencies".to_string(),
            "Add proper test cleanup and setup".to_string(),
            "Consider parallel test safety".to_string(),
        ],
    )
}

/// Generate recommendation for async/await misuse
pub fn generate_async_misuse_recommendation(
    pattern: &str,
    performance_impact: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Fix async pattern: {}", pattern),
        format!("Resource impact: {}", performance_impact),
        vec![
            "Use proper async/await patterns".to_string(),
            "Avoid blocking async contexts".to_string(),
            "Configure async runtime appropriately".to_string(),
            "Add timeout and cancellation handling".to_string(),
        ],
    )
}

/// Generate recommendation for resource leaks
pub fn generate_resource_leak_recommendation(
    resource_type: &str,
    cleanup_missing: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Add {} resource cleanup", resource_type),
        format!("Missing cleanup: {}", cleanup_missing),
        vec![
            "Implement Drop trait for automatic cleanup".to_string(),
            "Use RAII patterns for resource management".to_string(),
            "Add try-finally or defer patterns".to_string(),
            "Test resource cleanup in error scenarios".to_string(),
        ],
    )
}

/// Generate recommendation for collection inefficiencies
pub fn generate_collection_inefficiency_recommendation(
    collection_type: &str,
    inefficiency_type: &str,
) -> (String, String, Vec<String>) {
    (
        format!("Optimize {} usage", collection_type),
        format!("Inefficiency: {}", inefficiency_type),
        vec![
            "Review collection access patterns".to_string(),
            "Consider alternative data structures".to_string(),
            "Pre-allocate capacity where possible".to_string(),
            "Monitor collection resource usage".to_string(),
        ],
    )
}
