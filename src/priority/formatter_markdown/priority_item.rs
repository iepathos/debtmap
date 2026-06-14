//! Priority item formatting for markdown output
//!
//! Formats individual mixed priority items (function-level and file-level debt)

use crate::formatting::FormattingConfig;
use crate::priority::{DebtItem, FileDebtItem, FileDebtMetrics};
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
    format_file_priority_header(output, rank, item);
    format_god_object_metrics(output, &item.metrics);

    format_split_recommendations_markdown(output, item, verbosity, show_splits);

    writeln!(output, "**Recommendation:** {}", item.recommendation).unwrap();

    writeln!(output, "**Impact:** {}", format_file_impact(&item.impact)).unwrap();

    if verbosity >= 1 {
        format_file_scoring_breakdown(output, &item.metrics);
    }
}

fn format_file_priority_header(output: &mut String, rank: usize, item: &FileDebtItem) {
    writeln!(
        output,
        "### #{} [T1] Score: {:.1} [{}]",
        rank,
        item.score,
        get_severity_label(item.score)
    )
    .unwrap();

    writeln!(
        output,
        "**Type:** {}",
        file_priority_type_label(&item.metrics)
    )
    .unwrap();
    writeln!(
        output,
        "**File:** `{}` ({} lines, {} functions)",
        item.metrics.path.display(),
        item.metrics.total_lines,
        item.metrics.function_count
    )
    .unwrap();
}

fn file_priority_type_label(metrics: &FileDebtMetrics) -> &'static str {
    if is_god_object(metrics) {
        return "FILE - GOD OBJECT";
    }

    if exceeds_contextual_file_threshold(metrics) {
        "FILE - HIGH COMPLEXITY"
    } else {
        "FILE"
    }
}

fn is_god_object(metrics: &FileDebtMetrics) -> bool {
    metrics
        .god_object_analysis
        .as_ref()
        .is_some_and(|analysis| analysis.is_god_object)
}

fn exceeds_contextual_file_threshold(metrics: &FileDebtMetrics) -> bool {
    match &metrics.file_type {
        Some(file_type) => {
            use crate::organization::get_threshold;
            let threshold = get_threshold(file_type, metrics.function_count, metrics.total_lines);
            metrics.total_lines > threshold.base_threshold
        }
        None => metrics.total_lines > 500,
    }
}

fn format_god_object_metrics(output: &mut String, metrics: &FileDebtMetrics) {
    let Some(god_analysis) = metrics.god_object_analysis.as_ref() else {
        return;
    };

    if !god_analysis.is_god_object {
        return;
    }

    writeln!(output, "**God Object Metrics:**").unwrap();
    writeln!(output, "- Methods: {}", god_analysis.method_count).unwrap();
    writeln!(output, "- Fields: {}", god_analysis.field_count).unwrap();
    writeln!(
        output,
        "- Responsibilities: {}",
        god_analysis.responsibility_count
    )
    .unwrap();
    writeln!(
        output,
        "- God Object Score: {:.1}",
        god_analysis.god_object_score
    )
    .unwrap();

    if metrics.coverage_percent > 0.0 {
        writeln!(
            output,
            "- Test Coverage: {:.1}% ({} uncovered lines)",
            metrics.coverage_percent, metrics.uncovered_lines
        )
        .unwrap();
    }
}

fn format_file_scoring_breakdown(output: &mut String, metrics: &FileDebtMetrics) {
    writeln!(output, "\n**Scoring Breakdown:**").unwrap();
    writeln!(
        output,
        "- File size: {}",
        score_category(metrics.total_lines)
    )
    .unwrap();
    writeln!(
        output,
        "- Functions: {}",
        function_category(metrics.function_count)
    )
    .unwrap();
    writeln!(
        output,
        "- Complexity: {}",
        complexity_category(metrics.avg_complexity)
    )
    .unwrap();

    if metrics.function_count > 0 {
        writeln!(
            output,
            "- Dependencies: {} functions may have complex interdependencies",
            metrics.function_count
        )
        .unwrap();
    }
}

/// Write a hint message suggesting --show-splits flag
fn format_splits_hint(output: &mut String) {
    writeln!(output).unwrap();
    writeln!(
        output,
        "*(Use --show-splits for detailed module split recommendations)*"
    )
    .unwrap();
}

