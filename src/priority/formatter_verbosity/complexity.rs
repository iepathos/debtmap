use crate::priority::UnifiedDebtItem;
use colored::*;
use std::fmt::Write;

/// Format complexity summary line
pub fn format_complexity_summary(
    output: &mut String,
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
) {
    let (cyclomatic, cognitive, branch_count, nesting, _length) =
        crate::priority::formatter::extract_complexity_info(item);

    if cyclomatic > 0 || cognitive > 0 {
        if let Some(ref entropy) = item.entropy_details {
            writeln!(
                output,
                "├─ {} cyclomatic={} (dampened: {}, factor: {:.2}), est_branches={}, cognitive={}, nesting={}, entropy={:.2}",
                "COMPLEXITY:".bright_blue(),
                cyclomatic.to_string().yellow(),
                entropy.adjusted_complexity.to_string().yellow(),
                entropy.dampening_factor,
                branch_count.to_string().yellow(),
                cognitive.to_string().yellow(),
                nesting.to_string().yellow(),
                entropy.entropy_score
            )
            .unwrap();
        } else {
            writeln!(
                output,
                "├─ {} cyclomatic={}, est_branches={}, cognitive={}, nesting={}",
                "COMPLEXITY:".bright_blue(),
                cyclomatic.to_string().yellow(),
                branch_count.to_string().yellow(),
                cognitive.to_string().yellow(),
                nesting.to_string().yellow()
            )
            .unwrap();
        }
    }
}

/// Format pattern detection for state machine and coordinator patterns (spec 204)
/// Reads from item.detected_pattern (single source of truth) instead of re-detecting
pub fn format_pattern_detection(output: &mut String, item: &UnifiedDebtItem) {
    // Read stored pattern result instead of re-detecting
    if let Some(ref pattern) = item.detected_pattern {
        let metrics_str = pattern.display_metrics().join(", ");

        writeln!(
            output,
            "├─ {} {} {} ({}, confidence: {:.2})",
            "PATTERN:".bright_blue(),
            pattern.icon(),
            pattern.type_name().bright_magenta().bold(),
            metrics_str.cyan(),
            pattern.confidence
        )
        .unwrap();
    }
}

/// Pure function to classify complexity contribution
pub fn classify_complexity_contribution(complexity_factor: f64) -> &'static str {
    match complexity_factor {
        c if c > 10.0 => "VERY HIGH",
        c if c > 5.0 => "HIGH",
        c if c > 3.0 => "MEDIUM",
        _ => "LOW",
    }
}

/// Format complexity details section
pub fn format_complexity_details_section(
    item: &UnifiedDebtItem,
    _formatter: &crate::formatting::ColoredFormatter,
) -> Vec<String> {
    let mut lines = Vec::new();
    let tree_pipe = " ";

    lines.push(format!("{} {}", "-", "COMPLEXITY DETAILS:".bright_blue()));

    // Format cyclomatic complexity with entropy dampening if available
    if let Some(ref entropy) = item.entropy_details {
        lines.push(format!(
            "{}  {} cyclomatic={} (dampened: {}, factor: {:.2})",
            tree_pipe,
            "-",
            item.cyclomatic_complexity,
            entropy.adjusted_complexity,
            entropy.dampening_factor
        ));
    } else {
        lines.push(format!(
            "{}  {} cyclomatic={}",
            tree_pipe, "-", item.cyclomatic_complexity
        ));
    }

    lines.push(format!(
        "{}  {} cognitive={}",
        tree_pipe, "-", item.cognitive_complexity
    ));

    lines.push(format!(
        "{}  {} Function Length: {} lines",
        tree_pipe, "-", item.function_length
    ));

    lines.push(format!(
        "{}  {} Nesting Depth: {}",
        tree_pipe, "-", item.nesting_depth
    ));

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_complexity_contribution() {
        assert_eq!(classify_complexity_contribution(15.0), "VERY HIGH");
        assert_eq!(classify_complexity_contribution(10.1), "VERY HIGH");
        assert_eq!(classify_complexity_contribution(10.0), "HIGH");
        assert_eq!(classify_complexity_contribution(7.0), "HIGH");
        assert_eq!(classify_complexity_contribution(5.1), "HIGH");
        assert_eq!(classify_complexity_contribution(5.0), "MEDIUM");
        assert_eq!(classify_complexity_contribution(4.0), "MEDIUM");
        assert_eq!(classify_complexity_contribution(3.1), "MEDIUM");
        assert_eq!(classify_complexity_contribution(3.0), "LOW");
        assert_eq!(classify_complexity_contribution(1.0), "LOW");
    }
}
