/// Size validation and refinement for module splits (Spec 190).
///
/// This module ensures that recommended module splits are appropriately sized:
/// - Enforces minimum size thresholds (10 methods OR 150 lines by default)
/// - Merges undersized splits with semantically similar clusters
/// - Validates cohesion scores to prevent low-quality splits
/// - Ensures balanced distribution across splits
use super::god_object_analysis::{MergeRecord, ModuleSplit, Priority};

/// Configuration for split size validation (Spec 190)
#[derive(Debug, Clone)]
pub struct SplitSizeConfig {
    /// Minimum methods for a standard split
    pub min_methods: usize,
    /// Minimum lines for a standard split
    pub min_lines: usize,
    /// Minimum methods for high-cohesion utility modules
    pub utility_min_methods: usize,
    /// Cohesion threshold for utility module exception
    pub utility_cohesion_threshold: f64,
    /// Maximum size ratio between largest and smallest splits
    pub max_size_ratio: f64,
    /// Minimum cohesion score to accept a split
    pub min_cohesion_score: f64,
    /// Minimum similarity score required to merge clusters
    pub min_merge_similarity: f64,
}

impl Default for SplitSizeConfig {
    fn default() -> Self {
        Self {
            min_methods: 10,
            min_lines: 150,
            utility_min_methods: 5,
            utility_cohesion_threshold: 0.7,
            max_size_ratio: 2.0,
            min_cohesion_score: 0.3,
            min_merge_similarity: 0.4,
        }
    }
}

impl SplitSizeConfig {
    /// Check if a split meets minimum size requirements
    pub fn is_viable_split(&self, split: &ModuleSplit) -> bool {
        // Exception: high-cohesion utility modules
        if is_utility_module(split) {
            let cohesion = split.cohesion_score.unwrap_or(0.0);
            if cohesion > self.utility_cohesion_threshold {
                return split.method_count >= self.utility_min_methods;
            }
        }

        // Standard: must meet either method count OR line count threshold
        split.method_count >= self.min_methods || split.estimated_lines >= self.min_lines
    }

    /// Check if a split meets minimum cohesion requirements
    pub fn has_sufficient_cohesion(&self, split: &ModuleSplit) -> bool {
        split
            .cohesion_score
            .map(|score| score >= self.min_cohesion_score)
            .unwrap_or(true) // If no cohesion score, assume acceptable
    }
}

/// Validate and refine module splits with comprehensive size and quality checks.
///
/// Implements Spec 190 requirements:
/// 1. Filter out undersized splits
/// 2. Merge undersized splits with semantically similar clusters
/// 3. Validate cohesion scores
/// 4. Ensure balanced distribution
pub fn validate_and_refine_splits(splits: Vec<ModuleSplit>) -> Vec<ModuleSplit> {
    validate_and_refine_splits_with_config(splits, &SplitSizeConfig::default())
}

/// Validate splits with custom configuration
pub fn validate_and_refine_splits_with_config(
    splits: Vec<ModuleSplit>,
    config: &SplitSizeConfig,
) -> Vec<ModuleSplit> {
    if splits.is_empty() {
        return splits;
    }

    // Step 1: Calculate cohesion scores for all splits (if not already set)
    let splits_with_cohesion = splits
        .into_iter()
        .map(calculate_split_cohesion)
        .collect::<Vec<_>>();

    // Step 2: Partition into viable and undersized splits
    let (mut viable, undersized): (Vec<_>, Vec<_>) = splits_with_cohesion
        .into_iter()
        .partition(|s| config.is_viable_split(s));

    // Step 3: Merge undersized splits with viable ones
    for undersized_split in undersized {
        if let Some(merge_target_idx) = find_best_merge_target(&undersized_split, &viable, config) {
            // Merge into the target
            viable[merge_target_idx] = merge_splits(
                viable[merge_target_idx].clone(),
                undersized_split,
                config.min_merge_similarity,
            );
        }
        // If no viable merge target, drop the split
    }

    // Step 4: Filter by cohesion
    viable.retain(|s| config.has_sufficient_cohesion(s));

    // Step 5: Ensure balanced distribution
    let balanced = ensure_balanced_distribution(viable, config);

    // Step 6: Set priorities based on final sizes
    balanced
        .into_iter()
        .map(|split| prioritize_by_size(split, config))
        .collect()
}

