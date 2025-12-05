use crate::core::{AnalysisResults, DebtItem};
use crate::priority::UnifiedDebtItem;
use std::collections::HashMap;

/// Categorize debt items by type
pub fn categorize_debt(items: &[DebtItem]) -> HashMap<&'static str, Vec<&DebtItem>> {
    let mut categories: HashMap<&'static str, Vec<&DebtItem>> = HashMap::new();

    for item in items {
        let category = match item.debt_type {
            crate::core::DebtType::Complexity | crate::core::DebtType::TestComplexity => {
                "Complexity Issues"
            }
            crate::core::DebtType::Todo
            | crate::core::DebtType::Fixme
            | crate::core::DebtType::TestTodo => "TODOs and FIXMEs",
            crate::core::DebtType::Duplication | crate::core::DebtType::TestDuplication => {
                "Duplication"
            }
            crate::core::DebtType::CodeSmell | crate::core::DebtType::CodeOrganization => {
                "Code Quality"
            }
            crate::core::DebtType::Dependency => "Dependencies",
            crate::core::DebtType::ErrorSwallowing | crate::core::DebtType::ResourceManagement => {
                "Error Handling"
            }
            crate::core::DebtType::TestQuality => "Test Quality",
        };

        categories.entry(category).or_default().push(item);
    }

    categories
}

/// Calculate effort based on cyclomatic complexity
fn effort_from_complexity(cyclomatic: &u32) -> u32 {
    match cyclomatic {
        0..=5 => 1,
        6..=10 => 2,
        11..=20 => 4,
        _ => 8,
    }
}

/// Calculate effort based on test coverage
fn effort_from_coverage(coverage: &f64) -> u32 {
    match coverage {
        x if x > &0.8 => 1,
        x if x > &0.5 && x <= &0.8 => 2,
        x if x > &0.2 && x <= &0.5 => 4,
        _ => 8,
    }
}

/// Calculate effort based on risk score
fn effort_from_risk(risk_score: &f64) -> u32 {
    if risk_score > &8.0 {
        8
    } else if risk_score > &5.0 {
        4
    } else {
        2
    }
}

/// Calculate effort based on duplication instances
fn effort_from_duplication(instances: &u32) -> u32 {
    if instances > &5 {
        8
    } else {
        4
    }
}

/// Calculate effort based on nested loop depth
fn effort_from_loop_depth(depth: &u32) -> u32 {
    if depth > &3 {
        8
    } else {
        4
    }
}

/// Calculate base effort for a debt type
fn calculate_base_effort(debt_type: &crate::priority::DebtType) -> u32 {
    match debt_type {
        crate::priority::DebtType::ComplexityHotspot { cyclomatic, .. } => {
            effort_from_complexity(cyclomatic)
        }
        crate::priority::DebtType::TestingGap { coverage, .. } => effort_from_coverage(coverage),
        crate::priority::DebtType::Risk { risk_score, .. } => effort_from_risk(risk_score),
        crate::priority::DebtType::DeadCode { .. } => 2,
        crate::priority::DebtType::Duplication { instances, .. } => {
            effort_from_duplication(instances)
        }
        crate::priority::DebtType::TestComplexityHotspot { .. } => 4,
        crate::priority::DebtType::TestTodo { .. } => 2,
        crate::priority::DebtType::TestDuplication { .. } => 3,
        crate::priority::DebtType::ErrorSwallowing { .. } => 3,
        crate::priority::DebtType::AllocationInefficiency { .. } => 4,
        crate::priority::DebtType::StringConcatenation { .. } => 3,
        crate::priority::DebtType::NestedLoops { depth, .. } => effort_from_loop_depth(depth),
        crate::priority::DebtType::BlockingIO { .. } => 5,
        crate::priority::DebtType::SuboptimalDataStructure { .. } => 6,
        crate::priority::DebtType::GodObject { .. } => 16,
        crate::priority::DebtType::GodModule { .. } => 16,
        crate::priority::DebtType::FeatureEnvy { .. } => 8,
        crate::priority::DebtType::PrimitiveObsession { .. } => 4,
        crate::priority::DebtType::MagicValues { .. } => 2,
        crate::priority::DebtType::AssertionComplexity { .. } => 4,
        crate::priority::DebtType::FlakyTestPattern { .. } => 6,
        crate::priority::DebtType::AsyncMisuse { .. } => 8,
        crate::priority::DebtType::ResourceLeak { .. } => 10,
        crate::priority::DebtType::CollectionInefficiency { .. } => 4,
        // Type organization debt types (Spec 187)
        crate::priority::DebtType::ScatteredType { file_count, .. } => {
            // Effort scales with number of files to consolidate
            (file_count * 2).min(10) as u32
        }
        crate::priority::DebtType::OrphanedFunctions { function_count, .. } => {
            // Simple refactoring, scales with function count
            (function_count / 4).clamp(1, 4) as u32
        }
        crate::priority::DebtType::UtilitiesSprawl { distinct_types, .. } => {
            // Breaking up utilities is moderate effort
            (distinct_types / 2).clamp(4, 12) as u32
        }
    }
}

