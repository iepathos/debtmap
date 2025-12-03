//! Priority item formatting for markdown output
//!
//! Formats individual mixed priority items (function-level and file-level debt)

use crate::formatting::FormattingConfig;
use crate::priority::{DebtItem, FileDebtItem};
use std::fmt::Write;

use super::details::format_priority_item_markdown;
use super::utilities::{
    complexity_category, format_file_impact, function_category, get_file_extension,
    get_severity_label, score_category,
};

pub(crate) fn format_mixed_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &DebtItem,
    verbosity: u8,
    config: &FormattingConfig,
) {
    match item {
        DebtItem::Function(func_item) => {
            format_priority_item_markdown(output, rank, func_item, verbosity);
        }
        DebtItem::File(file_item) => {
            format_file_priority_item_markdown(
                output,
                rank,
                file_item,
                verbosity,
                config.show_splits,
            );
        }
    }
}

pub(crate) fn format_file_priority_item_markdown(
    output: &mut String,
    rank: usize,
    item: &FileDebtItem,
    verbosity: u8,
    show_splits: bool,
) {
    let severity = get_severity_label(item.score);

    // Determine file type using context-aware thresholds (spec 135)
    let type_label = if item.metrics.god_object_indicators.is_god_object {
        "FILE - GOD OBJECT"
    } else if let Some(ref file_type) = item.metrics.file_type {
        use crate::organization::get_threshold;
        let threshold = get_threshold(
            file_type,
            item.metrics.function_count,
            item.metrics.total_lines,
        );
        if item.metrics.total_lines > threshold.base_threshold {
            "FILE - HIGH COMPLEXITY"
        } else {
            "FILE"
        }
    } else {
        // Legacy behavior if no file type
        if item.metrics.total_lines > 500 {
            "FILE - HIGH COMPLEXITY"
        } else {
            "FILE"
        }
    };

    // File items (god objects) are always T1 Critical Architecture
    let tier_label = "[T1] ";

    // Header with rank, tier, and score
    writeln!(
        output,
        "### #{} {}Score: {:.1} [{}]",
        rank, tier_label, item.score, severity
    )
    .unwrap();

    writeln!(output, "**Type:** {}", type_label).unwrap();
    writeln!(
        output,
        "**File:** `{}` ({} lines, {} functions)",
        item.metrics.path.display(),
        item.metrics.total_lines,
        item.metrics.function_count
    )
    .unwrap();

    // God object details if applicable
    if item.metrics.god_object_indicators.is_god_object {
        writeln!(output, "**God Object Metrics:**").unwrap();
        writeln!(
            output,
            "- Methods: {}",
            item.metrics.god_object_indicators.methods_count
        )
        .unwrap();
        writeln!(
            output,
            "- Fields: {}",
            item.metrics.god_object_indicators.fields_count
        )
        .unwrap();
        writeln!(
            output,
            "- Responsibilities: {}",
            item.metrics.god_object_indicators.responsibilities
        )
        .unwrap();
        writeln!(
            output,
            "- God Object Score: {:.1}",
            item.metrics.god_object_indicators.god_object_score
        )
        .unwrap();

        // Show coverage data if available
        if item.metrics.coverage_percent > 0.0 {
            writeln!(
                output,
                "- Test Coverage: {:.1}% ({} uncovered lines)",
                item.metrics.coverage_percent, item.metrics.uncovered_lines
            )
            .unwrap();
        }
    }

    // Show split recommendations if available (Spec 208)
    format_split_recommendations_markdown(output, item, verbosity, show_splits);

    writeln!(output, "**Recommendation:** {}", item.recommendation).unwrap();

    writeln!(output, "**Impact:** {}", format_file_impact(&item.impact)).unwrap();

    if verbosity >= 1 {
        writeln!(output, "\n**Scoring Breakdown:**").unwrap();
        writeln!(
            output,
            "- File size: {}",
            score_category(item.metrics.total_lines)
        )
        .unwrap();
        writeln!(
            output,
            "- Functions: {}",
            function_category(item.metrics.function_count)
        )
        .unwrap();
        writeln!(
            output,
            "- Complexity: {}",
            complexity_category(item.metrics.avg_complexity)
        )
        .unwrap();
        if item.metrics.function_count > 0 {
            writeln!(
                output,
                "- Dependencies: {} functions may have complex interdependencies",
                item.metrics.function_count
            )
            .unwrap();
        }
    }
}

