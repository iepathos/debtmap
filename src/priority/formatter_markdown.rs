use crate::priority::{DebtType, UnifiedAnalysis, UnifiedDebtItem};
use std::fmt::Write;

/// Format priorities for markdown output without ANSI color codes
pub fn format_priorities_markdown(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    writeln!(output, "# Priority Technical Debt Fixes\n").unwrap();

    let top_items = analysis.get_top_priorities(limit);
    let count = top_items.len().min(limit);

    writeln!(
        output,
        "## Top {} Recommendations (by unified priority)\n",
        count
    )
    .unwrap();

    for (idx, item) in top_items.iter().enumerate() {
        format_priority_item_markdown(&mut output, idx + 1, item, verbosity);
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
        output.push_str(&format_score_breakdown(&item.unified_score));
    } else if verbosity >= 1 {
        // Show main contributing factors for verbosity >= 1
        output.push_str(&format_main_factors_with_debt_type(
            &item.unified_score,
            &item.debt_type,
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
        DebtType::Orchestration { .. } => "Orchestration",
        DebtType::Duplication { .. } => "Duplication",
        DebtType::Risk { .. } => "Risk",
        DebtType::TestComplexityHotspot { .. } => "Test Complexity",
        DebtType::TestTodo { .. } => "Test TODO",
        DebtType::TestDuplication { .. } => "Test Duplication",
        DebtType::ErrorSwallowing { .. } => "Error Swallowing",
        // Security debt types
        DebtType::HardcodedSecrets { .. } => "Hardcoded Secrets",
        DebtType::WeakCryptography { .. } => "Weak Cryptography",
        DebtType::SqlInjectionRisk { .. } => "SQL Injection Risk",
        DebtType::UnsafeCode { .. } => "Unsafe Code",
        DebtType::InputValidationGap { .. } => "Input Validation Gap",
        // Performance debt types
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
        // Basic Security and Performance debt types
        DebtType::BasicSecurity { .. } => "Security",
        DebtType::BasicPerformance { .. } => "Performance",
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

fn format_score_breakdown(unified_score: &crate::priority::UnifiedScore) -> String {
    let weights = crate::config::get_scoring_weights();
    let mut output = String::new();

    writeln!(&mut output, "\n#### Score Calculation\n").unwrap();
    writeln!(&mut output, "| Component | Value | Weight | Contribution |").unwrap();
    writeln!(&mut output, "|-----------|-------|--------|--------------|").unwrap();
    writeln!(
        &mut output,
        "| Complexity | {:.1} | {:.0}% | {:.2} |",
        unified_score.complexity_factor,
        weights.complexity * 100.0,
        unified_score.complexity_factor * weights.complexity
    )
    .unwrap();
    writeln!(
        &mut output,
        "| Coverage | {:.1} | {:.0}% | {:.2} |",
        unified_score.coverage_factor,
        weights.coverage * 100.0,
        unified_score.coverage_factor * weights.coverage
    )
    .unwrap();
    writeln!(
        &mut output,
        "| ROI | {:.1} | {:.0}% | {:.2} |",
        unified_score.roi_factor,
        weights.roi * 100.0,
        unified_score.roi_factor * weights.roi
    )
    .unwrap();
    writeln!(
        &mut output,
        "| Semantic | {:.1} | {:.0}% | {:.2} |",
        unified_score.semantic_factor,
        weights.semantic * 100.0,
        unified_score.semantic_factor * weights.semantic
    )
    .unwrap();
    writeln!(
        &mut output,
        "| Dependency | {:.1} | {:.0}% | {:.2} |",
        unified_score.dependency_factor,
        weights.dependency * 100.0,
        unified_score.dependency_factor * weights.dependency
    )
    .unwrap();

    // Only show security if non-zero
    if unified_score.security_factor > 0.0 {
        writeln!(
            &mut output,
            "| Security | {:.1} | {:.0}% | {:.2} |",
            unified_score.security_factor,
            weights.security * 100.0,
            unified_score.security_factor * weights.security
        )
        .unwrap();
    }

    // Only show organization if non-zero
    if unified_score.organization_factor > 0.0 {
        writeln!(
            &mut output,
            "| Organization | {:.1} | {:.0}% | {:.2} |",
            unified_score.organization_factor,
            weights.organization * 100.0,
            unified_score.organization_factor * weights.organization
        )
        .unwrap();
    }

    let base_score = unified_score.complexity_factor * weights.complexity
        + unified_score.coverage_factor * weights.coverage
        + unified_score.roi_factor * weights.roi
        + unified_score.semantic_factor * weights.semantic
        + unified_score.dependency_factor * weights.dependency
        + unified_score.security_factor * weights.security
        + unified_score.organization_factor * weights.organization;

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

fn format_main_factors_with_debt_type(
    unified_score: &crate::priority::UnifiedScore,
    debt_type: &crate::priority::DebtType,
) -> String {
    let weights = crate::config::get_scoring_weights();
    let mut factors = vec![];

    if unified_score.coverage_factor > 3.0 {
        factors.push(format!("Coverage gap ({:.0}%)", weights.coverage * 100.0));
    }
    if unified_score.roi_factor > 7.0 {
        factors.push(format!("High ROI ({:.0}%)", weights.roi * 100.0));
    }
    if unified_score.dependency_factor > 5.0 {
        factors.push(format!(
            "Critical path ({:.0}%)",
            weights.dependency * 100.0
        ));
    }
    if unified_score.complexity_factor > 5.0 {
        factors.push(format!("Complexity ({:.0}%)", weights.complexity * 100.0));
    }
    if unified_score.security_factor > 3.0 {
        factors.push(format!(
            "Security issues ({:.0}%)",
            weights.security * 100.0
        ));
    }
    if unified_score.organization_factor > 3.0 {
        factors.push(format!(
            "Organization issues ({:.0}%)",
            weights.organization * 100.0
        ));
    }

    // Add Security and Performance specific factors
    match debt_type {
        crate::priority::DebtType::BasicSecurity {
            severity,
            vulnerability_type,
            ..
        } => {
            factors.push(format!("Security vulnerability ({})", severity));
            if !vulnerability_type.is_empty() && vulnerability_type != "Security Issue" {
                factors.push(format!("{} detected", vulnerability_type));
            }
        }
        crate::priority::DebtType::BasicPerformance {
            impact, issue_type, ..
        } => {
            factors.push(format!("Performance impact ({})", impact));
            if !issue_type.is_empty() && issue_type != "Performance Issue" {
                factors.push(format!("{} detected", issue_type));
            }
        }
        crate::priority::DebtType::HardcodedSecrets {
            severity,
            secret_type,
        } => {
            factors.push(format!("Security vulnerability ({})", severity));
            factors.push(format!("Hardcoded {} detected", secret_type));
        }
        crate::priority::DebtType::SqlInjectionRisk { risk_level, .. } => {
            factors.push(format!("Security vulnerability ({})", risk_level));
            factors.push("SQL injection risk detected".to_string());
        }
        crate::priority::DebtType::UnsafeCode { safety_concern, .. } => {
            factors.push("Security vulnerability (High)".to_string());
            factors.push(format!("Unsafe code: {}", safety_concern));
        }
        crate::priority::DebtType::WeakCryptography { algorithm, .. } => {
            factors.push("Security vulnerability (High)".to_string());
            factors.push(format!("Weak crypto: {}", algorithm));
        }
        crate::priority::DebtType::NestedLoops { depth, .. } => {
            factors.push("Performance impact (High)".to_string());
            factors.push(format!("{} level nested loops", depth));
        }
        crate::priority::DebtType::BlockingIO { operation, .. } => {
            factors.push("Performance impact (High)".to_string());
            factors.push(format!("Blocking {}", operation));
        }
        crate::priority::DebtType::AllocationInefficiency { pattern, .. } => {
            factors.push("Performance impact (Medium)".to_string());
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
    use crate::priority::{FunctionVisibility, ImpactMetrics, UnifiedScore};

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

        let orchestration = DebtType::Orchestration {
            delegates_to: vec!["func1".to_string(), "func2".to_string()],
        };
        assert_eq!(format_debt_type(&orchestration), "Orchestration");
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
            roi_factor: 7.5,
            semantic_factor: 3.0,
            dependency_factor: 4.0,
            security_factor: 0.0,
            organization_factor: 0.0,
            role_multiplier: 1.2,
            final_score: 8.5,
        };

        let result = format_score_breakdown(&score);

        // Check for table headers
        assert!(result.contains("Score Calculation"));
        assert!(result.contains("| Component | Value | Weight | Contribution |"));

        // Check for component rows
        assert!(result.contains("| Complexity | 5.0"));
        assert!(result.contains("| Coverage | 8.0"));
        assert!(result.contains("| ROI | 7.5"));
        assert!(result.contains("| Semantic | 3.0"));
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
            roi_factor: 8.0,        // Above threshold
            semantic_factor: 2.0,
            dependency_factor: 6.0, // Above threshold
            security_factor: 0.0,
            organization_factor: 0.0,
            role_multiplier: 1.0,
            final_score: 7.0,
        };

        let debt_type = DebtType::Risk {
            risk_score: 5.0,
            factors: vec!["Test factor".to_string()],
        };

        let result = format_main_factors_with_debt_type(&score, &debt_type);

        assert!(result.contains("Main factors:"));
        assert!(result.contains("Coverage gap"));
        assert!(result.contains("High ROI"));
        assert!(result.contains("Critical path"));
        assert!(result.contains("Complexity"));
    }

    #[test]
    fn test_format_main_factors_with_no_factors() {
        let score = UnifiedScore {
            complexity_factor: 2.0, // Below all thresholds
            coverage_factor: 2.0,
            roi_factor: 3.0,
            semantic_factor: 1.0,
            dependency_factor: 2.0,
            security_factor: 0.0,
            organization_factor: 0.0,
            role_multiplier: 1.0,
            final_score: 2.0,
        };

        let debt_type = DebtType::Risk {
            risk_score: 1.0,
            factors: vec!["Test factor".to_string()],
        };

        let result = format_main_factors_with_debt_type(&score, &debt_type);
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
}
