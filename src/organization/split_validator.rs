/// Size validation and refinement for module splits.
///
/// This module ensures that recommended module splits are appropriately sized:
/// - Filters out undersized splits (<5 methods)
/// - Warns about borderline splits (20-40 methods)
/// - Splits oversized groups (>40 methods) into smaller chunks
use super::god_object_analysis::{ModuleSplit, Priority};

/// Validate and refine module splits to ensure proper sizing.
///
/// Filters out splits that are too small (<5 methods) or too large (>40 methods).
/// For oversized splits, uses a simple 2-level strategy to divide them.
///
/// # Arguments
///
/// * `splits` - Vector of module splits to validate
///
/// # Returns
///
/// Filtered and refined vector of module splits with appropriate sizing
pub fn validate_and_refine_splits(splits: Vec<ModuleSplit>) -> Vec<ModuleSplit> {
    splits
        .into_iter()
        .flat_map(|split| {
            let method_count = split.method_count;

            // Too small - skip
            if method_count < 5 {
                return vec![];
            }

            // Perfect size (5-20 methods) - prioritize high
            if method_count <= 20 {
                return vec![ModuleSplit {
                    priority: Priority::High,
                    ..split
                }];
            }

            // Borderline (20-40 methods) - warn but allow
            if method_count <= 40 {
                return vec![ModuleSplit {
                    warning: Some(format!(
                        "{} methods is borderline - consider further splitting",
                        method_count
                    )),
                    priority: Priority::Medium,
                    ..split
                }];
            }

            // Too large (>40 methods) - use simple 2-level split
            split_into_two_levels(split)
        })
        .collect()
}

/// Simple 2-level splitting for oversized modules.
///
/// Divides structs into two roughly equal groups when a module exceeds 40 methods.
///
/// # Arguments
///
/// * `split` - The oversized module split to divide
///
/// # Returns
///
/// Two module splits, each with approximately half the structs
fn split_into_two_levels(split: ModuleSplit) -> Vec<ModuleSplit> {
    if split.structs_to_move.is_empty() {
        // No structs to split - fall back to method-based split
        return split_methods_into_two(split);
    }

    // Divide structs into two roughly equal groups
    let mid = split.structs_to_move.len() / 2;
    let (first_half_structs, second_half_structs) = split.structs_to_move.split_at(mid);

    // For method count, split roughly evenly
    let first_half_methods = split.method_count / 2;
    let second_half_methods = split.method_count - first_half_methods;

    vec![
        ModuleSplit {
            suggested_name: format!("{}_part1", split.suggested_name),
            structs_to_move: first_half_structs.to_vec(),
            method_count: first_half_methods,
            estimated_lines: split.estimated_lines / 2,
            priority: Priority::Medium,
            warning: Some("Auto-split due to size".to_string()),
            responsibility: split.responsibility.clone(),
            methods_to_move: vec![], // Methods grouped by struct
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
            domain: String::new(),
            rationale: None,
            method: crate::organization::SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: split.classification_evidence.clone(),
            representative_methods: vec![],
            fields_needed: vec![],
            trait_suggestion: None,
            behavior_category: None,
        },
        ModuleSplit {
            suggested_name: format!("{}_part2", split.suggested_name),
            structs_to_move: second_half_structs.to_vec(),
            method_count: second_half_methods,
            estimated_lines: split.estimated_lines - (split.estimated_lines / 2),
            priority: Priority::Medium,
            warning: Some("Auto-split due to size".to_string()),
            responsibility: split.responsibility,
            methods_to_move: vec![],
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
            domain: String::new(),
            rationale: None,
            method: crate::organization::SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: split.classification_evidence,
            representative_methods: vec![],
            fields_needed: vec![],
            trait_suggestion: None,
            behavior_category: None,
        },
    ]
}

