use crate::priority::{
    DebtType, FunctionRole, FunctionVisibility, UnifiedAnalysis, UnifiedDebtItem,
};
use colored::*;
use std::fmt::Write;

#[path = "formatter_verbosity.rs"]
mod verbosity;
use self::verbosity::format_priority_item_with_verbosity;

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Default,        // Top 10 with clean formatting
    PrioritiesOnly, // Minimal list
    Detailed,       // Full analysis with priority overlay
    Top(usize),     // Top N items
    Tail(usize),    // Bottom N items (lowest priority)
}

pub fn format_priorities(analysis: &UnifiedAnalysis, format: OutputFormat) -> String {
    format_priorities_with_verbosity(analysis, format, 0)
}

pub fn format_priorities_with_verbosity(
    analysis: &UnifiedAnalysis,
    format: OutputFormat,
    verbosity: u8,
) -> String {
    match format {
        OutputFormat::Default => format_default_with_verbosity(analysis, 10, verbosity),
        OutputFormat::PrioritiesOnly => {
            format_priorities_only_with_verbosity(analysis, 10, verbosity)
        }
        OutputFormat::Detailed => format_detailed_with_verbosity(analysis, verbosity),
        OutputFormat::Top(n) => format_default_with_verbosity(analysis, n, verbosity),
        OutputFormat::Tail(n) => format_tail_with_verbosity(analysis, n, verbosity),
    }
}

fn format_default_with_verbosity(
    analysis: &UnifiedAnalysis,
    limit: usize,
    verbosity: u8,
) -> String {
    let mut output = String::new();

    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        "PRIORITY TECHNICAL DEBT FIXES".bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(output).unwrap();

    let top_items = analysis.get_top_priorities(limit);
    let count = top_items.len().min(limit);

    writeln!(
        output,
        "🎯 {} (by unified priority)",
        format!("TOP {count} RECOMMENDATIONS")
            .bright_yellow()
            .bold()
    )
    .unwrap();
    writeln!(output).unwrap();

    for (idx, item) in top_items.iter().enumerate() {
        format_priority_item_with_verbosity(&mut output, idx + 1, item, verbosity);
        writeln!(output).unwrap();
    }

    // Add summary
    writeln!(
        output,
        "📊 {}",
        format!("TOTAL DEBT SCORE: {:.0}", analysis.total_debt_score).bright_cyan()
    )
    .unwrap();

    if let Some(coverage) = analysis.overall_coverage {
        writeln!(
            output,
            "📈 {}",
            format!("OVERALL COVERAGE: {:.2}%", coverage).bright_green()
        )
        .unwrap();
    }

    output
}

#[allow(dead_code)]
fn format_default(analysis: &UnifiedAnalysis, limit: usize) -> String {
    format_default_with_verbosity(analysis, limit, 0)
}

fn format_priorities_only_with_verbosity(
    analysis: &UnifiedAnalysis,
    limit: usize,
    _verbosity: u8,
) -> String {
    // Use the original format_priorities_only for backward compatibility
    format_priorities_only(analysis, limit)
}

fn format_detailed_with_verbosity(analysis: &UnifiedAnalysis, verbosity: u8) -> String {
    let mut output = String::new();

    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        "UNIFIED PRIORITY ANALYSIS".bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(output).unwrap();

    for (idx, item) in analysis.items.iter().enumerate() {
        format_priority_item_with_verbosity(&mut output, idx + 1, item, verbosity);
        writeln!(output).unwrap();
    }

    output
}

fn format_tail_with_verbosity(analysis: &UnifiedAnalysis, n: usize, verbosity: u8) -> String {
    let mut output = String::new();

    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        "LOWEST PRIORITY ITEMS".bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(output).unwrap();

    let tail_items = analysis.get_bottom_priorities(n);
    let start_rank = (analysis.items.len() - tail_items.len()) + 1;

    for (idx, item) in tail_items.iter().enumerate() {
        format_priority_item_with_verbosity(&mut output, start_rank + idx, item, verbosity);
        writeln!(output).unwrap();
    }

    output
}

