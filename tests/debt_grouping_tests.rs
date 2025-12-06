use debtmap::core::{DebtItem, DebtType, Priority};
use debtmap::debt::{filter_by_priority, filter_by_type, group_by_file};
use std::path::PathBuf;

#[test]
fn test_group_by_file_empty() {
    let items = vec![];
    let grouped = group_by_file(items);
    assert!(grouped.is_empty());
}

#[test]
fn test_group_by_file_single_file() {
    let items = vec![
        DebtItem {
            id: "1".to_string(),
            debt_type: DebtType::Todo { reason: None },
            priority: Priority::Medium,
            file: PathBuf::from("src/main.rs"),
            line: 10,
            column: None,
            message: "TODO: Implement feature".to_string(),
            context: None,
        },
        DebtItem {
            id: "2".to_string(),
            debt_type: DebtType::Fixme { reason: None },
            priority: Priority::High,
            file: PathBuf::from("src/main.rs"),
            line: 20,
            column: None,
            message: "Bug here".to_string(),
            context: None,
        },
    ];

    let grouped = group_by_file(items);

    assert_eq!(grouped.len(), 1);
    assert!(grouped.contains_key(&PathBuf::from("src/main.rs")));
    assert_eq!(grouped[&PathBuf::from("src/main.rs")].len(), 2);
}

#[test]
fn test_group_by_file_multiple_files() {
    let items = vec![
        DebtItem {
            id: "1".to_string(),
            debt_type: DebtType::Todo { reason: None },
            priority: Priority::Low,
            file: PathBuf::from("src/file1.rs"),
            line: 10,
            column: None,
            message: "TODO: Task 1".to_string(),
            context: None,
        },
        DebtItem {
            id: "2".to_string(),
            debt_type: DebtType::CodeSmell { smell_type: None },
            priority: Priority::Medium,
            file: PathBuf::from("src/file2.rs"),
            line: 20,
            column: None,
            message: "Long method".to_string(),
            context: None,
        },
        DebtItem {
            id: "3".to_string(),
            debt_type: DebtType::Complexity {
                cyclomatic: 10,
                cognitive: 8,
            },
            priority: Priority::High,
            file: PathBuf::from("src/file1.rs"),
            line: 30,
            column: None,
            message: "Complex function".to_string(),
            context: None,
        },
    ];

    let grouped = group_by_file(items);

    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped[&PathBuf::from("src/file1.rs")].len(), 2);
    assert_eq!(grouped[&PathBuf::from("src/file2.rs")].len(), 1);
}

#[test]
fn test_group_by_file_preserves_debt_items() {
    let item = DebtItem {
        id: "unique-id".to_string(),
        debt_type: DebtType::Duplication {
            instances: 2,
            total_lines: 50,
        },
        priority: Priority::Critical,
        file: PathBuf::from("src/test.rs"),
        line: 42,
        column: None,
        message: "Duplicate code detected".to_string(),
        context: Some("Additional context".to_string()),
    };

    let items = vec![item.clone()];
    let grouped = group_by_file(items);

    let result = &grouped[&PathBuf::from("src/test.rs")][0];
    assert_eq!(result.id, "unique-id");
    assert_eq!(
        result.debt_type,
        DebtType::Duplication {
            instances: 2,
            total_lines: 50
        }
    );
    assert_eq!(result.priority, Priority::Critical);
    assert_eq!(result.line, 42);
    assert_eq!(result.message, "Duplicate code detected");
    assert_eq!(result.context, Some("Additional context".to_string()));
}

