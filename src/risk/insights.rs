use super::{FunctionRisk, RiskAnalyzer, RiskCategory, RiskInsight, TestingRecommendation};
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
        let risk_reduction = if rec.potential_risk_reduction < 0.5 {
            "<1".to_string()
        } else {
            format!("{:.0}", rec.potential_risk_reduction)
        };

        let roi_display = if roi_score >= 10.0 {
            format!("{roi_score:.0}")
        } else {
            format!("{roi_score:.1}")
        };

        // Format file path with line number
        let file_str = rec.file.to_string_lossy();
        let location_display = format!("{}:{}", file_str, rec.line);

        // Determine risk level string
        let risk_level = if rec.current_risk >= 8.0 {
            "HIGH"
        } else if rec.current_risk >= 5.0 {
            "MEDIUM"
        } else {
            "LOW"
        };

        // Format complexity based on test effort
        let complexity_desc = match rec.test_effort_estimate.estimated_difficulty {
            super::Difficulty::Trivial => "trivial",
            super::Difficulty::Simple => "simple",
            super::Difficulty::Moderate => "moderate",
            super::Difficulty::Complex => "complex",
            super::Difficulty::VeryComplex => "very complex",
        };

        let complexity_str = format!(
            "branches={}, cognitive={}",
            rec.test_effort_estimate.branch_count, rec.test_effort_estimate.cognitive_load
        );

        // Format dependency info
        let deps_info = format!(
            "{} upstream, {} downstream",
            rec.dependencies.len(),
            rec.dependents.len()
        );

        // Create the top border with proper spacing
        let header = format!("#{}", i + 1);
        let roi_label = format!("ROI: {roi_display}");
        let dash_count = 82 - header.len() - roi_label.len() - 8; // Account for "┌─ " + " ─┐" + spaces

        output.push_str(&format!(
            "┌─ {} {} {} ─┐\n",
            header,
            "─".repeat(dash_count),
            roi_label
        ));

        // Function and location line - pad to 78 chars (82 - 4 for "│ " and " │")
        let func_loc = format!("{}() @ {}", rec.function, location_display);
        output.push_str(&format!("│ {func_loc:<78} │\n"));

        // Divider - exactly 82 chars total
        output.push_str(
            "├────────────────────────────────────────────────────────────────────────────────┤\n",
        );

        // Risk line
        let risk_line = format!(
            "Risk: {} ({:.1})  Impact: -{}%  Complexity: {} ({})",
            risk_level, rec.current_risk, risk_reduction, complexity_desc, complexity_str
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
            let used_by = format!("Used by: {}", rec.dependents.join(", "));
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

pub fn format_actionable_insights(insight: &RiskInsight) -> String {
    let mut output = String::new();

    output.push_str("💡 ACTIONABLE INSIGHTS\n");
    output.push_str("──────────────────────\n");

    let critical_count = insight.risk_distribution.critical_count;
    let high_count = insight.risk_distribution.high_count;

    if critical_count > 0 {
        output.push_str(&format!(
            "• Focus testing on the {} critical risk function{} first\n",
            critical_count,
            if critical_count == 1 { "" } else { "s" }
        ));
    }

    if let Some(correlation) = insight.complexity_coverage_correlation {
        if correlation < -0.3 {
            output.push_str(&format!(
                "• ✅ Good news: Complex code tends to be better tested (correlation: {correlation:.2})\n"
            ));
        } else if correlation > 0.3 {
            output.push_str(&format!(
                "• ⚠️  Warning: Complex code lacks coverage (correlation: {correlation:.2})\n"
            ));
        }
    }

    let total_high_risk = critical_count + high_count;
    if total_high_risk > 0 {
        let estimated_effort: u32 = insight
            .top_risks
            .iter()
            .take(total_high_risk)
            .map(|r| r.test_effort.recommended_test_cases)
            .sum();

        if estimated_effort > 0 {
            output.push_str(&format!(
                "• Estimated test effort for safe risk level: {}-{} test cases\n",
                estimated_effort,
                estimated_effort + (estimated_effort / 2)
            ));
        }
    }

    if !insight.risk_reduction_opportunities.is_empty() {
        let total_reduction: f64 = insight
            .risk_reduction_opportunities
            .iter()
            .map(|r| r.potential_risk_reduction)
            .sum();

        if total_reduction >= 1.0 {
            output.push_str(&format!(
                "• Potential risk reduction from top 5 recommendations: {total_reduction:.0}%\n"
            ));
        }
    }

    output
}
