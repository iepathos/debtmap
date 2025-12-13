//! User actions (clipboard, editor).

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use super::app::{DetailPage, ResultsApp};
use crate::data_flow::DataFlowGraph;
use crate::priority::call_graph::FunctionId;
use crate::priority::UnifiedDebtItem;

/// Copy text to system clipboard and return status message
fn copy_to_clipboard(text: &str, description: &str) -> Result<String> {
    use arboard::Clipboard;

    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(text) {
            Ok(_) => Ok(format!("✓ Copied {} to clipboard", description)),
            Err(e) => {
                // Show error so user knows what happened
                Ok(format!("✗ Clipboard error: {}", e))
            }
        },
        Err(e) => {
            // Clipboard not available (SSH, headless, etc.)
            Ok(format!("✗ Clipboard not available: {}", e))
        }
    }
}

/// Copy file path to system clipboard and return status message
pub fn copy_path_to_clipboard(path: &Path) -> Result<String> {
    let path_str = path.to_string_lossy().to_string();
    copy_to_clipboard(&path_str, "path")
}

/// Copy detail page content to clipboard and return status message
pub fn copy_page_to_clipboard(
    item: &UnifiedDebtItem,
    page: DetailPage,
    app: &ResultsApp,
) -> Result<String> {
    let content = extract_page_text(item, page, app);
    copy_to_clipboard(&content, "page content")
}

/// Extract plain text content from a detail page
/// This matches the exact layout of the rendered TUI pages
fn extract_page_text(item: &UnifiedDebtItem, page: DetailPage, app: &ResultsApp) -> String {
    match page {
        DetailPage::Overview => extract_overview_text(item, app),
        DetailPage::Dependencies => extract_dependencies_text(item, app),
        DetailPage::GitContext => extract_git_context_text(item),
        DetailPage::Patterns => extract_patterns_text(item, &app.analysis().data_flow_graph),
        DetailPage::DataFlow => extract_data_flow_text(item, &app.analysis().data_flow_graph),
        DetailPage::Responsibilities => extract_responsibilities_text(item),
    }
}

// =============================================================================
// Text formatting helpers (match TUI layout exactly)
// =============================================================================

const INDENT: usize = 2;
const LABEL_WIDTH: usize = 24;
const GAP: usize = 4;

/// Add a section header (lowercase, matches TUI)
fn add_section_header(output: &mut String, title: &str) {
    output.push_str(title);
    output.push('\n');
}

/// Add a label-value pair with aligned columns (matches TUI exactly)
fn add_label_value(output: &mut String, label: &str, value: &str) {
    let label_with_indent = format!("{}{}", " ".repeat(INDENT), label);
    let padded_label = format!("{:width$}", label_with_indent, width = LABEL_WIDTH);
    let gap = " ".repeat(GAP);
    output.push_str(&format!("{}{}{}\n", padded_label, gap, value));
}

/// Add blank line
fn add_blank_line(output: &mut String) {
    output.push('\n');
}

// =============================================================================
// Overview page extraction
// =============================================================================

