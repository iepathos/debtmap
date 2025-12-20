//! Pure text extraction functions for TUI actions.
//!
//! All functions in this module are pure - they take data and return
//! formatted strings without any I/O operations. This makes them
//! easy to test and reason about.
//!
//! # Design
//!
//! Text extraction follows the "Pure Core, Imperative Shell" pattern:
//! - This module contains the pure core (formatting)
//! - Clipboard/editor modules handle I/O (imperative shell)
//!
//! # Architecture
//!
//! To avoid duplication between TUI rendering and text extraction, this module
//! uses `lines_to_plain_text` to convert `Vec<Line>` from the detail page
//! section builders into plain text. This ensures that when new sections are
//! added to the TUI, they are automatically included in the copy function.

use crate::data_flow::DataFlowGraph;
use crate::organization::calculate_file_cohesion;
use crate::priority::UnifiedDebtItem;
use crate::tui::theme::Theme;
use ratatui::text::Line;

use super::super::{
    app::ResultsApp,
    detail_page::DetailPage,
    detail_pages::{
        context, data_flow, dependencies, git_context, overview, patterns, responsibilities,
        score_breakdown,
    },
};

// =============================================================================
// Line to plain text conversion
// =============================================================================

/// Convert `Vec<Line>` from TUI section builders to plain text.
///
/// This is the key function that enables automatic inclusion of new sections
/// in the copy function. When a new section is added to the TUI renderer,
/// it will automatically be included in the copied text.
///
/// # Arguments
///
/// * `lines` - The lines from a TUI section builder
///
/// # Returns
///
/// A plain text string with each line joined by newlines.
pub fn lines_to_plain_text(lines: &[Line]) -> String {
    lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// =============================================================================
// Public API - Page text extraction
// =============================================================================

/// Extract plain text content from a detail page.
/// This matches the exact layout of the rendered TUI pages.
pub fn extract_page_text(item: &UnifiedDebtItem, page: DetailPage, app: &ResultsApp) -> String {
    match page {
        DetailPage::Overview => extract_overview_text(item, app),
        DetailPage::ScoreBreakdown => extract_score_breakdown_text(item),
        DetailPage::Context => extract_context_text(item),
        DetailPage::Dependencies => extract_dependencies_text(item, app),
        DetailPage::GitContext => extract_git_context_text(item),
        DetailPage::Patterns => extract_patterns_text(item),
        DetailPage::DataFlow => extract_data_flow_text(item, &app.analysis().data_flow_graph),
        DetailPage::Responsibilities => extract_responsibilities_text(item),
    }
}

/// Format a path as a string for clipboard copying
pub fn format_path_text(path: &std::path::Path) -> String {
    path.to_string_lossy().to_string()
}

// =============================================================================
// Overview page extraction
// =============================================================================

/// Standard width used for text extraction (matches typical terminal width)
const TEXT_EXTRACTION_WIDTH: u16 = 80;

/// Extract overview page content as plain text.
///
/// Uses the same section builders as the TUI renderer, converted to plain text.
/// This ensures that when new sections are added to the TUI, they are
/// automatically included in the copy function.
fn extract_overview_text(item: &UnifiedDebtItem, app: &ResultsApp) -> String {
    let theme = Theme::default();
    let width = TEXT_EXTRACTION_WIDTH;

    // Get items at this location (same logic as TUI renderer)
    let location_items = overview::get_items_at_location(app, item);

    // Calculate file cohesion (same as TUI renderer)
    let cohesion = calculate_file_cohesion(&item.location.file, &app.analysis().call_graph);

    // Compose sections using the same builders as the TUI renderer
    let all_lines: Vec<Line<'static>> = [
        overview::build_location_section(item, &theme, width),
        overview::build_score_section(&location_items, item, &theme, width),
        overview::build_god_object_section(item, &theme, width),
        overview::build_complexity_section(item, &theme, width),
        overview::build_coverage_section(item, &theme, width),
        overview::build_cohesion_section(cohesion.as_ref(), &theme, width),
        overview::build_debt_types_section(&location_items, item, &theme),
    ]
    .into_iter()
    .flatten()
    .collect();

    lines_to_plain_text(&all_lines)
}

