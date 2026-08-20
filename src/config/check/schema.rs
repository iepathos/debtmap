//! Canonical key registry and typo suggestions for configuration diagnostics.

pub(super) fn suggest_key(path: &str) -> Option<String> {
    let (parent, unknown) = path.rsplit_once('.').unwrap_or(("", path));
    let mut ranked: Vec<_> = candidates_for(parent)
        .iter()
        .map(|candidate| (*candidate, edit_distance(unknown, candidate)))
        .collect();
    ranked.sort_by_key(|(candidate, distance)| (*distance, *candidate));

    let (candidate, distance) = ranked.first().copied()?;
    let next_distance = ranked.get(1).map(|(_, distance)| *distance);
    let threshold = if unknown.len() <= 4 { 1 } else { 2 };
    (distance <= threshold && next_distance != Some(distance)).then(|| qualify(parent, candidate))
}

fn qualify(parent: &str, candidate: &str) -> String {
    if parent.is_empty() {
        candidate.to_string()
    } else {
        format!("{parent}.{candidate}")
    }
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut previous: Vec<usize> = (0..=right.chars().count()).collect();
    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_char) in right.chars().enumerate() {
            let substitution = previous[right_index] + usize::from(left_char != right_char);
            current.push(
                (current[right_index] + 1)
                    .min(previous[right_index + 1] + 1)
                    .min(substitution),
            );
        }
        previous = current;
    }
    previous.last().copied().unwrap_or(left.chars().count())
}

fn candidates_for(parent: &str) -> &'static [&'static str] {
    match parent {
        "" => ROOT_KEYS,
        "thresholds" => THRESHOLD_KEYS,
        "thresholds.validation" => VALIDATION_KEYS,
        "scoring" => SCORING_KEYS,
        "display" => DISPLAY_KEYS,
        "ignore" => IGNORE_KEYS,
        "external_api" => EXTERNAL_API_KEYS,
        "god_object_detection" => GOD_OBJECT_KEYS,
        "god_object_detection.rust"
        | "god_object_detection.python"
        | "god_object_detection.javascript" => GOD_OBJECT_THRESHOLD_KEYS,
        "languages" => LANGUAGE_KEYS,
        "languages.rust" | "languages.python" | "languages.javascript" | "languages.typescript" => {
            LANGUAGE_FEATURE_KEYS
        }
        "languages.go" => GO_KEYS,
        "languages.solidity" => SOLIDITY_KEYS,
        "languages.solidity.security" => SOLIDITY_SECURITY_KEYS,
        "output" => OUTPUT_KEYS,
        "analysis" => ANALYSIS_KEYS,
        _ => &[],
    }
}

const ROOT_KEYS: &[&str] = &[
    "analysis",
    "batch_analysis",
    "boilerplate_detection",
    "classification",
    "complexity_thresholds",
    "complexity_weights",
    "context",
    "context_multipliers",
    "context_suggestion",
    "coverage_expectations",
    "data_flow_scoring",
    "display",
    "entropy",
    "error_handling",
    "external_api",
    "functional_analysis",
    "god_object_detection",
    "ignore",
    "languages",
    "loc",
    "mapping_patterns",
    "normalization",
    "orchestration_adjustment",
    "orchestrator_detection",
    "output",
    "retry",
    "role_coverage_weights",
    "role_multiplier_config",
    "role_multipliers",
    "scoring",
    "scoring_rebalanced",
    "state_detection",
    "thresholds",
    "tiers",
];
const THRESHOLD_KEYS: &[&str] = &[
    "complexity",
    "duplication",
    "duplication_similarity",
    "file_size",
    "max_file_length",
    "max_function_length",
    "min_score_threshold",
    "minimum_cognitive_complexity",
    "minimum_cyclomatic_complexity",
    "minimum_debt_score",
    "minimum_risk_score",
    "validation",
];
const VALIDATION_KEYS: &[&str] = &[
    "max_average_complexity",
    "max_codebase_risk_score",
    "max_debt_density",
    "max_debt_items",
    "max_high_complexity_count",
    "max_high_risk_functions",
    "max_total_debt_score",
    "min_coverage_percentage",
];
const SCORING_KEYS: &[&str] = &[
    "complexity",
    "coverage",
    "dependency",
    "organization",
    "security",
    "semantic",
];
const DISPLAY_KEYS: &[&str] = &["items_per_tier", "tiered", "verbosity"];
const IGNORE_KEYS: &[&str] = &["patterns"];
const EXTERNAL_API_KEYS: &[&str] = &["api_files", "api_functions", "detect_external_api"];
const GOD_OBJECT_KEYS: &[&str] = &["enabled", "javascript", "python", "rust"];
const GOD_OBJECT_THRESHOLD_KEYS: &[&str] = &[
    "max_complexity",
    "max_fields",
    "max_lines",
    "max_methods",
    "max_traits",
];
const LANGUAGE_KEYS: &[&str] = &[
    "enabled",
    "go",
    "javascript",
    "python",
    "rust",
    "solidity",
    "typescript",
];
const LANGUAGE_FEATURE_KEYS: &[&str] = &[
    "detect_complexity",
    "detect_dead_code",
    "detect_duplication",
];
pub(super) const GO_KEYS: &[&str] = &[
    "detect_complexity",
    "detect_dead_code",
    "detect_duplication",
    "generated_code",
];
pub(super) const SOLIDITY_KEYS: &[&str] = &[
    "detect_complexity",
    "detect_dead_code",
    "detect_duplication",
    "large_contract_threshold",
    "security",
    "vendor_code",
];
const SOLIDITY_SECURITY_KEYS: &[&str] = &[
    "assembly_blocks",
    "block_timestamp_dependency",
    "delegatecall",
    "delegatecall_in_constructor",
    "encode_packed_collision",
    "floating_pragma",
    "hardcoded_addresses",
    "large_contracts",
    "missing_access_control",
    "push_without_length_cap",
    "reentrancy_heuristic",
    "selfdestruct",
    "tx_gas_price_dependency",
    "tx_origin",
    "unbounded_loops",
    "unchecked_arithmetic",
    "unchecked_calls",
    "unsafe_erc20_transfer",
];
const OUTPUT_KEYS: &[&str] = &[
    "default_format",
    "detail_level",
    "evidence_verbosity",
    "format",
    "min_confidence_warning",
    "signal_filters",
    "use_color",
];
const ANALYSIS_KEYS: &[&str] = &[
    "enable_cross_module_analysis",
    "enable_framework_patterns",
    "enable_function_pointer_tracking",
    "enable_trait_analysis",
    "max_analysis_depth",
];
pub(super) const OUTPUT_FORMAT_VALUES: &[&str] =
    &["html", "json", "markdown", "md", "text", "txt", "yaml"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_handles_insertions_and_substitutions() {
        assert_eq!(edit_distance("complexity", "complexty"), 1);
        assert_eq!(edit_distance("output", "outpit"), 1);
    }
}