/// Extract overview page content as plain text (matches TUI exactly)
fn extract_overview_text(item: &UnifiedDebtItem, app: &ResultsApp) -> String {
    use crate::priority::classification::Severity;
    use crate::priority::DebtType;

    let mut output = String::new();

    // Location section
    add_section_header(&mut output, "location");
    add_label_value(
        &mut output,
        "file",
        &item.location.file.display().to_string(),
    );
    add_label_value(&mut output, "function", &item.location.function);
    add_label_value(&mut output, "line", &item.location.line.to_string());
    add_blank_line(&mut output);

    // Get all items at this location
    let location_items: Vec<&UnifiedDebtItem> = app
        .analysis()
        .items
        .iter()
        .filter(|i| {
            i.location.file == item.location.file
                && i.location.function == item.location.function
                && i.location.line == item.location.line
        })
        .collect();

    // Score section
    add_section_header(&mut output, "score");

    if location_items.len() > 1 {
        // Multiple debt types - show combined score
        let combined_score: f64 = location_items
            .iter()
            .map(|i| i.unified_score.final_score.value())
            .sum();
        let severity = Severity::from_score_100(combined_score)
            .as_str()
            .to_lowercase();
        add_label_value(
            &mut output,
            "combined",
            &format!("{:.1}  [{}]", combined_score, severity),
        );
    } else {
        // Single debt type - show single score
        let severity = Severity::from_score_100(item.unified_score.final_score.value())
            .as_str()
            .to_lowercase();
        add_label_value(
            &mut output,
            "total",
            &format!(
                "{:.1}  [{}]",
                item.unified_score.final_score.value(),
                severity
            ),
        );
    }
    add_blank_line(&mut output);

    // For god objects, show structural metrics first
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
        add_section_header(&mut output, header);

        let method_label = match detection_type {
            Some(crate::organization::DetectionType::GodClass) => "methods",
            _ => "functions",
        };
        add_label_value(&mut output, method_label, &methods.to_string());

        if let Some(field_count) = fields {
            add_label_value(&mut output, "fields", &field_count.to_string());
        }

        add_label_value(
            &mut output,
            "responsibilities",
            &responsibilities.to_string(),
        );
        add_label_value(&mut output, "loc", &debt_lines.to_string());
        add_blank_line(&mut output);
    }

    // Complexity metrics section
    add_section_header(&mut output, "complexity");

    let is_god_object = matches!(item.debt_type, DebtType::GodObject { .. });
    let cyclomatic_label = if is_god_object {
        "accumulated cyclomatic"
    } else {
        "cyclomatic"
    };
    let cognitive_label = if is_god_object {
        "accumulated cognitive"
    } else {
        "cognitive"
    };
    let nesting_label = if is_god_object {
        "max nesting"
    } else {
        "nesting"
    };

    add_label_value(
        &mut output,
        cyclomatic_label,
        &item.cyclomatic_complexity.to_string(),
    );

    // Show cognitive complexity with entropy dampening if applicable
    let cognitive_display = if is_god_object {
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
    };
    add_label_value(&mut output, cognitive_label, &cognitive_display);
    add_label_value(&mut output, nesting_label, &item.nesting_depth.to_string());

    // For non-god objects, show function LOC
    if !is_god_object {
        add_label_value(&mut output, "loc", &item.function_length.to_string());
    }
    add_blank_line(&mut output);

    // Coverage section
    add_section_header(&mut output, "coverage");
    let coverage_value = if let Some(coverage) = item.transitive_coverage.as_ref().map(|c| c.direct)
    {
        format!("{:.1}%", coverage * 100.0)
    } else {
        "No data".to_string()
    };
    add_label_value(&mut output, "coverage", &coverage_value);
    add_blank_line(&mut output);

    // Recommendation section
    add_section_header(&mut output, "recommendation");
    add_label_value(&mut output, "action", &item.recommendation.primary_action);
    add_blank_line(&mut output);
    add_label_value(&mut output, "rationale", &item.recommendation.rationale);
    add_blank_line(&mut output);

    // Debt type section
    if location_items.len() > 1 {
        add_section_header(&mut output, "debt types");
        for debt_item in location_items.iter() {
            let debt_name = format_debt_type_name(&debt_item.debt_type);
            output.push_str(&format!("  {}\n", debt_name));
        }
    } else {
        add_section_header(&mut output, "debt type");
        output.push_str(&format!("  {}\n", format_debt_type_name(&item.debt_type)));
    }

    output
}

// =============================================================================
// Dependencies page extraction
// =============================================================================

