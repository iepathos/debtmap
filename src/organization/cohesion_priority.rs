/// Priority assignment based on cohesion scores and dependency quality
///
/// This module assigns priorities to module split recommendations based on
/// their cohesion scores and whether they're involved in circular dependencies.
use crate::organization::god_object_analysis::{ModuleSplit, Priority};
use std::collections::HashSet;

/// Assign priorities based on cohesion score and dependency quality
///
/// # Priority Rules
/// - High cohesion (>0.7) + no cycles → High priority
/// - High cohesion (>0.7) + in cycle → Medium priority (downgraded)
/// - Medium cohesion (0.5-0.7) → Medium priority
/// - Medium cohesion + in cycle → Low priority (downgraded)
/// - Low cohesion (<0.5) → Low priority
pub fn assign_cohesion_based_priority(splits: &mut [ModuleSplit], cycles: &[Vec<String>]) {
    // Build set of modules involved in cycles
    let modules_in_cycles: HashSet<String> = cycles
        .iter()
        .flat_map(|cycle| cycle.iter().cloned())
        .collect();

    for split in splits.iter_mut() {
        let cohesion = split.cohesion_score.unwrap_or(0.5);
        let in_cycle = modules_in_cycles.contains(&split.suggested_name);

        // Determine priority based on cohesion and cycles
        split.priority = match (cohesion, in_cycle) {
            (c, true) if c > 0.7 => {
                // High cohesion but in cycle - downgrade to medium
                split.warning = Some(format!(
                    "High cohesion ({:.2}) but involved in circular dependency",
                    c
                ));
                Priority::Medium
            }
            (c, false) if c > 0.7 => {
                // High cohesion, no cycles - excellent candidate
                Priority::High
            }
            (c, true) if c > 0.5 => {
                // Medium cohesion + cycle - downgrade to low
                split.warning = Some(format!(
                    "Moderate cohesion ({:.2}) and circular dependency",
                    c
                ));
                Priority::Low
            }
            (c, false) if c > 0.5 => {
                // Medium cohesion
                Priority::Medium
            }
            (c, _) => {
                // Low cohesion - questionable recommendation
                let warning_msg = if in_cycle {
                    format!(
                        "Low cohesion ({:.2}) and circular dependency - may not improve organization",
                        c
                    )
                } else {
                    format!("Low cohesion ({:.2}) - may not improve organization", c)
                };
                split.warning = Some(warning_msg);
                Priority::Low
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::god_object_analysis::ModuleSplit;

    fn create_test_split(name: &str, cohesion: f64) -> ModuleSplit {
        ModuleSplit {
            suggested_name: name.to_string(),
            methods_to_move: vec![],
            structs_to_move: vec![],
            responsibility: "test".to_string(),
            estimated_lines: 100,
            method_count: 5,
            warning: None,
            priority: Priority::Medium,
            cohesion_score: Some(cohesion),
            dependencies_in: vec![],
            dependencies_out: vec![],
            domain: String::new(),
            rationale: None,
            method: crate::organization::SplitAnalysisMethod::None,
            severity: None,
            interface_estimate: None,
            classification_evidence: None,
            representative_methods: vec![],
            fields_needed: vec![],
            trait_suggestion: None,
            behavior_category: None,
            ..Default::default()
        }
    }

    #[test]
    fn test_high_cohesion_no_cycle() {
        let mut splits = vec![create_test_split("ModuleA", 0.85)];
        assign_cohesion_based_priority(&mut splits, &[]);

        assert_eq!(splits[0].priority, Priority::High);
        assert!(splits[0].warning.is_none());
    }

    #[test]
    fn test_high_cohesion_with_cycle() {
        let mut splits = vec![create_test_split("ModuleA", 0.85)];
        let cycles = vec![vec!["ModuleA".to_string(), "ModuleB".to_string()]];
        assign_cohesion_based_priority(&mut splits, &cycles);

        assert_eq!(splits[0].priority, Priority::Medium);
        assert!(splits[0].warning.is_some());
        assert!(splits[0].warning.as_ref().unwrap().contains("circular"));
    }

    #[test]
    fn test_medium_cohesion_no_cycle() {
        let mut splits = vec![create_test_split("ModuleA", 0.6)];
        assign_cohesion_based_priority(&mut splits, &[]);

        assert_eq!(splits[0].priority, Priority::Medium);
    }

    #[test]
    fn test_medium_cohesion_with_cycle() {
        let mut splits = vec![create_test_split("ModuleA", 0.6)];
        let cycles = vec![vec!["ModuleA".to_string()]];
        assign_cohesion_based_priority(&mut splits, &cycles);

        assert_eq!(splits[0].priority, Priority::Low);
        assert!(splits[0].warning.is_some());
    }

    #[test]
    fn test_low_cohesion() {
        let mut splits = vec![create_test_split("ModuleA", 0.3)];
        assign_cohesion_based_priority(&mut splits, &[]);

        assert_eq!(splits[0].priority, Priority::Low);
        assert!(splits[0].warning.is_some());
        assert!(splits[0].warning.as_ref().unwrap().contains("Low cohesion"));
    }

    #[test]
    fn test_low_cohesion_with_cycle() {
        let mut splits = vec![create_test_split("ModuleA", 0.3)];
        let cycles = vec![vec!["ModuleA".to_string()]];
        assign_cohesion_based_priority(&mut splits, &cycles);

        assert_eq!(splits[0].priority, Priority::Low);
        assert!(splits[0].warning.is_some());
        assert!(splits[0]
            .warning
            .as_ref()
            .unwrap()
            .contains("circular dependency"));
    }

    #[test]
    fn test_multiple_splits_mixed_priorities() {
        let mut splits = vec![
            create_test_split("HighCohesion", 0.9),
            create_test_split("MediumCohesion", 0.6),
            create_test_split("LowCohesion", 0.2),
        ];
        assign_cohesion_based_priority(&mut splits, &[]);

        assert_eq!(splits[0].priority, Priority::High);
        assert_eq!(splits[1].priority, Priority::Medium);
        assert_eq!(splits[2].priority, Priority::Low);
    }
}
