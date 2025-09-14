use crate::priority::{
    DebtItem, DebtType, FileAggregateScore, FileDebtItem, UnifiedAnalysis, UnifiedDebtItem,
};
use std::fmt::Write;

/// Format priorities for markdown output without ANSI color codes
pub fn format_priorities_markdown(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    let version = env!("CARGO_PKG_VERSION");
    writeln!(output, "# Debtmap v{}\n", version).unwrap();

    let top_items = analysis.get_top_mixed_priorities(limit);
    let count = top_items.len().min(limit);

    writeln!(output, "## Top {} Recommendations\n", count).unwrap();

    for (idx, item) in top_items.iter().enumerate() {
        format_mixed_priority_item_markdown(&mut output, idx + 1, item, verbosity);
        writeln!(output).unwrap();
    }

    // Add summary
    writeln!(output, "---\n").unwrap();
    writeln!(
        output,
        "**Total Debt Score:** {:.0}",
        analysis.total_debt_score
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(output, "**Overall Coverage:** {:.2}%", coverage).unwrap();
    }

    output
}

fn format_mixed_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &DebtItem,
    verbosity: u8,
) {
    match item {
        DebtItem::Function(func_item) => {
            format_priority_item_markdown(output, rank, func_item, verbosity);
        }
        DebtItem::File(file_item) => {
            format_file_priority_item_markdown(output, rank, file_item, verbosity);
        }
        DebtItem::FileAggregate(agg_item) => {
            format_file_aggregate_item_markdown(output, rank, agg_item, verbosity);
        }
    }
}

fn format_file_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &FileDebtItem,
    verbosity: u8,
) {
    let severity = get_severity_label(item.score);

    // Determine file type
    let type_label = if item.metrics.god_object_indicators.is_god_object {
        "FILE - GOD OBJECT"
    } else if item.metrics.total_lines > 500 {
        "FILE - HIGH COMPLEXITY"
    } else {
        "FILE"
    };

    // Header with rank and score
    writeln!(
        output,
        "### #{} - Score: {:.1} [{}]",
        rank, item.score, severity
    )
    .unwrap();

    writeln!(output, "**Type:** {}", type_label).unwrap();
    writeln!(
        output,
        "**File:** `{}` ({} lines, {} functions)",
        item.metrics.path.display(),
        item.metrics.total_lines,
        item.metrics.function_count
    )
    .unwrap();

    // God object details if applicable
    if item.metrics.god_object_indicators.is_god_object {
        writeln!(output, "**God Object Metrics:**").unwrap();
        writeln!(
            output,
            "- Methods: {}",
            item.metrics.god_object_indicators.methods_count
        )
        .unwrap();
        writeln!(
            output,
            "- Fields: {}",
            item.metrics.god_object_indicators.fields_count
        )
        .unwrap();
        writeln!(
            output,
            "- Responsibilities: {}",
            item.metrics.god_object_indicators.responsibilities
        )
        .unwrap();
        writeln!(
            output,
            "- God Object Score: {:.1}",
            item.metrics.god_object_indicators.god_object_score
        )
        .unwrap();
    }

    writeln!(output, "**Recommendation:** {}", item.recommendation).unwrap();

    writeln!(output, "**Impact:** {}", format_file_impact(&item.impact)).unwrap();

    if verbosity >= 1 {
        writeln!(output, "\n**Scoring Breakdown:**").unwrap();
        writeln!(
            output,
            "- File size: {}",
            score_category(item.metrics.total_lines)
        )
        .unwrap();
        writeln!(
            output,
            "- Functions: {}",
            function_category(item.metrics.function_count)
        )
        .unwrap();
        writeln!(
            output,
            "- Complexity: {}",
            complexity_category(item.metrics.avg_complexity)
        )
        .unwrap();
        if item.metrics.function_count > 0 {
            writeln!(
                output,
                "- Dependencies: {} functions may have complex interdependencies",
                item.metrics.function_count
            )
            .unwrap();
        }
    }
}

fn format_file_aggregate_item_markdown(
    output: &mut String,
    rank: usize,
    item: &FileAggregateScore,
    verbosity: u8,
) {
    let severity = if item.aggregate_score >= 300.0 {
        "CRITICAL"
    } else if item.aggregate_score >= 200.0 {
        "HIGH"
    } else if item.aggregate_score >= 100.0 {
        "MEDIUM"
    } else {
        "LOW"
    };

    // Header with rank and score
    writeln!(
        output,
        "### #{} - Score: {:.1} [{}]",
        rank, item.aggregate_score, severity
    )
    .unwrap();

    writeln!(output, "**Type:** FILE AGGREGATE").unwrap();
    writeln!(
        output,
        "**File:** `{}` ({} functions, total score: {:.1})",
        item.file_path.display(),
        item.function_count,
        item.total_score
    )
    .unwrap();

    if item.problematic_functions > 0 {
        writeln!(
            output,
            "**Warning:** {} problematic functions (score > 5.0)",
            item.problematic_functions
        )
        .unwrap();
    }

    writeln!(output, "\n**Top Issues:**").unwrap();
    for (func_name, score) in &item.top_function_scores {
        writeln!(output, "- `{}`: {:.1}", func_name, score).unwrap();
    }

    writeln!(output, "\n**Action:** Comprehensive refactoring needed").unwrap();

    if verbosity >= 1 {
        writeln!(
            output,
            "\n**Aggregation Method:** {:?}",
            item.aggregation_method
        )
        .unwrap();
    }
}

