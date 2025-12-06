pub mod async_errors;
pub mod circular;
pub mod coupling;
pub mod duplication;
pub mod error_context;
pub mod error_propagation;
pub mod error_swallowing;
pub mod panic_patterns;
pub mod patterns;
pub mod public_api_detector;
pub mod smells;
pub mod suppression;

pub use crate::core::Priority;
use crate::core::{DebtItem, DebtType};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn categorize_debt(items: &[DebtItem]) -> HashMap<DebtType, Vec<DebtItem>> {
    items.iter().fold(HashMap::new(), |mut acc, item| {
        acc.entry(item.debt_type.clone())
            .or_default()
            .push(item.clone());
        acc
    })
}

pub fn prioritize_debt(items: &[DebtItem]) -> Vec<DebtItem> {
    let mut sorted: Vec<DebtItem> = items.to_vec();
    sorted.sort_by_key(|item| std::cmp::Reverse(item.priority));
    sorted
}

pub fn filter_by_priority(items: Vec<DebtItem>, min_priority: Priority) -> Vec<DebtItem> {
    items
        .into_iter()
        .filter(|item| item.priority >= min_priority)
        .collect()
}

pub fn filter_by_type(items: Vec<DebtItem>, debt_type: DebtType) -> Vec<DebtItem> {
    items
        .into_iter()
        .filter(|item| item.debt_type == debt_type)
        .collect()
}

pub fn group_by_file(items: Vec<DebtItem>) -> std::collections::HashMap<PathBuf, Vec<DebtItem>> {
    use std::collections::HashMap;

    items.into_iter().fold(HashMap::new(), |mut acc, item| {
        acc.entry(item.file.clone()).or_default().push(item);
        acc
    })
}

pub fn calculate_debt_score(item: &DebtItem) -> u32 {
    priority_weight(&item.priority) * type_weight(&item.debt_type)
}

fn priority_weight(priority: &Priority) -> u32 {
    match priority {
        Priority::Low => 1,
        Priority::Medium => 3,
        Priority::High => 5,
        Priority::Critical => 10,
    }
}

fn type_weight(debt_type: &DebtType) -> u32 {
    match debt_type {
        DebtType::Todo { .. } | DebtType::TestTodo { .. } => 1,
        DebtType::Fixme { .. }
        | DebtType::TestComplexity { .. }
        | DebtType::TestDuplication { .. } => 2,
        DebtType::CodeSmell { .. }
        | DebtType::Dependency { .. }
        | DebtType::CodeOrganization { .. }
        | DebtType::TestQuality { .. } => 3,
        DebtType::Duplication { .. }
        | DebtType::ErrorSwallowing { .. }
        | DebtType::ResourceManagement { .. } => 4,
        DebtType::Complexity { .. } => 5,
        DebtType::UtilitiesSprawl { .. }
        | DebtType::MagicValues { .. }
        | DebtType::FeatureEnvy { .. }
        | DebtType::PrimitiveObsession { .. } => 3,
        _ => 3, // Default weight for any other debt types
    }
}

