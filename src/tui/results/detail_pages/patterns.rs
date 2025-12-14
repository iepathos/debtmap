//! Patterns page (Page 4) - Detected patterns and pattern analysis.

use super::components::{add_blank_line, add_label_value, add_section_header};
use crate::core::LanguageSpecificData;
use crate::organization::anti_pattern_detector::AntiPatternSeverity;
use crate::organization::god_object::GodObjectAnalysis;
use crate::organization::AntiPatternReport;
use crate::output::PatternAnalysis;
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
    // Allow anti-patterns to be shown even for non-god objects
    let mut rendered = false;

    // Render anti-patterns if present (Spec 197)
    if let Some(ref report) = god_analysis.anti_pattern_report {
        rendered |= render_anti_patterns_section(lines, report, theme, width);
    }

    // Only show god object-specific sections for actual god objects
    if !god_analysis.is_god_object {
        return rendered;
    }

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

/// Render anti-pattern detection section (Spec 197). Returns true if anything was rendered.
fn render_anti_patterns_section(
    lines: &mut Vec<Line<'static>>,
    report: &AntiPatternReport,
    theme: &Theme,
    width: u16,
) -> bool {
    if report.anti_patterns.is_empty() {
        return false;
    }

    add_section_header(lines, "anti-patterns detected", theme);

    // Show quality score
    let quality_desc = if report.quality_score >= 90.0 {
        "excellent"
    } else if report.quality_score >= 70.0 {
        "good"
    } else if report.quality_score >= 50.0 {
        "needs improvement"
    } else {
        "poor"
    };
    add_label_value(
        lines,
        "quality score",
        format!("{:.0} ({})", report.quality_score, quality_desc),
        theme,
        width,
    );

    // Count by severity
    let mut critical_count = 0;
    let mut high_count = 0;
    let mut medium_count = 0;
    let mut low_count = 0;

    for pattern in &report.anti_patterns {
        match pattern.severity {
            AntiPatternSeverity::Critical => critical_count += 1,
            AntiPatternSeverity::High => high_count += 1,
            AntiPatternSeverity::Medium => medium_count += 1,
            AntiPatternSeverity::Low => low_count += 1,
        }
    }

    // Show severity breakdown
    if critical_count > 0 {
        add_label_value(lines, "critical", critical_count.to_string(), theme, width);
    }
    if high_count > 0 {
        add_label_value(lines, "high", high_count.to_string(), theme, width);
    }
    if medium_count > 0 {
        add_label_value(lines, "medium", medium_count.to_string(), theme, width);
    }
    if low_count > 0 {
        add_label_value(lines, "low", low_count.to_string(), theme, width);
    }

    // Show first few anti-patterns with descriptions
    add_blank_line(lines);

    for (i, pattern) in report.anti_patterns.iter().take(5).enumerate() {
        let severity_indicator = match pattern.severity {
            AntiPatternSeverity::Critical => "!!",
            AntiPatternSeverity::High => "! ",
            AntiPatternSeverity::Medium => "- ",
            AntiPatternSeverity::Low => "  ",
        };

        let pattern_text = format!("{} {:?}", severity_indicator, pattern.pattern_type);
        add_label_value(lines, &format!("#{}", i + 1), pattern_text, theme, width);

        // Show description if it fits
        if !pattern.description.is_empty() && width > 40 {
            let desc = if pattern.description.len() > (width - 10) as usize {
                format!("{}...", &pattern.description[..((width - 13) as usize)])
            } else {
                pattern.description.clone()
            };
            add_label_value(lines, "  desc", desc, theme, width);
        }
    }

    if report.anti_patterns.len() > 5 {
        add_label_value(
            lines,
            "...",
            format!("{} more patterns", report.anti_patterns.len() - 5),
            theme,
            width,
        );
    }

    add_blank_line(lines);
    true
}

/// Build all lines for the patterns page (pure function).
///
/// This is public so text_extraction can reuse it for clipboard copy.
pub fn build_page_lines(item: &UnifiedDebtItem, theme: &Theme, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Compose all section renderers
    let has_any_data = build_all_sections(&mut lines, item, theme, width);

    // If no data available
    if !has_any_data {
        lines.push(Line::from(vec![Span::styled(
            "No pattern data available",
            Style::default().fg(theme.muted),
        )]));
    }

    lines
}

/// Render patterns page showing detected patterns and pattern analysis
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

/// Build all pattern sections, returning true if any data was rendered
fn build_all_sections(
    lines: &mut Vec<Line<'static>>,
    item: &UnifiedDebtItem,
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

    // Purity analysis moved to Data Flow page (Page 5)

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
