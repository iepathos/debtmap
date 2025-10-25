use crate::core::AnalysisResults;
use anyhow::Result;
use std::io::Write;

use super::executive_summary::{
    HealthDashboard, QuickWins, StrategicPriority, SuccessMetrics, TeamGuidance,
};
use super::formatters::*;

/// Pure functions for health metrics formatting
pub fn format_health_metric(name: &str, value: String, status: &str) -> String {
    format!("| {} | {} | {} |", name, value, status)
}

pub fn classify_health_status(score: u32) -> &'static str {
    match score {
        70..=100 => "Good",
        _ => "Needs Attention",
    }
}

pub fn format_overall_health_metric(health_score: u32) -> String {
    format_health_metric(
        "**Overall Health**",
        format!("{}% {}", health_score, get_health_emoji(health_score)),
        classify_health_status(health_score),
    )
}

pub fn format_complexity_metric(avg_complexity: f64) -> String {
    format_health_metric(
        "**Average Complexity**",
        format!("{:.2}", avg_complexity),
        get_complexity_status(avg_complexity),
    )
}

pub fn format_coverage_metric(coverage: f64) -> String {
    format_health_metric(
        "**Code Coverage**",
        format!("{:.1}%", coverage * 100.0),
        get_coverage_status(coverage * 100.0),
    )
}

pub fn format_debt_metric(debt_count: usize) -> String {
    format_health_metric(
        "**Technical Debt**",
        format!("{} items", debt_count),
        get_debt_status(debt_count),
    )
}

pub fn build_health_metrics(
    health_score: u32,
    avg_complexity: f64,
    coverage_percentage: Option<f64>,
    debt_count: usize,
) -> Vec<String> {
    let mut metrics = vec![
        format_overall_health_metric(health_score),
        format_complexity_metric(avg_complexity),
    ];

    if let Some(coverage) = coverage_percentage {
        metrics.push(format_coverage_metric(coverage));
    }

    metrics.push(format_debt_metric(debt_count));
    metrics
}

/// Writer functions for health status sections
pub fn write_health_section_header<W: Write>(writer: &mut W, collapsible: bool) -> Result<()> {
    if collapsible {
        writeln!(writer, "<details open>")?;
        writeln!(
            writer,
            "<summary><strong>📊 Health Status</strong></summary>\n"
        )?;
    } else {
        writeln!(writer, "### 📊 Health Status\n")?;
    }
    Ok(())
}

pub fn write_health_metrics_table<W: Write>(
    writer: &mut W,
    health_score: u32,
    avg_complexity: f64,
    coverage_percentage: Option<f64>,
    results: &AnalysisResults,
) -> Result<()> {
    writeln!(writer, "| Metric | Value | Status |")?;
    writeln!(writer, "|--------|-------|--------|")?;

    let metrics = build_health_metrics(
        health_score,
        avg_complexity,
        coverage_percentage,
        results.technical_debt.items.len(),
    );

    for metric in metrics {
        writeln!(writer, "{}", metric)?;
    }

    Ok(())
}

pub fn write_health_section_footer<W: Write>(writer: &mut W, collapsible: bool) -> Result<()> {
    if collapsible {
        writeln!(writer, "\n</details>\n")?;
    }
    Ok(())
}

pub fn write_enhanced_health_dashboard<W: Write>(
    writer: &mut W,
    dashboard: &HealthDashboard,
) -> Result<()> {
    writeln!(writer, "### 📊 Codebase Health Dashboard\n")?;

    writeln!(writer, "| Metric | Status | Interpretation |")?;
    writeln!(writer, "|--------|--------|----------------|")?;
    writeln!(
        writer,
        "| **Overall Health** | {} | {} |",
        dashboard.overall_health.as_string(),
        dashboard.overall_health.interpretation()
    )?;
    writeln!(
        writer,
        "| **Trend** | {} | Health is {} over time |",
        dashboard.trend.as_string(),
        dashboard.trend.as_string().to_lowercase()
    )?;
    writeln!(
        writer,
        "| **Velocity Impact** | -{:.1}% | {} |",
        dashboard.velocity_impact.slowdown_percentage, dashboard.velocity_impact.description
    )?;
    writeln!(
        writer,
        "| **Risk Level** | {} | Requires {} attention |",
        dashboard.risk_level.as_string(),
        match dashboard.risk_level {
            super::executive_summary::RiskLevel::Low => "routine",
            super::executive_summary::RiskLevel::Moderate => "regular",
            super::executive_summary::RiskLevel::High => "focused",
            super::executive_summary::RiskLevel::Critical => "immediate",
        }
    )?;

    writeln!(writer)?;
    Ok(())
}

