//! Git history section formatting with pure classification functions.
//!
//! Follows Stillwater philosophy: pure functions for classification logic,
//! section formatters compose these for output.

use crate::priority::UnifiedDebtItem;
use crate::risk::context::{ContextDetails, ContextualRisk};
use colored::*;
use std::fmt::Write;

// ============================================================================
// Pure Classification Functions (Stillwater "still" core)
// ============================================================================

/// Pure function to classify change frequency stability
pub fn classify_stability(change_frequency: f64) -> &'static str {
    if change_frequency > 5.0 {
        "highly unstable"
    } else if change_frequency > 2.0 {
        "moderately unstable"
    } else {
        "stable"
    }
}

/// Pure function to classify bug density level
pub fn classify_bug_density(bug_density: f64) -> &'static str {
    if bug_density > 0.3 {
        "high"
    } else if bug_density > 0.1 {
        "moderate"
    } else {
        "low"
    }
}

/// Pure function to calculate risk multiplier
pub fn calculate_risk_multiplier(contextual_risk: &ContextualRisk) -> f64 {
    if contextual_risk.base_risk > 0.0 {
        contextual_risk.contextual_risk / contextual_risk.base_risk
    } else {
        1.0
    }
}

/// Pure data structure for git history display
#[derive(Debug, Clone)]
pub struct GitHistoryData {
    pub change_frequency: f64,
    pub bug_density: f64,
    pub age_days: u32,
    pub author_count: u32,
    pub base_risk: f64,
    pub contextual_risk: f64,
    pub multiplier: f64,
}

impl GitHistoryData {
    /// Extract git history data from contextual risk if available
    pub fn from_contextual_risk(risk: &ContextualRisk) -> Option<Self> {
        risk.contexts
            .iter()
            .find(|c| c.provider == "git_history")
            .and_then(|git_context| {
                if let ContextDetails::Historical {
                    change_frequency,
                    bug_density,
                    age_days,
                    author_count,
                } = git_context.details
                {
                    Some(GitHistoryData {
                        change_frequency,
                        bug_density,
                        age_days,
                        author_count: author_count as u32,
                        base_risk: risk.base_risk,
                        contextual_risk: risk.contextual_risk,
                        multiplier: calculate_risk_multiplier(risk),
                    })
                } else {
                    None
                }
            })
    }
}

// ============================================================================
// Section Formatters (Stillwater "water" shell - I/O at boundaries)
// ============================================================================

/// Format the main git history line
pub fn format_git_history_line(output: &mut String, data: &GitHistoryData) {
    writeln!(
        output,
        "├─ {} {:.1} changes/month, {:.1}% bugs, {} days old, {} authors",
        "GIT HISTORY:".bright_blue(),
        data.change_frequency,
        data.bug_density * 100.0,
        data.age_days,
        data.author_count
    )
    .unwrap();
}

/// Format the risk impact sub-line
pub fn format_risk_impact_line(output: &mut String, data: &GitHistoryData) {
    writeln!(
        output,
        "│  └─ {} base_risk={:.1} → contextual_risk={:.1} ({:.1}x multiplier)",
        "Risk Impact:".bright_cyan(),
        data.base_risk,
        data.contextual_risk,
        data.multiplier
    )
    .unwrap();
}

/// Format complete git history section
pub fn format_git_history_section(output: &mut String, item: &UnifiedDebtItem) {
    if let Some(ref contextual_risk) = item.contextual_risk {
        if let Some(data) = GitHistoryData::from_contextual_risk(contextual_risk) {
            format_git_history_line(output, &data);
            format_risk_impact_line(output, &data);
        }
    }
}

