//! Patterns page (Page 4) - Detected patterns and pattern analysis.

use super::components::{add_blank_line, add_label_value, add_section_header};
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

/// Render patterns page showing detected patterns and pattern analysis
pub fn render(
    frame: &mut Frame,
    _app: &ResultsApp,
    item: &UnifiedDebtItem,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();
    let mut has_any_data = false;

    // Pattern Analysis section
    if let Some(ref pattern_analysis) = item.pattern_analysis {
        has_any_data = true;
        add_section_header(&mut lines, "PATTERN ANALYSIS", theme);

        // Framework patterns
        if pattern_analysis.frameworks.has_patterns() {
            add_label_value(
                &mut lines,
                "Framework Patterns",
                "Detected".to_string(),
                theme,
            );
        }

        // Rust patterns
        if !pattern_analysis.rust_patterns.trait_impls.is_empty() {
            add_label_value(
                &mut lines,
                "Trait Implementations",
                pattern_analysis.rust_patterns.trait_impls.len().to_string(),
                theme,
            );
        }

        add_blank_line(&mut lines);
    }

    // Detected Pattern section
    if let Some(ref detected_pattern) = item.detected_pattern {
        has_any_data = true;
        add_section_header(&mut lines, "DETECTED PATTERNS", theme);

        add_label_value(
            &mut lines,
            "Pattern Type",
            format!("{:?}", detected_pattern.pattern_type),
            theme,
        );

        add_label_value(
            &mut lines,
            "Confidence",
            format!("{:.1}%", detected_pattern.confidence * 100.0),
            theme,
        );

        add_blank_line(&mut lines);
    }

    // Language-Specific section
    if let Some(ref lang_specific) = item.language_specific {
        has_any_data = true;
        add_section_header(&mut lines, "LANGUAGE-SPECIFIC (RUST)", theme);

        match lang_specific {
            crate::core::LanguageSpecificData::Rust(rust_data) => {
                if let Some(ref trait_impl) = rust_data.trait_impl {
                    add_label_value(
                        &mut lines,
                        "Trait Implementation",
                        format!("{:?}", trait_impl),
                        theme,
                    );
                }
                if !rust_data.async_patterns.is_empty() {
                    add_label_value(
                        &mut lines,
                        "Async Patterns",
                        format!("{} detected", rust_data.async_patterns.len()),
                        theme,
                    );
                }
                if !rust_data.error_patterns.is_empty() {
                    add_label_value(
                        &mut lines,
                        "Error Patterns",
                        format!("{} detected", rust_data.error_patterns.len()),
                        theme,
                    );
                }
                if !rust_data.builder_patterns.is_empty() {
                    add_label_value(
                        &mut lines,
                        "Builder Patterns",
                        format!("{} detected", rust_data.builder_patterns.len()),
                        theme,
                    );
                }
            }
        }

        add_blank_line(&mut lines);
    }

    // If no data available
    if !has_any_data {
        lines.push(Line::from(vec![Span::styled(
            "No pattern data available",
            Style::default().fg(theme.muted),
        )]));
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