pub fn write_quick_wins_section<W: Write>(writer: &mut W, quick_wins: &QuickWins) -> Result<()> {
    writeln!(writer, "### 🎯 Quick Wins (< 1 day effort)\n")?;

    writeln!(
        writer,
        "**{} items** fixable in **{} hours total**",
        quick_wins.count, quick_wins.total_effort_hours
    )?;

    if quick_wins.expected_impact.health_improvement > 0.0 {
        writeln!(writer)?;
        writeln!(writer, "**Expected Impact:**")?;
        writeln!(
            writer,
            "- Health score improvement: +{:.1}%",
            quick_wins.expected_impact.health_improvement
        )?;
        writeln!(
            writer,
            "- Complexity reduction: -{:.1}%",
            quick_wins.expected_impact.complexity_reduction
        )?;
        if quick_wins.expected_impact.coverage_increase > 0.0 {
            writeln!(
                writer,
                "- Coverage increase: +{:.1}%",
                quick_wins.expected_impact.coverage_increase
            )?;
        }
    }

    if !quick_wins.recommendations.is_empty() {
        writeln!(writer)?;
        writeln!(writer, "**Specific Actions:**")?;
        for recommendation in &quick_wins.recommendations {
            writeln!(writer, "- {}", recommendation)?;
        }
    }

    writeln!(writer)?;
    Ok(())
}

pub fn write_strategic_priorities_section<W: Write>(
    writer: &mut W,
    priorities: &[StrategicPriority],
) -> Result<()> {
    writeln!(writer, "### 🚨 Strategic Priorities\n")?;

    for (i, priority) in priorities.iter().enumerate() {
        writeln!(writer, "#### {}. {}\n", i + 1, priority.title)?;
        writeln!(writer, "**Location:** `{}`", priority.description)?;
        writeln!(
            writer,
            "**Effort:** {} | **Blocking Factor:** {:.1}/10",
            priority.effort_estimate.as_string(),
            priority.blocking_factor
        )?;
        writeln!(
            writer,
            "**Business Impact:** {}\n",
            priority.business_impact
        )?;
    }

    Ok(())
}

pub fn write_team_guidance_section<W: Write>(
    writer: &mut W,
    guidance: &TeamGuidance,
) -> Result<()> {
    writeln!(writer, "### 👥 Team Guidance\n")?;

    writeln!(
        writer,
        "**Recommended Debt Allocation:** {}% of sprint capacity",
        guidance.recommended_debt_allocation
    )?;

    if !guidance.focus_areas.is_empty() {
        writeln!(writer)?;
        writeln!(writer, "**Focus Areas:**")?;
        for area in &guidance.focus_areas {
            writeln!(writer, "- {}", area)?;
        }
    }

    if !guidance.process_improvements.is_empty() {
        writeln!(writer)?;
        writeln!(writer, "**Process Improvements:**")?;
        for improvement in &guidance.process_improvements {
            writeln!(writer, "- {}", improvement)?;
        }
    }

    writeln!(writer)?;
    Ok(())
}

pub fn write_success_metrics_section<W: Write>(
    writer: &mut W,
    metrics: &SuccessMetrics,
) -> Result<()> {
    writeln!(writer, "### ✅ Success Metrics\n")?;

    writeln!(writer, "| Target | Value | Timeline |")?;
    writeln!(writer, "|--------|-------|----------|")?;
    writeln!(
        writer,
        "| Health Score | {}% | {} |",
        metrics.target_health_score, metrics.timeline
    )?;
    writeln!(
        writer,
        "| Code Coverage | {:.0}% | {} |",
        metrics.target_coverage * 100.0,
        metrics.timeline
    )?;
    writeln!(
        writer,
        "| Complexity Reduction | -{:.0}% | {} |",
        metrics.target_complexity_reduction, metrics.timeline
    )?;

    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_health_status() {
        assert_eq!(classify_health_status(100), "Good");
        assert_eq!(classify_health_status(85), "Good");
        assert_eq!(classify_health_status(70), "Good");
        assert_eq!(classify_health_status(69), "Needs Attention");
        assert_eq!(classify_health_status(50), "Needs Attention");
        assert_eq!(classify_health_status(0), "Needs Attention");
    }

    #[test]
    fn test_format_health_metric() {
        let result = format_health_metric("Test Metric", "50%".to_string(), "Good");
        assert_eq!(result, "| Test Metric | 50% | Good |");
    }

    #[test]
    fn test_build_health_metrics() {
        let metrics = build_health_metrics(85, 5.5, Some(0.8), 10);
        assert_eq!(metrics.len(), 4);
        assert!(metrics[0].contains("Overall Health"));
        assert!(metrics[1].contains("Average Complexity"));
        assert!(metrics[2].contains("Code Coverage"));
        assert!(metrics[3].contains("Technical Debt"));
    }

    #[test]
    fn test_build_health_metrics_no_coverage() {
        let metrics = build_health_metrics(75, 8.0, None, 5);
        assert_eq!(metrics.len(), 3);
        assert!(metrics[0].contains("Overall Health"));
        assert!(metrics[1].contains("Average Complexity"));
        assert!(metrics[2].contains("Technical Debt"));
    }
}
