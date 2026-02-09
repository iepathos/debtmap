//! Pure formatting functions that transform data → structured output.
//!
//! This module contains pure functions with no I/O operations. All functions
//! are deterministic and fully testable without mocks or string buffers.
//!
//! # Examples
//!
//! ```no_run
//! use debtmap::priority::formatter::pure::format_priority_item;
//! use debtmap::priority::UnifiedDebtItem;
//! use debtmap::formatting::FormattingConfig;
//!
//! # let item = todo!();
//! let formatted = format_priority_item(
//!     1,              // rank
//!     &item,          // debt item
//!     0,              // verbosity
//!     FormattingConfig::default(),
//!     false,          // has coverage data
//! );
//!
//! assert_eq!(formatted.rank, 1);
//! ```

use super::context::{create_format_context, FormatContext};
use super::sections::generate_formatted_sections;
use crate::formatting::FormattingConfig;
use crate::priority::classification::{CoverageLevel, Severity};
use crate::priority::detected_pattern::DetectedPattern;
use crate::priority::formatted_output::{
    ContextProviderInfo, CoverageTag, FormattedPriorityItem, FormattedSection, SeverityInfo,
};
use crate::priority::UnifiedDebtItem;
use crate::risk::context::{ContextDetails, ContextualRisk};

/// Pure function: transforms debt item → formatted output.
///
/// This is a **pure function** with no side effects:
/// - No I/O operations
/// - No mutation of input data
/// - Deterministic output for same inputs
/// - Fully testable without mocks
///
/// # Arguments
///
/// * `rank` - Priority rank (1-based)
/// * `item` - The debt item to format
/// * `verbosity` - Verbosity level (0 = minimal, higher = more detail)
/// * `config` - Formatting configuration
/// * `has_coverage_data` - Whether coverage data is available
///
/// # Returns
///
/// A `FormattedPriorityItem` containing all sections needed for rendering.
///
/// # Examples
///
/// ```no_run
/// use debtmap::priority::formatter::pure::format_priority_item;
/// use debtmap::priority::classification::Severity;
/// use debtmap::formatting::FormattingConfig;
///
/// # let item = todo!();
/// let formatted = format_priority_item(
///     1,
///     &item,
///     0,
///     FormattingConfig::default(),
///     false,
/// );
///
/// assert_eq!(formatted.rank, 1);
/// assert_eq!(formatted.severity, Severity::from_score(item.unified_score.final_score));
/// ```
pub fn format_priority_item(
    rank: usize,
    item: &UnifiedDebtItem,
    _verbosity: u8,
    _config: FormattingConfig,
    has_coverage_data: bool,
) -> FormattedPriorityItem {
    let context = create_format_context(rank, item, has_coverage_data);
    let sections_data = generate_formatted_sections(&context);
    let severity = Severity::from_score(item.unified_score.final_score);

    let sections = build_all_sections(&context, &sections_data, item);

    FormattedPriorityItem {
        rank,
        score: context.score,
        severity,
        sections,
    }
}

/// Build all sections from context and sections data.
fn build_all_sections(
    context: &FormatContext,
    sections_data: &super::sections::FormattedSections,
    item: &UnifiedDebtItem,
) -> Vec<FormattedSection> {
    let mut sections = vec![
        build_header_section(context),
        build_location_section(context),
    ];

    sections.extend(build_context_dampening_section(context));
    sections.push(build_action_section(context));
    sections.push(build_impact_section(context));
    sections.extend(build_evidence_section(sections_data));
    sections.extend(build_complexity_section(context));
    sections.extend(build_pattern_section(&context.pattern_info));
    sections.extend(build_coverage_section(&context.coverage_info));
    sections.extend(build_contextual_risk_section(&item.contextual_risk));
    sections.extend(build_dependencies_section(context));
    sections.extend(build_debt_specific_section(sections_data));
    sections.push(build_rationale_section(context));

    sections
}

fn build_header_section(context: &FormatContext) -> FormattedSection {
    FormattedSection::Header {
        rank: context.rank,
        score: context.score,
        coverage_tag: context.coverage_info.as_ref().map(|cov| CoverageTag {
            text: cov.tag.clone(),
            color: cov.color,
        }),
        severity: SeverityInfo {
            label: context.severity_info.label.clone(),
            color: context.severity_info.color,
        },
    }
}