/// Extract dependencies page content as plain text (matches TUI exactly)
fn extract_dependencies_text(item: &UnifiedDebtItem, app: &ResultsApp) -> String {
    let mut output = String::new();

    // Function-level Dependency Metrics section
    add_section_header(&mut output, "function dependencies");
    add_label_value(
        &mut output,
        "upstream",
        &item.upstream_dependencies.to_string(),
    );
    add_label_value(
        &mut output,
        "downstream",
        &item.downstream_dependencies.to_string(),
    );

    let blast_radius = item.upstream_dependencies + item.downstream_dependencies;
    add_label_value(&mut output, "blast radius", &blast_radius.to_string());

    let is_critical = item.upstream_dependencies > 5 || item.downstream_dependencies > 10;
    add_label_value(
        &mut output,
        "critical",
        if is_critical { "Yes" } else { "No" },
    );

    // File-level Coupling Metrics section
    let file_metrics = app
        .analysis()
        .file_items
        .iter()
        .find(|f| f.metrics.path == item.location.file);

    if let Some(file_item) = file_metrics {
        let metrics = &file_item.metrics;
        let total_coupling = metrics.afferent_coupling + metrics.efferent_coupling;

        if total_coupling > 0 || metrics.instability > 0.0 {
            add_blank_line(&mut output);
            add_section_header(&mut output, "coupling profile");

            // Classification
            let classification = derive_coupling_classification(
                metrics.afferent_coupling,
                metrics.efferent_coupling,
                metrics.instability,
            );
            add_label_value(
                &mut output,
                "classification",
                &format!("[{}]", classification.to_uppercase()),
            );

            add_label_value(
                &mut output,
                "afferent (ca)",
                &metrics.afferent_coupling.to_string(),
            );
            add_label_value(
                &mut output,
                "efferent (ce)",
                &metrics.efferent_coupling.to_string(),
            );

            // Instability with progress bar
            let bar_width = 20;
            let filled = ((metrics.instability * bar_width as f64).round() as usize).min(bar_width);
            let empty = bar_width - filled;
            let filled_bar: String = "█".repeat(filled);
            let empty_bar: String = "░".repeat(empty);
            add_label_value(
                &mut output,
                "instability",
                &format!("{:.2} {}{}", metrics.instability, filled_bar, empty_bar),
            );

            // Context notes
            if total_coupling > 15 {
                output.push_str("Warning: High coupling may indicate architectural issues.\n");
            } else if metrics.instability < 0.1 && metrics.afferent_coupling > 0 {
                output.push_str("Note: Stable core - changes need careful review.\n");
            } else if metrics.instability > 0.9 {
                output.push_str("Note: Unstable leaf - safe to refactor.\n");
            }

            // Dependents list
            if !metrics.dependents.is_empty() {
                add_blank_line(&mut output);
                add_section_header(&mut output, "dependents (who uses this)");
                let max_display = 5;
                for item_path in metrics.dependents.iter().take(max_display) {
                    let display_name = item_path.rsplit('/').next().unwrap_or(item_path);
                    output.push_str(&format!("  • {}\n", display_name));
                }
                if metrics.dependents.len() > max_display {
                    output.push_str(&format!(
                        "    (+{} more)\n",
                        metrics.dependents.len() - max_display
                    ));
                }
            }

            // Dependencies list
            if !metrics.dependencies_list.is_empty() {
                add_blank_line(&mut output);
                add_section_header(&mut output, "dependencies (what this uses)");
                let max_display = 5;
                for item_path in metrics.dependencies_list.iter().take(max_display) {
                    let display_name = item_path.rsplit('/').next().unwrap_or(item_path);
                    output.push_str(&format!("  • {}\n", display_name));
                }
                if metrics.dependencies_list.len() > max_display {
                    output.push_str(&format!(
                        "    (+{} more)\n",
                        metrics.dependencies_list.len() - max_display
                    ));
                }
            }
        }
    }

    output
}

/// Derive coupling classification from metrics
fn derive_coupling_classification(afferent: usize, efferent: usize, instability: f64) -> String {
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

// =============================================================================
// Git context page extraction
// =============================================================================

/// Classify stability based on change frequency
fn classify_stability(change_frequency: f64) -> &'static str {
    if change_frequency < 1.0 {
        "Stable"
    } else if change_frequency < 5.0 {
        "Moderately Unstable"
    } else {
        "Highly Unstable"
    }
}

