use super::{
    Difficulty, FunctionRisk, RiskAnalyzer, RiskCategory, RiskInsight, TestingRecommendation,
};
use crate::risk::correlation::analyze_risk_insights;
use crate::risk::priority::prioritize_by_roi;
use im::Vector;

pub fn generate_risk_insights(
    functions: Vector<FunctionRisk>,
    analyzer: &RiskAnalyzer,
) -> RiskInsight {
    let mut insights = analyze_risk_insights(functions.clone());

    // Generate testing recommendations
    insights.risk_reduction_opportunities = prioritize_by_roi(&functions, analyzer);

    insights
}

pub fn format_critical_risks(risks: &Vector<FunctionRisk>) -> String {
    let mut output = String::new();

    // Filter out test functions from critical risks display
    let critical_risks: Vec<&FunctionRisk> = risks
        .iter()
        .filter(|r| !r.is_test_function && matches!(r.risk_category, RiskCategory::Critical))
        .take(5)
        .collect();

    if critical_risks.is_empty() {
        return output;
    }

    output.push_str("🔥 CRITICAL RISK FUNCTIONS (Complexity > 15, Coverage < 30%)\n");
    output.push_str("─────────────────────────────────────────────────────────────\n");

    for (i, risk) in critical_risks.iter().enumerate() {
        output.push_str(&format!(
            "{}. {}:{}::{}\n",
            i + 1,
            risk.file.display(),
            risk.line_range.0,
            risk.function_name
        ));
        output.push_str(&format!(
            "   Risk Score: {:.1} (CRITICAL)\n",
            risk.risk_score
        ));
        output.push_str(&format!(
            "   Cyclomatic: {} | Cognitive: {} | Coverage: {}\n",
            risk.cyclomatic_complexity,
            risk.cognitive_complexity,
            risk.coverage_percentage
                .map(|c| format!("{c:.0}%"))
                .unwrap_or_else(|| "N/A".to_string())
        ));
        output.push_str(&format!(
            "   Test Effort: {:?} ({}-{} test cases)\n",
            risk.test_effort.estimated_difficulty,
            risk.test_effort.recommended_test_cases,
            risk.test_effort.recommended_test_cases + 2
        ));
        output.push('\n');
    }

    output
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

// Pure functions for formatting components

pub fn format_risk_reduction(potential_risk_reduction: f64) -> String {
    if potential_risk_reduction < 0.5 {
        "<1".to_string()
    } else {
        format!("{potential_risk_reduction:.0}")
    }
}

pub fn format_roi_display(roi_score: f64) -> String {
    if roi_score >= 10.0 {
        format!("{roi_score:.0}")
    } else {
        format!("{roi_score:.1}")
    }
}

pub fn determine_risk_level(current_risk: f64) -> &'static str {
    if current_risk >= 8.0 {
        "HIGH"
    } else if current_risk >= 5.0 {
        "MEDIUM"
    } else {
        "LOW"
    }
}

pub fn format_difficulty(difficulty: &Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Trivial => "trivial",
        Difficulty::Simple => "simple",
        Difficulty::Moderate => "moderate",
        Difficulty::Complex => "complex",
        Difficulty::VeryComplex => "very complex",
    }
}

pub fn format_complexity_info(branch_count: u32, cognitive_load: u32) -> String {
    format!("branches={branch_count}, cognitive={cognitive_load}")
}

pub fn format_dependency_info(dependencies_count: usize, dependents_count: usize) -> String {
    format!("{dependencies_count} upstream, {dependents_count} downstream")
}

pub fn calculate_dash_count(header_len: usize, roi_label_len: usize) -> usize {
    82 - header_len - roi_label_len - 8 // Account for "┌─ " + " ─┐" + spaces
}

pub fn format_recommendation_box_header(index: usize, roi_display: &str) -> String {
    let index_num = index + 1;
    let header = format!("#{index_num}");
    let roi_label = format!("ROI: {roi_display}");
    let dash_count = calculate_dash_count(header.len(), roi_label.len());

    let dashes = "─".repeat(dash_count);
    format!("┌─ {header} {dashes} {roi_label} ─┐\n")
}

