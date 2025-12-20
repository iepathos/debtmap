//! Context page (Page 3) - AI context suggestions (Spec 263).
//!
//! Displays suggested code context that AI agents should read to understand
//! and fix a debt item. Shows primary scope and related contexts with
//! file:line ranges.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::priority::context::{ContextRelationship, ContextSuggestion};
use crate::priority::UnifiedDebtItem;
use crate::tui::results::app::ResultsApp;
use crate::tui::theme::Theme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Format relationship type as short label (per DESIGN.md 20-char column)
fn format_relationship(rel: &ContextRelationship) -> &'static str {
    match rel {
        ContextRelationship::Caller => "Caller",
        ContextRelationship::Callee => "Callee",
        ContextRelationship::TypeDefinition => "Type",
        ContextRelationship::TestCode => "Test",
        ContextRelationship::SiblingMethod => "Sibling",
        ContextRelationship::TraitDefinition => "Trait",
        ContextRelationship::ModuleHeader => "Module",
    }
}

/// Build the header section with total lines and confidence.
fn build_context_header(
    lines: &mut Vec<Line<'static>>,
    context: &ContextSuggestion,
    theme: &Theme,
) {
    let header_info = format!(
        "{} lines · {}% confidence",
        context.total_lines,
        (context.completeness_confidence * 100.0) as u32
    );

    lines.push(Line::from(vec![
        Span::styled("context to read", Style::default().fg(theme.muted)),
        Span::raw("                    "),
        Span::styled(header_info, Style::default().fg(theme.primary)),
    ]));
    add_blank_line(lines);
}

/// Build the primary scope section.
fn build_primary_section(
    lines: &mut Vec<Line<'static>>,
    context: &ContextSuggestion,
    theme: &Theme,
    width: u16,
) {
    add_section_header(lines, "primary", theme);

    // File:line range in primary color
    let range_text = format!(
        "{}:{}-{}",
        context.primary.file.display(),
        context.primary.start_line,
        context.primary.end_line
    );
    add_label_value(lines, "range", range_text, theme, width);

    // Symbol name if available
    if let Some(ref symbol) = context.primary.symbol {
        add_label_value(lines, "symbol", symbol.clone(), theme, width);
    }

    // Line count
    let line_count = context.primary.line_count();
    add_label_value(lines, "lines", line_count.to_string(), theme, width);

    add_blank_line(lines);
}

/// Build the related context section.
fn build_related_section(
    lines: &mut Vec<Line<'static>>,
    context: &ContextSuggestion,
    theme: &Theme,
    width: u16,
) {
    if context.related.is_empty() {
        return;
    }

    add_section_header(lines, "related", theme);

    for related in &context.related {
        let relationship = format_relationship(&related.relationship);

        // File:line range
        let range_text = format!(
            "{}:{}-{}",
            related.range.file.display(),
            related.range.start_line,
            related.range.end_line
        );
        add_label_value(lines, relationship, range_text, theme, width);

        // Reason on next line (indented)
        if !related.reason.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("                            "), // 28 chars to align with value column
                Span::styled(related.reason.clone(), Style::default().fg(theme.muted)),
            ]));
        }
    }

    add_blank_line(lines);
}

/// Build copy instructions section.
fn build_copy_instructions(lines: &mut Vec<Line<'static>>, theme: &Theme) {
    add_section_header(lines, "actions", theme);
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("c", Style::default().fg(theme.primary)),
        Span::raw(" - copy all context ranges"),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("p", Style::default().fg(theme.primary)),
        Span::raw(" - copy primary range only"),
    ]));
}

/// Build all lines for the context page (pure function).
///
/// This is public so text_extraction can reuse it for clipboard copy.
pub fn build_page_lines(item: &UnifiedDebtItem, theme: &Theme, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if let Some(ref context) = item.context_suggestion {
        build_context_header(&mut lines, context, theme);
        build_primary_section(&mut lines, context, theme, width);
        build_related_section(&mut lines, context, theme, width);
        build_copy_instructions(&mut lines, theme);
    } else {
        // Placeholder when no context available
        lines.push(Line::from(vec![Span::styled(
            "context to read",
            Style::default().fg(theme.muted),
        )]));
        add_blank_line(&mut lines);
        lines.push(Line::from(vec![Span::styled(
            "Context information not available.",
            Style::default().fg(theme.muted),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "Re-run analysis to generate context suggestions.",
            Style::default().fg(theme.muted),
        )]));
    }

    lines
}