#[allow(dead_code)]
fn format_tail(analysis: &UnifiedAnalysis, limit: usize) -> String {
    let mut output = String::new();

    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        "LOWEST PRIORITY TECHNICAL DEBT".bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(output).unwrap();

    let bottom_items = analysis.get_bottom_priorities(limit);
    let count = bottom_items.len().min(limit);
    let total_items = analysis.items.len();

    writeln!(
        output,
        "📉 {} (items {}-{})",
        format!("BOTTOM {count} ITEMS").bright_yellow().bold(),
        total_items.saturating_sub(count - 1),
        total_items
    )
    .unwrap();
    writeln!(output).unwrap();

    for (idx, item) in bottom_items.iter().enumerate() {
        if idx >= limit {
            break;
        }
        let rank = total_items - bottom_items.len() + idx + 1;
        format_priority_item(&mut output, rank, item);
        writeln!(output).unwrap();
    }

    // Add total debt score
    writeln!(output).unwrap();
    writeln!(
        output,
        "📊 {}",
        format!("TOTAL DEBT SCORE: {:.0}", analysis.total_debt_score)
            .bright_cyan()
            .bold()
    )
    .unwrap();

    // Add overall coverage if available
    if let Some(coverage) = analysis.overall_coverage {
        writeln!(
            output,
            "📈 {}",
            format!("OVERALL COVERAGE: {coverage:.2}%")
                .bright_green()
                .bold()
        )
        .unwrap();
    }

    output
}

fn format_priorities_only(analysis: &UnifiedAnalysis, limit: usize) -> String {
    let mut output = String::new();

    writeln!(output, "{}", "TOP PRIORITIES:".bright_white().bold()).unwrap();

    let top_items = analysis.get_top_priorities(limit);
    for (idx, item) in top_items.iter().enumerate() {
        if idx >= limit {
            break;
        }
        writeln!(
            output,
            "{}. {}: {}:{} {}()",
            idx + 1,
            get_action_verb(&item.debt_type),
            item.location.file.display(),
            item.location.line,
            item.location.function
        )
        .unwrap();
    }

    writeln!(output).unwrap();

    let critical_count = top_items
        .iter()
        .filter(|i| i.unified_score.final_score >= 8.0)
        .count();
    let high_count = top_items
        .iter()
        .filter(|i| i.unified_score.final_score >= 6.0 && i.unified_score.final_score < 8.0)
        .count();

    writeln!(
        output,
        "High-impact items: {critical_count} critical, {high_count} high priority"
    )
    .unwrap();
    writeln!(output, "Focus on measurable code quality improvements").unwrap();

    output
}

#[allow(dead_code)]
fn format_detailed(analysis: &UnifiedAnalysis) -> String {
    let mut output = String::new();

    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(
        output,
        "    {}",
        "UNIFIED PRIORITY ANALYSIS".bright_white().bold()
    )
    .unwrap();
    writeln!(output, "{}", "═".repeat(44).bright_blue()).unwrap();
    writeln!(output).unwrap();

    for (idx, item) in analysis.items.iter().enumerate() {
        format_detailed_item(&mut output, idx + 1, item);
        writeln!(output).unwrap();
    }

    output
}

