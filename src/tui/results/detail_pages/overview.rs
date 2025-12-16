//! Overview page (Page 1) - Core metrics and recommendation.
//!
//! Structured as pure section builders composed by a thin render shell,
//! following Stillwater philosophy: "Pure Core, Imperative Shell".

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::organization::{calculate_file_cohesion, FileCohesionResult};
use crate::output::unified::CohesionClassification;
use crate::priority::classification::Severity;
use crate::priority::{DebtType, UnifiedDebtItem};
use crate::tui::results::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

// ============================================================================
// Pure Section Builders (the "still" core)
// These are public so text_extraction can reuse them for clipboard copy.
// ============================================================================

/// Build location section lines (pure)
pub fn build_location_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    add_section_header(&mut lines, "location", theme);
    add_label_value(
        &mut lines,
        "file",
        item.location.file.display().to_string(),
        theme,
        width,
    );

    // Skip function and line for file-scope items (god files) since they're always
    // "[file-scope]" and "1" which don't add useful information
    if item.location.function != "[file-scope]" {
        add_label_value(
            &mut lines,
            "function",
            item.location.function.clone(),
            theme,
            width,
        );
        add_label_value(
            &mut lines,
            "line",
            item.location.line.to_string(),
            theme,
            width,
        );
    }

    add_blank_line(&mut lines);
    lines
}

/// Build score section lines (pure)
pub fn build_score_section(
    location_items: &[&UnifiedDebtItem],
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    add_section_header(&mut lines, "score", theme);

    if location_items.len() > 1 {
        let combined_score: f64 = location_items
            .iter()
            .map(|i| i.unified_score.final_score.value())
            .sum();
        let severity = Severity::from_score_100(combined_score)
            .as_str()
            .to_lowercase();
        add_label_value(
            &mut lines,
            "combined",
            format!("{:.1}  [{}]", combined_score, severity),
            theme,
            width,
        );
    } else {
        let severity = Severity::from_score_100(item.unified_score.final_score.value())
            .as_str()
            .to_lowercase();
        add_label_value(
            &mut lines,
            "total",
            format!(
                "{:.1}  [{}]",
                item.unified_score.final_score.value(),
                severity
            ),
            theme,
            width,
        );
    }
    add_blank_line(&mut lines);
    lines
}

/// Build god object structure section (pure) - returns empty if not a god object
pub fn build_god_object_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if let DebtType::GodObject {
        methods,
        fields,
        responsibilities,
        lines: debt_lines,
        ..
    } = &item.debt_type
    {
        let detection_type = item
            .god_object_indicators
            .as_ref()
            .map(|i| &i.detection_type);

        let header = match detection_type {
            Some(crate::organization::DetectionType::GodClass) => "god object structure",
            Some(crate::organization::DetectionType::GodFile) => "god file structure",
            Some(crate::organization::DetectionType::GodModule) => "god module structure",
            None => "god object structure",
        };
        add_section_header(&mut lines, header, theme);

        let method_label = match detection_type {
            Some(crate::organization::DetectionType::GodClass) => "methods",
            _ => "functions",
        };

        // Show weighted method count adjustment if available (like entropy dampening)
        let method_display = item
            .god_object_indicators
            .as_ref()
            .and_then(|i| i.weighted_method_count)
            .map(|weighted| format!("{} → {:.0} (pure-weighted)", methods, weighted))
            .unwrap_or_else(|| methods.to_string());
        add_label_value(&mut lines, method_label, method_display, theme, width);

        if let Some(field_count) = fields {
            add_label_value(&mut lines, "fields", field_count.to_string(), theme, width);
        }

        add_label_value(
            &mut lines,
            "responsibilities",
            responsibilities.to_string(),
            theme,
            width,
        );
        add_label_value(&mut lines, "loc", debt_lines.to_string(), theme, width);
        add_blank_line(&mut lines);
    }

    lines
}