// =============================================================================
// Score Breakdown page extraction
// =============================================================================

/// Extract score breakdown page content as plain text.
///
/// Uses the same section builders as the TUI renderer, converted to plain text.
fn extract_score_breakdown_text(item: &UnifiedDebtItem) -> String {
    let theme = Theme::default();
    let lines = score_breakdown::build_page_lines(item, &theme, TEXT_EXTRACTION_WIDTH);
    lines_to_plain_text(&lines)
}

// =============================================================================
// Context page extraction
// =============================================================================

/// Extract context page content as plain text.
///
/// Uses the same section builders as the TUI renderer, converted to plain text.
fn extract_context_text(item: &UnifiedDebtItem) -> String {
    let theme = Theme::default();
    let lines = context::build_page_lines(item, &theme, TEXT_EXTRACTION_WIDTH);
    lines_to_plain_text(&lines)
}

// =============================================================================
// Dependencies page extraction
// =============================================================================

/// Extract dependencies page content as plain text.
///
/// Uses the same section builders as the TUI renderer, converted to plain text.
fn extract_dependencies_text(item: &UnifiedDebtItem, _app: &ResultsApp) -> String {
    let theme = Theme::default();
    let lines = dependencies::build_page_lines(item, &theme, TEXT_EXTRACTION_WIDTH);
    lines_to_plain_text(&lines)
}

// =============================================================================
// Git context page extraction
// =============================================================================

/// Extract git context page content as plain text.
///
/// Uses the same section builders as the TUI renderer, converted to plain text.
fn extract_git_context_text(item: &UnifiedDebtItem) -> String {
    let theme = Theme::default();
    let lines = git_context::build_page_lines(item, &theme, TEXT_EXTRACTION_WIDTH);
    lines_to_plain_text(&lines)
}

// =============================================================================
// Patterns page extraction
// =============================================================================

/// Extract patterns page content as plain text.
///
/// Uses the same section builders as the TUI renderer, converted to plain text.
fn extract_patterns_text(item: &UnifiedDebtItem) -> String {
    let theme = Theme::default();
    let lines = patterns::build_page_lines(item, &theme, TEXT_EXTRACTION_WIDTH);
    lines_to_plain_text(&lines)
}

// =============================================================================
// Data flow page extraction
// =============================================================================

/// Extract data flow page content as plain text.
///
/// Uses the same section builders as the TUI renderer, converted to plain text.
fn extract_data_flow_text(item: &UnifiedDebtItem, data_flow_graph: &DataFlowGraph) -> String {
    let theme = Theme::default();
    let lines = data_flow::build_page_lines(item, data_flow_graph, &theme, TEXT_EXTRACTION_WIDTH);
    lines_to_plain_text(&lines)
}

// =============================================================================
// Responsibilities page extraction
// =============================================================================

/// Extract responsibilities page content as plain text.
///
/// Uses the same section builders as the TUI renderer, converted to plain text.
fn extract_responsibilities_text(item: &UnifiedDebtItem) -> String {
    let theme = Theme::default();
    let lines = responsibilities::build_page_lines(item, &theme, TEXT_EXTRACTION_WIDTH);
    lines_to_plain_text(&lines)
}

// =============================================================================
// Utility functions
// =============================================================================

