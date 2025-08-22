pub mod circular;
pub mod coupling;
pub mod duplication;
pub mod error_swallowing;
pub mod patterns;
pub mod smells;
pub mod suppression;

pub use crate::core::Priority;
use crate::core::{DebtItem, DebtType};
use std::collections::HashMap;
use std::path::PathBuf;

pub fn categorize_debt(items: Vec<DebtItem>) -> HashMap<DebtType, Vec<DebtItem>> {
    items.into_iter().fold(HashMap::new(), |mut acc, item| {
        acc.entry(item.debt_type).or_default().push(item.clone());
        acc
    })
}

pub fn prioritize_debt(items: Vec<DebtItem>) -> Vec<DebtItem> {
    let mut sorted = items;
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
        DebtType::Todo | DebtType::TestTodo => 1,
        DebtType::Fixme | DebtType::TestComplexity | DebtType::TestDuplication => 2,
        DebtType::CodeSmell
        | DebtType::Dependency
        | DebtType::CodeOrganization
        | DebtType::TestQuality => 3,
        DebtType::Duplication | DebtType::ErrorSwallowing | DebtType::ResourceManagement => 4,
        DebtType::Complexity => 5,
        DebtType::Security => 10,
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
        assert_eq!(type_weight(&DebtType::Todo), 1);
        assert_eq!(type_weight(&DebtType::TestTodo), 1);
    }

    #[test]
    fn test_type_weight_fixme() {
        assert_eq!(type_weight(&DebtType::Fixme), 2);
        assert_eq!(type_weight(&DebtType::TestComplexity), 2);
        assert_eq!(type_weight(&DebtType::TestDuplication), 2);
    }

    #[test]
    fn test_type_weight_medium() {
        assert_eq!(type_weight(&DebtType::CodeSmell), 3);
        assert_eq!(type_weight(&DebtType::Dependency), 3);
        assert_eq!(type_weight(&DebtType::CodeOrganization), 3);
        assert_eq!(type_weight(&DebtType::TestQuality), 3);
    }

    #[test]
    fn test_type_weight_high() {
        assert_eq!(type_weight(&DebtType::Duplication), 4);
        assert_eq!(type_weight(&DebtType::ErrorSwallowing), 4);
        assert_eq!(type_weight(&DebtType::ResourceManagement), 4);
    }

    #[test]
    fn test_type_weight_complexity() {
        assert_eq!(type_weight(&DebtType::Complexity), 5);
    }

    #[test]
    fn test_type_weight_security() {
        assert_eq!(type_weight(&DebtType::Security), 10);
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
        let low_todo = create_test_item(DebtType::Todo, Priority::Low);
        assert_eq!(calculate_debt_score(&low_todo), 1); // 1 * 1

        let high_security = create_test_item(DebtType::Security, Priority::High);
        assert_eq!(calculate_debt_score(&high_security), 50); // 5 * 10

        let critical_complexity = create_test_item(DebtType::Complexity, Priority::Critical);
        assert_eq!(calculate_debt_score(&critical_complexity), 50); // 10 * 5
    }

    #[test]
    fn test_total_debt_score() {
        let items = vec![
            create_test_item(DebtType::Todo, Priority::Low),
            create_test_item(DebtType::Fixme, Priority::Medium),
            create_test_item(DebtType::Security, Priority::Critical),
        ];
        assert_eq!(total_debt_score(&items), 1 + 6 + 100); // 107
    }

    #[test]
    fn test_categorize_debt() {
        let items = vec![
            create_test_item(DebtType::Todo, Priority::Low),
            create_test_item(DebtType::Todo, Priority::Medium),
            create_test_item(DebtType::Fixme, Priority::High),
        ];

        let categorized = categorize_debt(items);
        assert_eq!(categorized.get(&DebtType::Todo).unwrap().len(), 2);
        assert_eq!(categorized.get(&DebtType::Fixme).unwrap().len(), 1);
    }

    #[test]
    fn test_prioritize_debt() {
        let items = vec![
            create_test_item(DebtType::Todo, Priority::Low),
            create_test_item(DebtType::Todo, Priority::Critical),
            create_test_item(DebtType::Todo, Priority::Medium),
        ];

        let prioritized = prioritize_debt(items);
        assert_eq!(prioritized[0].priority, Priority::Critical);
        assert_eq!(prioritized[1].priority, Priority::Medium);
        assert_eq!(prioritized[2].priority, Priority::Low);
    }

    #[test]
    fn test_filter_by_priority() {
        let items = vec![
            create_test_item(DebtType::Todo, Priority::Low),
            create_test_item(DebtType::Todo, Priority::Medium),
            create_test_item(DebtType::Todo, Priority::High),
        ];

        let filtered = filter_by_priority(items, Priority::Medium);
        assert_eq!(filtered.len(), 2);
        assert!(filtered
            .iter()
            .all(|item| item.priority >= Priority::Medium));
    }

    #[test]
    fn test_filter_by_type() {
        let items = vec![
            create_test_item(DebtType::Todo, Priority::Low),
            create_test_item(DebtType::Fixme, Priority::Low),
            create_test_item(DebtType::Todo, Priority::High),
        ];

        let filtered = filter_by_type(items, DebtType::Todo);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|item| item.debt_type == DebtType::Todo));
    }

    #[test]
    fn test_group_by_file() {
        let items = vec![
            DebtItem {
                id: "file1_todo".to_string(),
                debt_type: DebtType::Todo,
                priority: Priority::Low,
                file: PathBuf::from("file1.rs"),
                line: 1,
                column: Some(1),
                message: "Test".to_string(),
                context: None,
            },
            DebtItem {
                id: "file1_fixme".to_string(),
                debt_type: DebtType::Fixme,
                priority: Priority::Low,
                file: PathBuf::from("file1.rs"),
                line: 10,
                column: Some(1),
                message: "Test".to_string(),
                context: None,
            },
            DebtItem {
                id: "file2_todo".to_string(),
                debt_type: DebtType::Todo,
                priority: Priority::Low,
                file: PathBuf::from("file2.rs"),
                line: 1,
                column: Some(1),
                message: "Test".to_string(),
                context: None,
            },
        ];

        let grouped = group_by_file(items);
        assert_eq!(grouped.get(&PathBuf::from("file1.rs")).unwrap().len(), 2);
        assert_eq!(grouped.get(&PathBuf::from("file2.rs")).unwrap().len(), 1);
    }
}