/// Build complexity metrics section (pure)
pub fn build_complexity_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    add_section_header(&mut lines, "complexity", theme);

    let is_god_object = matches!(item.debt_type, DebtType::GodObject { .. });
    let (cyclomatic_label, cognitive_label, nesting_label) = if is_god_object {
        (
            "accumulated cyclomatic",
            "accumulated cognitive",
            "max nesting",
        )
    } else {
        ("cyclomatic", "cognitive", "nesting")
    };

    add_label_value(
        &mut lines,
        cyclomatic_label,
        item.cyclomatic_complexity.to_string(),
        theme,
        width,
    );

    let cognitive_display = format_cognitive_display(item, is_god_object);
    add_label_value(&mut lines, cognitive_label, cognitive_display, theme, width);
    add_label_value(
        &mut lines,
        nesting_label,
        item.nesting_depth.to_string(),
        theme,
        width,
    );

    if !is_god_object {
        add_label_value(
            &mut lines,
            "loc",
            item.function_length.to_string(),
            theme,
            width,
        );
    }
    add_blank_line(&mut lines);
    lines
}

/// Format cognitive complexity display with optional dampening info (pure)
pub fn format_cognitive_display(item: &UnifiedDebtItem, is_god_object: bool) -> String {
    if is_god_object {
        item.god_object_indicators
            .as_ref()
            .and_then(|g| g.aggregated_entropy.as_ref())
            .filter(|e| e.dampening_factor < 1.0)
            .map(|e| {
                format!(
                    "{} → {} (dampened)",
                    e.original_complexity, e.adjusted_cognitive
                )
            })
            .unwrap_or_else(|| item.cognitive_complexity.to_string())
    } else {
        item.entropy_details
            .as_ref()
            .filter(|e| e.dampening_factor < 1.0)
            .map(|e| {
                format!(
                    "{} → {} (dampened)",
                    e.original_complexity, e.adjusted_cognitive
                )
            })
            .unwrap_or_else(|| item.cognitive_complexity.to_string())
    }
}

/// Build coverage section lines (pure)
pub fn build_coverage_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    add_section_header(&mut lines, "coverage", theme);
    let coverage_value = item
        .transitive_coverage
        .as_ref()
        .map(|c| format!("{:.1}%", c.direct * 100.0))
        .unwrap_or_else(|| "No data".to_string());
    add_label_value(&mut lines, "coverage", coverage_value, theme, width);
    add_blank_line(&mut lines);
    lines
}

/// Build cohesion section lines (pure) - displays file cohesion metrics (spec 198)
///
/// Returns empty if cohesion data is not available (e.g., file has fewer than 3 functions).
pub fn build_cohesion_section(
    cohesion: Option<&FileCohesionResult>,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let Some(cohesion) = cohesion else {
        return lines;
    };

    add_section_header(&mut lines, "file cohesion", theme);

    // Display score with classification
    let classification = CohesionClassification::from_score(cohesion.score);
    let score_display = format!(
        "{:.1}%  [{}]",
        cohesion.score * 100.0,
        classification.to_string().to_lowercase()
    );
    add_label_value(&mut lines, "score", score_display, theme, width);

    // Display call breakdown
    add_label_value(
        &mut lines,
        "internal calls",
        cohesion.internal_calls.to_string(),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "external calls",
        cohesion.external_calls.to_string(),
        theme,
        width,
    );
    add_label_value(
        &mut lines,
        "functions",
        cohesion.functions_analyzed.to_string(),
        theme,
        width,
    );

    add_blank_line(&mut lines);
    lines
}

/// Build recommendation section lines (pure)
pub fn build_recommendation_section(
    item: &UnifiedDebtItem,
    theme: &Theme,
    width: u16,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    add_section_header(&mut lines, "recommendation", theme);
    add_label_value(
        &mut lines,
        "action",
        item.recommendation.primary_action.clone(),
        theme,
        width,
    );
    add_blank_line(&mut lines);
    add_label_value(
        &mut lines,
        "rationale",
        item.recommendation.rationale.clone(),
        theme,
        width,
    );
    add_blank_line(&mut lines);
    lines
}

/// Build debt types section lines (pure)
pub fn build_debt_types_section(
    location_items: &[&UnifiedDebtItem],
    item: &UnifiedDebtItem,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if location_items.len() > 1 {
        add_section_header(&mut lines, "debt types", theme);
        for debt_item in location_items.iter() {
            let debt_name = format_debt_type_name(&debt_item.debt_type);
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(debt_name, Style::default().fg(theme.secondary())),
            ]));
        }
    } else {
        add_section_header(&mut lines, "debt type", theme);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format_debt_type_name(&item.debt_type),
                Style::default().fg(theme.secondary()),
            ),
        ]));
    }

    lines
}

// ============================================================================
// Render Shell (the "water" boundary)
// ============================================================================