fn build_location_section(context: &FormatContext) -> FormattedSection {
    FormattedSection::Location {
        file: context.location_info.file.clone(),
        line: context.location_info.line,
        function: context.location_info.function.clone(),
    }
}

fn build_context_dampening_section(context: &FormatContext) -> Option<FormattedSection> {
    context
        .context_info
        .as_ref()
        .map(|info| FormattedSection::ContextDampening {
            description: info.description.clone(),
            dampening_percentage: ((1.0 - info.multiplier) * 100.0) as i32,
        })
}

fn build_action_section(context: &FormatContext) -> FormattedSection {
    FormattedSection::Action {
        action: context.action.clone(),
    }
}

fn build_impact_section(context: &FormatContext) -> FormattedSection {
    FormattedSection::Impact {
        complexity_reduction: context.impact.complexity_reduction as u32,
        risk_reduction: context.impact.risk_reduction,
    }
}

fn build_evidence_section(
    sections_data: &super::sections::FormattedSections,
) -> Option<FormattedSection> {
    sections_data
        .evidence
        .as_ref()
        .map(|text| FormattedSection::Evidence { text: text.clone() })
}

fn build_complexity_section(context: &FormatContext) -> Option<FormattedSection> {
    if !context.complexity_info.has_complexity {
        return None;
    }
    Some(FormattedSection::Complexity {
        cyclomatic: context.complexity_info.cyclomatic,
        cognitive: context.complexity_info.cognitive,
        nesting: context.complexity_info.nesting,
        entropy: context
            .complexity_info
            .entropy_analysis
            .as_ref()
            .map(|e| e.entropy_score),
    })
}

/// Parse pattern display metrics into key-value pairs.
fn parse_pattern_metrics(pattern: &DetectedPattern) -> Vec<(String, String)> {
    pattern
        .display_metrics()
        .iter()
        .filter_map(|metric| {
            metric
                .split_once(": ")
                .map(|(k, v)| (k.to_string(), v.to_string()))
        })
        .collect()
}

fn build_pattern_section(pattern_info: &Option<DetectedPattern>) -> Option<FormattedSection> {
    pattern_info
        .as_ref()
        .map(|pattern| FormattedSection::Pattern {
            pattern_type: pattern.type_name().to_string(),
            icon: pattern.icon().to_string(),
            metrics: parse_pattern_metrics(pattern),
            confidence: pattern.confidence,
        })
}

fn build_coverage_section(
    coverage_info: &Option<super::context::CoverageInfo>,
) -> Option<FormattedSection> {
    coverage_info
        .as_ref()
        .and_then(|info| info.coverage_percentage)
        .map(|percentage| FormattedSection::Coverage {
            percentage,
            level: CoverageLevel::from_percentage(percentage),
            details: Some(format!("{:.1}%", percentage)),
        })
}

/// Calculate risk multiplier from base and contextual risk.
fn calculate_risk_multiplier(base_risk: f64, contextual_risk: f64) -> f64 {
    if base_risk > 0.1 {
        contextual_risk / base_risk
    } else {
        1.0
    }
}

/// Format historical context details into a display string.
fn format_historical_details(details: &ContextDetails) -> Option<String> {
    match details {
        ContextDetails::Historical {
            change_frequency,
            bug_density,
            age_days,
            author_count,
            ..
        } => Some(format!(
            "changes/mo: {:.1}, bug density: {:.1}%, age: {}d, authors: {}",
            change_frequency,
            bug_density * 100.0,
            age_days,
            author_count
        )),
        _ => None,
    }
}

/// Build provider info from a context.
fn build_provider_info(ctx: &crate::risk::context::Context) -> ContextProviderInfo {
    ContextProviderInfo {
        name: ctx.provider.clone(),
        contribution: ctx.contribution,
        weight: ctx.weight,
        impact: ctx.contribution * ctx.weight,
        details: format_historical_details(&ctx.details),
    }
}