// Re-export format_debt_type_name from overview module (single source of truth)
pub use overview::format_debt_type_name;

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Span;

    // Test helper functions (moved from main module to avoid unused warnings)

    fn classify_stability(change_frequency: f64) -> &'static str {
        if change_frequency < 1.0 {
            "Stable"
        } else if change_frequency < 5.0 {
            "Moderately Unstable"
        } else {
            "Highly Unstable"
        }
    }

    fn entropy_description(score: f64) -> &'static str {
        if score < 0.3 {
            "low (repetitive)"
        } else if score < 0.5 {
            "medium (typical)"
        } else {
            "high (chaotic)"
        }
    }

    fn derive_coupling_classification(
        afferent: usize,
        efferent: usize,
        instability: f64,
    ) -> String {
        let total = afferent + efferent;

        if total > 15 {
            "Highly Coupled".to_string()
        } else if total <= 2 {
            "Isolated".to_string()
        } else if instability < 0.3 && afferent > efferent {
            "Stable Core".to_string()
        } else if instability > 0.7 && efferent > afferent {
            "Leaf Module".to_string()
        } else {
            "Utility Module".to_string()
        }
    }

    fn get_clipboard_fix_suggestion(reasons: &[String]) -> Option<&'static str> {
        if reasons.len() > 2 {
            return None;
        }

        let first_reason = reasons.first()?.to_lowercase();

        if first_reason.contains("i/o")
            || first_reason.contains("print")
            || first_reason.contains("log")
        {
            Some("Move logging to caller - function becomes pure")
        } else if first_reason.contains("time") || first_reason.contains("now") {
            Some("Pass time as parameter instead of calling now()")
        } else if first_reason.contains("random") || first_reason.contains("rand") {
            Some("Inject RNG as parameter for deterministic behavior")
        } else if first_reason.contains("mutable param") || first_reason.contains("&mut") {
            Some("Consider taking &self instead of &mut self")
        } else {
            None
        }
    }

    #[test]
    fn test_lines_to_plain_text_single_line() {
        let lines = vec![Line::from("hello world")];
        let result = lines_to_plain_text(&lines);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_lines_to_plain_text_multiple_spans() {
        let lines = vec![Line::from(vec![
            Span::raw("label"),
            Span::raw("    "),
            Span::raw("value"),
        ])];
        let result = lines_to_plain_text(&lines);
        assert_eq!(result, "label    value");
    }

    #[test]
    fn test_lines_to_plain_text_multiple_lines() {
        let lines = vec![
            Line::from("first line"),
            Line::from("second line"),
            Line::from("third line"),
        ];
        let result = lines_to_plain_text(&lines);
        assert_eq!(result, "first line\nsecond line\nthird line");
    }

    #[test]
    fn test_lines_to_plain_text_empty() {
        let lines: Vec<Line> = vec![];
        let result = lines_to_plain_text(&lines);
        assert_eq!(result, "");
    }

    #[test]
    fn test_format_path_text() {
        use std::path::PathBuf;
        let path = PathBuf::from("/tmp/test.rs");
        let text = format_path_text(&path);
        assert_eq!(text, "/tmp/test.rs");
    }

    #[test]
    fn test_classify_stability() {
        assert_eq!(classify_stability(0.5), "Stable");
        assert_eq!(classify_stability(2.0), "Moderately Unstable");
        assert_eq!(classify_stability(10.0), "Highly Unstable");
    }

    #[test]
    fn test_entropy_description() {
        assert_eq!(entropy_description(0.1), "low (repetitive)");
        assert_eq!(entropy_description(0.4), "medium (typical)");
        assert_eq!(entropy_description(0.7), "high (chaotic)");
    }

    #[test]
    fn test_derive_coupling_classification() {
        assert_eq!(derive_coupling_classification(20, 5, 0.5), "Highly Coupled");
        assert_eq!(derive_coupling_classification(1, 1, 0.5), "Isolated");
        assert_eq!(derive_coupling_classification(8, 2, 0.2), "Stable Core");
        assert_eq!(derive_coupling_classification(2, 8, 0.8), "Leaf Module");
        assert_eq!(derive_coupling_classification(5, 5, 0.5), "Utility Module");
    }

    #[test]
    fn test_get_clipboard_fix_suggestion() {
        let io_reasons = vec!["i/o operation".to_string()];
        assert!(get_clipboard_fix_suggestion(&io_reasons).is_some());

        let time_reasons = vec!["calls now()".to_string()];
        assert!(get_clipboard_fix_suggestion(&time_reasons).is_some());

        let many_reasons = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert!(get_clipboard_fix_suggestion(&many_reasons).is_none());
    }

    #[test]
    fn test_format_debt_type_name() {
        use crate::priority::DebtType;

        let complexity = DebtType::ComplexityHotspot {
            cyclomatic: 10,
            cognitive: 20,
        };
        assert_eq!(format_debt_type_name(&complexity), "High Complexity");

        let god_object = DebtType::GodObject {
            methods: 50,
            fields: Some(20),
            responsibilities: 5,
            lines: 1000,
            god_object_score: 100.0,
        };
        assert_eq!(format_debt_type_name(&god_object), "God Object");
    }
}