/// Extract git context page content as plain text (matches TUI exactly)
fn extract_git_context_text(item: &UnifiedDebtItem) -> String {
    use crate::risk::context::ContextDetails;

    let mut output = String::new();

    if let Some(ref contextual_risk) = item.contextual_risk {
        // Look for git history context
        let git_context = contextual_risk
            .contexts
            .iter()
            .find(|ctx| ctx.provider == "git_history");

        if let Some(ctx) = git_context {
            if let ContextDetails::Historical {
                change_frequency,
                bug_density,
                age_days,
                author_count,
            } = &ctx.details
            {
                // Change Patterns section
                add_section_header(&mut output, "change patterns");
                add_label_value(
                    &mut output,
                    "frequency",
                    &format!("{:.2} changes/month", change_frequency),
                );

                let stability = classify_stability(*change_frequency);
                add_label_value(&mut output, "stability", stability);
                add_label_value(&mut output, "bugs", &format!("{:.1}%", bug_density * 100.0));
                add_label_value(&mut output, "age", &format!("{} days", age_days));
                add_label_value(&mut output, "contributors", &author_count.to_string());
                add_blank_line(&mut output);
            }
        }

        // Risk Impact section
        add_section_header(&mut output, "risk impact");
        add_label_value(
            &mut output,
            "base",
            &format!("{:.1}", contextual_risk.base_risk),
        );
        add_label_value(
            &mut output,
            "contextual",
            &format!("{:.1}", contextual_risk.contextual_risk),
        );

        let multiplier = if contextual_risk.base_risk > 0.0 {
            contextual_risk.contextual_risk / contextual_risk.base_risk
        } else {
            1.0
        };
        add_label_value(&mut output, "multiplier", &format!("{:.2}x", multiplier));
        add_blank_line(&mut output);
    }

    // Context Dampening section (if applicable)
    if let Some(ref file_type) = item.context_type {
        add_section_header(&mut output, "context dampening");
        add_label_value(&mut output, "file type", &format!("{:?}", file_type));

        if let Some(multiplier) = item.context_multiplier {
            let reduction = (1.0 - multiplier) * 100.0;
            add_label_value(&mut output, "reduction", &format!("{:.1}%", reduction));
        }
        add_blank_line(&mut output);
    }

    // If no data available
    if output.is_empty() {
        output.push_str("No git context data available\n");
    }

    output
}

// =============================================================================
// Patterns page extraction
// =============================================================================

/// Pure function to determine entropy level description
fn entropy_description(score: f64) -> &'static str {
    if score < 0.3 {
        "low (repetitive)"
    } else if score < 0.5 {
        "medium (typical)"
    } else {
        "high (chaotic)"
    }
}

/// Format entropy analysis section - returns None if no entropy data
fn format_entropy_section(
    entropy: &crate::priority::unified_scorer::EntropyDetails,
) -> Option<String> {
    let mut output = String::new();
    add_section_header(&mut output, "entropy analysis");

    let entropy_desc = entropy_description(entropy.entropy_score);
    add_label_value(
        &mut output,
        "entropy",
        &format!("{:.3} {}", entropy.entropy_score, entropy_desc),
    );
    add_label_value(
        &mut output,
        "repetition",
        &format!("{:.3}", entropy.pattern_repetition),
    );

    if entropy.dampening_factor < 1.0 {
        add_label_value(
            &mut output,
            "dampening",
            &format!("{:.3}x reduction", entropy.dampening_factor),
        );
        add_label_value(
            &mut output,
            "cognitive complexity",
            &format!(
                "{} → {} (dampened)",
                entropy.original_complexity, entropy.adjusted_cognitive
            ),
        );
    } else {
        add_label_value(&mut output, "dampening", "No");
    }

    add_blank_line(&mut output);
    Some(output)
}

/// Format pattern analysis section - returns None if no significant pattern data
fn format_pattern_analysis_section(
    pattern_analysis: &crate::output::PatternAnalysis,
) -> Option<String> {
    let has_frameworks = pattern_analysis.frameworks.has_patterns();
    let has_traits = !pattern_analysis.rust_patterns.trait_impls.is_empty();

    if !has_frameworks && !has_traits {
        return None;
    }

    let mut output = String::new();
    add_section_header(&mut output, "pattern analysis");

    if has_frameworks {
        add_label_value(&mut output, "frameworks", "Detected");
    }

    if has_traits {
        add_label_value(
            &mut output,
            "traits",
            &pattern_analysis.rust_patterns.trait_impls.len().to_string(),
        );
    }

    add_blank_line(&mut output);
    Some(output)
}

/// Format detected pattern section - returns None if no detected patterns
fn format_detected_pattern_section(
    detected_pattern: &crate::priority::detected_pattern::DetectedPattern,
) -> Option<String> {
    let mut output = String::new();
    add_section_header(&mut output, "detected patterns");
    add_label_value(
        &mut output,
        "pattern",
        &format!("{:?}", detected_pattern.pattern_type),
    );
    add_label_value(
        &mut output,
        "confidence",
        &format!("{:.1}%", detected_pattern.confidence * 100.0),
    );
    add_blank_line(&mut output);
    Some(output)
}