/// Render overview page showing core information
pub fn render(
    frame: &mut Frame,
    app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let location_items = get_items_at_location(app, item);

    // Calculate file cohesion for the selected item's file (spec 198)
    let cohesion = calculate_file_cohesion(&item.location.file, &app.analysis().call_graph);

    // Compose pure section builders (still water pattern)
    let lines: Vec<Line<'static>> = [
        build_location_section(item, theme, area.width),
        build_score_section(&location_items, item, theme, area.width),
        build_god_object_section(item, theme, area.width),
        build_complexity_section(item, theme, area.width),
        build_coverage_section(item, theme, area.width),
        build_cohesion_section(cohesion.as_ref(), theme, area.width),
        build_recommendation_section(item, theme, area.width),
        build_debt_types_section(&location_items, item, theme),
    ]
    .into_iter()
    .flatten()
    .collect();

    // I/O boundary: render the widget
    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Get all debt items at the same location as the selected item
pub fn get_items_at_location<'a>(
    app: &'a ResultsApp,
    selected: &UnifiedDebtItem,
) -> Vec<&'a UnifiedDebtItem> {
    app.analysis()
        .items
        .iter()
        .filter(|item| {
            item.location.file == selected.location.file
                && item.location.function == selected.location.function
                && item.location.line == selected.location.line
        })
        .collect()
}

/// Format debt type as human-readable name
pub fn format_debt_type_name(debt_type: &crate::priority::DebtType) -> String {
    #[allow(unused_imports)]
    use crate::priority::score_types::Score0To100;
    use crate::priority::DebtType;
    match debt_type {
        DebtType::ComplexityHotspot { .. } => "High Complexity".to_string(),
        DebtType::TestingGap { .. } => "Testing Gap".to_string(),
        DebtType::DeadCode { .. } => "Dead Code".to_string(),
        DebtType::Duplication { .. } => "Duplication".to_string(),
        DebtType::Risk { .. } => "Risk".to_string(),
        DebtType::TestComplexityHotspot { .. } => "Test Complexity".to_string(),
        DebtType::TestTodo { .. } => "Test TODO".to_string(),
        DebtType::TestDuplication { .. } => "Test Duplication".to_string(),
        DebtType::ErrorSwallowing { .. } => "Error Swallowing".to_string(),
        DebtType::AllocationInefficiency { .. } => "Allocation Inefficiency".to_string(),
        DebtType::StringConcatenation { .. } => "String Concatenation".to_string(),
        DebtType::NestedLoops { .. } => "Nested Loops".to_string(),
        DebtType::BlockingIO { .. } => "Blocking I/O".to_string(),
        DebtType::SuboptimalDataStructure { .. } => "Suboptimal Data Structure".to_string(),
        DebtType::GodObject { .. } => "God Object".to_string(),
        DebtType::FeatureEnvy { .. } => "Feature Envy".to_string(),
        DebtType::PrimitiveObsession { .. } => "Primitive Obsession".to_string(),
        DebtType::MagicValues { .. } => "Magic Values".to_string(),
        DebtType::AssertionComplexity { .. } => "Assertion Complexity".to_string(),
        DebtType::FlakyTestPattern { .. } => "Flaky Test Pattern".to_string(),
        DebtType::AsyncMisuse { .. } => "Async Misuse".to_string(),
        DebtType::ResourceLeak { .. } => "Resource Leak".to_string(),
        DebtType::CollectionInefficiency { .. } => "Collection Inefficiency".to_string(),
        DebtType::ScatteredType { .. } => "Scattered Type".to_string(),
        DebtType::OrphanedFunctions { .. } => "Orphaned Functions".to_string(),
        DebtType::UtilitiesSprawl { .. } => "Utilities Sprawl".to_string(),
        // Default for legacy variants
        _ => "Other".to_string(),
    }
}