fn score_category(lines: usize) -> &'static str {
    match lines {
        0..=200 => "LOW",
        201..=500 => "MODERATE",
        501..=1000 => "HIGH",
        _ => "CRITICAL",
    }
}

fn function_category(count: usize) -> &'static str {
    match count {
        0..=10 => "LOW",
        11..=20 => "MODERATE",
        21..=50 => "HIGH",
        _ => "EXCESSIVE",
    }
}

fn complexity_category(avg: f64) -> &'static str {
    match avg as usize {
        0..=5 => "LOW",
        6..=10 => "MODERATE",
        11..=20 => "HIGH",
        _ => "VERY HIGH",
    }
}

fn format_file_impact(impact: &crate::priority::FileImpact) -> String {
    let mut parts = vec![];

    if impact.complexity_reduction > 0.0 {
        parts.push(format!(
            "Reduce complexity by {:.0}%",
            impact.complexity_reduction
        ));
    }
    if impact.test_effort > 0.0 {
        parts.push(format!("Test effort: {:.1}", impact.test_effort));
    }
    if impact.maintainability_improvement > 0.0 {
        parts.push("Enable parallel development".to_string());
    }

    if parts.is_empty() {
        "No measurable impact".to_string()
    } else {
        parts.join(", ")
    }
}

fn format_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    let severity = get_severity_label(item.unified_score.final_score);

    // Header with rank and score
    writeln!(
        output,
        "### #{} - Score: {:.1} [{}]",
        rank, item.unified_score.final_score, severity
    )
    .unwrap();

    // Show score breakdown for verbosity >= 2
    if verbosity >= 2 {
        output.push_str(&format_score_breakdown_with_coverage(
            &item.unified_score,
            item.transitive_coverage.as_ref(),
        ));
    } else if verbosity >= 1 {
        // Show main contributing factors for verbosity >= 1
        output.push_str(&format_main_factors_with_coverage(
            &item.unified_score,
            &item.debt_type,
            item.transitive_coverage.as_ref(),
        ));
    }

    // Location and type
    writeln!(
        output,
        "**Type:** {} | **Location:** `{}:{} {}()`",
        format_debt_type(&item.debt_type),
        item.location.file.display(),
        item.location.line,
        item.location.function
    )
    .unwrap();

    // Action and impact
    writeln!(output, "**Action:** {}", item.recommendation.primary_action).unwrap();
    writeln!(
        output,
        "**Impact:** {}",
        format_impact(&item.expected_impact)
    )
    .unwrap();

    // Complexity details
    if let Some(complexity) = extract_complexity_info(&item.debt_type) {
        writeln!(output, "**Complexity:** {}", complexity).unwrap();
    }

    // Dependencies
    if verbosity >= 1 {
        writeln!(output, "\n#### Dependencies").unwrap();
        writeln!(
            output,
            "- **Upstream:** {} | **Downstream:** {}",
            item.upstream_dependencies, item.downstream_dependencies
        )
        .unwrap();

        if !item.upstream_callers.is_empty() && verbosity >= 2 {
            let caller_info = format_dependency_list(&item.upstream_callers, 3, "Called by");
            if !caller_info.is_empty() {
                writeln!(output, "{}", caller_info).unwrap();
            }
        }

        if !item.downstream_callees.is_empty() && verbosity >= 2 {
            let callee_info = format_dependency_list(&item.downstream_callees, 3, "Calls");
            if !callee_info.is_empty() {
                writeln!(output, "{}", callee_info).unwrap();
            }
        }
    }

    // Rationale
    writeln!(output, "\n**Why:** {}", item.recommendation.rationale).unwrap();
}

fn get_severity_label(score: f64) -> &'static str {
    match score {
        s if s >= 9.0 => "CRITICAL",
        s if s >= 7.0 => "HIGH",
        s if s >= 5.0 => "MEDIUM",
        s if s >= 3.0 => "LOW",
        _ => "MINIMAL",
    }
}