/// Estimate effort required to fix a debt item
pub fn estimate_effort(item: &UnifiedDebtItem) -> u32 {
    let base_effort = calculate_base_effort(&item.debt_type);
    base_effort * 2 // Account for testing and review
}

/// Extract module dependencies from analysis (simplified)
pub fn extract_module_dependencies(items: &[UnifiedDebtItem]) -> HashMap<String, Vec<String>> {
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();

    for item in items {
        let module = item
            .location
            .file
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        for _callee in &item.downstream_callees {
            // Simplified: assume callees are in different modules
            let target_module = "dependency".to_string();
            if module != target_module {
                deps.entry(module.clone()).or_default().push(target_module);
            }
        }
    }

    // Deduplicate
    for dependencies in deps.values_mut() {
        dependencies.sort();
        dependencies.dedup();
    }

    deps
}

/// Get top complex functions from analysis results
pub fn get_top_complex_functions(results: &AnalysisResults, limit: usize) -> Vec<String> {
    let mut functions: Vec<_> = results
        .complexity
        .metrics
        .iter()
        .map(|m| (m.name.clone(), m.cyclomatic))
        .collect();

    functions.sort_by(|a, b| b.1.cmp(&a.1));
    functions.truncate(limit);

    functions
        .into_iter()
        .map(|(name, complexity)| format!("{} (complexity: {})", name, complexity))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        ComplexityReport as ComplexityResults, DebtItem, DebtType, DependencyReport,
        FunctionMetrics, Priority, TechnicalDebtReport,
    };
    use crate::priority::{
        ActionableRecommendation, DebtType as PriorityDebtType, FunctionRole, FunctionVisibility,
        ImpactMetrics, Location as PriorityLocation, TransitiveCoverage, UnifiedDebtItem,
        UnifiedScore,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_effort_from_complexity() {
        assert_eq!(effort_from_complexity(&3), 1);
        assert_eq!(effort_from_complexity(&5), 1);
        assert_eq!(effort_from_complexity(&6), 2);
        assert_eq!(effort_from_complexity(&10), 2);
        assert_eq!(effort_from_complexity(&11), 4);
        assert_eq!(effort_from_complexity(&20), 4);
        assert_eq!(effort_from_complexity(&21), 8);
        assert_eq!(effort_from_complexity(&100), 8);
    }

    #[test]
    fn test_effort_from_coverage() {
        // High coverage (>0.8) = low effort (1)
        assert_eq!(effort_from_coverage(&0.9), 1);
        assert_eq!(effort_from_coverage(&0.81), 1);
        // Medium-high coverage (>0.5, <=0.8) = medium-low effort (2)
        assert_eq!(effort_from_coverage(&0.8), 2);
        assert_eq!(effort_from_coverage(&0.6), 2);
        assert_eq!(effort_from_coverage(&0.51), 2);
        // Low coverage (>0.2, <=0.5) = medium-high effort (4)
        assert_eq!(effort_from_coverage(&0.5), 4);
        assert_eq!(effort_from_coverage(&0.3), 4);
        assert_eq!(effort_from_coverage(&0.21), 4);
        // Very low coverage (<=0.2) = high effort (8)
        assert_eq!(effort_from_coverage(&0.2), 8);
        assert_eq!(effort_from_coverage(&0.1), 8);
        assert_eq!(effort_from_coverage(&0.0), 8);
    }

    #[test]
    fn test_effort_from_risk() {
        assert_eq!(effort_from_risk(&10.0), 8);
        assert_eq!(effort_from_risk(&8.1), 8);
        assert_eq!(effort_from_risk(&8.0), 4);
        assert_eq!(effort_from_risk(&6.0), 4);
        assert_eq!(effort_from_risk(&5.1), 4);
        assert_eq!(effort_from_risk(&5.0), 2);
        assert_eq!(effort_from_risk(&3.0), 2);
        assert_eq!(effort_from_risk(&1.0), 2);
    }

    #[test]
    fn test_effort_from_duplication() {
        assert_eq!(effort_from_duplication(&10), 8);
        assert_eq!(effort_from_duplication(&6), 8);
        assert_eq!(effort_from_duplication(&5), 4);
        assert_eq!(effort_from_duplication(&3), 4);
        assert_eq!(effort_from_duplication(&1), 4);
    }

    #[test]
    fn test_effort_from_loop_depth() {
        assert_eq!(effort_from_loop_depth(&5), 8);
        assert_eq!(effort_from_loop_depth(&4), 8);
        assert_eq!(effort_from_loop_depth(&3), 4);
        assert_eq!(effort_from_loop_depth(&2), 4);
        assert_eq!(effort_from_loop_depth(&1), 4);
    }

    #[test]
    fn test_calculate_base_effort_complexity_hotspot() {
        let debt_type = PriorityDebtType::ComplexityHotspot {
            cyclomatic: 15,
            cognitive: 20,
            adjusted_cyclomatic: None,
        };
        assert_eq!(calculate_base_effort(&debt_type), 4);
    }

    #[test]
    fn test_calculate_base_effort_testing_gap() {
        let debt_type = PriorityDebtType::TestingGap {
            coverage: 0.7,
            cyclomatic: 10,
            cognitive: 15,
        };
        assert_eq!(calculate_base_effort(&debt_type), 2);
    }

    #[test]
    fn test_calculate_base_effort_risk() {
        let debt_type = PriorityDebtType::Risk {
            risk_score: 7.5,
            factors: vec!["High complexity".to_string(), "No coverage".to_string()],
        };
        assert_eq!(calculate_base_effort(&debt_type), 4);
    }

    #[test]
    fn test_calculate_base_effort_dead_code() {
        let debt_type = PriorityDebtType::DeadCode {
            visibility: FunctionVisibility::Private,
            cyclomatic: 5,
            cognitive: 10,
            usage_hints: vec![],
        };
        assert_eq!(calculate_base_effort(&debt_type), 2);
    }

    #[test]
    fn test_calculate_base_effort_duplication() {
        let debt_type = PriorityDebtType::Duplication {
            instances: 7,
            total_lines: 100,
        };
        assert_eq!(calculate_base_effort(&debt_type), 8);
    }

    #[test]
    fn test_calculate_base_effort_nested_loops() {
        let debt_type = PriorityDebtType::NestedLoops {
            depth: 4,
            complexity_estimate: "O(n^4)".to_string(),
        };
        assert_eq!(calculate_base_effort(&debt_type), 8);
    }

    #[test]
    fn test_calculate_base_effort_fixed_values() {
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::TestComplexityHotspot {
                cyclomatic: 10,
                cognitive: 15,
                threshold: 10
            }),
            4
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::TestTodo {
                priority: crate::core::Priority::High,
                reason: Some("Test todo".to_string())
            }),
            2
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::TestDuplication {
                instances: 3,
                total_lines: 50,
                similarity: 0.9
            }),
            3
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::ErrorSwallowing {
                pattern: "catch-all".to_string(),
                context: Some("handler".to_string())
            }),
            3
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::AllocationInefficiency {
                pattern: "vec-in-loop".to_string(),
                impact: "High".to_string()
            }),
            4
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::StringConcatenation {
                loop_type: "for".to_string(),
                iterations: Some(15)
            }),
            3
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::BlockingIO {
                operation: "file_read".to_string(),
                context: "loop".to_string()
            }),
            5
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::SuboptimalDataStructure {
                current_type: "Vec".to_string(),
                recommended_type: "HashSet".to_string()
            }),
            6
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::GodObject {
                methods: 50,
                fields: 25,
                responsibilities: 50,
                god_object_score: 100.0
            }),
            16
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::FeatureEnvy {
                external_class: "OtherClass".to_string(),
                usage_ratio: 0.8
            }),
            8
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::PrimitiveObsession {
                primitive_type: "i32".to_string(),
                domain_concept: "UserId".to_string()
            }),
            4
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::MagicValues {
                value: "42".to_string(),
                occurrences: 3
            }),
            2
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::AssertionComplexity {
                assertion_count: 5,
                complexity_score: 3.5
            }),
            4
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::FlakyTestPattern {
                pattern_type: "sleep".to_string(),
                reliability_impact: "High".to_string()
            }),
            6
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::AsyncMisuse {
                pattern: "blocking-in-async".to_string(),
                performance_impact: "High".to_string()
            }),
            8
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::ResourceLeak {
                resource_type: "File".to_string(),
                cleanup_missing: "close".to_string()
            }),
            10
        );
        assert_eq!(
            calculate_base_effort(&PriorityDebtType::CollectionInefficiency {
                collection_type: "Vec".to_string(),
                inefficiency_type: "linear_search".to_string()
            }),
            4
        );
    }

    #[test]
    fn test_estimate_effort() {
        let item = UnifiedDebtItem {
            location: PriorityLocation {
                file: PathBuf::from("test.rs"),
                function: "test_func".to_string(),
                line: 10,
            },
            debt_type: PriorityDebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
                adjusted_cyclomatic: None,
            },
            unified_score: UnifiedScore {
                complexity_factor: 5.0,
                coverage_factor: 3.0,
                dependency_factor: 2.0,
                role_multiplier: 1.0,
                final_score: 10.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor".to_string(),
                rationale: "High complexity".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 10.0,
                lines_reduction: 20,
                complexity_reduction: 5.0,
                risk_reduction: 3.0,
            },
            transitive_coverage: Some(TransitiveCoverage {
                direct: 0.5,
                transitive: 0.6,
                propagated_from: vec![],
                uncovered_lines: vec![],
            }),
            file_context: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 2,
            function_length: 50,
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
            entropy_details: None,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None,
        };

        assert_eq!(estimate_effort(&item), 8); // 4 * 2
    }

    #[test]
    fn test_categorize_debt() {
        let items = vec![
            DebtItem {
                id: "1".to_string(),
                debt_type: DebtType::Complexity,
                priority: Priority::High,
                file: PathBuf::from("test1.rs"),
                line: 10,
                column: None,
                message: "Complex function".to_string(),
                context: None,
            },
            DebtItem {
                id: "2".to_string(),
                debt_type: DebtType::Todo,
                priority: Priority::Medium,
                file: PathBuf::from("test2.rs"),
                line: 20,
                column: None,
                message: "TODO: Fix this".to_string(),
                context: None,
            },
            DebtItem {
                id: "3".to_string(),
                debt_type: DebtType::Duplication,
                priority: Priority::Medium,
                file: PathBuf::from("test3.rs"),
                line: 30,
                column: None,
                message: "Duplicated code".to_string(),
                context: None,
            },
        ];

        let categories = categorize_debt(&items);

        assert_eq!(categories.len(), 3);
        assert!(categories.contains_key("Complexity Issues"));
        assert!(categories.contains_key("TODOs and FIXMEs"));
        assert!(categories.contains_key("Duplication"));
        assert_eq!(categories["Complexity Issues"].len(), 1);
        assert_eq!(categories["TODOs and FIXMEs"].len(), 1);
        assert_eq!(categories["Duplication"].len(), 1);
    }

    #[test]
    fn test_get_top_complex_functions() {
        use crate::core::ComplexitySummary;
        use chrono::Utc;

        let results = AnalysisResults {
            project_path: PathBuf::from("test_project"),
            timestamp: Utc::now(),
            complexity: ComplexityResults {
                metrics: vec![
                    FunctionMetrics {
                        name: "func1".to_string(),
                        file: PathBuf::from("file1.rs"),
                        line: 10,
                        cyclomatic: 20,
                        cognitive: 25,
                        nesting: 2,
                        length: 50,
                        is_test: false,
                        visibility: Some("pub".to_string()),
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                        purity_reason: None,
                        call_dependencies: None,
                        detected_patterns: None,
                        upstream_callers: None,
                        downstream_callees: None,
                        mapping_pattern_result: None,
                        adjusted_complexity: None,
                        composition_metrics: None,
                        language_specific: None,
                        purity_level: None,
                    },
                    FunctionMetrics {
                        name: "func2".to_string(),
                        file: PathBuf::from("file2.rs"),
                        line: 20,
                        cyclomatic: 15,
                        cognitive: 20,
                        nesting: 1,
                        length: 40,
                        is_test: false,
                        visibility: None,
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                        purity_reason: None,
                        call_dependencies: None,
                        detected_patterns: None,
                        upstream_callers: None,
                        downstream_callees: None,
                        mapping_pattern_result: None,
                        adjusted_complexity: None,
                        composition_metrics: None,
                        language_specific: None,
                        purity_level: None,
                    },
                    FunctionMetrics {
                        name: "func3".to_string(),
                        file: PathBuf::from("file3.rs"),
                        line: 30,
                        cyclomatic: 25,
                        cognitive: 30,
                        nesting: 3,
                        length: 60,
                        is_test: false,
                        visibility: Some("pub(crate)".to_string()),
                        is_trait_method: false,
                        in_test_module: false,
                        entropy_score: None,
                        is_pure: None,
                        purity_confidence: None,
                        purity_reason: None,
                        call_dependencies: None,
                        detected_patterns: None,
                        upstream_callers: None,
                        downstream_callees: None,
                        mapping_pattern_result: None,
                        adjusted_complexity: None,
                        composition_metrics: None,
                        language_specific: None,
                        purity_level: None,
                    },
                ],
                summary: ComplexitySummary {
                    total_functions: 3,
                    average_complexity: 20.0,
                    max_complexity: 25,
                    high_complexity_count: 2,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
            file_contexts: std::collections::HashMap::new(),
        };

        let top_functions = get_top_complex_functions(&results, 2);

        assert_eq!(top_functions.len(), 2);
        assert_eq!(top_functions[0], "func3 (complexity: 25)");
        assert_eq!(top_functions[1], "func1 (complexity: 20)");
    }

    #[test]
    fn test_extract_module_dependencies() {
        let items = vec![UnifiedDebtItem {
            location: PriorityLocation {
                file: PathBuf::from("/project/src/module1/file.rs"),
                function: "test_func".to_string(),
                line: 10,
            },
            debt_type: PriorityDebtType::DeadCode {
                visibility: FunctionVisibility::Private,
                cyclomatic: 1,
                cognitive: 1,
                usage_hints: vec![],
            },
            unified_score: UnifiedScore {
                complexity_factor: 1.0,
                coverage_factor: 1.0,
                dependency_factor: 1.0,
                role_multiplier: 1.0,
                final_score: 1.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: String::new(),
                rationale: String::new(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                coverage_improvement: 0.0,
                lines_reduction: 0,
                complexity_reduction: 0.0,
                risk_reduction: 0.0,
            },
            transitive_coverage: Some(TransitiveCoverage {
                direct: 0.0,
                transitive: 0.0,
                propagated_from: vec![],
                uncovered_lines: vec![],
            }),
            file_context: None,
            upstream_dependencies: 0,
            downstream_dependencies: 1,
            upstream_callers: vec![],
            downstream_callees: vec!["func1".to_string()],
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 1,
            cognitive_complexity: 1,
            entropy_details: None,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None,
        }];

        let deps = extract_module_dependencies(&items);

        assert!(deps.contains_key("module1"));
        assert_eq!(deps["module1"], vec!["dependency"]);
    }
}