/// Calculate cohesion score for a split based on method relationships
fn calculate_split_cohesion(mut split: ModuleSplit) -> ModuleSplit {
    // If cohesion already calculated, keep it
    if split.cohesion_score.is_some() {
        return split;
    }

    // Simple heuristic: cohesion based on method naming similarity
    let cohesion = if split.methods_to_move.len() < 2 {
        1.0 // Single method is trivially cohesive
    } else {
        calculate_naming_cohesion(&split.methods_to_move)
    };

    split.cohesion_score = Some(cohesion);
    split
}

/// Calculate cohesion based on method naming patterns
fn calculate_naming_cohesion(methods: &[String]) -> f64 {
    if methods.len() < 2 {
        return 1.0;
    }

    // Extract common prefixes and patterns
    let prefixes: Vec<String> = methods
        .iter()
        .filter_map(|m| extract_method_prefix(m))
        .collect();

    if prefixes.is_empty() {
        return 0.5; // No clear pattern = moderate cohesion
    }

    // Calculate how many methods share common prefixes
    let unique_prefixes: std::collections::HashSet<_> = prefixes.iter().collect();
    let sharing_ratio = 1.0 - (unique_prefixes.len() as f64 / methods.len() as f64);

    // Scale to 0.4-1.0 range (avoid very low scores for naming-based cohesion)
    0.4 + (sharing_ratio * 0.6)
}

/// Extract method prefix (first word before underscore or camelCase boundary)
fn extract_method_prefix(method: &str) -> Option<String> {
    // Handle snake_case
    if method.contains('_') {
        return method.split('_').next().map(|s| s.to_lowercase());
    }

    // Handle camelCase - find first uppercase after start
    for (i, c) in method.char_indices() {
        if i > 0 && c.is_uppercase() {
            return Some(method[..i].to_lowercase());
        }
    }

    // Single word method
    Some(method.to_lowercase())
}

/// Find the best merge target for an undersized split
fn find_best_merge_target(
    undersized: &ModuleSplit,
    viable_splits: &[ModuleSplit],
    config: &SplitSizeConfig,
) -> Option<usize> {
    if viable_splits.is_empty() {
        return None;
    }

    viable_splits
        .iter()
        .enumerate()
        .map(|(idx, split)| {
            let similarity = calculate_semantic_similarity(undersized, split);
            (idx, similarity)
        })
        .filter(|(_, sim)| *sim >= config.min_merge_similarity)
        .max_by(|(_, sim1), (_, sim2)| sim1.partial_cmp(sim2).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(idx, _)| idx)
}

/// Calculate semantic similarity between two splits
fn calculate_semantic_similarity(split1: &ModuleSplit, split2: &ModuleSplit) -> f64 {
    // Weighted combination of different similarity signals
    let naming_sim = method_naming_similarity(split1, split2);
    let responsibility_sim = responsibility_similarity(split1, split2);
    let domain_sim = domain_similarity(split1, split2);

    // Weights: naming 30%, responsibility 40%, domain 30%
    0.3 * naming_sim + 0.4 * responsibility_sim + 0.3 * domain_sim
}

