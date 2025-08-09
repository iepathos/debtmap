use crate::core::{DebtItem, DebtType, Priority};
use regex::Regex;
use std::path::PathBuf;

pub fn find_todos_and_fixmes(content: &str, file: &PathBuf) -> Vec<DebtItem> {
    let mut items = Vec::new();

    let todo_regex =
        Regex::new(r"(?i)\b(TODO|FIXME|HACK|XXX|BUG|OPTIMIZE|REFACTOR):\s*(.*)").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(captures) = todo_regex.captures(line) {
            let marker = captures.get(1).unwrap().as_str().to_uppercase();
            let message = captures.get(2).unwrap().as_str().trim().to_string();

            let (debt_type, priority) = match marker.as_str() {
                "FIXME" | "BUG" => (DebtType::Fixme, Priority::High),
                "TODO" => (DebtType::Todo, Priority::Medium),
                "HACK" | "XXX" => (DebtType::CodeSmell, Priority::High),
                "OPTIMIZE" => (DebtType::CodeSmell, Priority::Low),
                "REFACTOR" => (DebtType::CodeSmell, Priority::Medium),
                _ => (DebtType::Todo, Priority::Low),
            };

            items.push(DebtItem {
                id: format!("{}-{}-{}", debt_type, file.display(), line_num + 1),
                debt_type,
                priority,
                file: file.clone(),
                line: line_num + 1,
                message: format!("{marker}: {message}"),
                context: Some(line.trim().to_string()),
            });
        }
    }

    items
}

pub fn find_code_smells(content: &str, file: &PathBuf) -> Vec<DebtItem> {
    let mut items = Vec::new();

    let long_line_threshold = 120;
    let deep_nesting_threshold = 4;

    for (line_num, line) in content.lines().enumerate() {
        if line.len() > long_line_threshold {
            items.push(DebtItem {
                id: format!("long-line-{}-{}", file.display(), line_num + 1),
                debt_type: DebtType::CodeSmell,
                priority: Priority::Low,
                file: file.clone(),
                line: line_num + 1,
                message: format!(
                    "Line exceeds {} characters ({})",
                    long_line_threshold,
                    line.len()
                ),
                context: None,
            });
        }

        let indent_count = line.chars().take_while(|c| c.is_whitespace()).count() / 4;
        if indent_count > deep_nesting_threshold {
            items.push(DebtItem {
                id: format!("deep-nesting-{}-{}", file.display(), line_num + 1),
                debt_type: DebtType::CodeSmell,
                priority: Priority::Medium,
                file: file.clone(),
                line: line_num + 1,
                message: format!("Deep nesting level: {indent_count}"),
                context: Some(line.trim().to_string()),
            });
        }
    }

    items
}

pub fn detect_duplicate_strings(content: &str, file: &PathBuf) -> Vec<DebtItem> {
    let mut items = Vec::new();
    let string_regex = Regex::new(r#"["']([^"']{20,})["']"#).unwrap();
    let mut string_counts: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();

    for (line_num, line) in content.lines().enumerate() {
        for captures in string_regex.captures_iter(line) {
            let string = captures.get(1).unwrap().as_str().to_string();
            string_counts.entry(string).or_default().push(line_num + 1);
        }
    }

    for (string, lines) in string_counts {
        if lines.len() > 2 {
            items.push(DebtItem {
                id: format!("duplicate-string-{}-{}", file.display(), lines[0]),
                debt_type: DebtType::Duplication,
                priority: Priority::Low,
                file: file.clone(),
                line: lines[0],
                message: format!(
                    "String '{}' appears {} times",
                    if string.len() > 50 {
                        &string[..50]
                    } else {
                        &string
                    },
                    lines.len()
                ),
                context: Some(format!("Lines: {lines:?}")),
            });
        }
    }

    items
}

pub fn combine_debt_items(items: Vec<Vec<DebtItem>>) -> Vec<DebtItem> {
    items.into_iter().flatten().collect()
}

pub fn deduplicate_debt_items(items: Vec<DebtItem>) -> Vec<DebtItem> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.id.clone()))
        .collect()
}
