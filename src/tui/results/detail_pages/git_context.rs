//! Git Context page (Page 3) - Git history and risk analysis.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::priority::formatter_verbosity::git_history::classify_stability;
use crate::priority::UnifiedDebtItem;
use crate::risk::context::ContextDetails;
use crate::tui::results::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Build all lines for the git context page (pure function).
///
/// This is public so text_extraction can reuse it for clipboard copy.
pub fn build_page_lines(item: &UnifiedDebtItem, theme: &Theme, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if let Some(ref contextual_risk) = item.contextual_risk {
        // Look for git history context
        let git_context = contextual_risk
            .contexts
            .iter()
            .find(|ctx| ctx.provider == "git_history");

        if let Some(ctx) = git_context {
            if let ContextDetails::Historical {
                change_frequency,
                bug_density: _,
                age_days,
                author_count,
                total_commits,
                bug_fix_count,
            } = &ctx.details
            {
                // Change Patterns section
                add_section_header(&mut lines, "change patterns", theme);

                // Show commits and frequency together for clarity
                // "N commits (X.XX/month)" makes it clear what the frequency represents
                let frequency_display = if *total_commits == 0 {
                    "0 commits".to_string()
                } else {
                    format!(
                        "{} commit{} ({:.2}/month)",
                        total_commits,
                        if *total_commits == 1 { "" } else { "s" },
                        change_frequency
                    )
                };
                add_label_value(&mut lines, "activity", frequency_display, theme, width);

                let stability = classify_stability(*change_frequency);
                add_label_value(&mut lines, "stability", stability.to_string(), theme, width);

                // Show bug fixes as "N fixes / M changes" for clarity
                // Changes = total_commits - 1 (excluding introduction)
                let changes = total_commits.saturating_sub(1);
                let fix_display = if changes == 0 {
                    "no changes since intro".to_string()
                } else {
                    format!(
                        "{} fix{} / {} change{}",
                        bug_fix_count,
                        if *bug_fix_count == 1 { "" } else { "es" },
                        changes,
                        if changes == 1 { "" } else { "s" }
                    )
                };
                add_label_value(&mut lines, "fix rate", fix_display, theme, width);
                add_label_value(
                    &mut lines,
                    "age",
                    format!("{} days", age_days),
                    theme,
                    width,
                );
                add_label_value(
                    &mut lines,
                    "contributors",
                    author_count.to_string(),
                    theme,
                    width,
                );
                add_blank_line(&mut lines);
            }
        }

        // Risk Impact section
        add_section_header(&mut lines, "risk impact", theme);
        add_label_value(
            &mut lines,
            "base",
            format!("{:.1}", contextual_risk.base_risk),
            theme,
            width,
        );
        add_label_value(
            &mut lines,
            "contextual",
            format!("{:.1}", contextual_risk.contextual_risk),
            theme,
            width,
        );

        let multiplier = if contextual_risk.base_risk > 0.0 {
            contextual_risk.contextual_risk / contextual_risk.base_risk
        } else {
            1.0
        };
        add_label_value(
            &mut lines,
            "multiplier",
            format!("{:.2}x", multiplier),
            theme,
            width,
        );
        add_blank_line(&mut lines);
    }

    // Context Dampening section (if applicable)
    if let Some(ref file_type) = item.context_type {
        add_section_header(&mut lines, "context dampening", theme);
        add_label_value(
            &mut lines,
            "file type",
            format!("{:?}", file_type),
            theme,
            width,
        );

        if let Some(multiplier) = item.context_multiplier {
            let reduction = (1.0 - multiplier) * 100.0;
            add_label_value(
                &mut lines,
                "reduction",
                format!("{:.1}%", reduction),
                theme,
                width,
            );
        }
        add_blank_line(&mut lines);
    }

    // If no data available
    if lines.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "No git context data available",
            Style::default().fg(theme.muted),
        )]));
    }

    lines
}