/// Split methods into two groups when no struct information is available.
fn split_methods_into_two(split: ModuleSplit) -> Vec<ModuleSplit> {
    let mid = split.methods_to_move.len() / 2;
    let (first_half, second_half) = split.methods_to_move.split_at(mid);

    let first_half_count = split.method_count / 2;
    let second_half_count = split.method_count - first_half_count;

    vec![
        ModuleSplit {
            suggested_name: format!("{}_part1", split.suggested_name),
            methods_to_move: first_half.to_vec(),
            structs_to_move: vec![],
            method_count: first_half_count,
            estimated_lines: split.estimated_lines / 2,
            priority: Priority::Medium,
            warning: Some("Auto-split due to size".to_string()),
            responsibility: split.responsibility.clone(),
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
            domain: String::new(),
            rationale: None,
            method: crate::organization::SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: split.classification_evidence.clone(),
            representative_methods: vec![],
            fields_needed: vec![],
            trait_suggestion: None,
            behavior_category: None,
        },
        ModuleSplit {
            suggested_name: format!("{}_part2", split.suggested_name),
            methods_to_move: second_half.to_vec(),
            structs_to_move: vec![],
            method_count: second_half_count,
            estimated_lines: split.estimated_lines - (split.estimated_lines / 2),
            priority: Priority::Medium,
            warning: Some("Auto-split due to size".to_string()),
            responsibility: split.responsibility,
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
            domain: String::new(),
            rationale: None,
            method: crate::organization::SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: split.classification_evidence,
            representative_methods: vec![],
            fields_needed: vec![],
            trait_suggestion: None,
            behavior_category: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_split(
        name: &str,
        method_count: usize,
        structs: Vec<&str>,
        priority: Priority,
    ) -> ModuleSplit {
        ModuleSplit {
            suggested_name: name.to_string(),
            methods_to_move: vec![],
            structs_to_move: structs.into_iter().map(|s| s.to_string()).collect(),
            responsibility: "test".to_string(),
            estimated_lines: method_count * 20,
            method_count,
            warning: None,
            priority,
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
            domain: String::new(),
            rationale: None,
            method: crate::organization::SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: None,
        }
    }

    #[test]
    fn test_reject_undersized_splits() {
        let split = ModuleSplit {
            suggested_name: "undersized".to_string(),
            methods_to_move: vec!["m1".into(), "m2".into()],
            structs_to_move: vec![],
            responsibility: "test".to_string(),
            estimated_lines: 50,
            method_count: 3,
            warning: None,
            priority: Priority::Low,
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
            domain: String::new(),
            rationale: None,
            method: crate::organization::SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: None,
        };

        let validated = validate_and_refine_splits(vec![split]);
        assert_eq!(validated.len(), 0); // Too small, filtered out
    }

    #[test]
    fn test_accept_valid_splits() {
        let split = make_split("valid", 15, vec!["S1", "S2"], Priority::Medium);

        let validated = validate_and_refine_splits(vec![split.clone()]);
        assert_eq!(validated.len(), 1);
        assert_eq!(validated[0].method_count, 15);
        assert_eq!(validated[0].priority, Priority::High); // Upgraded to high
        assert_eq!(validated[0].warning, None);
    }

    #[test]
    fn test_warn_borderline_splits() {
        let split = make_split("borderline", 35, vec!["S1", "S2", "S3"], Priority::High);

        let validated = validate_and_refine_splits(vec![split]);
        assert_eq!(validated.len(), 1);
        assert_eq!(validated[0].priority, Priority::Medium); // Downgraded
        assert!(validated[0].warning.is_some());
        assert!(validated[0]
            .warning
            .as_ref()
            .unwrap()
            .contains("borderline"));
    }

    #[test]
    fn test_split_oversized_modules() {
        let split = make_split(
            "oversized",
            60,
            vec!["S1", "S2", "S3", "S4"],
            Priority::Medium,
        );

        let validated = validate_and_refine_splits(vec![split]);

        // Should be split into 2 parts
        assert_eq!(validated.len(), 2);
        assert!(validated.iter().all(|s| s.method_count <= 40));
        assert_eq!(validated[0].suggested_name, "oversized_part1");
        assert_eq!(validated[1].suggested_name, "oversized_part2");
        assert!(validated[0].warning.is_some());
        assert!(validated[1].warning.is_some());
    }

    #[test]
    fn test_multiple_splits_mixed_sizes() {
        let splits = vec![
            make_split("too_small", 2, vec![], Priority::Low),
            make_split("perfect", 10, vec![], Priority::Medium),
            make_split("too_large", 50, vec!["S1", "S2"], Priority::High),
        ];

        let validated = validate_and_refine_splits(splits);

        // Should have: filtered out too_small, kept perfect, split too_large into 2
        assert_eq!(validated.len(), 3);

        // Perfect should be prioritized high
        let perfect = validated.iter().find(|s| s.suggested_name == "perfect");
        assert!(perfect.is_some());
        assert_eq!(perfect.unwrap().priority, Priority::High);

        // Too large should be split
        let has_part1 = validated
            .iter()
            .any(|s| s.suggested_name == "too_large_part1");
        let has_part2 = validated
            .iter()
            .any(|s| s.suggested_name == "too_large_part2");
        assert!(has_part1);
        assert!(has_part2);
    }

    #[test]
    fn test_edge_case_exactly_5_methods() {
        let split = make_split("minimum", 5, vec![], Priority::Low);

        let validated = validate_and_refine_splits(vec![split]);
        assert_eq!(validated.len(), 1); // Should be kept
        assert_eq!(validated[0].priority, Priority::High); // Prioritized
    }

    #[test]
    fn test_edge_case_exactly_40_methods() {
        let split = make_split("maximum", 40, vec![], Priority::High);

        let validated = validate_and_refine_splits(vec![split]);
        assert_eq!(validated.len(), 1); // Should be kept
        assert_eq!(validated[0].priority, Priority::Medium); // Downgraded
        assert!(validated[0].warning.is_some()); // Warning added
    }
}