fn build_contextual_risk_section(
    contextual_risk: &Option<ContextualRisk>,
) -> Option<FormattedSection> {
    contextual_risk.as_ref().map(|risk| {
        let providers: Vec<ContextProviderInfo> = risk
            .contexts
            .iter()
            .filter(|ctx| ctx.contribution > 0.05)
            .map(build_provider_info)
            .collect();

        FormattedSection::ContextualRisk {
            base_risk: risk.base_risk,
            contextual_risk: risk.contextual_risk,
            multiplier: calculate_risk_multiplier(risk.base_risk, risk.contextual_risk),
            providers,
        }
    })
}

fn build_dependencies_section(context: &FormatContext) -> Option<FormattedSection> {
    if !context.dependency_info.has_dependencies {
        return None;
    }
    Some(FormattedSection::Dependencies {
        upstream: context.dependency_info.upstream,
        downstream: context.dependency_info.downstream,
        callers: context.dependency_info.upstream_callers.clone(),
        callees: context.dependency_info.downstream_callees.clone(),
    })
}

fn build_debt_specific_section(
    sections_data: &super::sections::FormattedSections,
) -> Option<FormattedSection> {
    sections_data
        .debt_specific
        .as_ref()
        .map(|text| FormattedSection::DebtSpecific { text: text.clone() })
}