/// Format language-specific section - returns None if no relevant data
fn format_language_specific_section(
    lang_specific: &crate::core::LanguageSpecificData,
) -> Option<String> {
    match lang_specific {
        crate::core::LanguageSpecificData::Rust(rust_data) => {
            let has_trait = rust_data.trait_impl.is_some();
            let has_async = !rust_data.async_patterns.is_empty();
            let has_errors = !rust_data.error_patterns.is_empty();
            let has_builders = !rust_data.builder_patterns.is_empty();

            if !has_trait && !has_async && !has_errors && !has_builders {
                return None;
            }

            let mut output = String::new();
            add_section_header(&mut output, "language-specific (rust)");

            if let Some(ref trait_impl) = rust_data.trait_impl {
                add_label_value(&mut output, "trait", &format!("{:?}", trait_impl));
            }
            if has_async {
                add_label_value(
                    &mut output,
                    "async",
                    &format!("{} detected", rust_data.async_patterns.len()),
                );
            }
            if has_errors {
                add_label_value(
                    &mut output,
                    "errors",
                    &format!("{} detected", rust_data.error_patterns.len()),
                );
            }
            if has_builders {
                add_label_value(
                    &mut output,
                    "builders",
                    &format!("{} detected", rust_data.builder_patterns.len()),
                );
            }

            add_blank_line(&mut output);
            Some(output)
        }
    }
}

/// Format purity analysis section - returns None if no purity data
fn format_purity_section(func_id: &FunctionId, data_flow: &DataFlowGraph) -> Option<String> {
    let purity_info = data_flow.get_purity_info(func_id)?;

    let mut output = String::new();
    add_section_header(&mut output, "purity analysis");
    add_label_value(
        &mut output,
        "pure",
        if purity_info.is_pure { "Yes" } else { "No" },
    );
    add_label_value(
        &mut output,
        "confidence",
        &format!("{:.1}%", purity_info.confidence * 100.0),
    );

    if !purity_info.impurity_reasons.is_empty() {
        add_label_value(
            &mut output,
            "reasons",
            &purity_info.impurity_reasons.join(", "),
        );
    }

    add_blank_line(&mut output);
    Some(output)
}

/// Format error handling section - returns None if no error swallowing data
fn format_error_handling_section(
    error_count: Option<u32>,
    error_patterns: Option<&Vec<String>>,
) -> Option<String> {
    if error_count.is_none() && error_patterns.is_none() {
        return None;
    }

    let mut output = String::new();
    add_section_header(&mut output, "error handling");

    if let Some(count) = error_count {
        add_label_value(&mut output, "errors swallowed", &count.to_string());
    }

    if let Some(patterns) = error_patterns {
        for pattern in patterns {
            add_label_value(&mut output, "pattern", pattern);
        }
    }

    add_blank_line(&mut output);
    Some(output)
}

/// Format god object aggregated entropy section - returns None if no entropy data
fn format_god_object_entropy_section(
    entropy: &crate::priority::unified_scorer::EntropyDetails,
) -> Option<String> {
    let mut output = String::new();
    add_section_header(&mut output, "god object entropy (aggregated)");

    let entropy_desc = entropy_description(entropy.entropy_score);
    add_label_value(
        &mut output,
        "entropy",
        &format!("{:.3} {}", entropy.entropy_score, entropy_desc),
    );
    add_label_value(
        &mut output,
        "repetition",
        &format!("{:.3}", entropy.pattern_repetition),
    );
    add_label_value(
        &mut output,
        "total complexity",
        &format!(
            "{} (original) → {} (adjusted)",
            entropy.original_complexity, entropy.adjusted_cognitive
        ),
    );

    if entropy.dampening_factor < 1.0 {
        add_label_value(
            &mut output,
            "dampening",
            &format!("{:.3}x reduction", entropy.dampening_factor),
        );
    }

    add_blank_line(&mut output);
    Some(output)
}

/// Format god object aggregated error handling section - returns None if no error data
fn format_god_object_error_handling_section(
    error_count: Option<u32>,
    error_patterns: Option<&Vec<String>>,
) -> Option<String> {
    let has_count = error_count.is_some();
    let has_patterns = error_patterns
        .as_ref()
        .map(|p| !p.is_empty())
        .unwrap_or(false);

    if !has_count && !has_patterns {
        return None;
    }

    let mut output = String::new();
    add_section_header(&mut output, "god object error handling (aggregated)");

    if let Some(count) = error_count {
        add_label_value(
            &mut output,
            "errors swallowed",
            &format!("{} across all functions", count),
        );
    }

    if let Some(patterns) = error_patterns {
        add_label_value(&mut output, "unique patterns", &patterns.len().to_string());
        for pattern in patterns {
            add_label_value(&mut output, "pattern", pattern);
        }
    }

    add_blank_line(&mut output);
    Some(output)
}

