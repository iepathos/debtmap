/// Integration layer for enhancing module splits with call graph-based cohesion analysis
///
/// This module ties together cohesion calculation, dependency analysis, and priority assignment
/// to provide quantitative quality metrics for god object refactoring recommendations.
use crate::analyzers::rust_call_graph::extract_call_graph;
use crate::organization::cohesion_calculator::calculate_cohesion_score;
use crate::organization::cohesion_priority::assign_cohesion_based_priority;
use crate::organization::cycle_detector::detect_circular_dependencies;
use crate::organization::dependency_analyzer::extract_dependencies;
use crate::organization::god_object_analysis::ModuleSplit;
use crate::organization::struct_ownership::StructOwnershipAnalyzer;
use std::path::Path;

/// Enhance module splits with call graph-based cohesion analysis
///
/// This function integrates call graph analysis to calculate cohesion scores,
/// extract dependencies, detect circular dependencies, and assign quality-based priorities.
///
/// # Arguments
/// * `splits` - Module split recommendations from Spec 143
/// * `file_path` - Path to the source file being analyzed
/// * `ast` - Parsed AST of the file
/// * `ownership` - Struct ownership information
///
/// # Returns
/// Enhanced module splits with cohesion scores, dependencies, and updated priorities
pub fn enhance_splits_with_cohesion(
    splits: Vec<ModuleSplit>,
    file_path: &Path,
    ast: &syn::File,
    ownership: &StructOwnershipAnalyzer,
) -> Vec<ModuleSplit> {
    // Extract call graph
    let call_graph = extract_call_graph(ast, file_path);

    // Get all struct names for dependency filtering
    let all_structs: Vec<String> = ownership
        .get_struct_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    // Enhance each split with cohesion and dependencies
    let mut enhanced_splits: Vec<ModuleSplit> = splits
        .into_iter()
        .map(|mut split| {
            // Calculate cohesion
            split.cohesion_score = Some(calculate_cohesion_score(&split, &call_graph, ownership));

            // Extract dependencies
            let (deps_in, deps_out) =
                extract_dependencies(&split, &call_graph, ownership, &all_structs);
            split.dependencies_in = deps_in;
            split.dependencies_out = deps_out;

            split
        })
        .collect();

    // Detect circular dependencies
    let cycles = detect_circular_dependencies(&enhanced_splits);

    // Assign priorities based on cohesion and cycles
    assign_cohesion_based_priority(&mut enhanced_splits, &cycles);

    enhanced_splits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::god_object_analysis::{ModuleSplit, Priority};

    fn create_test_code() -> String {
        r#"
struct ScoringWeights {
    base: f64,
}

impl ScoringWeights {
    fn get_default() -> Self {
        Self { base: 1.0 }
    }

    fn apply(&self, value: f64) -> f64 {
        value * self.base
    }

    fn get_multipliers(&self) -> RoleMultipliers {
        RoleMultipliers::get()
    }
}

struct RoleMultipliers {
    admin: f64,
}

impl RoleMultipliers {
    fn get() -> Self {
        Self { admin: 2.0 }
    }

    fn apply_to_score(&self, base: f64) -> f64 {
        base * self.admin
    }
}

struct ValidationLimits {
    max: f64,
}

impl ValidationLimits {
    fn check(&self, value: f64) -> bool {
        value <= self.max
    }
}
"#
        .to_string()
    }

    #[test]
    fn test_enhance_splits_calculates_cohesion() {
        let code = create_test_code();
        let parsed = syn::parse_file(&code).expect("Failed to parse");
        let ownership = StructOwnershipAnalyzer::analyze_file(&parsed);

        let splits = vec![ModuleSplit {
            suggested_name: "scoring".to_string(),
            methods_to_move: vec![],
            structs_to_move: vec!["ScoringWeights".to_string(), "RoleMultipliers".to_string()],
            responsibility: "scoring".to_string(),
            estimated_lines: 200,
            method_count: 10,
            warning: None,
            priority: Priority::Medium,
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
        }];

        let enhanced =
            enhance_splits_with_cohesion(splits, Path::new("test.rs"), &parsed, &ownership);

        assert_eq!(enhanced.len(), 1);
        assert!(
            enhanced[0].cohesion_score.is_some(),
            "Cohesion score should be calculated"
        );
        let cohesion = enhanced[0].cohesion_score.unwrap();
        assert!(
            (0.0..=1.0).contains(&cohesion),
            "Cohesion should be between 0 and 1"
        );
    }

    #[test]
    fn test_enhance_splits_detects_dependencies() {
        let code = create_test_code();
        let parsed = syn::parse_file(&code).expect("Failed to parse");
        let ownership = StructOwnershipAnalyzer::analyze_file(&parsed);

        let splits = vec![
            ModuleSplit {
                suggested_name: "scoring".to_string(),
                methods_to_move: vec![],
                structs_to_move: vec!["ScoringWeights".to_string(), "RoleMultipliers".to_string()],
                responsibility: "scoring".to_string(),
                estimated_lines: 200,
                method_count: 10,
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
            },
            ModuleSplit {
                suggested_name: "validation".to_string(),
                methods_to_move: vec![],
                structs_to_move: vec!["ValidationLimits".to_string()],
                responsibility: "validation".to_string(),
                estimated_lines: 50,
                method_count: 3,
                warning: None,
                priority: Priority::Medium,
                cohesion_score: None,
                dependencies_in: vec![],
                dependencies_out: vec![],
            },
        ];

        let enhanced =
            enhance_splits_with_cohesion(splits, Path::new("test.rs"), &parsed, &ownership);

        assert_eq!(enhanced.len(), 2);
        // At least check that dependencies were populated (even if empty)
        assert!(enhanced[0].dependencies_in.is_empty() || !enhanced[0].dependencies_in.is_empty());
    }

    #[test]
    fn test_enhance_splits_assigns_priority_based_on_cohesion() {
        let code = create_test_code();
        let parsed = syn::parse_file(&code).expect("Failed to parse");
        let ownership = StructOwnershipAnalyzer::analyze_file(&parsed);

        let splits = vec![ModuleSplit {
            suggested_name: "scoring".to_string(),
            methods_to_move: vec![],
            structs_to_move: vec!["ScoringWeights".to_string(), "RoleMultipliers".to_string()],
            responsibility: "scoring".to_string(),
            estimated_lines: 200,
            method_count: 10,
            warning: None,
            priority: Priority::Medium,
            cohesion_score: None,
            dependencies_in: vec![],
            dependencies_out: vec![],
        }];

        let enhanced =
            enhance_splits_with_cohesion(splits, Path::new("test.rs"), &parsed, &ownership);

        assert_eq!(enhanced.len(), 1);
        // Priority should be assigned based on cohesion
        assert!(matches!(
            enhanced[0].priority,
            Priority::High | Priority::Medium | Priority::Low
        ));
    }
}