/// Render git context page showing change patterns and risk impact
pub fn render(
    frame: &mut Frame,
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let lines = build_page_lines(item, theme, area.width);

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false })
        .scroll(app.detail_scroll_offset());

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::debt_types::DebtType;
    use crate::priority::semantic_classifier::FunctionRole;
    use crate::priority::unified_scorer::{Location, UnifiedDebtItem, UnifiedScore};
    use crate::priority::{ActionableRecommendation, ImpactMetrics};
    use crate::risk::context::{Context, ContextDetails, ContextualRisk};
    use crate::tui::theme::Theme;
    use std::path::PathBuf;

    fn create_test_item_with_git_context(
        change_frequency: f64,
        bug_density: f64,
        age_days: u32,
        author_count: usize,
    ) -> UnifiedDebtItem {
        // Derive reasonable defaults
        let total_commits = ((change_frequency * age_days as f64 / 30.0).round() as u32).max(1);
        let changes = total_commits.saturating_sub(1);
        let bug_fix_count = (bug_density * changes as f64).round() as u32;
        create_test_item_with_git_context_full(
            change_frequency,
            bug_density,
            age_days,
            author_count,
            total_commits,
            bug_fix_count,
        )
    }

    fn create_test_item_with_git_context_full(
        change_frequency: f64,
        bug_density: f64,
        age_days: u32,
        author_count: usize,
        total_commits: u32,
        bug_fix_count: u32,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_fn".to_string(),
                line: 1,
            },
            debt_type: DebtType::Complexity {
                cyclomatic: 10,
                cognitive: 15,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: 50.0,
                base_score: Some(50.0),
                exponential_factor: Some(1.0),
                risk_boost: Some(1.0),
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Test".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 1,
            function_length: 100,
            cyclomatic_complexity: 10,
            cognitive_complexity: 15,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: Some(ContextualRisk {
                base_risk: 10.0,
                contextual_risk: 25.0,
                contexts: vec![Context {
                    provider: "git_history".to_string(),
                    contribution: 15.0,
                    weight: 1.5,
                    details: ContextDetails::Historical {
                        change_frequency,
                        bug_density,
                        age_days,
                        author_count,
                        total_commits,
                        bug_fix_count,
                    },
                }],
                explanation: "Test explanation".to_string(),
            }),
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        }
    }

    fn create_test_item_without_context() -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                function: "test_fn".to_string(),
                line: 1,
            },
            debt_type: DebtType::Complexity {
                cyclomatic: 5,
                cognitive: 8,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: 30.0,
                base_score: Some(30.0),
                exponential_factor: Some(1.0),
                risk_boost: Some(1.0),
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Test".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 1,
            function_length: 50,
            cyclomatic_complexity: 5,
            cognitive_complexity: 8,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            file_context: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        }
    }

    #[test]
    fn test_build_page_lines_with_git_context() {
        let theme = Theme::default();
        let item = create_test_item_with_git_context(2.5, 0.15, 100, 3);

        let lines = build_page_lines(&item, &theme, 80);

        // Convert lines to string for easier assertion
        let text: String = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");

        // New format shows "N commits (X.XX/month)" for clarity
        assert!(
            text.contains("commits") && text.contains("2.50/month"),
            "Should show commits with frequency: got {}",
            text
        );
        assert!(
            text.contains("Moderately Unstable"),
            "Should classify as moderately unstable"
        );
        // New format shows "N fixes / M changes" instead of percentage
        assert!(
            text.contains("fix") && text.contains("change"),
            "Should show fix rate as 'N fixes / M changes': got {}",
            text
        );
        assert!(text.contains("100 days"), "Should show age in days");
        assert!(text.contains("3"), "Should show author count");
    }

    #[test]
    fn test_build_page_lines_stability_thresholds() {
        let theme = Theme::default();

        // Test stable threshold (< 1.0)
        let stable_item = create_test_item_with_git_context(0.5, 0.0, 365, 1);
        let stable_lines = build_page_lines(&stable_item, &theme, 80);
        let stable_text: String = stable_lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(stable_text.contains("Stable"), "< 1.0 should be Stable");

        // Test moderately unstable threshold (>= 1.0 and < 5.0)
        let mod_item = create_test_item_with_git_context(3.0, 0.0, 100, 2);
        let mod_lines = build_page_lines(&mod_item, &theme, 80);
        let mod_text: String = mod_lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(
            mod_text.contains("Moderately Unstable"),
            ">= 1.0 and < 5.0 should be Moderately Unstable"
        );

        // Test highly unstable threshold (>= 5.0)
        let unstable_item = create_test_item_with_git_context(7.0, 0.5, 30, 5);
        let unstable_lines = build_page_lines(&unstable_item, &theme, 80);
        let unstable_text: String = unstable_lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(
            unstable_text.contains("Highly Unstable"),
            ">= 5.0 should be Highly Unstable"
        );
    }

    #[test]
    fn test_build_page_lines_risk_multiplier() {
        let theme = Theme::default();
        let item = create_test_item_with_git_context(2.0, 0.1, 50, 2);

        let lines = build_page_lines(&item, &theme, 80);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // base_risk=10.0, contextual_risk=25.0, multiplier should be 2.5x
        assert!(text.contains("10.0"), "Should show base risk");
        assert!(text.contains("25.0"), "Should show contextual risk");
        assert!(text.contains("2.50x"), "Should show 2.5x multiplier");
    }

    #[test]
    fn test_build_page_lines_no_context() {
        let theme = Theme::default();
        let item = create_test_item_without_context();

        let lines = build_page_lines(&item, &theme, 80);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        assert!(
            text.contains("No git context data available"),
            "Should show no data message"
        );
    }

    #[test]
    fn test_build_page_lines_zero_base_risk() {
        let theme = Theme::default();
        let mut item = create_test_item_with_git_context(1.0, 0.0, 10, 1);

        // Set base_risk to 0 to test division protection
        if let Some(ref mut risk) = item.contextual_risk {
            risk.base_risk = 0.0;
            risk.contextual_risk = 10.0;
        }

        let lines = build_page_lines(&item, &theme, 80);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Should show 1.00x multiplier (fallback for zero base risk)
        assert!(
            text.contains("1.00x"),
            "Should show 1.0x multiplier when base_risk is 0"
        );
    }
}
