//! Patterns page (Page 4) - Detected patterns and pattern analysis.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::core::LanguageSpecificData;
use crate::data_flow::{DataFlowGraph, PurityInfo};
use crate::organization::god_object::GodObjectAnalysis;
use crate::output::PatternAnalysis;
use crate::priority::call_graph::FunctionId;
use crate::priority::detected_pattern::DetectedPattern;
use crate::priority::unified_scorer::EntropyDetails;
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

/// Describe entropy level based on score
fn entropy_description(score: f64) -> &'static str {
    if score < 0.3 {
        "low (repetitive)"
    } else if score < 0.5 {
        "medium (typical)"
    } else {
        "high (chaotic)"
    }
}

/// Render entropy analysis section. Returns true if anything was rendered.
fn render_entropy_section(
    lines: &mut Vec<Line<'static>>,
    entropy: &EntropyDetails,
    theme: &Theme,
    width: u16,
) -> bool {
    add_section_header(lines, "entropy analysis", theme);

    add_label_value(
        lines,
        "entropy",
        format!(
            "{:.3} {}",
            entropy.entropy_score,
            entropy_description(entropy.entropy_score)
        ),
        theme,
        width,
    );

    add_label_value(
        lines,
        "repetition",
        format!("{:.3}", entropy.pattern_repetition),
        theme,
        width,
    );

    if entropy.dampening_factor < 1.0 {
        add_label_value(
            lines,
            "dampening",
            format!("{:.3}x reduction", entropy.dampening_factor),
            theme,
            width,
        );
        add_label_value(
            lines,
            "cognitive complexity",
            format!(
                "{} → {} (dampened)",
                entropy.original_complexity, entropy.adjusted_cognitive
            ),
            theme,
            width,
        );
    } else {
        add_label_value(lines, "dampening", "No".to_string(), theme, width);
    }

    add_blank_line(lines);
    true
}

/// Render pattern analysis section. Returns true if anything was rendered.
fn render_pattern_analysis_section(
    lines: &mut Vec<Line<'static>>,
    pattern_analysis: &PatternAnalysis,
    theme: &Theme,
    width: u16,
) -> bool {
    add_section_header(lines, "pattern analysis", theme);

    if pattern_analysis.frameworks.has_patterns() {
        add_label_value(lines, "frameworks", "Detected".to_string(), theme, width);
    }

    if !pattern_analysis.rust_patterns.trait_impls.is_empty() {
        add_label_value(
            lines,
            "traits",
            pattern_analysis.rust_patterns.trait_impls.len().to_string(),
            theme,
            width,
        );
    }

    add_blank_line(lines);
    true
}

/// Render detected pattern section. Returns true if anything was rendered.
fn render_detected_pattern_section(
    lines: &mut Vec<Line<'static>>,
    detected_pattern: &DetectedPattern,
    theme: &Theme,
    width: u16,
) -> bool {
    add_section_header(lines, "detected patterns", theme);

    add_label_value(
        lines,
        "pattern",
        format!("{:?}", detected_pattern.pattern_type),
        theme,
        width,
    );

    add_label_value(
        lines,
        "confidence",
        format!("{:.1}%", detected_pattern.confidence * 100.0),
        theme,
        width,
    );

    add_blank_line(lines);
    true
}

/// Render language-specific section. Returns true if anything was rendered.
fn render_language_specific_section(
    lines: &mut Vec<Line<'static>>,
    lang_specific: &LanguageSpecificData,
    theme: &Theme,
    width: u16,
) -> bool {
    add_section_header(lines, "language-specific (rust)", theme);

    match lang_specific {
        LanguageSpecificData::Rust(rust_data) => {
            if let Some(ref trait_impl) = rust_data.trait_impl {
                add_label_value(lines, "trait", format!("{:?}", trait_impl), theme, width);
            }
            if !rust_data.async_patterns.is_empty() {
                add_label_value(
                    lines,
                    "async",
                    format!("{} detected", rust_data.async_patterns.len()),
                    theme,
                    width,
                );
            }
            if !rust_data.error_patterns.is_empty() {
                add_label_value(
                    lines,
                    "errors",
                    format!("{} detected", rust_data.error_patterns.len()),
                    theme,
                    width,
                );
            }
            if !rust_data.builder_patterns.is_empty() {
                add_label_value(
                    lines,
                    "builders",
                    format!("{} detected", rust_data.builder_patterns.len()),
                    theme,
                    width,
                );
            }
        }
    }

    add_blank_line(lines);
    true
}

/// Render purity analysis section. Returns true if anything was rendered.
fn render_purity_section(
    lines: &mut Vec<Line<'static>>,
    purity_info: &PurityInfo,
    theme: &Theme,
    width: u16,
) -> bool {
    add_section_header(lines, "purity analysis", theme);

    add_label_value(
        lines,
        "pure",
        if purity_info.is_pure { "Yes" } else { "No" }.to_string(),
        theme,
        width,
    );

    add_label_value(
        lines,
        "confidence",
        format!("{:.1}%", purity_info.confidence * 100.0),
        theme,
        width,
    );

    if !purity_info.impurity_reasons.is_empty() {
        add_label_value(
            lines,
            "reasons",
            purity_info.impurity_reasons.join(", "),
            theme,
            width,
        );
    }

    add_blank_line(lines);
    true
}

