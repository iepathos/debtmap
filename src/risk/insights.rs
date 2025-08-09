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

pub fn format_risk_matrix_terminal() -> String {
    let mut output = String::new();

    output.push_str("📈 RISK DISTRIBUTION MATRIX\n");
    output.push_str("────────────────────────────\n");
    output.push_str("Coverage % →\n");
    output.push_str("100 │ ✓✓✓ │ ✓✓✓ │ ✓✓  │ ⚠   │\n");
    output.push_str(" 75 │ ✓✓✓ │ ✓✓  │ ⚠   │ ⚠⚠  │\n");
    output.push_str(" 50 │ ✓✓  │ ⚠   │ ⚠⚠  │ 🔥🔥 │\n");
    output.push_str(" 25 │ ⚠   │ ⚠⚠  │ 🔥  │ 🔥🔥 │\n");
    output.push_str("  0 │ ✓   │ ⚠⚠  │ 🔥  │ 🔥🔥🔥│\n");
    output.push_str("    └─────┴─────┴─────┴─────┘\n");
    output.push_str("      1-5   5-10  10-20  20+\n");
    output.push_str("           Complexity →\n");
    output.push_str("\n");
    output.push_str("✓ = Low Risk  ⚠ = Medium Risk  🔥 = Critical Risk\n");

    output
}

pub fn format_critical_risks(risks: &Vector<FunctionRisk>) -> String {
    let mut output = String::new();

    let critical_risks: Vec<&FunctionRisk> = risks
        .iter()
        .filter(|r| matches!(r.risk_category, RiskCategory::Critical))
        .take(5)
        .collect();

    if critical_risks.is_empty() {
        return output;
    }

    output.push_str("🔥 CRITICAL RISK FUNCTIONS (Complexity > 15, Coverage < 30%)\n");
    output.push_str("─────────────────────────────────────────────────────────────\n");

    for (i, risk) in critical_risks.iter().enumerate() {
        output.push_str(&format!(
            "{}. {}::{}\n",
            i + 1,
            risk.file.display(),
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
                .map(|c| format!("{:.0}%", c))
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

pub fn format_recommendations(recommendations: &Vector<TestingRecommendation>) -> String {
    let mut output = String::new();

    if recommendations.is_empty() {
        return output;
    }

    output.push_str("🎯 TOP 5 TESTING RECOMMENDATIONS (Maximum Risk Reduction)\n");
    output.push_str("──────────────────────────────────────────────────────────\n");
    output.push_str("Priority | Function | Impact | ROI Score\n");
    output.push_str("---------|----------|--------|-----------\n");

    for (i, rec) in recommendations.iter().enumerate() {
        let roi_score = rec.current_risk / (rec.test_effort_estimate.cognitive_load as f64 + 1.0);
        output.push_str(&format!(
            "{} | {}() | -{:.0}% risk | {:.1}\n",
            i + 1,
            rec.function,
            rec.potential_risk_reduction,
            roi_score
        ));
        output.push_str(&format!("  └─ Why: {}\n\n", rec.rationale));
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
            "1. Focus testing on the {} critical risk functions first\n",
            critical_count
        ));
    }

    if let Some(correlation) = insight.complexity_coverage_correlation {
        if correlation < -0.3 {
            output.push_str("2. Good news: Complex code tends to be better tested\n");
        } else if correlation > 0.3 {
            output.push_str("2. Warning: Complex code tends to have less coverage\n");
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

        output.push_str(&format!(
            "3. Estimated effort to reach safe risk level: {}-{} test cases\n",
            estimated_effort,
            estimated_effort + (estimated_effort / 2)
        ));
    }

    if !insight.risk_reduction_opportunities.is_empty() {
        let total_reduction: f64 = insight
            .risk_reduction_opportunities
            .iter()
            .map(|r| r.potential_risk_reduction)
            .sum();

        output.push_str(&format!(
            "4. Potential risk reduction from recommended tests: {:.0}%\n",
            total_reduction
        ));
    }

    output
}