/// Calculate similarity based on method naming patterns
fn method_naming_similarity(split1: &ModuleSplit, split2: &ModuleSplit) -> f64 {
    let prefixes1: Vec<_> = split1
        .methods_to_move
        .iter()
        .filter_map(|m| extract_method_prefix(m))
        .collect();

    let prefixes2: Vec<_> = split2
        .methods_to_move
        .iter()
        .filter_map(|m| extract_method_prefix(m))
        .collect();

    if prefixes1.is_empty() || prefixes2.is_empty() {
        return 0.0;
    }

    // Calculate Jaccard similarity of prefixes
    let set1: std::collections::HashSet<_> = prefixes1.iter().collect();
    let set2: std::collections::HashSet<_> = prefixes2.iter().collect();

    let intersection = set1.intersection(&set2).count();
    let union = set1.union(&set2).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Calculate similarity based on responsibility categories
fn responsibility_similarity(split1: &ModuleSplit, split2: &ModuleSplit) -> f64 {
    let resp1 = split1.responsibility.to_lowercase();
    let resp2 = split2.responsibility.to_lowercase();

    if resp1 == resp2 {
        1.0
    } else if resp1.contains(&resp2) || resp2.contains(&resp1) {
        0.7
    } else {
        0.0
    }
}

/// Calculate similarity based on domain classification
fn domain_similarity(split1: &ModuleSplit, split2: &ModuleSplit) -> f64 {
    if split1.domain.is_empty() || split2.domain.is_empty() {
        return 0.5; // Unknown domains = neutral similarity
    }

    let domain1 = split1.domain.to_lowercase();
    let domain2 = split2.domain.to_lowercase();

    if domain1 == domain2 {
        1.0
    } else if domain1.contains(&domain2) || domain2.contains(&domain1) {
        0.6
    } else {
        0.0
    }
}

/// Merge two splits together
fn merge_splits(
    mut target: ModuleSplit,
    source: ModuleSplit,
    similarity_score: f64,
) -> ModuleSplit {
    // Combine methods
    target
        .methods_to_move
        .extend(source.methods_to_move.clone());

    // Combine structs
    target.structs_to_move.extend(source.structs_to_move);

    // Update counts
    target.method_count += source.method_count;
    target.estimated_lines += source.estimated_lines;

    // Record merge history
    target.merge_history.push(MergeRecord {
        merged_from: source.suggested_name.clone(),
        reason: format!(
            "Merged {} ({} methods) due to size constraint",
            source.suggested_name, source.method_count
        ),
        similarity_score,
    });

    // Update responsibility if source had one
    if !source.responsibility.is_empty() && target.responsibility != source.responsibility {
        target.responsibility = format!("{} & {}", target.responsibility, source.responsibility);
    }

    // Recalculate cohesion for merged split
    target.cohesion_score = None;
    calculate_split_cohesion(target)
}

/// Ensure balanced distribution of split sizes
fn ensure_balanced_distribution(
    mut splits: Vec<ModuleSplit>,
    config: &SplitSizeConfig,
) -> Vec<ModuleSplit> {
    if splits.len() < 2 {
        return splits;
    }

    // Iterate until distribution is balanced
    for _ in 0..10 {
        // Max 10 iterations to prevent infinite loops
        let sizes: Vec<_> = splits.iter().map(|s| s.method_count).collect();
        let max_size = *sizes.iter().max().unwrap_or(&0);
        let min_size = *sizes.iter().min().unwrap_or(&0);

        if min_size == 0 || max_size as f64 / min_size as f64 <= config.max_size_ratio {
            break; // Distribution is balanced
        }

        // Find largest split
        let largest_idx = splits
            .iter()
            .enumerate()
            .max_by_key(|(_, s)| s.method_count)
            .map(|(idx, _)| idx);

        if let Some(idx) = largest_idx {
            // Try to split the largest cluster
            if let Some(sub_splits) = split_into_two(&splits[idx]) {
                splits.remove(idx);
                splits.extend(sub_splits);
            } else {
                break; // Can't split further
            }
        } else {
            break;
        }
    }

    splits
}

/// Split a module into two roughly equal parts
fn split_into_two(split: &ModuleSplit) -> Option<Vec<ModuleSplit>> {
    if split.method_count < 20 {
        return None; // Too small to split
    }

    let mid = split.methods_to_move.len() / 2;
    let (first_half, second_half) = split.methods_to_move.split_at(mid);

    let first_count = split.method_count / 2;
    let second_count = split.method_count - first_count;

    Some(vec![
        ModuleSplit {
            suggested_name: format!("{}_part1", split.suggested_name),
            methods_to_move: first_half.to_vec(),
            structs_to_move: vec![],
            method_count: first_count,
            estimated_lines: split.estimated_lines / 2,
            priority: Priority::Medium,
            warning: Some("Auto-split for balanced distribution".to_string()),
            responsibility: split.responsibility.clone(),
            cohesion_score: None,
            merge_history: vec![],
            ..Default::default()
        },
        ModuleSplit {
            suggested_name: format!("{}_part2", split.suggested_name),
            methods_to_move: second_half.to_vec(),
            structs_to_move: vec![],
            method_count: second_count,
            estimated_lines: split.estimated_lines - (split.estimated_lines / 2),
            priority: Priority::Medium,
            warning: Some("Auto-split for balanced distribution".to_string()),
            responsibility: split.responsibility.clone(),
            cohesion_score: None,
            merge_history: vec![],
            ..Default::default()
        },
    ])
}

/// Check if a split represents a utility module
fn is_utility_module(split: &ModuleSplit) -> bool {
    let responsibility = split.responsibility.to_lowercase();
    let domain = split.domain.to_lowercase();

    responsibility.contains("data structure")
        || responsibility.contains("utilities")
        || responsibility.contains("helper")
        || domain.contains("utilities")
}

/// Set priority based on split size
fn prioritize_by_size(mut split: ModuleSplit, _config: &SplitSizeConfig) -> ModuleSplit {
    let method_count = split.method_count;

    split.priority = if method_count <= 20 {
        Priority::High // Perfect size
    } else if method_count <= 40 {
        if split.warning.is_none() {
            split.warning = Some(format!(
                "{} methods is borderline - consider further splitting",
                method_count
            ));
        }
        Priority::Medium
    } else {
        if split.warning.is_none() {
            split.warning = Some(format!(
                "{} methods is large for a single module",
                method_count
            ));
        }
        Priority::Low
    };

    split
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_split(
        name: &str,
        method_count: usize,
        methods: Vec<&str>,
        responsibility: &str,
    ) -> ModuleSplit {
        ModuleSplit {
            suggested_name: name.to_string(),
            methods_to_move: methods.into_iter().map(|s| s.to_string()).collect(),
            structs_to_move: vec![],
            responsibility: responsibility.to_string(),
            estimated_lines: method_count * 15,
            method_count,
            warning: None,
            priority: Priority::Medium,
            cohesion_score: None,
            merge_history: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn test_reject_undersized_splits() {
        let config = SplitSizeConfig::default();
        let split = make_split("undersized", 3, vec!["m1", "m2", "m3"], "test");

        assert!(!config.is_viable_split(&split));
    }

    #[test]
    fn test_accept_valid_splits() {
        let config = SplitSizeConfig::default();
        let split = make_split(
            "valid",
            15,
            vec![
                "format_a", "format_b", "format_c", "format_d", "format_e", "format_f", "format_g",
                "format_h", "format_i", "format_j", "format_k", "format_l", "format_m", "format_n",
                "format_o",
            ],
            "formatting",
        );

        assert!(config.is_viable_split(&split));
    }

    #[test]
    fn test_utility_module_exception() {
        let config = SplitSizeConfig::default();
        let mut split = make_split(
            "utility",
            5,
            vec!["new", "default", "clone", "eq", "hash"],
            "data structure operations",
        );
        split.cohesion_score = Some(0.85);

        assert!(config.is_viable_split(&split));
    }

    #[test]
    fn test_semantic_similarity_high() {
        let split1 = make_split(
            "format_module",
            10,
            vec![
                "format_item",
                "format_header",
                "format_footer",
                "format_details",
            ],
            "formatting",
        );
        let split2 = make_split(
            "display_module",
            10,
            vec!["format_output", "format_table", "format_row"],
            "formatting",
        );

        let similarity = calculate_semantic_similarity(&split1, &split2);
        assert!(
            similarity > 0.5,
            "Expected high similarity, got {}",
            similarity
        );
    }

    #[test]
    fn test_semantic_similarity_low() {
        let split1 = make_split(
            "format_module",
            10,
            vec!["format_item", "format_header"],
            "formatting",
        );
        let split2 = make_split(
            "validate_module",
            10,
            vec!["validate_input", "check_errors"],
            "validation",
        );

        let similarity = calculate_semantic_similarity(&split1, &split2);
        assert!(
            similarity < 0.5,
            "Expected low similarity, got {}",
            similarity
        );
    }

    #[test]
    fn test_merge_splits() {
        let target = make_split("target", 15, vec!["format_a", "format_b"], "formatting");
        let source = make_split("source", 5, vec!["format_c"], "formatting");

        let merged = merge_splits(target, source, 0.8);

        assert_eq!(merged.method_count, 20);
        assert_eq!(merged.methods_to_move.len(), 3);
        assert_eq!(merged.merge_history.len(), 1);
        assert_eq!(merged.merge_history[0].merged_from, "source");
        assert_eq!(merged.merge_history[0].similarity_score, 0.8);
    }

    #[test]
    fn test_naming_cohesion_high() {
        let methods = vec![
            "format_item".to_string(),
            "format_header".to_string(),
            "format_footer".to_string(),
        ];

        let cohesion = calculate_naming_cohesion(&methods);
        assert!(cohesion > 0.7, "Expected high cohesion, got {}", cohesion);
    }

    #[test]
    fn test_naming_cohesion_low() {
        let methods = vec![
            "format_item".to_string(),
            "validate_input".to_string(),
            "parse_data".to_string(),
        ];

        let cohesion = calculate_naming_cohesion(&methods);
        assert!(cohesion < 0.7, "Expected low cohesion, got {}", cohesion);
    }

    #[test]
    fn test_validate_and_refine_merges_undersized() {
        let splits = vec![
            make_split(
                "large",
                15,
                vec!["format_a", "format_b", "format_c"],
                "formatting",
            ),
            make_split("tiny", 3, vec!["format_d"], "formatting"),
        ];

        let config = SplitSizeConfig::default();
        let refined = validate_and_refine_splits_with_config(splits, &config);

        // Should have 1 split (tiny merged into large)
        assert_eq!(refined.len(), 1);
        assert!(refined[0].method_count >= 15);
        assert!(!refined[0].merge_history.is_empty());
    }

    #[test]
    fn test_balanced_distribution() {
        let config = SplitSizeConfig::default();
        let splits = vec![
            make_split("huge", 80, vec![], "test"),
            make_split("small", 10, vec![], "test"),
        ];

        let balanced = ensure_balanced_distribution(splits, &config);

        // Should split the huge one
        let sizes: Vec<_> = balanced.iter().map(|s| s.method_count).collect();
        let max = *sizes.iter().max().unwrap();
        let min = *sizes.iter().min().unwrap();

        assert!(max as f64 / min as f64 <= config.max_size_ratio * 1.5); // Allow some tolerance
    }

    #[test]
    fn test_extract_method_prefix() {
        assert_eq!(
            extract_method_prefix("format_output"),
            Some("format".to_string())
        );
        assert_eq!(
            extract_method_prefix("validateInput"),
            Some("validate".to_string())
        );
        assert_eq!(extract_method_prefix("simple"), Some("simple".to_string()));
    }

    #[test]
    fn test_is_utility_module() {
        let mut split = make_split("util", 5, vec![], "data structure operations");
        assert!(is_utility_module(&split));

        split.responsibility = "formatting".to_string();
        assert!(!is_utility_module(&split));

        split.responsibility = "helper functions".to_string();
        assert!(is_utility_module(&split));
    }

    #[test]
    fn test_cohesion_validation() {
        let config = SplitSizeConfig::default();
        let mut split = make_split("low_cohesion", 15, vec![], "test");
        split.cohesion_score = Some(0.2);

        assert!(!config.has_sufficient_cohesion(&split));

        split.cohesion_score = Some(0.5);
        assert!(config.has_sufficient_cohesion(&split));
    }
}