/// Render error handling section. Returns true if anything was rendered.
fn render_error_handling_section(
    lines: &mut Vec<Line<'static>>,
    error_count: Option<u32>,
    error_patterns: Option<&Vec<String>>,
    theme: &Theme,
    width: u16,
) -> bool {
    add_section_header(lines, "error handling", theme);

    if let Some(count) = error_count {
        add_label_value(lines, "errors swallowed", count.to_string(), theme, width);
    }

    if let Some(patterns) = error_patterns {
        for pattern in patterns {
            add_label_value(lines, "pattern", pattern.clone(), theme, width);
        }
    }

    add_blank_line(lines);
    true
}

/// Render god object entropy section. Returns true if anything was rendered.
fn render_god_object_entropy_section(
    lines: &mut Vec<Line<'static>>,
    entropy: &EntropyDetails,
    theme: &Theme,
    width: u16,
) -> bool {
    add_section_header(lines, "god object entropy (aggregated)", theme);

    add_label_value(
        lines,
        "entropy",
        format!(
            "{:.3} {}",
            entropy.entropy_score,
            entropy_description(entropy.entropy_score)
        ),
        theme,
        width,
    );

    add_label_value(
        lines,
        "repetition",
        format!("{:.3}", entropy.pattern_repetition),
        theme,
        width,
    );

    add_label_value(
        lines,
        "total complexity",
        format!(
            "{} (original) → {} (adjusted)",
            entropy.original_complexity, entropy.adjusted_cognitive
        ),
        theme,
        width,
    );

    if entropy.dampening_factor < 1.0 {
        add_label_value(
            lines,
            "dampening",
            format!("{:.3}x reduction", entropy.dampening_factor),
            theme,
            width,
        );
    }

    add_blank_line(lines);
    true
}

/// Render god object error handling section. Returns true if anything was rendered.
fn render_god_object_error_section(
    lines: &mut Vec<Line<'static>>,
    error_count: Option<u32>,
    error_patterns: Option<&Vec<String>>,
    theme: &Theme,
    width: u16,
) -> bool {
    let has_error_data =
        error_count.is_some() || error_patterns.map(|p| !p.is_empty()).unwrap_or(false);

    if !has_error_data {
        return false;
    }

    add_section_header(lines, "god object error handling (aggregated)", theme);

    if let Some(count) = error_count {
        add_label_value(
            lines,
            "errors swallowed",
            format!("{} across all functions", count),
            theme,
            width,
        );
    }

    if let Some(patterns) = error_patterns {
        add_label_value(
            lines,
            "unique patterns",
            patterns.len().to_string(),
            theme,
            width,
        );
        for pattern in patterns {
            add_label_value(lines, "pattern", pattern.clone(), theme, width);
        }
    }

    add_blank_line(lines);
    true
}

/// Render god object patterns section. Returns true if anything was rendered.
fn render_god_object_patterns_section(
    lines: &mut Vec<Line<'static>>,
    god_analysis: &GodObjectAnalysis,
    theme: &Theme,
    width: u16,
) -> bool {
    if !god_analysis.is_god_object {
        return false;
    }

    let mut rendered = false;

    if let Some(ref entropy) = god_analysis.aggregated_entropy {
        rendered |= render_god_object_entropy_section(lines, entropy, theme, width);
    }

    rendered |= render_god_object_error_section(
        lines,
        god_analysis.aggregated_error_swallowing_count,
        god_analysis.aggregated_error_swallowing_patterns.as_ref(),
        theme,
        width,
    );

    rendered
}

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
    let width = area.width;

    // Compose all section renderers
    let has_any_data = render_all_sections(&mut lines, item, data_flow, theme, width);

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

/// Render all pattern sections, returning true if any data was rendered
fn render_all_sections(
    lines: &mut Vec<Line<'static>>,
    item: &UnifiedDebtItem,
    data_flow: &DataFlowGraph,
    theme: &Theme,
    width: u16,
) -> bool {
    let mut has_any_data = false;

    // Entropy Analysis section
    if let Some(ref entropy) = item.entropy_details {
        has_any_data |= render_entropy_section(lines, entropy, theme, width);
    }

    // Pattern Analysis section
    if let Some(ref pattern_analysis) = item.pattern_analysis {
        has_any_data |= render_pattern_analysis_section(lines, pattern_analysis, theme, width);
    }

    // Detected Pattern section
    if let Some(ref detected_pattern) = item.detected_pattern {
        has_any_data |= render_detected_pattern_section(lines, detected_pattern, theme, width);
    }

    // Language-Specific section
    if let Some(ref lang_specific) = item.language_specific {
        has_any_data |= render_language_specific_section(lines, lang_specific, theme, width);
    }

    // Purity Analysis section
    let func_id = FunctionId::new(
        item.location.file.clone(),
        item.location.function.clone(),
        item.location.line,
    );
    if let Some(purity_info) = data_flow.get_purity_info(&func_id) {
        has_any_data |= render_purity_section(lines, purity_info, theme, width);
    }

    // Error Swallowing section (for regular functions)
    if item.error_swallowing_count.is_some() || item.error_swallowing_patterns.is_some() {
        has_any_data |= render_error_handling_section(
            lines,
            item.error_swallowing_count,
            item.error_swallowing_patterns.as_ref(),
            theme,
            width,
        );
    }

    // God Object Aggregated Patterns (for god objects)
    if let Some(ref god_indicators) = item.god_object_indicators {
        has_any_data |= render_god_object_patterns_section(lines, god_indicators, theme, width);
    }

    has_any_data
}