pub fn format_recommendations(recommendations: &Vector<TestingRecommendation>) -> String {
    let mut output = String::new();

    if recommendations.is_empty() {
        return output;
    }

    output.push_str("🎯 TOP 5 TESTING RECOMMENDATIONS\n");
    output.push_str("Ordered by ROI (Risk Reduction / Test Effort)\n");
    output.push('\n');

    for (i, rec) in recommendations.iter().take(5).enumerate() {
        let roi_score = rec.roi.unwrap_or(0.1);
        let risk_reduction = format_risk_reduction(rec.potential_risk_reduction);
        let roi_display = format_roi_display(roi_score);

        // Format file path with line number
        let file_str = rec.file.to_string_lossy();
        let line = rec.line;
        let location_display = format!("{file_str}:{line}");

        // Determine risk level string
        let risk_level = determine_risk_level(rec.current_risk);

        // Format complexity based on test effort
        let complexity_desc = format_difficulty(&rec.test_effort_estimate.estimated_difficulty);
        let complexity_str = format_complexity_info(
            rec.test_effort_estimate.branch_count,
            rec.test_effort_estimate.cognitive_load,
        );

        // Format dependency info
        let deps_info = format_dependency_info(rec.dependencies.len(), rec.dependents.len());

        // Create the top border with proper spacing
        output.push_str(&format_recommendation_box_header(i, &roi_display));

        // Function and location line - pad to 78 chars (82 - 4 for "│ " and " │")
        let func_name = &rec.function;
        let func_loc = format!("{func_name}() @ {location_display}");
        output.push_str(&format!("│ {func_loc:<78} │\n"));

        // Divider - exactly 82 chars total
        output.push_str(
            "├────────────────────────────────────────────────────────────────────────────────┤\n",
        );

        // Risk line
        let current_risk = rec.current_risk;
        let risk_line = format!(
            "Risk: {risk_level} ({current_risk:.1})  Impact: -{risk_reduction}%  Complexity: {complexity_desc} ({complexity_str})"
        );
        output.push_str(&format!("│ {risk_line:<78} │\n"));

        // Dependencies line
        let deps_line = format!("Dependencies: {deps_info}");
        output.push_str(&format!("│ {deps_line:<78} │\n"));

        // Rationale lines (wrapped)
        let rationale_lines = wrap_text(&rec.rationale, 78);
        for line in rationale_lines {
            output.push_str(&format!("│ {line:<78} │\n"));
        }

        // Add used by info if present
        if !rec.dependents.is_empty() {
            let dependents = rec.dependents.join(", ");
            let used_by = format!("Used by: {dependents}");
            let used_by_lines = wrap_text(&used_by, 78);
            for line in used_by_lines {
                output.push_str(&format!("│ {line:<78} │\n"));
            }
        }

        output.push_str(
            "└────────────────────────────────────────────────────────────────────────────────┘\n",
        );
        output.push('\n');
    }

    output
}

fn format_critical_functions_insight(critical_count: usize) -> Option<String> {
    (critical_count > 0).then(|| {
        format!(
            "• Focus testing on the {} critical risk function{} first\n",
            critical_count,
            if critical_count == 1 { "" } else { "s" }
        )
    })
}

fn format_correlation_insight(correlation: f64) -> Option<String> {
    match correlation {
        c if c < -0.3 => Some(format!(
            "• ✅ Good news: Complex code tends to be better tested (correlation: {c:.2})\n"
        )),
        c if c > 0.3 => Some(format!(
            "• ⚠️  Warning: Complex code lacks coverage (correlation: {c:.2})\n"
        )),
        _ => None,
    }
}

fn calculate_estimated_effort(insight: &RiskInsight, total_high_risk: usize) -> u32 {
    insight
        .top_risks
        .iter()
        .take(total_high_risk)
        .map(|r| r.test_effort.recommended_test_cases)
        .sum()
}

fn format_effort_insight(estimated_effort: u32) -> Option<String> {
    (estimated_effort > 0).then(|| {
        format!(
            "• Estimated test effort for safe risk level: {}-{} test cases\n",
            estimated_effort,
            estimated_effort + (estimated_effort / 2)
        )
    })
}

fn calculate_total_risk_reduction(insight: &RiskInsight) -> f64 {
    insight
        .risk_reduction_opportunities
        .iter()
        .map(|r| r.potential_risk_reduction)
        .sum()
}

fn format_risk_reduction_insight(total_reduction: f64) -> Option<String> {
    (total_reduction >= 1.0).then(|| {
        format!("• Potential risk reduction from top 5 recommendations: {total_reduction:.0}%\n")
    })
}

pub fn format_actionable_insights(insight: &RiskInsight) -> String {
    let critical_count = insight.risk_distribution.critical_count;
    let high_count = insight.risk_distribution.high_count;
    let total_high_risk = critical_count + high_count;

    let insights = [
        format_critical_functions_insight(critical_count),
        insight
            .complexity_coverage_correlation
            .and_then(format_correlation_insight),
        (total_high_risk > 0)
            .then(|| {
                let effort = calculate_estimated_effort(insight, total_high_risk);
                format_effort_insight(effort)
            })
            .flatten(),
        (!insight.risk_reduction_opportunities.is_empty())
            .then(|| {
                let reduction = calculate_total_risk_reduction(insight);
                format_risk_reduction_insight(reduction)
            })
            .flatten(),
    ];

    format!(
        "💡 ACTIONABLE INSIGHTS\n──────────────────────\n{}",
        insights.into_iter().flatten().collect::<String>()
    )
}