/// Extract patterns page content as plain text (matches TUI exactly)
fn extract_patterns_text(item: &UnifiedDebtItem, data_flow: &DataFlowGraph) -> String {
    let func_id = FunctionId::new(
        item.location.file.clone(),
        item.location.function.clone(),
        item.location.line,
    );

    // Collect all section formatters as Options
    let sections: Vec<Option<String>> = vec![
        item.entropy_details
            .as_ref()
            .and_then(format_entropy_section),
        item.pattern_analysis
            .as_ref()
            .and_then(format_pattern_analysis_section),
        item.detected_pattern
            .as_ref()
            .and_then(format_detected_pattern_section),
        item.language_specific
            .as_ref()
            .and_then(format_language_specific_section),
        format_purity_section(&func_id, data_flow),
        format_error_handling_section(
            item.error_swallowing_count,
            item.error_swallowing_patterns.as_ref(),
        ),
        item.god_object_indicators
            .as_ref()
            .filter(|g| g.is_god_object)
            .and_then(|g| g.aggregated_entropy.as_ref())
            .and_then(format_god_object_entropy_section),
        item.god_object_indicators
            .as_ref()
            .filter(|g| g.is_god_object)
            .and_then(|g| {
                format_god_object_error_handling_section(
                    g.aggregated_error_swallowing_count,
                    g.aggregated_error_swallowing_patterns.as_ref(),
                )
            }),
    ];

    // Compose sections using functional patterns
    let output: String = sections.into_iter().flatten().collect();

    if output.is_empty() {
        "No pattern data available\n".to_string()
    } else {
        output
    }
}

// =============================================================================
// Data flow page extraction
// =============================================================================

/// Extract data flow page content as plain text (matches TUI exactly)
fn extract_data_flow_text(item: &UnifiedDebtItem, data_flow: &DataFlowGraph) -> String {
    let func_id = FunctionId::new(
        item.location.file.clone(),
        item.location.function.clone(),
        item.location.line,
    );

    let mut output = String::new();

    // Mutation Analysis Section (spec 257: binary signals)
    if let Some(mutation_info) = data_flow.get_mutation_info(&func_id) {
        add_section_header(&mut output, "mutation analysis");
        add_label_value(
            &mut output,
            "has mutations",
            if mutation_info.has_mutations {
                "yes"
            } else {
                "no"
            },
        );
        // Escape analysis removed - not providing actionable signals

        if !mutation_info.detected_mutations.is_empty() {
            add_blank_line(&mut output);
            add_section_header(&mut output, "detected mutations (best-effort)");
            for mutation in &mutation_info.detected_mutations {
                output.push_str(&format!("                            {}\n", mutation));
            }
        }

        add_blank_line(&mut output);
    }

    // I/O Operations Section
    if let Some(io_ops) = data_flow.get_io_operations(&func_id) {
        if !io_ops.is_empty() {
            add_section_header(&mut output, "i/o operations");

            for op in io_ops {
                output.push_str(&format!(
                    "  {} at line {} (variables: {})\n",
                    op.operation_type,
                    op.line,
                    op.variables.join(", ")
                ));
            }

            add_blank_line(&mut output);
        }
    }

    // Escape/taint analysis removed - not providing actionable debt signals

    // If no data available
    if output.is_empty() {
        output.push_str("No data flow analysis data available for this function.\n");
    }

    output
}

// =============================================================================
// Responsibilities page extraction
// =============================================================================

