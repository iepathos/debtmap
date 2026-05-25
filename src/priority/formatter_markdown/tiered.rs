//! Tiered display formatting for markdown output
//!
//! Formats technical debt analysis results grouped by severity tier
//! (Critical, High, Moderate, Low)

use crate::priority::{DebtItem, DisplayGroup, FileDebtItem, Tier, UnifiedDebtItem};
use std::fmt::Write;

use super::utilities::{
    extract_complexity_info, format_debt_type, format_file_impact, format_impact,
};

pub(crate) fn format_tier_section(
    output: &mut String,
    groups: &[DisplayGroup],
    tier: Tier,
    verbosity: u8,
) {
    if groups.is_empty() {
        return;
    }

    writeln!(output, "### {}", tier.header()).unwrap();
    writeln!(output, "_Estimated effort: {}_\n", tier.effort_estimate()).unwrap();

    let max_items_per_tier = 5;
    let mut items_shown = 0;

    for group in groups {
        if items_shown >= max_items_per_tier && verbosity < 2 {
            let remaining: usize = groups.iter().skip(items_shown).map(|g| g.items.len()).sum();
            if remaining > 0 {
                writeln!(
                    output,
                    "\n_... and {} more items in this tier_\n",
                    remaining
                )
                .unwrap();
            }
            break;
        }

        format_display_group(output, group, verbosity);
        items_shown += group.items.len();
    }

    writeln!(output).unwrap();
}

pub(crate) fn format_display_group(output: &mut String, group: &DisplayGroup, verbosity: u8) {
    if group.items.len() > 1 && group.batch_action.is_some() {
        // Format as grouped items
        writeln!(
            output,
            "#### {} ({} items)",
            group.debt_type,
            group.items.len()
        )
        .unwrap();

        if let Some(action) = &group.batch_action {
            writeln!(output, "**Batch Action:** {}\n", action).unwrap();
        }

        if verbosity >= 1 {
            writeln!(output, "**Items:**").unwrap();
            for (idx, item) in group.items.iter().take(3).enumerate() {
                format_debt_item_brief(output, idx + 1, item);
            }
            if group.items.len() > 3 {
                writeln!(
                    output,
                    "- _... and {} more similar items_",
                    group.items.len() - 3
                )
                .unwrap();
            }
        } else {
            let total_score: f64 = group.items.iter().map(|i| i.score()).sum();
            writeln!(output, "- Combined Score: {:.1}", total_score).unwrap();
            writeln!(output, "- Count: {} items", group.items.len()).unwrap();
        }
    } else {
        // Format as individual item
        for item in &group.items {
            format_debt_item_detailed(output, item, verbosity);
        }
    }
    writeln!(output).unwrap();
}

pub(crate) fn format_debt_item_brief(output: &mut String, rank: usize, item: &DebtItem) {
    match item {
        DebtItem::Function(func) => {
            writeln!(
                output,
                "- #{} `{}` (Score: {:.1})",
                rank, func.location.function, func.unified_score.final_score
            )
            .unwrap();
        }
        DebtItem::File(file) => {
            writeln!(
                output,
                "- #{} `{}` (Score: {:.1})",
                rank,
                file.metrics.path.display(),
                file.score
            )
            .unwrap();
        }
    }
}

pub(crate) fn format_debt_item_detailed(output: &mut String, item: &DebtItem, verbosity: u8) {
    match item {
        DebtItem::Function(func) => {
            format_function_debt_item(output, func, verbosity);
        }
        DebtItem::File(file) => {
            format_file_debt_item(output, file, verbosity);
        }
    }
}

pub(crate) fn format_function_debt_item(
    output: &mut String,
    item: &UnifiedDebtItem,
    verbosity: u8,
) {
    let score = item.unified_score.final_score;
    writeln!(
        output,
        "#### {} - Score: {:.1}",
        item.location.function, score
    )
    .unwrap();

    writeln!(
        output,
        "**Location:** `{}:{}`",
        item.location.file.display(),
        item.location.line
    )
    .unwrap();

    writeln!(output, "**Type:** {}", format_debt_type(&item.debt_type)).unwrap();

    writeln!(output, "**Action:** {}", item.recommendation.primary_action).unwrap();

    if let Some(complexity) = extract_complexity_info(&item.debt_type) {
        writeln!(output, "**Complexity:** {}", complexity).unwrap();
    }

    if verbosity >= 1 {
        writeln!(
            output,
            "**Impact:** {}",
            format_impact(&item.expected_impact)
        )
        .unwrap();
        writeln!(output, "**Why:** {}", item.recommendation.rationale).unwrap();
    }
}

fn file_display_name(path: &std::path::Path) -> &str {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
}

fn build_file_type_label(item: &FileDebtItem) -> String {
    use crate::organization::get_threshold;
    match &item.metrics.file_type {
        Some(file_type) => {
            let threshold = get_threshold(
                file_type,
                item.metrics.function_count,
                item.metrics.total_lines,
            );
            if item.metrics.total_lines > threshold.base_threshold {
                format!("**Type:** LARGE FILE ({:?})", file_type)
            } else {
                format!("**Type:** COMPLEX FILE ({:?})", file_type)
            }
        }
        None if item.metrics.total_lines > 500 => "**Type:** LARGE FILE".to_string(),
        None => "**Type:** COMPLEX FILE".to_string(),
    }
}

