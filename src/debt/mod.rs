pub mod circular;
pub mod coupling;
pub mod duplication;
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
    let priority_score = match item.priority {
        Priority::Low => 1,
        Priority::Medium => 3,
        Priority::High => 5,
        Priority::Critical => 10,
    };

    let type_score = match item.debt_type {
        DebtType::Todo => 1,
        DebtType::Fixme => 2,
        DebtType::CodeSmell => 3,
        DebtType::Duplication => 4,
        DebtType::Complexity => 5,
        DebtType::Dependency => 3,
    };

    priority_score * type_score
}

pub fn total_debt_score(items: &[DebtItem]) -> u32 {
    items.iter().map(calculate_debt_score).sum()
}