/// Format split recommendations for markdown output (Spec 208)
pub(crate) fn format_split_recommendations_markdown(
    output: &mut String,
    item: &FileDebtItem,
    verbosity: u8,
    show_splits: bool,
) {
    let indicators = &item.metrics.god_object_indicators;
    let extension = get_file_extension(&item.metrics.path);

    // If we have detailed split recommendations, use them
    if !indicators.recommended_splits.is_empty() {
        // Spec 208: Only show detailed splits when --show-splits flag is provided
        if !show_splits {
            // Show a brief hint that splits are available
            writeln!(output).unwrap();
            writeln!(
                output,
                "*(Use --show-splits for detailed module split recommendations)*"
            )
            .unwrap();
        } else {
            // Full splits display when --show-splits is enabled
            writeln!(output).unwrap();

            // Use different header based on number of splits
            if indicators.recommended_splits.len() == 1 {
                writeln!(
                    output,
                    "**EXTRACTION CANDIDATE** (single cohesive group found):"
                )
                .unwrap();
            } else {
                writeln!(
                    output,
                    "**RECOMMENDED SPLITS** ({} modules):",
                    indicators.recommended_splits.len()
                )
                .unwrap();
            }

            writeln!(output).unwrap();

            for split in indicators.recommended_splits.iter() {
                // Show module name and responsibility
                writeln!(output, "- **{}.{}**", split.suggested_name, extension).unwrap();

                let priority_indicator = match split.priority {
                    crate::priority::file_metrics::Priority::High => "High",
                    crate::priority::file_metrics::Priority::Medium => "Medium",
                    crate::priority::file_metrics::Priority::Low => "Low",
                };

                writeln!(
                    output,
                    "  - Category: {} | Priority: {}",
                    split.responsibility, priority_indicator
                )
                .unwrap();
                writeln!(
                    output,
                    "  - Size: {} methods, ~{} lines",
                    split.methods_to_move.len(),
                    split.estimated_lines,
                )
                .unwrap();

                // Display classification evidence if available and verbosity > 0
                if verbosity > 0 {
                    if let Some(ref evidence) = split.classification_evidence {
                        writeln!(
                            output,
                            "  - Confidence: {:.1}% | Signals: {}",
                            evidence.confidence * 100.0,
                            evidence.evidence.len()
                        )
                        .unwrap();

                        // Show alternatives warning if confidence is low
                        if evidence.confidence < 0.80 && !evidence.alternatives.is_empty() {
                            writeln!(
                                output,
                                "  - **⚠ Low confidence classification - review recommended**"
                            )
                            .unwrap();
                        }
                    }
                }

                // Show representative methods (Spec 178: methods before structs)
                if !split.representative_methods.is_empty() {
                    let total_methods = split.representative_methods.len();
                    let sample_size = 5.min(total_methods);

                    writeln!(output, "  - Methods ({} total):", total_methods).unwrap();

                    for method in split.representative_methods.iter().take(sample_size) {
                        writeln!(output, "    - `{}()`", method).unwrap();
                    }

                    if total_methods > sample_size {
                        writeln!(output, "    - ... and {} more", total_methods - sample_size)
                            .unwrap();
                    }
                } else if !split.methods_to_move.is_empty() {
                    // Fallback to methods_to_move if representative_methods not populated
                    let total_methods = split.methods_to_move.len();
                    let sample_size = 5.min(total_methods);

                    writeln!(output, "  - Methods ({} total):", total_methods).unwrap();

                    for method in split.methods_to_move.iter().take(sample_size) {
                        writeln!(output, "    - `{}()`", method).unwrap();
                    }

                    if total_methods > sample_size {
                        writeln!(output, "    - ... and {} more", total_methods - sample_size)
                            .unwrap();
                    }
                }

                // Show fields needed (Spec 178: field dependencies)
                if !split.fields_needed.is_empty() {
                    writeln!(
                        output,
                        "  - Fields needed: {}",
                        split.fields_needed.join(", ")
                    )
                    .unwrap();
                }

                // Show trait extraction suggestion (Spec 178)
                if let Some(ref trait_suggestion) = split.trait_suggestion {
                    if verbosity > 0 {
                        writeln!(output, "  - Trait extraction:").unwrap();
                        for line in trait_suggestion.lines() {
                            writeln!(output, "    {}", line).unwrap();
                        }
                    }
                }

                // Show structs being moved (secondary to methods, per Spec 178)
                if !split.structs_to_move.is_empty() {
                    writeln!(output, "  - Structs: {}", split.structs_to_move.join(", ")).unwrap();
                }

                // Show warning if present
                if let Some(warning) = &split.warning {
                    writeln!(output, "  - **⚠ {}**", warning).unwrap();
                }

                writeln!(output).unwrap();
            }

            // Add explanation if only 1 group found
            if indicators.recommended_splits.len() == 1 {
                writeln!(
                    output,
                    "*NOTE: Only one cohesive group detected. This suggests methods are tightly coupled.*"
                )
                .unwrap();
                writeln!(
                    output,
                    "*Consider splitting by feature/responsibility rather than call patterns.*"
                )
                .unwrap();
                writeln!(output).unwrap();
            }
        }
    } else {
        // No detailed splits available - provide diagnostic message (Spec 149)
        // Spec 208: Only show this diagnostic when --show-splits is enabled
        if show_splits {
            writeln!(output).unwrap();
            writeln!(output, "**NO DETAILED SPLITS AVAILABLE**").unwrap();
            writeln!(
                output,
                "- Analysis could not generate responsibility-based splits for this file."
            )
            .unwrap();
            writeln!(output, "- This may indicate:").unwrap();
            writeln!(
                output,
                "  - File has too few functions (< 3 per responsibility)"
            )
            .unwrap();
            writeln!(output, "  - Functions lack clear responsibility signals").unwrap();
            writeln!(output, "  - File may be test-only or configuration").unwrap();
            writeln!(
                output,
                "- Consider manual analysis or consult documentation."
            )
            .unwrap();
            writeln!(output).unwrap();
        }
        // Note: When show_splits is false and no splits are available, we don't show anything
        // since there's no hint to offer (no splits exist to display)
    }
}