/// Extract responsibilities page content as plain text (matches TUI exactly)
fn extract_responsibilities_text(item: &UnifiedDebtItem) -> String {
    let mut output = String::new();

    // Check for god object responsibilities first
    let mut god_object_shown = false;

    if let Some(indicators) = &item.god_object_indicators {
        // Show responsibilities even if score is below threshold
        if !indicators.responsibilities.is_empty() {
            god_object_shown = true;
            add_section_header(&mut output, "responsibilities");

            for resp in indicators.responsibilities.iter() {
                let method_count = indicators
                    .responsibility_method_counts
                    .get(resp)
                    .copied()
                    .unwrap_or(0);

                let resp_text = resp.to_lowercase();
                let count_text = if method_count > 0 {
                    format!("{} methods", method_count)
                } else {
                    String::new()
                };

                add_label_value(&mut output, &resp_text, &count_text);
            }
        }
    }

    // Fall back to single responsibility category
    if !god_object_shown {
        add_section_header(&mut output, "responsibility");
        let category = item
            .responsibility_category
            .as_deref()
            .unwrap_or("unclassified");
        add_label_value(&mut output, "category", &category.to_lowercase());
    }

    // Add explanatory note for god objects
    if let Some(indicators) = &item.god_object_indicators {
        if indicators.is_god_object {
            add_blank_line(&mut output);
            output.push_str("Note: God objects are structural issues (too many\n");
            output.push_str("responsibilities). Focus on splitting by responsibility.\n");
        }
    }

    output
}

/// Format debt type as human-readable name
fn format_debt_type_name(debt_type: &crate::priority::DebtType) -> String {
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
        _ => "Other".to_string(),
    }
}

/// Open file in editor (suspends TUI during editing)
pub fn open_in_editor(path: &Path, line: Option<usize>) -> Result<()> {
    use crossterm::{
        cursor::MoveTo,
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{
            disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
            LeaveAlternateScreen,
        },
    };
    use std::io;

    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    let mut cmd = Command::new(&editor);

    // Support common editor line number syntax
    match (editor.as_str(), line) {
        ("vim" | "nvim" | "vi", Some(n)) => {
            cmd.arg(format!("+{}", n));
            cmd.arg(path);
        }
        ("code" | "code-insiders", Some(n)) => {
            cmd.arg("--goto");
            cmd.arg(format!("{}:{}", path.display(), n));
        }
        ("emacs", Some(n)) => {
            cmd.arg(format!("+{}", n));
            cmd.arg(path);
        }
        ("subl" | "sublime" | "sublime_text", Some(n)) => {
            cmd.arg(format!("{}:{}", path.display(), n));
        }
        ("hx" | "helix", Some(n)) => {
            cmd.arg(format!("{}:{}", path.display(), n));
        }
        ("nano", Some(n)) => {
            cmd.arg(format!("+{}", n));
            cmd.arg(path);
        }
        _ => {
            // Default: just open the file
            cmd.arg(path);
        }
    }

    // Suspend TUI: disable raw mode, leave alternate screen, disable mouse
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)
        .context("Failed to leave alternate screen")?;

    // Clear the main screen to prevent flash of old terminal content
    execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))
        .context("Failed to clear screen")?;

    // Launch editor and wait for it to complete
    let status = cmd
        .status()
        .with_context(|| format!("Failed to launch editor: {}", editor))?;

    // Resume TUI: re-enter alternate screen, enable mouse, re-enable raw mode
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)
        .context("Failed to re-enter alternate screen")?;
    enable_raw_mode().context("Failed to re-enable raw mode")?;

    // Drain any pending events from the queue to avoid stale input
    use crossterm::event;
    while event::poll(std::time::Duration::from_millis(0))? {
        let _ = event::read()?;
    }

    if !status.success() {
        anyhow::bail!("Editor exited with status: {}", status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_copy_path_succeeds_or_fails_gracefully() {
        let path = PathBuf::from("/tmp/test.rs");
        // This might fail in CI/headless, but should not panic
        let result = copy_path_to_clipboard(&path);
        assert!(result.is_ok()); // Should always return Ok with status message
        let message = result.unwrap();
        assert!(message.contains("Copied") || message.contains("Clipboard"));
    }

    #[test]
    #[ignore] // Requires terminal context (TUI must be active)
    fn test_editor_command_construction() {
        // This test requires a terminal in raw mode, which isn't available during normal test runs
        // Manual testing: run `cargo test test_editor_command_construction -- --ignored --nocapture`
        let path = PathBuf::from("/tmp/test.rs");
        std::env::set_var("EDITOR", "true"); // Use `true` command (always succeeds, does nothing)

        let result = open_in_editor(&path, Some(42));
        assert!(result.is_ok());
    }
}