#[test]
fn test_filter_by_priority_minimum_threshold() {
    let items = vec![
        DebtItem {
            id: "1".to_string(),
            debt_type: DebtType::Todo { reason: None },
            priority: Priority::Low,
            file: PathBuf::from("file.rs"),
            line: 10,
            column: None,
            message: "Low priority".to_string(),
            context: None,
        },
        DebtItem {
            id: "2".to_string(),
            debt_type: DebtType::Fixme { reason: None },
            priority: Priority::High,
            file: PathBuf::from("file.rs"),
            line: 20,
            column: None,
            message: "High priority".to_string(),
            context: None,
        },
        DebtItem {
            id: "3".to_string(),
            debt_type: DebtType::Todo { reason: None },
            priority: Priority::Medium,
            file: PathBuf::from("file.rs"),
            line: 30,
            column: None,
            message: "Medium priority".to_string(),
            context: None,
        },
    ];

    // Filter for Medium or higher priority
    let filtered = filter_by_priority(items.clone(), Priority::Medium);
    assert_eq!(filtered.len(), 2); // Medium and High
    assert!(filtered
        .iter()
        .all(|item| item.priority >= Priority::Medium));

    // Filter for Low or higher (all items)
    let filtered = filter_by_priority(items.clone(), Priority::Low);
    assert_eq!(filtered.len(), 3);

    // Filter for High or higher
    let filtered = filter_by_priority(items, Priority::High);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].priority, Priority::High);
}

#[test]
fn test_filter_by_priority_none_match() {
    let items = vec![
        DebtItem {
            id: "1".to_string(),
            debt_type: DebtType::Todo { reason: None },
            priority: Priority::Low,
            file: PathBuf::from("file.rs"),
            line: 10,
            column: None,
            message: "Low".to_string(),
            context: None,
        },
        DebtItem {
            id: "2".to_string(),
            debt_type: DebtType::Fixme { reason: None },
            priority: Priority::Medium,
            file: PathBuf::from("file.rs"),
            line: 20,
            column: None,
            message: "Medium".to_string(),
            context: None,
        },
    ];

    let filtered = filter_by_priority(items, Priority::Critical);
    assert!(filtered.is_empty());
}

#[test]
fn test_filter_by_type_single() {
    let items = vec![
        DebtItem {
            id: "1".to_string(),
            debt_type: DebtType::Todo { reason: None },
            priority: Priority::Medium,
            file: PathBuf::from("file.rs"),
            line: 10,
            column: None,
            message: "TODO".to_string(),
            context: None,
        },
        DebtItem {
            id: "2".to_string(),
            debt_type: DebtType::CodeSmell { smell_type: None },
            priority: Priority::High,
            file: PathBuf::from("file.rs"),
            line: 20,
            column: None,
            message: "Smell".to_string(),
            context: None,
        },
        DebtItem {
            id: "3".to_string(),
            debt_type: DebtType::Todo { reason: None },
            priority: Priority::Low,
            file: PathBuf::from("file.rs"),
            line: 30,
            column: None,
            message: "Another TODO".to_string(),
            context: None,
        },
    ];

    let filtered = filter_by_type(items, DebtType::Todo { reason: None });

    assert_eq!(filtered.len(), 2);
    assert!(filtered
        .iter()
        .all(|item| item.debt_type == DebtType::Todo { reason: None }));
}

#[test]
fn test_filter_by_type_all_types() {
    let types = vec![
        DebtType::Todo { reason: None },
        DebtType::Fixme { reason: None },
        DebtType::CodeSmell { smell_type: None },
        DebtType::Duplication {
            instances: 2,
            total_lines: 50,
        },
        DebtType::Complexity {
            cyclomatic: 10,
            cognitive: 8,
        },
        DebtType::Dependency {
            dependency_type: None,
        },
    ];

    let items: Vec<DebtItem> = types
        .iter()
        .enumerate()
        .map(|(i, dt)| DebtItem {
            id: format!("{i}"),
            debt_type: dt.clone(),
            priority: Priority::Medium,
            file: PathBuf::from("file.rs"),
            line: i * 10,
            column: None,
            message: format!("Item {i}"),
            context: None,
        })
        .collect();

    for debt_type in types {
        let filtered = filter_by_type(items.clone(), debt_type.clone());
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].debt_type, debt_type);
    }
}