/// Format a single context provider contribution
fn format_provider_contribution(
    output: &mut String,
    provider: &str,
    contribution: f64,
    weight: f64,
    details: &ContextDetails,
) {
    writeln!(
        output,
        "│  └─ {}: +{:.1} impact (weight: {:.1})",
        provider.bright_cyan(),
        contribution,
        weight
    )
    .unwrap();

    // Add detail lines for historical context
    if let ContextDetails::Historical {
        change_frequency,
        bug_density,
        ..
    } = details
    {
        let stability_desc = classify_stability(*change_frequency);
        let bug_desc = classify_bug_density(*bug_density);

        writeln!(
            output,
            "│     - Change frequency: {:.1}/month ({})",
            change_frequency, stability_desc
        )
        .unwrap();
        writeln!(
            output,
            "│     - Bug density: {:.1}% ({})",
            bug_density * 100.0,
            bug_desc
        )
        .unwrap();
    }
}

/// Format context provider contributions section (verbose mode only)
pub fn format_context_provider_contributions(
    output: &mut String,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    if verbosity < 1 {
        return;
    }

    if let Some(ref contextual_risk) = item.contextual_risk {
        if contextual_risk.contexts.is_empty() {
            return;
        }

        writeln!(
            output,
            "├─ {}",
            "Context Provider Contributions:".bright_blue()
        )
        .unwrap();

        for context in &contextual_risk.contexts {
            format_provider_contribution(
                output,
                &context.provider,
                context.contribution,
                context.weight,
                &context.details,
            );
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_stability() {
        assert_eq!(classify_stability(10.0), "highly unstable");
        assert_eq!(classify_stability(5.1), "highly unstable");
        assert_eq!(classify_stability(5.0), "moderately unstable");
        assert_eq!(classify_stability(3.0), "moderately unstable");
        assert_eq!(classify_stability(2.1), "moderately unstable");
        assert_eq!(classify_stability(2.0), "stable");
        assert_eq!(classify_stability(1.0), "stable");
        assert_eq!(classify_stability(0.0), "stable");
    }

    #[test]
    fn test_classify_bug_density() {
        assert_eq!(classify_bug_density(0.5), "high");
        assert_eq!(classify_bug_density(0.31), "high");
        assert_eq!(classify_bug_density(0.3), "moderate");
        assert_eq!(classify_bug_density(0.2), "moderate");
        assert_eq!(classify_bug_density(0.11), "moderate");
        assert_eq!(classify_bug_density(0.1), "low");
        assert_eq!(classify_bug_density(0.05), "low");
        assert_eq!(classify_bug_density(0.0), "low");
    }

    #[test]
    fn test_calculate_risk_multiplier() {
        let risk = ContextualRisk {
            base_risk: 10.0,
            contextual_risk: 25.0,
            contexts: vec![],
            explanation: String::new(),
        };
        assert!((calculate_risk_multiplier(&risk) - 2.5).abs() < 0.001);

        // Test zero base risk
        let risk_zero = ContextualRisk {
            base_risk: 0.0,
            contextual_risk: 10.0,
            contexts: vec![],
            explanation: String::new(),
        };
        assert!((calculate_risk_multiplier(&risk_zero) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_format_git_history_line() {
        colored::control::set_override(false);

        let data = GitHistoryData {
            change_frequency: 3.5,
            bug_density: 0.15,
            age_days: 100,
            author_count: 5,
            base_risk: 10.0,
            contextual_risk: 20.0,
            multiplier: 2.0,
        };

        let mut output = String::new();
        format_git_history_line(&mut output, &data);

        assert!(output.contains("GIT HISTORY:"));
        assert!(output.contains("3.5 changes/month"));
        assert!(output.contains("15.0% bugs"));
        assert!(output.contains("100 days old"));
        assert!(output.contains("5 authors"));

        colored::control::unset_override();
    }

    #[test]
    fn test_format_risk_impact_line() {
        colored::control::set_override(false);

        let data = GitHistoryData {
            change_frequency: 3.5,
            bug_density: 0.15,
            age_days: 100,
            author_count: 5,
            base_risk: 10.0,
            contextual_risk: 20.0,
            multiplier: 2.0,
        };

        let mut output = String::new();
        format_risk_impact_line(&mut output, &data);

        assert!(output.contains("Risk Impact:"));
        assert!(output.contains("base_risk=10.0"));
        assert!(output.contains("contextual_risk=20.0"));
        assert!(output.contains("2.0x multiplier"));

        colored::control::unset_override();
    }
}