fn build_rationale_section(context: &FormatContext) -> FormattedSection {
    FormattedSection::Rationale {
        text: context.rationale.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{
        ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedScore,
    };

    fn create_test_item(score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: "test.rs".into(),
                function: "test_function".to_string(),
                line: 10,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 10,
                cognitive: 15,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 5.0,
                dependency_factor: 5.0,
                role_multiplier: 1.0,
                final_score: score.max(0.0),
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                // Spec 260: Score transparency fields
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor this function".to_string(),
                rationale: "High complexity detected".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: Some(2.5),
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 5.0,
                risk_reduction: 3.5,
            },
            transitive_coverage: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 3,
            function_length: 50,
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
            contextual_risk: None, // spec 203
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        }
    }

    #[test]
    fn format_priority_item_pure() {
        let item = create_test_item(8.5);

        let formatted = format_priority_item(1, &item, 0, FormattingConfig::default(), false);

        // Test without I/O
        assert_eq!(formatted.rank, 1);
        assert_eq!(formatted.score, 8.5);
        assert_eq!(formatted.severity, Severity::Critical);

        // Verify we have expected sections
        let has_header = formatted
            .sections
            .iter()
            .any(|s| matches!(s, FormattedSection::Header { .. }));
        let has_location = formatted
            .sections
            .iter()
            .any(|s| matches!(s, FormattedSection::Location { .. }));
        let has_action = formatted
            .sections
            .iter()
            .any(|s| matches!(s, FormattedSection::Action { .. }));
        let has_impact = formatted
            .sections
            .iter()
            .any(|s| matches!(s, FormattedSection::Impact { .. }));
        let has_rationale = formatted
            .sections
            .iter()
            .any(|s| matches!(s, FormattedSection::Rationale { .. }));

        assert!(has_header, "Should have header section");
        assert!(has_location, "Should have location section");
        assert!(has_action, "Should have action section");
        assert!(has_impact, "Should have impact section");
        assert!(has_rationale, "Should have rationale section");
    }

    #[test]
    fn severity_matches_score() {
        let test_cases = vec![
            (10.0, Severity::Critical),
            (8.0, Severity::Critical),
            (7.0, Severity::High),
            (5.0, Severity::Medium),
            (3.0, Severity::Low),
        ];

        for (score, expected_severity) in test_cases {
            let item = create_test_item(score);
            let formatted = format_priority_item(1, &item, 0, FormattingConfig::default(), false);
            assert_eq!(
                formatted.severity, expected_severity,
                "Score {} should map to {:?}",
                score, expected_severity
            );
        }
    }

    #[test]
    fn location_extracted_correctly() {
        let item = create_test_item(5.0);
        let formatted = format_priority_item(1, &item, 0, FormattingConfig::default(), false);

        let location = formatted.sections.iter().find_map(|s| match s {
            FormattedSection::Location {
                file,
                line,
                function,
            } => Some((file, line, function)),
            _ => None,
        });

        assert!(location.is_some(), "Should have location section");
        let (file, line, function) = location.unwrap();
        assert_eq!(file.to_str().unwrap(), "test.rs");
        assert_eq!(*line, 10);
        assert_eq!(function, "test_function");
    }

    #[test]
    fn complexity_section_included_when_has_complexity() {
        let item = create_test_item(5.0);
        let formatted = format_priority_item(1, &item, 0, FormattingConfig::default(), false);

        let has_complexity = formatted
            .sections
            .iter()
            .any(|s| matches!(s, FormattedSection::Complexity { .. }));

        assert!(has_complexity, "Should have complexity section");
    }

    #[test]
    fn rank_preserved() {
        let item = create_test_item(5.0);
        let rank = 42;
        let formatted = format_priority_item(rank, &item, 0, FormattingConfig::default(), false);

        assert_eq!(formatted.rank, rank);
    }

    // Property-based tests with proptest

    use proptest::prelude::*;

    proptest! {
        /// Property: rank is always preserved in formatted output
        #[test]
        fn prop_rank_preserved(rank in 1usize..1000) {
            let item = create_test_item(5.0);
            let formatted = format_priority_item(rank, &item, 0, FormattingConfig::default(), false);
            prop_assert_eq!(formatted.rank, rank);
        }

        /// Property: score is always preserved in formatted output
        #[test]
        fn prop_score_preserved(score in 0.0f64..20.0) {
            let item = create_test_item(score);
            let formatted = format_priority_item(1, &item, 0, FormattingConfig::default(), false);
            prop_assert_eq!(formatted.score, score);
        }

        /// Property: formatted item always has location section
        #[test]
        fn prop_formatted_item_always_has_location(rank in 1usize..100, score in 0.0f64..20.0) {
            let item = create_test_item(score);
            let formatted = format_priority_item(rank, &item, 0, FormattingConfig::default(), false);

            let has_location = formatted.sections.iter().any(|s| matches!(s, FormattedSection::Location { .. }));
            prop_assert!(has_location, "Formatted item must always have location section");
        }

        /// Property: formatted item always has required core sections
        #[test]
        fn prop_has_required_sections(rank in 1usize..100, score in 0.0f64..20.0) {
            let item = create_test_item(score);
            let formatted = format_priority_item(rank, &item, 0, FormattingConfig::default(), false);

            let has_header = formatted.sections.iter().any(|s| matches!(s, FormattedSection::Header { .. }));
            let has_location = formatted.sections.iter().any(|s| matches!(s, FormattedSection::Location { .. }));
            let has_action = formatted.sections.iter().any(|s| matches!(s, FormattedSection::Action { .. }));
            let has_impact = formatted.sections.iter().any(|s| matches!(s, FormattedSection::Impact { .. }));
            let has_rationale = formatted.sections.iter().any(|s| matches!(s, FormattedSection::Rationale { .. }));

            prop_assert!(has_header, "Must have header section");
            prop_assert!(has_location, "Must have location section");
            prop_assert!(has_action, "Must have action section");
            prop_assert!(has_impact, "Must have impact section");
            prop_assert!(has_rationale, "Must have rationale section");
        }

        /// Property: score correctly maps to severity level
        #[test]
        fn prop_score_maps_to_severity(score in 0.0f64..20.0) {
            let item = create_test_item(score);
            let formatted = format_priority_item(1, &item, 0, FormattingConfig::default(), false);

            let expected_severity = Severity::from_score(score);
            prop_assert_eq!(formatted.severity, expected_severity);
        }

        /// Property: pure function is deterministic (same inputs → same outputs)
        #[test]
        fn prop_deterministic(rank in 1usize..100, score in 0.0f64..20.0) {
            let item = create_test_item(score);
            let result1 = format_priority_item(rank, &item, 0, FormattingConfig::default(), false);
            let result2 = format_priority_item(rank, &item, 0, FormattingConfig::default(), false);

            // Compare key fields for determinism
            prop_assert_eq!(result1.rank, result2.rank);
            prop_assert_eq!(result1.score, result2.score);
            prop_assert_eq!(result1.severity, result2.severity);
            prop_assert_eq!(result1.sections.len(), result2.sections.len());
        }
    }
}