pub fn total_debt_score(items: &[DebtItem]) -> u32 {
    items.iter().map(calculate_debt_score).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_item(debt_type: DebtType, priority: Priority) -> DebtItem {
        DebtItem {
            id: format!("{:?}_{:?}_test", debt_type, priority),
            debt_type,
            priority,
            file: PathBuf::from("test.rs"),
            line: 1,
            column: Some(1),
            message: "Test item".to_string(),
            context: None,
        }
    }

    #[test]
    fn test_type_weight_todo() {
        assert_eq!(type_weight(&DebtType::Todo { reason: None }), 1);
        assert_eq!(
            type_weight(&DebtType::TestTodo {
                priority: Priority::Low,
                reason: None
            }),
            1
        );
    }

    #[test]
    fn test_type_weight_fixme() {
        assert_eq!(type_weight(&DebtType::Fixme { reason: None }), 2);
        assert_eq!(
            type_weight(&DebtType::TestComplexity {
                cyclomatic: 5,
                cognitive: 4
            }),
            2
        );
        assert_eq!(
            type_weight(&DebtType::TestDuplication {
                instances: 1,
                total_lines: 10,
                similarity: 0.9
            }),
            2
        );
    }

    #[test]
    fn test_type_weight_medium() {
        assert_eq!(type_weight(&DebtType::CodeSmell { smell_type: None }), 3);
        assert_eq!(
            type_weight(&DebtType::Dependency {
                dependency_type: None
            }),
            3
        );
        assert_eq!(
            type_weight(&DebtType::CodeOrganization { issue_type: None }),
            3
        );
        assert_eq!(type_weight(&DebtType::TestQuality { issue_type: None }), 3);
    }

    #[test]
    fn test_type_weight_high() {
        assert_eq!(
            type_weight(&DebtType::Duplication {
                instances: 2,
                total_lines: 20
            }),
            4
        );
        assert_eq!(
            type_weight(&DebtType::ErrorSwallowing {
                pattern: "test_pattern".to_string(),
                context: None
            }),
            4
        );
        assert_eq!(
            type_weight(&DebtType::ResourceManagement { issue_type: None }),
            4
        );
    }

    #[test]
    fn test_type_weight_complexity() {
        assert_eq!(
            type_weight(&DebtType::Complexity {
                cyclomatic: 10,
                cognitive: 8
            }),
            5
        );
    }

    #[test]
    fn test_type_weight_all_variants() {
        // Test all debt type variants to ensure complete coverage
        assert_eq!(type_weight(&DebtType::Todo { reason: None }), 1);
        assert_eq!(type_weight(&DebtType::Fixme { reason: None }), 2);
        assert_eq!(type_weight(&DebtType::CodeSmell { smell_type: None }), 3);
        assert_eq!(
            type_weight(&DebtType::Duplication {
                instances: 2,
                total_lines: 20
            }),
            4
        );
        assert_eq!(
            type_weight(&DebtType::Complexity {
                cyclomatic: 10,
                cognitive: 8
            }),
            5
        );
        assert_eq!(
            type_weight(&DebtType::Dependency {
                dependency_type: None
            }),
            3
        );
        assert_eq!(
            type_weight(&DebtType::ErrorSwallowing {
                pattern: "test_pattern".to_string(),
                context: None
            }),
            4
        );
        assert_eq!(
            type_weight(&DebtType::ResourceManagement { issue_type: None }),
            4
        );
        assert_eq!(
            type_weight(&DebtType::CodeOrganization { issue_type: None }),
            3
        );
        assert_eq!(
            type_weight(&DebtType::TestComplexity {
                cyclomatic: 5,
                cognitive: 4
            }),
            2
        );
        assert_eq!(
            type_weight(&DebtType::TestTodo {
                priority: Priority::Low,
                reason: None
            }),
            1
        );
        assert_eq!(
            type_weight(&DebtType::TestDuplication {
                instances: 1,
                total_lines: 10,
                similarity: 0.9
            }),
            2
        );
        assert_eq!(type_weight(&DebtType::TestQuality { issue_type: None }), 3);
    }

    #[test]
    fn test_priority_weight() {
        assert_eq!(priority_weight(&Priority::Low), 1);
        assert_eq!(priority_weight(&Priority::Medium), 3);
        assert_eq!(priority_weight(&Priority::High), 5);
        assert_eq!(priority_weight(&Priority::Critical), 10);
    }

    #[test]
    fn test_calculate_debt_score() {
        let low_todo = create_test_item(DebtType::Todo { reason: None }, Priority::Low);
        assert_eq!(calculate_debt_score(&low_todo), 1); // 1 * 1

        let critical_complexity = create_test_item(
            DebtType::Complexity {
                cyclomatic: 10,
                cognitive: 8,
            },
            Priority::Critical,
        );
        assert_eq!(calculate_debt_score(&critical_complexity), 50); // 10 * 5

        let fixme_medium = create_test_item(DebtType::Fixme { reason: None }, Priority::Medium);
        assert_eq!(calculate_debt_score(&fixme_medium), 6); // 3 * 2

        let complexity_high = create_test_item(
            DebtType::Complexity {
                cyclomatic: 10,
                cognitive: 8,
            },
            Priority::High,
        );
        assert_eq!(calculate_debt_score(&complexity_high), 25); // 5 * 5
    }

    #[test]
    fn test_total_debt_score() {
        let items = vec![
            create_test_item(DebtType::Todo { reason: None }, Priority::Low),
            create_test_item(DebtType::Fixme { reason: None }, Priority::Medium),
            create_test_item(
                DebtType::Complexity {
                    cyclomatic: 10,
                    cognitive: 8,
                },
                Priority::Critical,
            ),
        ];
        assert_eq!(total_debt_score(&items), 1 + 6 + 50); // 57
    }

    #[test]
    fn test_total_debt_score_complex() {
        let items = vec![
            create_test_item(DebtType::Todo { reason: None }, Priority::Low), // 1
            create_test_item(DebtType::Fixme { reason: None }, Priority::Medium), // 6
            create_test_item(
                DebtType::Complexity {
                    cyclomatic: 10,
                    cognitive: 8,
                },
                Priority::High,
            ), // 25
        ];

        assert_eq!(total_debt_score(&items), 32);
    }

    #[test]
    fn test_total_debt_score_empty() {
        let items: Vec<DebtItem> = vec![];
        assert_eq!(total_debt_score(&items), 0);
    }

    #[test]
    fn test_categorize_debt() {
        let todo_type = DebtType::Todo { reason: None };
        let fixme_type = DebtType::Fixme { reason: None };
        let complexity_type = DebtType::Complexity {
            cyclomatic: 10,
            cognitive: 8,
        };

        let items = vec![
            create_test_item(todo_type.clone(), Priority::Low),
            create_test_item(todo_type.clone(), Priority::Medium),
            create_test_item(fixme_type.clone(), Priority::High),
            create_test_item(complexity_type.clone(), Priority::Critical),
        ];

        let categorized = categorize_debt(&items);
        assert_eq!(categorized.len(), 3);
        assert_eq!(categorized.get(&todo_type).unwrap().len(), 2);
        assert_eq!(categorized.get(&fixme_type).unwrap().len(), 1);
        assert_eq!(categorized.get(&complexity_type).unwrap().len(), 1);
    }

    #[test]
    fn test_prioritize_debt() {
        let items = vec![
            create_test_item(DebtType::Todo { reason: None }, Priority::Low),
            create_test_item(DebtType::Fixme { reason: None }, Priority::Critical),
            create_test_item(DebtType::CodeSmell { smell_type: None }, Priority::Medium),
            create_test_item(
                DebtType::Complexity {
                    cyclomatic: 10,
                    cognitive: 8,
                },
                Priority::High,
            ),
        ];

        let prioritized = prioritize_debt(&items);

        assert_eq!(prioritized[0].priority, Priority::Critical);
        assert_eq!(prioritized[1].priority, Priority::High);
        assert_eq!(prioritized[2].priority, Priority::Medium);
        assert_eq!(prioritized[3].priority, Priority::Low);
    }

    #[test]
    fn test_filter_by_priority() {
        let items = vec![
            create_test_item(DebtType::Todo { reason: None }, Priority::Low),
            create_test_item(DebtType::Fixme { reason: None }, Priority::Medium),
            create_test_item(DebtType::CodeSmell { smell_type: None }, Priority::High),
            create_test_item(
                DebtType::Complexity {
                    cyclomatic: 10,
                    cognitive: 8,
                },
                Priority::Critical,
            ),
        ];

        let filtered = filter_by_priority(items, Priority::Medium);

        assert_eq!(filtered.len(), 3);
        assert!(filtered
            .iter()
            .all(|item| item.priority >= Priority::Medium));
    }

    #[test]
    fn test_filter_by_type() {
        let todo_type = DebtType::Todo { reason: None };
        let items = vec![
            create_test_item(todo_type.clone(), Priority::Low),
            create_test_item(todo_type.clone(), Priority::Medium),
            create_test_item(DebtType::Fixme { reason: None }, Priority::High),
            create_test_item(
                DebtType::Complexity {
                    cyclomatic: 10,
                    cognitive: 8,
                },
                Priority::Critical,
            ),
        ];

        let filtered = filter_by_type(items, todo_type.clone());

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|item| item.debt_type == todo_type));
    }

    #[test]
    fn test_group_by_file() {
        let items = vec![
            DebtItem {
                id: "file1_todo".to_string(),
                debt_type: DebtType::Todo { reason: None },
                priority: Priority::Low,
                file: PathBuf::from("file1.rs"),
                line: 1,
                column: Some(1),
                message: "Test".to_string(),
                context: None,
            },
            DebtItem {
                id: "file1_fixme".to_string(),
                debt_type: DebtType::Fixme { reason: None },
                priority: Priority::Low,
                file: PathBuf::from("file1.rs"),
                line: 10,
                column: Some(1),
                message: "Test".to_string(),
                context: None,
            },
            DebtItem {
                id: "file2_smell".to_string(),
                debt_type: DebtType::CodeSmell { smell_type: None },
                priority: Priority::High,
                file: PathBuf::from("file2.rs"),
                line: 20,
                column: Some(1),
                message: "Test".to_string(),
                context: None,
            },
        ];

        let grouped = group_by_file(items);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped.get(&PathBuf::from("file1.rs")).unwrap().len(), 2);
        assert_eq!(grouped.get(&PathBuf::from("file2.rs")).unwrap().len(), 1);
    }

    #[test]
    fn test_edge_cases() {
        // Test with empty collections
        let empty_items: Vec<DebtItem> = vec![];

        let categorized = categorize_debt(&empty_items);
        assert_eq!(categorized.len(), 0);

        let prioritized = prioritize_debt(&empty_items);
        assert_eq!(prioritized.len(), 0);

        let filtered = filter_by_priority(empty_items.clone(), Priority::High);
        assert_eq!(filtered.len(), 0);

        let grouped = group_by_file(empty_items);
        assert_eq!(grouped.len(), 0);
    }
}
