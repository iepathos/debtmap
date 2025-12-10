//! Patterns page (Page 4) - Detected patterns and pattern analysis.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::data_flow::DataFlowGraph;
use crate::priority::call_graph::FunctionId;
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
    data_flow: &DataFlowGraph,
    area: Rect,
    theme: &Theme,
) {
    let mut lines = Vec::new();
    let mut has_any_data = false;

    // Entropy Analysis section
    if let Some(ref entropy) = item.entropy_details {
        has_any_data = true;
        add_section_header(&mut lines, "entropy analysis", theme);

        let entropy_desc = if entropy.entropy_score < 0.3 {
            "low (repetitive)"
        } else if entropy.entropy_score < 0.5 {
            "medium (typical)"
        } else {
            "high (chaotic)"
        };

        add_label_value(
            &mut lines,
            "entropy",
            format!("{:.3} {}", entropy.entropy_score, entropy_desc),
            theme,
            area.width,
        );

        add_label_value(
            &mut lines,
            "repetition",
            format!("{:.3}", entropy.pattern_repetition),
            theme,
            area.width,
        );

        if entropy.dampening_factor < 1.0 {
            add_label_value(
                &mut lines,
                "dampening",
                format!("{:.3}x reduction", entropy.dampening_factor),
                theme,
                area.width,
            );

            add_label_value(
                &mut lines,
                "cognitive complexity",
                format!(
                    "{} → {} (dampened)",
                    entropy.original_complexity, entropy.adjusted_cognitive
                ),
                theme,
                area.width,
            );
        } else {
            add_label_value(&mut lines, "dampening", "No".to_string(), theme, area.width);
        }

        add_blank_line(&mut lines);
    }

    // Pattern Analysis section
    if let Some(ref pattern_analysis) = item.pattern_analysis {
        has_any_data = true;
        add_section_header(&mut lines, "pattern analysis", theme);

        // Framework patterns
        if pattern_analysis.frameworks.has_patterns() {
            add_label_value(
                &mut lines,
                "frameworks",
                "Detected".to_string(),
                theme,
                area.width,
            );
        }

        // Rust patterns
        if !pattern_analysis.rust_patterns.trait_impls.is_empty() {
            add_label_value(
                &mut lines,
                "traits",
                pattern_analysis.rust_patterns.trait_impls.len().to_string(),
                theme,
                area.width,
            );
        }

        add_blank_line(&mut lines);
    }

    // Detected Pattern section
    if let Some(ref detected_pattern) = item.detected_pattern {
        has_any_data = true;
        add_section_header(&mut lines, "detected patterns", theme);

        add_label_value(
            &mut lines,
            "pattern",
            format!("{:?}", detected_pattern.pattern_type),
            theme,
            area.width,
        );

        add_label_value(
            &mut lines,
            "confidence",
            format!("{:.1}%", detected_pattern.confidence * 100.0),
            theme,
            area.width,
        );

        add_blank_line(&mut lines);
    }

    // Language-Specific section
    if let Some(ref lang_specific) = item.language_specific {
        has_any_data = true;
        add_section_header(&mut lines, "language-specific (rust)", theme);

        match lang_specific {
            crate::core::LanguageSpecificData::Rust(rust_data) => {
                if let Some(ref trait_impl) = rust_data.trait_impl {
                    add_label_value(
                        &mut lines,
                        "trait",
                        format!("{:?}", trait_impl),
                        theme,
                        area.width,
                    );
                }
                if !rust_data.async_patterns.is_empty() {
                    add_label_value(
                        &mut lines,
                        "async",
                        format!("{} detected", rust_data.async_patterns.len()),
                        theme,
                        area.width,
                    );
                }
                if !rust_data.error_patterns.is_empty() {
                    add_label_value(
                        &mut lines,
                        "errors",
                        format!("{} detected", rust_data.error_patterns.len()),
                        theme,
                        area.width,
                    );
                }
                if !rust_data.builder_patterns.is_empty() {
                    add_label_value(
                        &mut lines,
                        "builders",
                        format!("{} detected", rust_data.builder_patterns.len()),
                        theme,
                        area.width,
                    );
                }
            }
        }

        add_blank_line(&mut lines);
    }

    // Purity Analysis section (moved from Data Flow page)
    let func_id = FunctionId::new(
        item.location.file.clone(),
        item.location.function.clone(),
        item.location.line,
    );

    if let Some(purity_info) = data_flow.get_purity_info(&func_id) {
        has_any_data = true;
        add_section_header(&mut lines, "purity analysis", theme);

        add_label_value(
            &mut lines,
            "pure",
            if purity_info.is_pure { "Yes" } else { "No" }.to_string(),
            theme,
            area.width,
        );

        add_label_value(
            &mut lines,
            "confidence",
            format!("{:.1}%", purity_info.confidence * 100.0),
            theme,
            area.width,
        );

        if !purity_info.impurity_reasons.is_empty() {
            add_label_value(
                &mut lines,
                "reasons",
                purity_info.impurity_reasons.join(", "),
                theme,
                area.width,
            );
        }

        add_blank_line(&mut lines);
    }

    // Error Swallowing section (for regular functions)
    if item.error_swallowing_count.is_some() || item.error_swallowing_patterns.is_some() {
        has_any_data = true;
        add_section_header(&mut lines, "error handling", theme);

        if let Some(count) = item.error_swallowing_count {
            add_label_value(
                &mut lines,
                "errors swallowed",
                count.to_string(),
                theme,
                area.width,
            );
        }

        if let Some(ref patterns) = item.error_swallowing_patterns {
            for pattern in patterns {
                add_label_value(&mut lines, "pattern", pattern.clone(), theme, area.width);
            }
        }

        add_blank_line(&mut lines);
    }

    // God Object Aggregated Patterns (for god objects)
    if let Some(ref god_indicators) = item.god_object_indicators {
        if god_indicators.is_god_object {
            // Show aggregated entropy for god objects
            if let Some(ref entropy) = god_indicators.aggregated_entropy {
                has_any_data = true;
                add_section_header(&mut lines, "god object entropy (aggregated)", theme);

                let entropy_desc = if entropy.entropy_score < 0.3 {
                    "low (repetitive)"
                } else if entropy.entropy_score < 0.5 {
                    "medium (typical)"
                } else {
                    "high (chaotic)"
                };

                add_label_value(
                    &mut lines,
                    "entropy",
                    format!("{:.3} {}", entropy.entropy_score, entropy_desc),
                    theme,
                    area.width,
                );

                add_label_value(
                    &mut lines,
                    "repetition",
                    format!("{:.3}", entropy.pattern_repetition),
                    theme,
                    area.width,
                );

                add_label_value(
                    &mut lines,
                    "total complexity",
                    format!(
                        "{} (original) → {} (adjusted)",
                        entropy.original_complexity, entropy.adjusted_cognitive
                    ),
                    theme,
                    area.width,
                );

                if entropy.dampening_factor < 1.0 {
                    add_label_value(
                        &mut lines,
                        "dampening",
                        format!("{:.3}x reduction", entropy.dampening_factor),
                        theme,
                        area.width,
                    );
                }

                add_blank_line(&mut lines);
            }

            // Show aggregated error swallowing for god objects
            let has_error_data = god_indicators.aggregated_error_swallowing_count.is_some()
                || god_indicators
                    .aggregated_error_swallowing_patterns
                    .as_ref()
                    .map(|p| !p.is_empty())
                    .unwrap_or(false);

            if has_error_data {
                has_any_data = true;
                add_section_header(&mut lines, "god object error handling (aggregated)", theme);

                if let Some(count) = god_indicators.aggregated_error_swallowing_count {
                    add_label_value(
                        &mut lines,
                        "errors swallowed",
                        format!("{} across all functions", count),
                        theme,
                        area.width,
                    );
                }

                if let Some(ref patterns) = god_indicators.aggregated_error_swallowing_patterns {
                    add_label_value(
                        &mut lines,
                        "unique patterns",
                        patterns.len().to_string(),
                        theme,
                        area.width,
                    );
                    for pattern in patterns {
                        add_label_value(&mut lines, "pattern", pattern.clone(), theme, area.width);
                    }
                }

                add_blank_line(&mut lines);
            }
        }
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