#[allow(dead_code)]
fn format_priority_item(output: &mut String, rank: usize, item: &UnifiedDebtItem) {
    let severity = get_severity_label(item.unified_score.final_score);
    let severity_color = get_severity_color(item.unified_score.final_score);

    writeln!(
        output,
        "#{} {} [{}]",
        rank.to_string().bright_cyan().bold(),
        format!("SCORE: {:.1}", item.unified_score.final_score).bright_white(),
        severity.color(severity_color).bold()
    )
    .unwrap();

    writeln!(
        output,
        "├─ {}: {}:{} {}()",
        format_debt_type(&item.debt_type).bright_yellow(),
        item.location.file.display(),
        item.location.line,
        item.location.function.bright_green()
    )
    .unwrap();

    writeln!(
        output,
        "├─ ACTION: {}",
        item.recommendation.primary_action.bright_white()
    )
    .unwrap();

    writeln!(
        output,
        "├─ IMPACT: {}",
        format_impact(&item.expected_impact).bright_cyan()
    )
    .unwrap();

    // Add complexity details with branch information
    let (cyclomatic, cognitive, branch_count, nesting, _length) = extract_complexity_info(item);
    if cyclomatic > 0 || cognitive > 0 {
        writeln!(
            output,
            "├─ COMPLEXITY: cyclomatic={}, branches={}, cognitive={}, nesting={}",
            cyclomatic.to_string().dimmed(),
            branch_count.to_string().dimmed(),
            cognitive.to_string().dimmed(),
            nesting.to_string().dimmed()
        )
        .unwrap();
    }

    // Add dependency information with caller/callee names
    let (upstream, downstream) = extract_dependency_info(item);
    if upstream > 0 || downstream > 0 {
        writeln!(
            output,
            "├─ DEPENDENCIES: {} upstream, {} downstream",
            upstream.to_string().dimmed(),
            downstream.to_string().dimmed()
        )
        .unwrap();

        // Add upstream callers if present
        if !item.upstream_callers.is_empty() {
            let callers_display = if item.upstream_callers.len() <= 3 {
                item.upstream_callers.join(", ")
            } else {
                format!(
                    "{}, ... ({} more)",
                    item.upstream_callers[..3].join(", "),
                    item.upstream_callers.len() - 3
                )
            };
            writeln!(output, "│  ├─ CALLERS: {}", callers_display.bright_blue()).unwrap();
        }

        // Add downstream callees if present
        if !item.downstream_callees.is_empty() {
            let callees_display = if item.downstream_callees.len() <= 3 {
                item.downstream_callees.join(", ")
            } else {
                format!(
                    "{}, ... ({} more)",
                    item.downstream_callees[..3].join(", "),
                    item.downstream_callees.len() - 3
                )
            };
            writeln!(output, "│  └─ CALLS: {}", callees_display.bright_green()).unwrap();
        }
    }

    // Add dead code specific information
    if let DebtType::DeadCode {
        visibility,
        usage_hints,
        ..
    } = &item.debt_type
    {
        writeln!(
            output,
            "├─ VISIBILITY: {} function with no callers",
            format_visibility(visibility).yellow()
        )
        .unwrap();

        for hint in usage_hints {
            writeln!(output, "│  • {}", hint.dimmed()).unwrap();
        }
    }

    writeln!(output, "└─ WHY: {}", item.recommendation.rationale.dimmed()).unwrap();
}

#[allow(dead_code)]
fn format_detailed_item(output: &mut String, rank: usize, item: &UnifiedDebtItem) {
    writeln!(
        output,
        "#{} {}() - UNIFIED SCORE: {:.1}",
        rank,
        item.location.function.bright_green(),
        item.unified_score.final_score
    )
    .unwrap();

    writeln!(
        output,
        "├─ Function Role: {} ({:.1}x multiplier)",
        format_role(item.function_role),
        item.unified_score.role_multiplier
    )
    .unwrap();

    writeln!(output, "├─ Score Breakdown:").unwrap();
    writeln!(
        output,
        "│  ├─ Coverage Factor: {:.1}",
        item.unified_score.coverage_factor
    )
    .unwrap();

    if let Some(ref cov) = item.transitive_coverage {
        writeln!(
            output,
            "│  │  └─ ({:.0}% direct, {:.0}% transitive)",
            cov.direct * 100.0,
            cov.transitive * 100.0
        )
        .unwrap();
    }

    writeln!(
        output,
        "│  ├─ Complexity Factor: {:.1}",
        item.unified_score.complexity_factor
    )
    .unwrap();
    writeln!(
        output,
        "│  ├─ Dependency Factor: {:.1}",
        item.unified_score.dependency_factor
    )
    .unwrap();
    writeln!(
        output,
        "│  └─ Security Factor: {:.1}",
        item.unified_score.security_factor
    )
    .unwrap();

    writeln!(
        output,
        "└─ Recommendation: {}",
        item.recommendation.primary_action
    )
    .unwrap();

    for step in &item.recommendation.implementation_steps {
        writeln!(output, "   • {step}").unwrap();
    }
}

