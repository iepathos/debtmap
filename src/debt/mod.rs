pub mod circular;
pub mod coupling;
pub mod duplication;
pub mod error_swallowing;
pub mod patterns;
pub mod smells;
pub mod suppression;

use crate::core::{DebtItem, DebtType, Priority};
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
    const WEIGHTS: &[(DebtType, u32)] = &[
        (DebtType::Todo, 1),
        (DebtType::Fixme, 2),
        (DebtType::CodeSmell, 3),
        (DebtType::Duplication, 4),
        (DebtType::Complexity, 5),
        (DebtType::Dependency, 3),
        (DebtType::ErrorSwallowing, 4),
        (DebtType::ResourceManagement, 4),
        (DebtType::CodeOrganization, 3),
        (DebtType::Performance, 4),
        (DebtType::Security, 10), // Security issues get highest weight
        (DebtType::TestComplexity, 2),
        (DebtType::TestTodo, 1),
        (DebtType::TestDuplication, 2),
        (DebtType::TestQuality, 3),
    ];

    WEIGHTS
        .iter()
        .find(|(dt, _)| dt == debt_type)
        .map(|(_, weight)| *weight)
        .unwrap_or(1)
}

pub fn total_debt_score(items: &[DebtItem]) -> u32 {
    items.iter().map(calculate_debt_score).sum()
}