/// Render context page showing AI context suggestions.
pub fn render(
    frame: &mut Frame,
    _app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let lines = build_page_lines(item, theme, area.width);

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Format context for clipboard (AI-friendly format).
pub fn format_context_for_clipboard(context: &ContextSuggestion) -> String {
    let mut out = String::new();

    // Primary range
    out.push_str(&format!(
        "Primary: {}:{}-{}\n",
        context.primary.file.display(),
        context.primary.start_line,
        context.primary.end_line
    ));

    if let Some(ref symbol) = context.primary.symbol {
        out.push_str(&format!("  Symbol: {}\n", symbol));
    }

    // Related ranges
    for related in &context.related {
        out.push_str(&format!(
            "Related ({}): {}:{}-{}\n",
            format_relationship(&related.relationship),
            related.range.file.display(),
            related.range.start_line,
            related.range.end_line
        ));
        if !related.reason.is_empty() {
            out.push_str(&format!("  Reason: {}\n", related.reason));
        }
    }

    // Summary
    out.push_str(&format!(
        "\nTotal: {} lines, {}% confidence\n",
        context.total_lines,
        (context.completeness_confidence * 100.0) as u32
    ));

    out
}

/// Format primary range only for clipboard.
pub fn format_primary_for_clipboard(context: &ContextSuggestion) -> String {
    let mut out = format!(
        "{}:{}-{}",
        context.primary.file.display(),
        context.primary.start_line,
        context.primary.end_line
    );

    if let Some(ref symbol) = context.primary.symbol {
        out.push_str(&format!(" ({})", symbol));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::context::{FileRange, RelatedContext};
    use std::path::PathBuf;

    fn sample_context() -> ContextSuggestion {
        ContextSuggestion {
            primary: FileRange {
                file: PathBuf::from("src/main.rs"),
                start_line: 10,
                end_line: 50,
                symbol: Some("main".to_string()),
            },
            related: vec![
                RelatedContext {
                    range: FileRange {
                        file: PathBuf::from("src/lib.rs"),
                        start_line: 100,
                        end_line: 150,
                        symbol: Some("helper".to_string()),
                    },
                    relationship: ContextRelationship::Callee,
                    reason: "Called by main".to_string(),
                },
                RelatedContext {
                    range: FileRange {
                        file: PathBuf::from("tests/test.rs"),
                        start_line: 1,
                        end_line: 20,
                        symbol: None,
                    },
                    relationship: ContextRelationship::TestCode,
                    reason: "Test coverage".to_string(),
                },
            ],
            total_lines: 111,
            completeness_confidence: 0.85,
        }
    }

    #[test]
    fn format_context_includes_primary() {
        let context = sample_context();
        let text = format_context_for_clipboard(&context);

        assert!(text.contains("Primary: src/main.rs:10-50"));
        assert!(text.contains("Symbol: main"));
    }

    #[test]
    fn format_context_includes_related() {
        let context = sample_context();
        let text = format_context_for_clipboard(&context);

        assert!(text.contains("Related (Callee): src/lib.rs:100-150"));
        assert!(text.contains("Related (Test): tests/test.rs:1-20"));
    }

    #[test]
    fn format_context_includes_summary() {
        let context = sample_context();
        let text = format_context_for_clipboard(&context);

        assert!(text.contains("111 lines"));
        assert!(text.contains("85% confidence"));
    }

    #[test]
    fn format_primary_simple() {
        let context = sample_context();
        let text = format_primary_for_clipboard(&context);

        assert_eq!(text, "src/main.rs:10-50 (main)");
    }

    #[test]
    fn format_relationship_labels() {
        assert_eq!(format_relationship(&ContextRelationship::Caller), "Caller");
        assert_eq!(format_relationship(&ContextRelationship::Callee), "Callee");
        assert_eq!(
            format_relationship(&ContextRelationship::TypeDefinition),
            "Type"
        );
        assert_eq!(format_relationship(&ContextRelationship::TestCode), "Test");
        assert_eq!(
            format_relationship(&ContextRelationship::SiblingMethod),
            "Sibling"
        );
        assert_eq!(
            format_relationship(&ContextRelationship::TraitDefinition),
            "Trait"
        );
        assert_eq!(
            format_relationship(&ContextRelationship::ModuleHeader),
            "Module"
        );
    }
}