fn write_god_object_section(
    output: &mut String,
    analysis: &crate::organization::GodObjectAnalysis,
) {
    writeln!(output, "**Type:** GOD OBJECT").unwrap();
    writeln!(
        output,
        "**Metrics:** {} methods, {} fields, {} responsibilities",
        analysis.method_count, analysis.field_count, analysis.responsibility_count
    )
    .unwrap();
}

fn write_file_type_section(output: &mut String, item: &FileDebtItem) {
    match item
        .metrics
        .god_object_analysis
        .as_ref()
        .filter(|a| a.is_god_object)
    {
        Some(analysis) => write_god_object_section(output, analysis),
        None => writeln!(output, "{}", build_file_type_label(item)).unwrap(),
    }
}

pub(crate) fn format_file_debt_item(output: &mut String, item: &FileDebtItem, verbosity: u8) {
    writeln!(
        output,
        "#### {} - Score: {:.1}",
        file_display_name(&item.metrics.path),
        item.score
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
    write_file_type_section(output, item);
    writeln!(output, "**Recommendation:** {}", item.recommendation).unwrap();
    if verbosity >= 1 {
        writeln!(output, "**Impact:** {}", format_file_impact(&item.impact)).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::{FileDebtItem, FileDebtMetrics};
    use std::path::PathBuf;

    fn make_god_analysis(
        is_god_object: bool,
        method_count: usize,
        field_count: usize,
        responsibility_count: usize,
    ) -> crate::organization::GodObjectAnalysis {
        crate::organization::GodObjectAnalysis {
            is_god_object,
            method_count,
            weighted_method_count: None,
            field_count,
            responsibility_count,
            lines_of_code: 500,
            complexity_sum: 100,
            god_object_score: 0.8,
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

    fn make_file_item(
        path: &str,
        total_lines: usize,
        function_count: usize,
        god_analysis: Option<crate::organization::GodObjectAnalysis>,
        file_type: Option<crate::organization::FileType>,
    ) -> FileDebtItem {
        FileDebtItem {
            metrics: FileDebtMetrics {
                path: PathBuf::from(path),
                total_lines,
                function_count,
                god_object_analysis: god_analysis,
                file_type,
                ..Default::default()
            },
            score: 42.0,
            priority_rank: 1,
            recommendation: "Refactor this file".to_string(),
            impact: Default::default(),
        }
    }

    #[test]
    fn file_display_name_returns_filename_from_path() {
        let path = std::path::Path::new("/some/deep/path/my_file.rs");
        assert_eq!(file_display_name(path), "my_file.rs");
    }

    #[test]
    fn file_display_name_falls_back_for_empty_path() {
        let path = std::path::Path::new("");
        assert_eq!(file_display_name(path), "unknown");
    }

    #[test]
    fn build_file_type_label_large_file_when_over_500_lines_no_type() {
        let item = make_file_item("big.rs", 600, 20, None, None);
        assert_eq!(build_file_type_label(&item), "**Type:** LARGE FILE");
    }

    #[test]
    fn build_file_type_label_complex_file_when_under_500_lines_no_type() {
        let item = make_file_item("small.rs", 200, 10, None, None);
        assert_eq!(build_file_type_label(&item), "**Type:** COMPLEX FILE");
    }

    #[test]
    fn format_file_debt_item_includes_heading_and_score() {
        let item = make_file_item("src/foo.rs", 300, 15, None, None);
        let mut output = String::new();
        format_file_debt_item(&mut output, &item, 0);
        assert!(output.contains("#### foo.rs - Score: 42.0"));
    }

    #[test]
    fn format_file_debt_item_includes_file_metadata() {
        let item = make_file_item("src/foo.rs", 300, 15, None, None);
        let mut output = String::new();
        format_file_debt_item(&mut output, &item, 0);
        assert!(output.contains("300 lines"));
        assert!(output.contains("15 functions"));
    }

    #[test]
    fn format_file_debt_item_god_object_shows_metrics() {
        let analysis = make_god_analysis(true, 45, 12, 7);
        let item = make_file_item("src/god.rs", 1000, 45, Some(analysis), None);
        let mut output = String::new();
        format_file_debt_item(&mut output, &item, 0);
        assert!(output.contains("GOD OBJECT"));
        assert!(output.contains("45 methods"));
        assert!(output.contains("12 fields"));
        assert!(output.contains("7 responsibilities"));
    }

    #[test]
    fn format_file_debt_item_non_god_object_shows_complex_file() {
        let analysis = make_god_analysis(false, 5, 2, 1);
        let item = make_file_item("src/small.rs", 150, 5, Some(analysis), None);
        let mut output = String::new();
        format_file_debt_item(&mut output, &item, 0);
        assert!(output.contains("COMPLEX FILE"));
        assert!(!output.contains("GOD OBJECT"));
    }

    #[test]
    fn format_file_debt_item_impact_hidden_at_verbosity_zero() {
        let item = make_file_item("src/foo.rs", 200, 10, None, None);
        let mut output = String::new();
        format_file_debt_item(&mut output, &item, 0);
        assert!(!output.contains("**Impact:**"));
    }

    #[test]
    fn format_file_debt_item_impact_shown_at_verbosity_one() {
        let item = make_file_item("src/foo.rs", 200, 10, None, None);
        let mut output = String::new();
        format_file_debt_item(&mut output, &item, 1);
        assert!(output.contains("**Impact:**"));
    }

    #[test]
    fn format_file_debt_item_recommendation_always_shown() {
        let item = make_file_item("src/foo.rs", 200, 10, None, None);
        let mut output = String::new();
        format_file_debt_item(&mut output, &item, 0);
        assert!(output.contains("**Recommendation:** Refactor this file"));
    }
}