fn format_debt_type(debt_type: &DebtType) -> &'static str {
    match debt_type {
        DebtType::TestingGap { .. } => "Testing Gap",
        DebtType::ComplexityHotspot { .. } => "Complexity",
        DebtType::DeadCode { .. } => "Dead Code",
        DebtType::Duplication { .. } => "Duplication",
        DebtType::Risk { .. } => "Risk",
        DebtType::TestComplexityHotspot { .. } => "Test Complexity",
        DebtType::TestTodo { .. } => "Test TODO",
        DebtType::TestDuplication { .. } => "Test Duplication",
        DebtType::ErrorSwallowing { .. } => "Error Swallowing",
        // Resource Management debt types
        DebtType::AllocationInefficiency { .. } => "Allocation Inefficiency",
        DebtType::StringConcatenation { .. } => "String Concatenation",
        DebtType::NestedLoops { .. } => "Nested Loops",
        DebtType::BlockingIO { .. } => "Blocking I/O",
        DebtType::SuboptimalDataStructure { .. } => "Suboptimal Data Structure",
        // Organization debt types
        DebtType::GodObject { .. } => "God Object",
        DebtType::FeatureEnvy { .. } => "Feature Envy",
        DebtType::PrimitiveObsession { .. } => "Primitive Obsession",
        DebtType::MagicValues { .. } => "Magic Values",
        // Testing quality debt types
        DebtType::AssertionComplexity { .. } => "Assertion Complexity",
        DebtType::FlakyTestPattern { .. } => "Flaky Test Pattern",
        // Resource management debt types
        DebtType::AsyncMisuse { .. } => "Async Misuse",
        DebtType::ResourceLeak { .. } => "Resource Leak",
        DebtType::CollectionInefficiency { .. } => "Collection Inefficiency",
    }
}

fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
    let mut parts = vec![];

    if impact.complexity_reduction > 0.0 {
        parts.push(format!("-{:.1} complexity", impact.complexity_reduction));
    }
    if impact.risk_reduction > 0.1 {
        parts.push(format!("-{:.1} risk", impact.risk_reduction));
    }
    if impact.coverage_improvement > 0.01 {
        parts.push(format!("+{:.0}% coverage", impact.coverage_improvement));
    }
    if impact.lines_reduction > 0 {
        parts.push(format!("-{} lines", impact.lines_reduction));
    }

    if parts.is_empty() {
        "No measurable impact".to_string()
    } else {
        parts.join(", ")
    }
}

fn extract_complexity_info(debt_type: &DebtType) -> Option<String> {
    match debt_type {
        DebtType::ComplexityHotspot {
            cyclomatic,
            cognitive,
        }
        | DebtType::TestComplexityHotspot {
            cyclomatic,
            cognitive,
            ..
        } => Some(format!(
            "cyclomatic={}, cognitive={}",
            cyclomatic, cognitive
        )),
        DebtType::TestingGap {
            cyclomatic,
            cognitive,
            ..
        } => Some(format!(
            "cyclomatic={}, cognitive={}",
            cyclomatic, cognitive
        )),
        DebtType::Risk { .. } => None,
        DebtType::DeadCode {
            cyclomatic,
            cognitive,
            ..
        } => Some(format!(
            "cyclomatic={}, cognitive={}",
            cyclomatic, cognitive
        )),
        _ => None,
    }
}

fn format_score_breakdown_with_coverage(
    unified_score: &crate::priority::UnifiedScore,
    transitive_coverage: Option<&crate::priority::coverage_propagation::TransitiveCoverage>,
) -> String {
    let weights = crate::config::get_scoring_weights();
    let mut output = String::new();

    writeln!(&mut output, "\n#### Score Calculation\n").unwrap();
    writeln!(
        &mut output,
        "| Component | Value | Weight | Contribution | Details |"
    )
    .unwrap();
    writeln!(
        &mut output,
        "|-----------|-------|--------|--------------|----------|"
    )
    .unwrap();
    writeln!(
        &mut output,
        "| Complexity | {:.1} | {:.0}% | {:.2} | |",
        unified_score.complexity_factor,
        weights.complexity * 100.0,
        unified_score.complexity_factor * weights.complexity
    )
    .unwrap();

    // Add coverage details if available
    let coverage_details = if let Some(trans_cov) = transitive_coverage {
        format!("Line: {:.2}%", trans_cov.direct * 100.0)
    } else {
        "No data".to_string()
    };
    writeln!(
        &mut output,
        "| Coverage | {:.1} | {:.0}% | {:.2} | {} |",
        unified_score.coverage_factor,
        weights.coverage * 100.0,
        unified_score.coverage_factor * weights.coverage,
        coverage_details
    )
    .unwrap();
    // Semantic and ROI factors removed per spec 55 and 58
    writeln!(
        &mut output,
        "| Dependency | {:.1} | {:.0}% | {:.2} | |",
        unified_score.dependency_factor,
        weights.dependency * 100.0,
        unified_score.dependency_factor * weights.dependency
    )
    .unwrap();

    // Organization factor removed per spec 58 - redundant with complexity factor

    // New weights after removing security: complexity, coverage, dependency
    let base_score = unified_score.complexity_factor * weights.complexity
        + unified_score.coverage_factor * weights.coverage
        + unified_score.dependency_factor * weights.dependency;

    writeln!(&mut output).unwrap();
    writeln!(&mut output, "- **Base Score:** {:.2}", base_score).unwrap();
    writeln!(
        &mut output,
        "- **Role Adjustment:** ×{:.2}",
        unified_score.role_multiplier
    )
    .unwrap();
    writeln!(
        &mut output,
        "- **Final Score:** {:.2}",
        unified_score.final_score
    )
    .unwrap();
    writeln!(&mut output).unwrap();

    output
}