/// Write diagnostic message when no splits are available
fn format_no_splits_diagnostic(output: &mut String) {
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

/// Format a method list with sampling (shows max 5 methods)
fn format_split_methods(output: &mut String, methods: &[String]) {
    if methods.is_empty() {
        return;
    }
    let total = methods.len();
    let sample_size = 5.min(total);

    writeln!(output, "  - Methods ({} total):", total).unwrap();
    for method in methods.iter().take(sample_size) {
        writeln!(output, "    - `{}()`", method).unwrap();
    }
    if total > sample_size {
        writeln!(output, "    - ... and {} more", total - sample_size).unwrap();
    }
}

/// Format classification evidence with confidence warnings
fn format_split_evidence(
    output: &mut String,
    evidence: &crate::analysis::multi_signal_aggregation::AggregatedClassification,
) {
    writeln!(
        output,
        "  - Confidence: {:.1}% | Signals: {}",
        evidence.confidence * 100.0,
        evidence.evidence.len()
    )
    .unwrap();

    if evidence.confidence < 0.80 && !evidence.alternatives.is_empty() {
        writeln!(
            output,
            "  - **⚠ Low confidence classification - review recommended**"
        )
        .unwrap();
    }
}

/// Format a single module split recommendation
fn format_single_split(
    output: &mut String,
    split: &crate::organization::god_object::ModuleSplit,
    extension: &str,
    verbosity: u8,
) {
    // Module name and responsibility
    writeln!(output, "- **{}.{}**", split.suggested_name, extension).unwrap();

    let priority_indicator = match split.priority {
        crate::organization::Priority::High => "High",
        crate::organization::Priority::Medium => "Medium",
        crate::organization::Priority::Low => "Low",
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

    // Evidence (conditional on verbosity)
    if verbosity > 0
        && let Some(ref evidence) = split.classification_evidence
    {
        format_split_evidence(output, evidence);
    }

    // Methods list (prefer representative_methods, fallback to methods_to_move)
    let methods = if !split.representative_methods.is_empty() {
        &split.representative_methods
    } else {
        &split.methods_to_move
    };
    format_split_methods(output, methods);

    // Fields needed
    if !split.fields_needed.is_empty() {
        writeln!(
            output,
            "  - Fields needed: {}",
            split.fields_needed.join(", ")
        )
        .unwrap();
    }

    // Trait extraction (conditional on verbosity)
    if let Some(ref trait_suggestion) = split.trait_suggestion
        && verbosity > 0
    {
        writeln!(output, "  - Trait extraction:").unwrap();
        for line in trait_suggestion.lines() {
            writeln!(output, "    {}", line).unwrap();
        }
    }

    // Structs
    if !split.structs_to_move.is_empty() {
        writeln!(output, "  - Structs: {}", split.structs_to_move.join(", ")).unwrap();
    }

    // Warning
    if let Some(warning) = &split.warning {
        writeln!(output, "  - **⚠ {}**", warning).unwrap();
    }

    writeln!(output).unwrap();
}

/// Write a note explaining single cohesive group detection
fn format_single_group_note(output: &mut String) {
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

/// Format detailed splits display with header and all split recommendations
fn format_detailed_splits(
    output: &mut String,
    god_analysis: &crate::organization::GodObjectAnalysis,
    extension: &str,
    verbosity: u8,
) {
    writeln!(output).unwrap();

    // Use different header based on number of splits
    if god_analysis.recommended_splits.len() == 1 {
        writeln!(
            output,
            "**EXTRACTION CANDIDATE** (single cohesive group found):"
        )
        .unwrap();
    } else {
        writeln!(
            output,
            "**RECOMMENDED SPLITS** ({} modules):",
            god_analysis.recommended_splits.len()
        )
        .unwrap();
    }

    writeln!(output).unwrap();

    for split in god_analysis.recommended_splits.iter() {
        format_single_split(output, split, extension, verbosity);
    }

    // Add explanation if only 1 group found
    if god_analysis.recommended_splits.len() == 1 {
        format_single_group_note(output);
    }
}

/// Format split recommendations for markdown output (Spec 208)
pub(crate) fn format_split_recommendations_markdown(
    output: &mut String,
    item: &FileDebtItem,
    verbosity: u8,
    show_splits: bool,
) {
    let god_analysis = match &item.metrics.god_object_analysis {
        Some(analysis) => analysis,
        None => return,
    };

    if god_analysis.recommended_splits.is_empty() {
        if show_splits {
            format_no_splits_diagnostic(output);
        }
        return;
    }

    if !show_splits {
        format_splits_hint(output);
        return;
    }

    let extension = get_file_extension(&item.metrics.path);
    format_detailed_splits(output, god_analysis, extension, verbosity);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::multi_signal_aggregation::{
        AggregatedClassification, ResponsibilityCategory, SignalEvidence, SignalType,
    };
    use crate::organization::Priority;
    use crate::organization::file_classifier::FileType;
    use crate::organization::god_object::ModuleSplit;
    use std::path::PathBuf;

    fn test_file_metrics(total_lines: usize) -> FileDebtMetrics {
        FileDebtMetrics {
            path: PathBuf::from("src/example.rs"),
            total_lines,
            function_count: 12,
            avg_complexity: 4.0,
            coverage_percent: 75.0,
            uncovered_lines: 25,
            ..Default::default()
        }
    }

    fn test_god_analysis(is_god_object: bool) -> crate::organization::GodObjectAnalysis {
        crate::organization::GodObjectAnalysis {
            is_god_object,
            method_count: 40,
            weighted_method_count: None,
            field_count: 8,
            responsibility_count: 3,
            lines_of_code: 500,
            complexity_sum: 100,
            god_object_score: 82.0,
            recommended_splits: Vec::new(),
            confidence: crate::organization::GodObjectConfidence::Definite,
            responsibilities: Vec::new(),
            responsibility_method_counts: Default::default(),
            purity_distribution: None,
            module_structure: None,
            detection_type: crate::organization::DetectionType::GodClass,
            struct_name: None,
            struct_line: None,
            struct_location: None,
            visibility_breakdown: None,
            domain_count: 0,
            domain_diversity: 0.0,
            struct_ratio: 0.0,
            analysis_method: crate::organization::SplitAnalysisMethod::default(),
            cross_domain_severity: None,
            domain_diversity_metrics: None,
            aggregated_entropy: None,
            aggregated_error_swallowing_count: None,
            aggregated_error_swallowing_patterns: None,
            layering_impact: None,
            anti_pattern_report: None,
            complexity_metrics: None,
            trait_method_summary: None,
        }
    }

    #[test]
    fn file_priority_type_label_uses_legacy_threshold_without_file_type() {
        assert_eq!(file_priority_type_label(&test_file_metrics(500)), "FILE");
        assert_eq!(
            file_priority_type_label(&test_file_metrics(501)),
            "FILE - HIGH COMPLEXITY"
        );
    }

    #[test]
    fn file_priority_type_label_uses_contextual_threshold() {
        let mut metrics = test_file_metrics(401);
        metrics.function_count = 100;
        metrics.file_type = Some(FileType::BusinessLogic);

        assert_eq!(file_priority_type_label(&metrics), "FILE - HIGH COMPLEXITY");
    }

    #[test]
    fn file_priority_type_label_prefers_god_object() {
        let mut metrics = test_file_metrics(50);
        metrics.god_object_analysis = Some(test_god_analysis(true));

        assert_eq!(file_priority_type_label(&metrics), "FILE - GOD OBJECT");
    }

    #[test]
    fn format_god_object_metrics_includes_coverage_when_available() {
        let mut output = String::new();
        let mut metrics = test_file_metrics(600);
        metrics.god_object_analysis = Some(test_god_analysis(true));

        format_god_object_metrics(&mut output, &metrics);

        assert!(output.contains("God Object Metrics"));
        assert!(output.contains("- Methods: 40"));
        assert!(output.contains("- Fields: 8"));
        assert!(output.contains("- Responsibilities: 3"));
        assert!(output.contains("- God Object Score: 82.0"));
        assert!(output.contains("- Test Coverage: 75.0% (25 uncovered lines)"));
    }

    #[test]
    fn format_split_methods_shows_sample_when_many() {
        let mut output = String::new();
        let methods: Vec<String> = vec!["a", "b", "c", "d", "e", "f", "g"]
            .into_iter()
            .map(String::from)
            .collect();

        format_split_methods(&mut output, &methods);

        assert!(output.contains("Methods (7 total)"));
        assert!(output.contains("`a()`"));
        assert!(output.contains("`e()`"));
        assert!(output.contains("... and 2 more"));
        assert!(!output.contains("`f()`"));
    }

    #[test]
    fn format_split_methods_empty_produces_no_output() {
        let mut output = String::new();
        format_split_methods(&mut output, &[]);
        assert!(output.is_empty());
    }

    #[test]
    fn format_split_methods_all_shown_when_five_or_less() {
        let mut output = String::new();
        let methods: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();

        format_split_methods(&mut output, &methods);

        assert!(output.contains("Methods (3 total)"));
        assert!(output.contains("`a()`"));
        assert!(output.contains("`b()`"));
        assert!(output.contains("`c()`"));
        assert!(!output.contains("... and"));
    }

    #[test]
    fn format_split_evidence_shows_confidence_and_signals() {
        let mut output = String::new();
        let evidence = AggregatedClassification {
            primary: ResponsibilityCategory::FileIO,
            confidence: 0.92,
            evidence: vec![
                SignalEvidence {
                    signal_type: SignalType::IoDetection,
                    category: ResponsibilityCategory::FileIO,
                    confidence: 0.95,
                    weight: 1.0,
                    contribution: 0.95,
                    description: "File I/O detected".to_string(),
                },
                SignalEvidence {
                    signal_type: SignalType::Name,
                    category: ResponsibilityCategory::FileIO,
                    confidence: 0.80,
                    weight: 0.5,
                    contribution: 0.40,
                    description: "Name pattern match".to_string(),
                },
            ],
            alternatives: vec![],
        };

        format_split_evidence(&mut output, &evidence);

        assert!(output.contains("Confidence: 92.0%"));
        assert!(output.contains("Signals: 2"));
        assert!(!output.contains("Low confidence"));
    }

    #[test]
    fn format_split_evidence_warns_on_low_confidence() {
        let mut output = String::new();
        let evidence = AggregatedClassification {
            primary: ResponsibilityCategory::Unknown,
            confidence: 0.65,
            evidence: vec![SignalEvidence {
                signal_type: SignalType::Name,
                category: ResponsibilityCategory::Unknown,
                confidence: 0.65,
                weight: 0.5,
                contribution: 0.325,
                description: "Weak signal".to_string(),
            }],
            alternatives: vec![(ResponsibilityCategory::Validation, 0.40)],
        };

        format_split_evidence(&mut output, &evidence);

        assert!(output.contains("Confidence: 65.0%"));
        assert!(output.contains("Low confidence"));
    }

    #[test]
    fn format_splits_hint_shows_suggestion() {
        let mut output = String::new();
        format_splits_hint(&mut output);
        assert!(output.contains("--show-splits"));
        assert!(output.contains("detailed module split recommendations"));
    }

    #[test]
    fn format_no_splits_diagnostic_lists_reasons() {
        let mut output = String::new();
        format_no_splits_diagnostic(&mut output);
        assert!(output.contains("NO DETAILED SPLITS AVAILABLE"));
        assert!(output.contains("too few functions"));
        assert!(output.contains("responsibility signals"));
    }

    #[test]
    fn format_single_group_note_explains_tight_coupling() {
        let mut output = String::new();
        format_single_group_note(&mut output);
        assert!(output.contains("one cohesive group"));
        assert!(output.contains("tightly coupled"));
        assert!(output.contains("feature/responsibility"));
    }

    #[test]
    fn format_single_split_shows_basic_info() {
        let mut output = String::new();
        let split = ModuleSplit {
            suggested_name: "io_handler".to_string(),
            responsibility: "I/O Operations".to_string(),
            methods_to_move: vec![
                "read_file".to_string(),
                "write_file".to_string(),
                "open_connection".to_string(),
            ],
            estimated_lines: 150,
            priority: Priority::High,
            ..Default::default()
        };

        format_single_split(&mut output, &split, "rs", 0);

        assert!(output.contains("**io_handler.rs**"));
        assert!(output.contains("Category: I/O Operations"));
        assert!(output.contains("Priority: High"));
        assert!(output.contains("Size: 3 methods, ~150 lines"));
        assert!(output.contains("Methods (3 total)"));
    }

    #[test]
    fn format_single_split_prefers_representative_methods() {
        let mut output = String::new();
        let split = ModuleSplit {
            suggested_name: "parser".to_string(),
            responsibility: "Parsing".to_string(),
            methods_to_move: vec!["internal1".to_string(), "internal2".to_string()],
            estimated_lines: 80,
            priority: Priority::Medium,
            representative_methods: vec!["parse_json".to_string(), "parse_yaml".to_string()],
            ..Default::default()
        };

        format_single_split(&mut output, &split, "rs", 0);

        assert!(output.contains("`parse_json()`"));
        assert!(output.contains("`parse_yaml()`"));
        assert!(!output.contains("`internal1()`"));
    }

    #[test]
    fn format_single_split_shows_warning_when_present() {
        let mut output = String::new();
        let split = ModuleSplit {
            suggested_name: "legacy".to_string(),
            responsibility: "Legacy".to_string(),
            methods_to_move: vec!["old_method".to_string()],
            estimated_lines: 50,
            priority: Priority::Low,
            warning: Some("May require significant refactoring".to_string()),
            ..Default::default()
        };

        format_single_split(&mut output, &split, "rs", 0);

        assert!(output.contains("**⚠ May require significant refactoring**"));
    }
}