#[cfg_attr(test, allow(dead_code))]
pub(crate) fn _format_total_impact(output: &mut String, analysis: &UnifiedAnalysis) {
    writeln!(output).unwrap();
    writeln!(
        output,
        "📊 {}",
        "TOTAL IMPACT IF ALL FIXED".bright_green().bold()
    )
    .unwrap();

    let impact = &analysis.total_impact;

    if impact.coverage_improvement > 0.0 {
        writeln!(
            output,
            "• +{:.1}% test coverage potential",
            impact.coverage_improvement
        )
        .unwrap();
    }

    if impact.lines_reduction > 0 {
        writeln!(output, "• -{} lines of code", impact.lines_reduction).unwrap();
    }

    if impact.complexity_reduction > 0.0 {
        writeln!(
            output,
            "• -{:.0}% average complexity",
            impact.complexity_reduction
        )
        .unwrap();
    }

    writeln!(
        output,
        "• {} actionable items prioritized by measurable impact",
        analysis.items.len()
    )
    .unwrap();
}

pub fn format_impact(impact: &crate::priority::ImpactMetrics) -> String {
    let mut parts = Vec::new();

    if impact.coverage_improvement > 0.0 {
        // Show function-level coverage improvement
        if impact.coverage_improvement >= 100.0 {
            parts.push("Full test coverage".to_string());
        } else if impact.coverage_improvement >= 50.0 {
            parts.push(format!(
                "+{}% function coverage",
                impact.coverage_improvement as i32
            ));
        } else {
            // For complex functions that need refactoring first
            parts.push("Partial coverage after refactor".to_string());
        }
    }

    if impact.complexity_reduction > 0.0 {
        parts.push(format!(
            "-{} complexity",
            impact.complexity_reduction as i32
        ));
    }

    if impact.risk_reduction > 0.0 {
        parts.push(format!("-{:.1} risk", impact.risk_reduction));
    }

    if impact.lines_reduction > 0 {
        parts.push(format!("-{} LOC", impact.lines_reduction));
    }

    if parts.is_empty() {
        "Improved maintainability".to_string()
    } else {
        parts.join(", ")
    }
}

pub fn format_debt_type(debt_type: &DebtType) -> &'static str {
    match debt_type {
        DebtType::TestingGap { .. } => "TEST GAP",
        DebtType::ComplexityHotspot { .. } => "COMPLEXITY",
        DebtType::DeadCode { .. } => "DEAD CODE",
        DebtType::Orchestration { .. } => "ORCHESTRATION",
        DebtType::Duplication { .. } => "DUPLICATION",
        DebtType::Risk { .. } => "RISK",
        DebtType::TestComplexityHotspot { .. } => "TEST COMPLEXITY",
        DebtType::TestTodo { .. } => "TEST TODO",
        DebtType::TestDuplication { .. } => "TEST DUPLICATION",
        DebtType::ErrorSwallowing { .. } => "ERROR SWALLOWING",
        // Security debt types
        DebtType::HardcodedSecrets { .. } => "HARDCODED SECRETS",
        DebtType::WeakCryptography { .. } => "WEAK CRYPTO",
        DebtType::SqlInjectionRisk { .. } => "SQL INJECTION",
        DebtType::UnsafeCode { .. } => "UNSAFE CODE",
        DebtType::InputValidationGap { .. } => "INPUT VALIDATION",
        // Resource Management debt types
        DebtType::AllocationInefficiency { .. } => "ALLOCATION",
        DebtType::StringConcatenation { .. } => "STRING CONCAT",
        DebtType::NestedLoops { .. } => "NESTED LOOPS",
        DebtType::BlockingIO { .. } => "BLOCKING I/O",
        DebtType::SuboptimalDataStructure { .. } => "DATA STRUCTURE",
        // Organization debt types
        DebtType::GodObject { .. } => "GOD OBJECT",
        DebtType::FeatureEnvy { .. } => "FEATURE ENVY",
        DebtType::PrimitiveObsession { .. } => "PRIMITIVE OBSESSION",
        DebtType::MagicValues { .. } => "MAGIC VALUES",
        // Testing quality debt types
        DebtType::AssertionComplexity { .. } => "ASSERTION COMPLEXITY",
        DebtType::FlakyTestPattern { .. } => "FLAKY TEST",
        // Resource management debt types
        DebtType::AsyncMisuse { .. } => "ASYNC MISUSE",
        DebtType::ResourceLeak { .. } => "RESOURCE LEAK",
        DebtType::CollectionInefficiency { .. } => "COLLECTION INEFFICIENCY",
        // Basic Security debt type (for core::DebtType integration)
        DebtType::BasicSecurity { .. } => "SECURITY",
    }
}