// ============================================================================
// Tests (unit tests for pure section builders)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::score_types::Score0To100;
    use crate::priority::unified_scorer::{Location, UnifiedScore};
    use crate::priority::{ActionableRecommendation, FunctionRole, ImpactMetrics};
    use crate::tui::theme::Theme;

    /// Create a test debt item with specified properties
    fn create_test_item(
        function_name: &str,
        final_score: f64,
        debt_type: DebtType,
    ) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: std::path::PathBuf::from("test.rs"),
                line: 10,
                function: function_name.to_string(),
            },
            unified_score: UnifiedScore {
                final_score: Score0To100::new(final_score),
                complexity_factor: 0.8,
                coverage_factor: 0.6,
                dependency_factor: 0.5,
                role_multiplier: 1.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
            },
            debt_type,
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor to reduce complexity".to_string(),
                rationale: "Test recommendation".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 5.0,
                coverage_improvement: 0.1,
                lines_reduction: 10,
                risk_reduction: 0.2,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 1,
            downstream_dependencies: 2,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 2,
            function_length: 50,
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            entropy_details: None,
            entropy_adjusted_cognitive: None,
            entropy_dampening_factor: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
        }
    }

    fn complexity_item(name: &str) -> UnifiedDebtItem {
        create_test_item(
            name,
            75.0,
            DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            },
        )
    }

    fn testing_gap_item(name: &str) -> UnifiedDebtItem {
        create_test_item(
            name,
            60.0,
            DebtType::TestingGap {
                coverage: 0.1,
                cyclomatic: 10,
                cognitive: 15,
            },
        )
    }

    // --- build_location_section tests ---

    #[test]
    fn location_section_contains_file_function_line() {
        let item = complexity_item("test_func");
        let theme = Theme::default();
        let lines = build_location_section(&item, &theme, 80);

        // Should have header + 3 label-value pairs + blank line = 5 lines
        assert!(lines.len() >= 4);

        // Verify content by checking line spans
        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("location"));
        assert!(content.contains("test.rs"));
        assert!(content.contains("test_func"));
        assert!(content.contains("10"));
    }

    // --- build_score_section tests ---

    #[test]
    fn score_section_single_item_shows_total() {
        let item = complexity_item("func");
        let items: Vec<&UnifiedDebtItem> = vec![&item];
        let theme = Theme::default();

        let lines = build_score_section(&items, &item, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("score"));
        assert!(content.contains("total"));
        assert!(content.contains("75.0"));
    }

    #[test]
    fn score_section_multiple_items_shows_combined() {
        let item1 = complexity_item("func1");
        let item2 = testing_gap_item("func1"); // Same location
        let items: Vec<&UnifiedDebtItem> = vec![&item1, &item2];
        let theme = Theme::default();

        let lines = build_score_section(&items, &item1, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("combined"));
        assert!(content.contains("135.0")); // 75 + 60
    }

    // --- build_god_object_section tests ---

    #[test]
    fn god_object_section_empty_for_non_god_objects() {
        let item = complexity_item("func");
        let theme = Theme::default();

        let lines = build_god_object_section(&item, &theme, 80);

        assert!(lines.is_empty());
    }

    #[test]
    fn god_object_section_populated_for_god_objects() {
        let item = create_test_item(
            "BigClass",
            90.0,
            DebtType::GodObject {
                methods: 25,
                fields: Some(15),
                responsibilities: 8,
                lines: 500,
                god_object_score: Score0To100::new(85.0),
            },
        );
        let theme = Theme::default();

        let lines = build_god_object_section(&item, &theme, 80);

        assert!(!lines.is_empty());

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("structure"));
        assert!(content.contains("25")); // methods
        assert!(content.contains("15")); // fields
        assert!(content.contains("8")); // responsibilities
        assert!(content.contains("500")); // lines
    }

    // --- build_complexity_section tests ---

    #[test]
    fn complexity_section_includes_metrics() {
        let item = complexity_item("func");
        let theme = Theme::default();

        let lines = build_complexity_section(&item, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("complexity"));
        assert!(content.contains("cyclomatic"));
        assert!(content.contains("cognitive"));
        assert!(content.contains("nesting"));
        assert!(content.contains("loc"));
    }

    #[test]
    fn complexity_section_god_object_uses_accumulated_labels() {
        let item = create_test_item(
            "BigClass",
            90.0,
            DebtType::GodObject {
                methods: 25,
                fields: Some(15),
                responsibilities: 8,
                lines: 500,
                god_object_score: Score0To100::new(85.0),
            },
        );
        let theme = Theme::default();

        let lines = build_complexity_section(&item, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("accumulated cyclomatic"));
        assert!(content.contains("accumulated cognitive"));
        assert!(content.contains("max nesting"));
        // God objects don't show loc in complexity section (shown in structure section)
        assert!(!content.contains("  loc  "));
    }

    // --- build_coverage_section tests ---

    #[test]
    fn coverage_section_no_data_shows_placeholder() {
        let item = complexity_item("func");
        let theme = Theme::default();

        let lines = build_coverage_section(&item, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("coverage"));
        assert!(content.contains("No data"));
    }

    #[test]
    fn coverage_section_with_data_shows_percentage() {
        let mut item = complexity_item("func");
        item.transitive_coverage =
            Some(crate::priority::coverage_propagation::TransitiveCoverage {
                direct: 0.85,
                transitive: 0.72,
                propagated_from: vec![],
                uncovered_lines: vec![],
            });
        let theme = Theme::default();

        let lines = build_coverage_section(&item, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("85.0%"));
    }

    // --- build_recommendation_section tests ---

    #[test]
    fn recommendation_section_contains_action_and_rationale() {
        let item = complexity_item("func");
        let theme = Theme::default();

        let lines = build_recommendation_section(&item, &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("recommendation"));
        assert!(content.contains("action"));
        assert!(content.contains("rationale"));
        assert!(content.contains("Refactor"));
    }

    // --- build_debt_types_section tests ---

    #[test]
    fn debt_types_section_single_item() {
        let item = complexity_item("func");
        let items: Vec<&UnifiedDebtItem> = vec![&item];
        let theme = Theme::default();

        let lines = build_debt_types_section(&items, &item, &theme);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("debt type")); // singular
        assert!(content.contains("High Complexity"));
    }

    #[test]
    fn debt_types_section_multiple_items() {
        let item1 = complexity_item("func");
        let item2 = testing_gap_item("func");
        let items: Vec<&UnifiedDebtItem> = vec![&item1, &item2];
        let theme = Theme::default();

        let lines = build_debt_types_section(&items, &item1, &theme);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("debt types")); // plural
        assert!(content.contains("High Complexity"));
        assert!(content.contains("Testing Gap"));
    }

    // --- format_debt_type_name tests ---

    #[test]
    fn format_debt_type_name_all_variants() {
        assert_eq!(
            format_debt_type_name(&DebtType::ComplexityHotspot {
                cyclomatic: 1,
                cognitive: 1
            }),
            "High Complexity"
        );
        assert_eq!(
            format_debt_type_name(&DebtType::TestingGap {
                coverage: 0.0,
                cyclomatic: 1,
                cognitive: 1
            }),
            "Testing Gap"
        );
        assert_eq!(
            format_debt_type_name(&DebtType::GodObject {
                methods: 1,
                fields: None,
                responsibilities: 1,
                lines: 1,
                god_object_score: Score0To100::new(50.0),
            }),
            "God Object"
        );
    }

    // --- format_cognitive_display tests ---

    #[test]
    fn format_cognitive_display_no_dampening() {
        let item = complexity_item("func");
        let display = format_cognitive_display(&item, false);
        assert_eq!(display, "20"); // Just the cognitive complexity value
    }

    // --- build_cohesion_section tests (spec 198) ---

    #[test]
    fn cohesion_section_empty_when_no_cohesion_data() {
        let theme = Theme::default();
        let lines = build_cohesion_section(None, &theme, 80);
        assert!(lines.is_empty());
    }

    #[test]
    fn cohesion_section_high_cohesion() {
        let theme = Theme::default();
        let cohesion = FileCohesionResult {
            score: 0.85,
            internal_calls: 17,
            external_calls: 3,
            functions_analyzed: 5,
        };

        let lines = build_cohesion_section(Some(&cohesion), &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("file cohesion"));
        assert!(content.contains("85.0%"));
        assert!(content.contains("high"));
        assert!(content.contains("17")); // internal calls
        assert!(content.contains("3")); // external calls
        assert!(content.contains("5")); // functions
    }

    #[test]
    fn cohesion_section_medium_cohesion() {
        let theme = Theme::default();
        let cohesion = FileCohesionResult {
            score: 0.55,
            internal_calls: 11,
            external_calls: 9,
            functions_analyzed: 8,
        };

        let lines = build_cohesion_section(Some(&cohesion), &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("55.0%"));
        assert!(content.contains("medium"));
    }

    #[test]
    fn cohesion_section_low_cohesion() {
        let theme = Theme::default();
        let cohesion = FileCohesionResult {
            score: 0.25,
            internal_calls: 5,
            external_calls: 15,
            functions_analyzed: 6,
        };

        let lines = build_cohesion_section(Some(&cohesion), &theme, 80);

        let content: String = lines
            .iter()
            .flat_map(|l| l.spans.iter())
            .map(|s| s.content.as_ref())
            .collect();

        assert!(content.contains("25.0%"));
        assert!(content.contains("low"));
    }
}