fn format_main_factors_with_coverage(
    unified_score: &crate::priority::UnifiedScore,
    debt_type: &crate::priority::DebtType,
    transitive_coverage: Option<&crate::priority::coverage_propagation::TransitiveCoverage>,
) -> String {
    let weights = crate::config::get_scoring_weights();
    let mut factors = vec![];

    // Show coverage info - both good and bad coverage are important factors
    if let Some(trans_cov) = transitive_coverage {
        let coverage_pct = trans_cov.direct * 100.0;
        if coverage_pct >= 95.0 {
            factors.push(format!("Excellent coverage {:.1}%", coverage_pct));
        } else if coverage_pct >= 80.0 {
            factors.push(format!("Good coverage {:.1}%", coverage_pct));
        } else if unified_score.coverage_factor > 3.0 {
            factors.push(format!(
                "Line coverage {:.1}% (weight: {:.0}%)",
                coverage_pct,
                weights.coverage * 100.0
            ));
        }
    } else if unified_score.coverage_factor > 3.0 {
        factors.push(format!(
            "No coverage data (weight: {:.0}%)",
            weights.coverage * 100.0
        ));
    }
    if unified_score.complexity_factor > 5.0 {
        factors.push(format!(
            "Complexity (weight: {:.0}%)",
            weights.complexity * 100.0
        ));
    } else if unified_score.complexity_factor > 3.0 {
        factors.push("Moderate complexity".to_string());
    }

    if unified_score.dependency_factor > 5.0 {
        factors.push(format!(
            "Critical path (weight: {:.0}%)",
            weights.dependency * 100.0
        ));
    }
    // Organization factor removed per spec 58 - redundant with complexity factor

    // Add specific factors for various debt types
    match debt_type {
        crate::priority::DebtType::NestedLoops { depth, .. } => {
            factors.push("Complexity impact (High)".to_string());
            factors.push(format!("{} level nested loops", depth));
        }
        crate::priority::DebtType::BlockingIO { operation, .. } => {
            factors.push("Resource management issue".to_string());
            factors.push(format!("Blocking {}", operation));
        }
        crate::priority::DebtType::AllocationInefficiency { pattern, .. } => {
            factors.push("Resource management issue".to_string());
            factors.push(format!("Allocation: {}", pattern));
        }
        _ => {} // No additional factors for other debt types
    }

    if !factors.is_empty() {
        format!("*Main factors: {}*\n", factors.join(", "))
    } else {
        String::new()
    }
}