#[allow(dead_code)]
fn format_role(role: FunctionRole) -> &'static str {
    match role {
        FunctionRole::PureLogic => "PureLogic",
        FunctionRole::Orchestrator => "Orchestrator",
        FunctionRole::IOWrapper => "IOWrapper",
        FunctionRole::EntryPoint => "EntryPoint",
        FunctionRole::PatternMatch => "PatternMatch",
        FunctionRole::Unknown => "Unknown",
    }
}

fn get_action_verb(debt_type: &DebtType) -> &'static str {
    match debt_type {
        DebtType::TestingGap { .. } => "Add tests",
        DebtType::ComplexityHotspot { .. } => "Reduce complexity",
        DebtType::DeadCode { .. } => "Remove dead code",
        DebtType::Orchestration { .. } => "Refactor to pure functions",
        DebtType::Duplication { .. } => "Extract duplication",
        DebtType::Risk { .. } => "Fix debt",
        DebtType::TestComplexityHotspot { .. } => "Simplify test",
        DebtType::TestTodo { .. } => "Complete TODO",
        DebtType::TestDuplication { .. } => "Remove test duplication",
        DebtType::ErrorSwallowing { .. } => "Fix error swallowing",
        // Security debt types
        DebtType::HardcodedSecrets { .. } => "Remove hardcoded secrets",
        DebtType::WeakCryptography { .. } => "Upgrade cryptography",
        DebtType::SqlInjectionRisk { .. } => "Secure SQL queries",
        DebtType::UnsafeCode { .. } => "Justify or remove unsafe",
        DebtType::InputValidationGap { .. } => "Add input validation",
        // Resource Management debt types
        DebtType::AllocationInefficiency { .. } => "Optimize allocations",
        DebtType::StringConcatenation { .. } => "Use string builder",
        DebtType::NestedLoops { .. } => "Reduce loop complexity",
        DebtType::BlockingIO { .. } => "Make async",
        DebtType::SuboptimalDataStructure { .. } => "Change data structure",
        // Organization debt types
        DebtType::GodObject { .. } => "Split responsibilities",
        DebtType::FeatureEnvy { .. } => "Move method",
        DebtType::PrimitiveObsession { .. } => "Create domain type",
        DebtType::MagicValues { .. } => "Extract constants",
        // Testing quality debt types
        DebtType::AssertionComplexity { .. } => "Simplify assertions",
        DebtType::FlakyTestPattern { .. } => "Fix test reliability",
        // Resource management debt types
        DebtType::AsyncMisuse { .. } => "Fix async pattern",
        DebtType::ResourceLeak { .. } => "Add cleanup",
        DebtType::CollectionInefficiency { .. } => "Optimize collection usage",
        // Basic Security debt type
        DebtType::BasicSecurity { .. } => "Fix security issue",
    }
}

pub fn get_severity_label(score: f64) -> &'static str {
    if score >= 8.0 {
        "CRITICAL"
    } else if score >= 6.0 {
        "HIGH"
    } else if score >= 4.0 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

pub fn get_severity_color(score: f64) -> colored::Color {
    if score >= 8.0 {
        Color::Red
    } else if score >= 6.0 {
        Color::Yellow
    } else if score >= 4.0 {
        Color::Blue
    } else {
        Color::Green
    }
}