fn format_dependency_list(items: &[String], max_shown: usize, list_type: &str) -> String {
    if items.is_empty() {
        return String::new();
    }

    let list = if items.len() > max_shown {
        format!(
            "{}, ... ({} more)",
            items
                .iter()
                .take(max_shown)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            items.len() - max_shown
        )
    } else {
        items.to_vec().join(", ")
    };

    format!("- **{}:** {}", list_type, list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        ActionableRecommendation, FunctionRole, FunctionVisibility, ImpactMetrics, Location,
        TransitiveCoverage, UnifiedDebtItem, UnifiedScore,
    };

    #[test]
    fn test_get_severity_label() {
        assert_eq!(get_severity_label(10.0), "CRITICAL");
        assert_eq!(get_severity_label(9.5), "CRITICAL");
        assert_eq!(get_severity_label(9.0), "CRITICAL");
        assert_eq!(get_severity_label(8.0), "HIGH");
        assert_eq!(get_severity_label(7.0), "HIGH");
        assert_eq!(get_severity_label(6.0), "MEDIUM");
        assert_eq!(get_severity_label(5.0), "MEDIUM");
        assert_eq!(get_severity_label(4.0), "LOW");
        assert_eq!(get_severity_label(3.0), "LOW");
        assert_eq!(get_severity_label(2.0), "MINIMAL");
        assert_eq!(get_severity_label(0.5), "MINIMAL");
    }

    #[test]
    fn test_format_debt_type() {
        let test_gap = DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 20,
        };
        assert_eq!(format_debt_type(&test_gap), "Testing Gap");

        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 30,
        };
        assert_eq!(format_debt_type(&complexity), "Complexity");

        let dead_code = DebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 5,
            cognitive: 10,
            usage_hints: vec![],
        };
        assert_eq!(format_debt_type(&dead_code), "Dead Code");
    }

    #[test]
    fn test_format_impact_with_all_metrics() {
        let impact = ImpactMetrics {
            complexity_reduction: 5.5,
            risk_reduction: 0.3,
            coverage_improvement: 15.5,
            lines_reduction: 25,
        };

        let result = format_impact(&impact);
        assert!(result.contains("-5.5 complexity"));
        assert!(result.contains("-0.3 risk"));
        // 15.5 rounds to 16 with {:.0} formatting
        assert!(result.contains("+16% coverage"));
        assert!(result.contains("-25 lines"));
    }

    #[test]
    fn test_format_impact_with_no_metrics() {
        let impact = ImpactMetrics {
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
            coverage_improvement: 0.0,
            lines_reduction: 0,
        };

        let result = format_impact(&impact);
        assert_eq!(result, "No measurable impact");
    }

    #[test]
    fn test_format_impact_with_partial_metrics() {
        let impact = ImpactMetrics {
            complexity_reduction: 3.0,
            risk_reduction: 0.05,        // Below threshold
            coverage_improvement: 0.005, // Below threshold
            lines_reduction: 10,
        };

        let result = format_impact(&impact);
        assert!(result.contains("-3.0 complexity"));
        assert!(!result.contains("risk"));
        assert!(!result.contains("coverage"));
        assert!(result.contains("-10 lines"));
    }

    #[test]
    fn test_extract_complexity_info() {
        let complexity_hotspot = DebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 30,
        };
        assert_eq!(
            extract_complexity_info(&complexity_hotspot),
            Some("cyclomatic=15, cognitive=30".to_string())
        );

        let test_gap = DebtType::TestingGap {
            coverage: 0.0,
            cyclomatic: 10,
            cognitive: 20,
        };
        assert_eq!(
            extract_complexity_info(&test_gap),
            Some("cyclomatic=10, cognitive=20".to_string())
        );

        let dead_code = DebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 5,
            cognitive: 10,
            usage_hints: vec![],
        };
        assert_eq!(
            extract_complexity_info(&dead_code),
            Some("cyclomatic=5, cognitive=10".to_string())
        );

        let risk = DebtType::Risk {
            risk_score: 8.5,
            factors: vec!["complex".to_string()],
        };
        assert_eq!(extract_complexity_info(&risk), None);
    }

    #[test]
    fn test_format_score_breakdown() {
        let score = UnifiedScore {
            complexity_factor: 5.0,
            coverage_factor: 8.0,
            dependency_factor: 4.0,
            role_multiplier: 1.2,
            final_score: 8.5,
        };

        let result = format_score_breakdown_with_coverage(&score, None);

        // Check for table headers
        assert!(result.contains("Score Calculation"));
        assert!(result.contains("| Component | Value | Weight | Contribution |"));

        // Check for component rows
        assert!(result.contains("| Complexity | 5.0"));
        assert!(result.contains("| Coverage | 8.0"));
        // ROI and Semantic removed from scoring per spec 55 and 58
        assert!(result.contains("| Dependency | 4.0"));

        // Check for summary lines (with markdown formatting)
        assert!(result.contains("**Base Score:**"));
        assert!(result.contains("**Role Adjustment:** ×1.20"));
        assert!(result.contains("**Final Score:** 8.50"));
    }

    #[test]
    fn test_format_main_factors_with_multiple_factors() {
        let score = UnifiedScore {
            complexity_factor: 6.0, // Above threshold
            coverage_factor: 4.0,   // Above threshold
            dependency_factor: 6.0, // Above threshold
            role_multiplier: 1.0,
            final_score: 7.0,
        };

        let debt_type = DebtType::Risk {
            risk_score: 5.0,
            factors: vec!["Test factor".to_string()],
        };

        let result = format_main_factors_with_coverage(&score, &debt_type, None);

        assert!(result.contains("Main factors:"));
        assert!(result.contains("No coverage data") || result.contains("Line coverage"));
        // ROI removed from scoring per spec 55 and 58
        assert!(result.contains("Critical path"));
        assert!(result.contains("Complexity"));
    }

    #[test]
    fn test_format_main_factors_with_no_factors() {
        let score = UnifiedScore {
            complexity_factor: 2.0, // Below all thresholds
            coverage_factor: 2.0,
            dependency_factor: 2.0,
            role_multiplier: 1.0,
            final_score: 2.0,
        };

        let debt_type = DebtType::Risk {
            risk_score: 1.0,
            factors: vec!["Test factor".to_string()],
        };

        let result = format_main_factors_with_coverage(&score, &debt_type, None);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_dependency_list_empty() {
        let items: Vec<String> = vec![];
        let result = format_dependency_list(&items, 3, "Called by");
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_dependency_list_few_items() {
        let items = vec!["func1".to_string(), "func2".to_string()];
        let result = format_dependency_list(&items, 3, "Called by");
        assert_eq!(result, "- **Called by:** func1, func2");
    }

    #[test]
    fn test_format_dependency_list_many_items() {
        let items = vec![
            "func1".to_string(),
            "func2".to_string(),
            "func3".to_string(),
            "func4".to_string(),
            "func5".to_string(),
        ];
        let result = format_dependency_list(&items, 3, "Calls");
        assert_eq!(result, "- **Calls:** func1, func2, func3, ... (2 more)");
    }

    #[test]
    fn test_format_dependency_list_exactly_max() {
        let items = vec![
            "func1".to_string(),
            "func2".to_string(),
            "func3".to_string(),
        ];
        let result = format_dependency_list(&items, 3, "Dependencies");
        assert_eq!(result, "- **Dependencies:** func1, func2, func3");
    }

    // Helper function to create test UnifiedDebtItem
    fn create_test_debt_item() -> UnifiedDebtItem {
        use std::path::PathBuf;

        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                line: 100,
                function: "test_function".to_string(),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
            unified_score: UnifiedScore {
                complexity_factor: 7.0,
                coverage_factor: 8.0,
                dependency_factor: 6.0,
                role_multiplier: 1.2,
                final_score: 8.5,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor complex function".to_string(),
                rationale: "High complexity makes it hard to maintain".to_string(),
                implementation_steps: vec![
                    "Extract helper functions".to_string(),
                    "Add unit tests".to_string(),
                ],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 5.0,
                risk_reduction: 0.2,
                coverage_improvement: 25.0,
                lines_reduction: 30,
            },
            transitive_coverage: Some(TransitiveCoverage {
                direct: 0.45,
                transitive: 0.55,
                propagated_from: vec![],
                uncovered_lines: vec![101, 102, 103],
            }),
            upstream_dependencies: 3,
            downstream_dependencies: 5,
            upstream_callers: vec![
                "caller1".to_string(),
                "caller2".to_string(),
                "caller3".to_string(),
            ],
            downstream_callees: vec!["callee1".to_string(), "callee2".to_string()],
            nesting_depth: 3,
            function_length: 150,
            cyclomatic_complexity: 15,
            cognitive_complexity: 25,
            entropy_details: None,
            is_pure: None,
            purity_confidence: None,
            god_object_indicators: None,
        }
    }

    #[test]
    fn test_format_priority_item_markdown_minimal_verbosity() {
        let mut output = String::new();
        let item = create_test_debt_item();

        format_priority_item_markdown(&mut output, 1, &item, 0);

        // Check basic elements are present
        assert!(output.contains("#1 - Score: 8.5 [HIGH]"));
        assert!(output.contains("**Type:** Complexity"));
        assert!(output.contains("**Location:** `test.rs:100 test_function()`"));
        assert!(output.contains("**Action:** Refactor complex function"));
        assert!(output.contains("**Impact:**"));
        assert!(output.contains("**Complexity:** cyclomatic=15, cognitive=25"));
        assert!(output.contains("**Why:** High complexity makes it hard to maintain"));

        // Should NOT include score breakdown or dependencies at verbosity 0
        assert!(!output.contains("#### Dependencies"));
        assert!(!output.contains("Coverage Gap"));
    }

    #[test]
    fn test_format_priority_item_markdown_verbosity_1() {
        let mut output = String::new();
        let item = create_test_debt_item();

        format_priority_item_markdown(&mut output, 2, &item, 1);

        // Should include main factors but not full breakdown
        assert!(output.contains("#2 - Score: 8.5 [HIGH]"));
        assert!(output.contains("Main factors"));

        // Should include dependencies section
        assert!(output.contains("#### Dependencies"));
        assert!(output.contains("**Upstream:** 3 | **Downstream:** 5"));

        // Should NOT include caller/callee lists at verbosity 1
        assert!(!output.contains("Called by"));
        assert!(!output.contains("Calls"));
    }

    #[test]
    fn test_format_priority_item_markdown_verbosity_2() {
        let mut output = String::new();
        let item = create_test_debt_item();

        format_priority_item_markdown(&mut output, 3, &item, 2);

        // Should include full score breakdown
        assert!(output.contains("Score Calculation"));
        assert!(output.contains("Component"));
        assert!(output.contains("Complexity"));

        // Should include dependencies with detailed lists
        assert!(output.contains("#### Dependencies"));
        assert!(output.contains("**Upstream:** 3 | **Downstream:** 5"));
        assert!(output.contains("Called by"));
        assert!(output.contains("caller1, caller2, caller3"));
        assert!(output.contains("Calls"));
        assert!(output.contains("callee1, callee2"));
    }

    #[test]
    fn test_format_priority_item_markdown_critical_score() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.unified_score.final_score = 9.5;

        format_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("[CRITICAL]"));
    }

    #[test]
    fn test_format_priority_item_markdown_low_score() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.unified_score.final_score = 3.5;

        format_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("[LOW]"));
    }

    #[test]
    fn test_format_priority_item_markdown_no_complexity() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.debt_type = DebtType::Risk {
            risk_score: 7.5,
            factors: vec!["Factor1".to_string()],
        };

        format_priority_item_markdown(&mut output, 1, &item, 0);

        // Should not have complexity section for Risk type
        assert!(!output.contains("**Complexity:**"));
        assert!(output.contains("**Type:** Risk"));
    }

    #[test]
    fn test_format_priority_item_markdown_empty_dependencies() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.upstream_callers.clear();
        item.downstream_callees.clear();

        format_priority_item_markdown(&mut output, 1, &item, 2);

        // Should still show dependency counts but no lists
        assert!(output.contains("**Upstream:** 3 | **Downstream:** 5"));
        assert!(!output.contains("Called by"));
        assert!(!output.contains("Calls"));
    }

    #[test]
    fn test_format_priority_item_markdown_large_rank() {
        let mut output = String::new();
        let item = create_test_debt_item();

        format_priority_item_markdown(&mut output, 999, &item, 0);

        assert!(output.contains("#999 - Score:"));
    }

    #[test]
    fn test_format_priority_item_markdown_no_transitive_coverage() {
        let mut output = String::new();
        let mut item = create_test_debt_item();
        item.transitive_coverage = None;

        format_priority_item_markdown(&mut output, 1, &item, 2);

        // Should still work without transitive coverage
        assert!(output.contains("#1 - Score: 8.5"));
        // Coverage information should be omitted in breakdown
    }

    #[test]
    fn test_format_file_priority_item_markdown_basic() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/main.rs"),
                total_lines: 250,
                function_count: 10,
                class_count: 2,
                avg_complexity: 5.5,
                max_complexity: 12,
                total_complexity: 55,
                coverage_percent: 0.75,
                uncovered_lines: 25,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 5,
                    fields_count: 3,
                    responsibilities: 2,
                    is_god_object: false,
                    god_object_score: 0.0,
                },
                function_scores: vec![10.0, 8.0, 6.0],
            },
            score: 45.2,
            priority_rank: 1,
            recommendation: "Refactor complex functions".to_string(),
            impact: FileImpact {
                complexity_reduction: 15.0,
                maintainability_improvement: 20.0,
                test_effort: 10.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("### #1 - Score: 45.2"));
        assert!(output.contains("**Type:** FILE"));
        assert!(output.contains("src/main.rs"));
        assert!(output.contains("250 lines, 10 functions"));
        assert!(output.contains("**Recommendation:** Refactor complex functions"));
        assert!(!output.contains("**God Object Metrics:**"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_god_object() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/god_class.rs"),
                total_lines: 800,
                function_count: 50,
                class_count: 1,
                avg_complexity: 8.5,
                max_complexity: 25,
                total_complexity: 425,
                coverage_percent: 0.60,
                uncovered_lines: 320,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 45,
                    fields_count: 20,
                    responsibilities: 8,
                    is_god_object: true,
                    god_object_score: 3.5,
                },
                function_scores: vec![],
            },
            score: 125.8,
            priority_rank: 1,
            recommendation: "Split into multiple focused modules".to_string(),
            impact: FileImpact {
                complexity_reduction: 50.0,
                maintainability_improvement: 60.0,
                test_effort: 30.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("### #1 - Score: 125.8"));
        assert!(output.contains("**Type:** FILE - GOD OBJECT"));
        assert!(output.contains("**God Object Metrics:**"));
        assert!(output.contains("- Methods: 45"));
        assert!(output.contains("- Fields: 20"));
        assert!(output.contains("- Responsibilities: 8"));
        assert!(output.contains("- God Object Score: 3.5"));
        assert!(output.contains("**Recommendation:** Split into multiple focused modules"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_high_complexity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/complex.rs"),
                total_lines: 600,
                function_count: 15,
                class_count: 3,
                avg_complexity: 12.0,
                max_complexity: 30,
                total_complexity: 180,
                coverage_percent: 0.50,
                uncovered_lines: 300,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 12,
                    fields_count: 8,
                    responsibilities: 4,
                    is_god_object: false,
                    god_object_score: 0.0,
                },
                function_scores: vec![],
            },
            score: 85.3,
            priority_rank: 2,
            recommendation: "Reduce complexity and improve test coverage".to_string(),
            impact: FileImpact {
                complexity_reduction: 35.0,
                maintainability_improvement: 40.0,
                test_effort: 25.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 2, &item, 0);

        assert!(output.contains("### #2 - Score: 85.3"));
        assert!(output.contains("**Type:** FILE - HIGH COMPLEXITY"));
        assert!(output.contains("600 lines"));
        assert!(!output.contains("**God Object Metrics:**"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_with_verbosity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/verbose.rs"),
                total_lines: 350,
                function_count: 12,
                class_count: 2,
                avg_complexity: 7.5,
                max_complexity: 18,
                total_complexity: 90,
                coverage_percent: 0.65,
                uncovered_lines: 122,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 10,
                    fields_count: 5,
                    responsibilities: 3,
                    is_god_object: false,
                    god_object_score: 0.0,
                },
                function_scores: vec![],
            },
            score: 55.7,
            priority_rank: 3,
            recommendation: "Consider refactoring".to_string(),
            impact: FileImpact {
                complexity_reduction: 20.0,
                maintainability_improvement: 25.0,
                test_effort: 15.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 3, &item, 1);

        assert!(output.contains("### #3 - Score: 55.7"));
        assert!(output.contains("**Scoring Breakdown:**"));
        assert!(output.contains("- File size:"));
        assert!(output.contains("- Functions:"));
        assert!(output.contains("- Complexity:"));
        assert!(output.contains("- Dependencies: 12 functions may have complex interdependencies"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_zero_functions() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/empty.rs"),
                total_lines: 100,
                function_count: 0,
                class_count: 0,
                avg_complexity: 0.0,
                max_complexity: 0,
                total_complexity: 0,
                coverage_percent: 1.0,
                uncovered_lines: 0,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 0,
                    fields_count: 0,
                    responsibilities: 0,
                    is_god_object: false,
                    god_object_score: 0.0,
                },
                function_scores: vec![],
            },
            score: 5.0,
            priority_rank: 10,
            recommendation: "No action needed".to_string(),
            impact: FileImpact {
                complexity_reduction: 0.0,
                maintainability_improvement: 0.0,
                test_effort: 0.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 10, &item, 1);

        assert!(output.contains("0 functions"));
        assert!(!output.contains("- Dependencies:"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_critical_severity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/critical.rs"),
                total_lines: 1000,
                function_count: 60,
                class_count: 5,
                avg_complexity: 15.0,
                max_complexity: 40,
                total_complexity: 900,
                coverage_percent: 0.30,
                uncovered_lines: 700,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 55,
                    fields_count: 30,
                    responsibilities: 12,
                    is_god_object: true,
                    god_object_score: 5.0,
                },
                function_scores: vec![],
            },
            score: 150.0,
            priority_rank: 1,
            recommendation: "Urgent refactoring required".to_string(),
            impact: FileImpact {
                complexity_reduction: 70.0,
                maintainability_improvement: 80.0,
                test_effort: 50.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 1, &item, 0);

        assert!(output.contains("[CRITICAL]"));
        assert!(output.contains("**Type:** FILE - GOD OBJECT"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_low_severity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/simple.rs"),
                total_lines: 150,
                function_count: 5,
                class_count: 1,
                avg_complexity: 3.0,
                max_complexity: 5,
                total_complexity: 15,
                coverage_percent: 0.90,
                uncovered_lines: 15,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 4,
                    fields_count: 2,
                    responsibilities: 1,
                    is_god_object: false,
                    god_object_score: 0.0,
                },
                function_scores: vec![],
            },
            score: 18.5,
            priority_rank: 15,
            recommendation: "Good state, minor improvements possible".to_string(),
            impact: FileImpact {
                complexity_reduction: 5.0,
                maintainability_improvement: 8.0,
                test_effort: 3.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 15, &item, 0);

        assert!(output.contains("[CRITICAL]")); // Score 18.5 is CRITICAL (>=9.0)
        assert!(output.contains("**Type:** FILE"));
    }

    #[test]
    fn test_format_file_priority_item_markdown_extreme_verbosity() {
        use crate::priority::{FileDebtItem, FileDebtMetrics, FileImpact, GodObjectIndicators};
        use std::path::PathBuf;

        let item = FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from("src/detailed.rs"),
                total_lines: 750,
                function_count: 25,
                class_count: 4,
                avg_complexity: 9.2,
                max_complexity: 22,
                total_complexity: 230,
                coverage_percent: 0.55,
                uncovered_lines: 337,
                god_object_indicators: GodObjectIndicators {
                    methods_count: 20,
                    fields_count: 12,
                    responsibilities: 5,
                    is_god_object: false,
                    god_object_score: 0.0,
                },
                function_scores: vec![],
            },
            score: 72.4,
            priority_rank: 4,
            recommendation: "Significant refactoring recommended".to_string(),
            impact: FileImpact {
                complexity_reduction: 40.0,
                maintainability_improvement: 45.0,
                test_effort: 28.0,
            },
        };

        let mut output = String::new();
        format_file_priority_item_markdown(&mut output, 4, &item, 2);

        // With verbosity 2, should include all details
        assert!(output.contains("**Scoring Breakdown:**"));
        assert!(output.contains("- File size:"));
        assert!(output.contains("HIGH")); // 750 lines is HIGH category
        assert!(output.contains("- Functions:"));
        assert!(output.contains("HIGH")); // 25 functions is HIGH category
        assert!(output.contains("- Complexity:"));
        assert!(output.contains("MODERATE")); // avg 9.2 is MODERATE category
    }
}