pub fn extract_complexity_info(item: &UnifiedDebtItem) -> (u32, u32, u32, u32, usize) {
    // Always show complexity metrics from the item itself, regardless of debt type
    let cyclomatic = item.cyclomatic_complexity;
    let cognitive = item.cognitive_complexity;
    let branch_count = cyclomatic; // Use cyclomatic as proxy for branch count

    (
        cyclomatic,
        cognitive,
        branch_count,
        item.nesting_depth,
        item.function_length,
    )
}

pub fn extract_dependency_info(item: &UnifiedDebtItem) -> (usize, usize) {
    (item.upstream_dependencies, item.downstream_dependencies)
}

#[allow(dead_code)]
fn format_visibility(visibility: &FunctionVisibility) -> &'static str {
    match visibility {
        FunctionVisibility::Private => "private",
        FunctionVisibility::Crate => "crate-public",
        FunctionVisibility::Public => "public",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::CallGraph;
    use crate::priority::unified_scorer::Location;
    use crate::priority::{ActionableRecommendation, ImpactMetrics, UnifiedScore};
    use std::path::PathBuf;

    fn strip_ansi_codes(s: &str) -> String {
        // Simple regex to strip ANSI escape codes
        let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
        re.replace_all(s, "").to_string()
    }

    fn create_test_item(score: f64) -> UnifiedDebtItem {
        // Use score as part of line number to make each test item unique
        // This prevents duplicate detection from filtering test items
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_func".to_string(),
                line: (score * 10.0) as usize,
            },
            debt_type: DebtType::TestingGap {
                coverage: 0.1,
                cyclomatic: 5,
                cognitive: 7,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 8.0,
                dependency_factor: 3.0,
                security_factor: 0.0,
                role_multiplier: 1.0,
                final_score: score,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Add unit tests".to_string(),
                rationale: "Low coverage critical function".to_string(),
                implementation_steps: vec!["Write tests".to_string()],
                related_items: vec![],
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 50.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 3.5,
            },
            transitive_coverage: None,
            upstream_dependencies: 2,
            downstream_dependencies: 3,
            upstream_callers: vec!["main".to_string(), "process_data".to_string()],
            downstream_callees: vec![
                "validate".to_string(),
                "transform".to_string(),
                "save".to_string(),
            ],
            nesting_depth: 1,
            function_length: 15,
            cyclomatic_complexity: 5,
            cognitive_complexity: 7,
            is_pure: None,
            purity_confidence: None,
            entropy_details: None,
        }
    }

    #[test]
    fn test_format_priorities_only() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.add_item(create_test_item(9.0));
        analysis.add_item(create_test_item(7.0));
        analysis.add_item(create_test_item(5.0));
        analysis.sort_by_priority();

        let output = format_priorities(&analysis, OutputFormat::PrioritiesOnly);
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("TOP PRIORITIES:"));
        assert!(output_plain.contains("1. Add tests"));
        assert!(output_plain.contains("High-impact items"));
    }

    #[test]
    fn test_format_default() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.add_item(create_test_item(9.0));
        analysis.add_item(create_test_item(7.0));
        analysis.sort_by_priority();
        analysis.calculate_total_impact();

        let output = format_priorities(&analysis, OutputFormat::Default);

        // Strip ANSI color codes for testing
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("PRIORITY TECHNICAL DEBT FIXES"));
        assert!(output_plain.contains("TOP 2 RECOMMENDATIONS"));
        assert!(output_plain.contains("SCORE: 9.0"));
        assert!(output_plain.contains("[CRITICAL]"));
        // assert!(output_plain.contains("TOTAL IMPACT"));
    }

    #[test]
    fn test_severity_labels() {
        assert_eq!(get_severity_label(9.0), "CRITICAL");
        assert_eq!(get_severity_label(7.0), "HIGH");
        assert_eq!(get_severity_label(5.0), "MEDIUM");
        assert_eq!(get_severity_label(2.0), "LOW");
    }

    #[test]
    fn test_format_total_impact_with_all_improvements() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.total_impact = ImpactMetrics {
            coverage_improvement: 25.5,
            lines_reduction: 150,
            complexity_reduction: 12.7,
            risk_reduction: 8.2,
        };
        analysis.add_item(create_test_item(7.0));
        analysis.add_item(create_test_item(5.0));

        let mut output = String::new();
        super::_format_total_impact(&mut output, &analysis);
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("TOTAL IMPACT IF ALL FIXED"));
        assert!(output_plain.contains("+25.5% test coverage potential"));
        assert!(output_plain.contains("-150 lines of code"));
        assert!(output_plain.contains("-13% average complexity")); // 12.7 rounds to 13
        assert!(output_plain.contains("2 actionable items prioritized"));
    }

    #[test]
    fn test_format_total_impact_coverage_only() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.total_impact = ImpactMetrics {
            coverage_improvement: 45.3,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 5.0,
        };
        analysis.add_item(create_test_item(8.0));

        let mut output = String::new();
        super::_format_total_impact(&mut output, &analysis);
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("+45.3% test coverage potential"));
        assert!(!output_plain.contains("lines of code")); // Should not show 0 lines
        assert!(!output_plain.contains("average complexity")); // Should not show 0 complexity
        assert!(output_plain.contains("1 actionable items prioritized"));
    }

    #[test]
    fn test_format_total_impact_complexity_and_lines() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.total_impact = ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 75,
            complexity_reduction: 8.9,
            risk_reduction: 3.2,
        };
        analysis.add_item(create_test_item(6.0));
        analysis.add_item(create_test_item(4.0));
        analysis.add_item(create_test_item(3.0));

        let mut output = String::new();
        super::_format_total_impact(&mut output, &analysis);
        let output_plain = strip_ansi_codes(&output);

        assert!(!output_plain.contains("test coverage")); // Should not show 0 coverage
        assert!(output_plain.contains("-75 lines of code"));
        assert!(output_plain.contains("-9% average complexity")); // 8.9 rounds to 9
        assert!(output_plain.contains("3 actionable items prioritized"));
    }

    #[test]
    fn test_format_total_impact_no_improvements() {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        analysis.total_impact = ImpactMetrics {
            coverage_improvement: 0.0,
            lines_reduction: 0,
            complexity_reduction: 0.0,
            risk_reduction: 0.0,
        };
        // Empty analysis with no items

        let mut output = String::new();
        super::_format_total_impact(&mut output, &analysis);
        let output_plain = strip_ansi_codes(&output);

        assert!(output_plain.contains("TOTAL IMPACT IF ALL FIXED"));
        assert!(!output_plain.contains("test coverage")); // No coverage improvement
        assert!(!output_plain.contains("lines of code")); // No lines reduction
        assert!(!output_plain.contains("average complexity")); // No complexity reduction
        assert!(output_plain.contains("0 actionable items prioritized"));
    }

    #[test]
    fn test_debt_type_formatting() {
        assert_eq!(
            format_debt_type(&DebtType::TestingGap {
                coverage: 0.1,
                cyclomatic: 5,
                cognitive: 7
            }),
            "TEST GAP"
        );
        assert_eq!(
            format_debt_type(&DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 15
            }),
            "COMPLEXITY"
        );
        assert_eq!(
            format_debt_type(&DebtType::Duplication {
                instances: 3,
                total_lines: 60
            }),
            "DUPLICATION"
        );
    }

    #[test]
    fn test_format_role_pure_logic() {
        assert_eq!(format_role(FunctionRole::PureLogic), "PureLogic");
    }

    #[test]
    fn test_format_role_orchestrator() {
        assert_eq!(format_role(FunctionRole::Orchestrator), "Orchestrator");
    }

    #[test]
    fn test_format_role_io_wrapper() {
        assert_eq!(format_role(FunctionRole::IOWrapper), "IOWrapper");
    }

    #[test]
    fn test_format_role_entry_point() {
        assert_eq!(format_role(FunctionRole::EntryPoint), "EntryPoint");
    }

    #[test]
    fn test_format_role_unknown() {
        assert_eq!(format_role(FunctionRole::Unknown), "Unknown");
    }
}
