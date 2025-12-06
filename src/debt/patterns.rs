use crate::core::{DebtItem, DebtType, Priority};
use crate::debt::suppression::SuppressionContext;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

pub fn find_todos_and_fixmes(content: &str, file: &Path) -> Vec<DebtItem> {
    find_todos_and_fixmes_with_suppression(content, file, None)
}

pub fn find_todos_and_fixmes_with_suppression(
    content: &str,
    file: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem> {
    let todo_regex =
        Regex::new(r"(?i)\b(TODO|FIXME|HACK|XXX|BUG|OPTIMIZE|REFACTOR):\s*(.*)").unwrap();

    content
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            todo_regex.captures(line).and_then(|captures| {
                let marker = captures.get(1).unwrap().as_str().to_uppercase();
                let message = captures.get(2).unwrap().as_str().trim();

                let (debt_type, priority) = classify_marker(&marker);
                let line_number = line_num + 1;

                // Check if this line is suppressed
                if let Some(ctx) = suppression {
                    if ctx.is_suppressed(line_number, &debt_type) {
                        return None;
                    }
                }

                Some(DebtItem {
                    id: format!("{}-{}-{}", debt_type, file.display(), line_number),
                    debt_type,
                    priority,
                    file: file.to_path_buf(),
                    line: line_number,
                    column: None,
                    message: format!("{marker}: {message}"),
                    context: Some(line.trim().to_string()),
                })
            })
        })
        .collect()
}

fn classify_marker(marker: &str) -> (DebtType, Priority) {
    match marker {
        "FIXME" | "BUG" => (
            DebtType::Fixme {
                reason: Some(marker.to_string()),
            },
            Priority::High,
        ),
        "TODO" => (DebtType::Todo { reason: None }, Priority::Medium),
        "HACK" | "XXX" => (
            DebtType::CodeSmell {
                smell_type: Some(marker.to_string()),
            },
            Priority::High,
        ),
        "OPTIMIZE" => (
            DebtType::CodeSmell {
                smell_type: Some("optimization".to_string()),
            },
            Priority::Low,
        ),
        "REFACTOR" => (
            DebtType::CodeSmell {
                smell_type: Some("refactoring".to_string()),
            },
            Priority::Medium,
        ),
        _ => (DebtType::Todo { reason: None }, Priority::Low),
    }
}

pub fn find_code_smells(content: &str, file: &Path) -> Vec<DebtItem> {
    find_code_smells_with_suppression(content, file, None)
}

pub fn find_code_smells_with_suppression(
    content: &str,
    file: &Path,
    suppression: Option<&SuppressionContext>,
) -> Vec<DebtItem> {
    const LONG_LINE_THRESHOLD: usize = 120;
    const DEEP_NESTING_THRESHOLD: usize = 4;

    content
        .lines()
        .enumerate()
        .flat_map(|(line_num, line)| {
            let line_number = line_num + 1;
            let file_path = file.to_path_buf();

            // Check if this line is suppressed for code smells
            if let Some(ctx) = suppression {
                if ctx.is_suppressed(line_number, &DebtType::CodeSmell { smell_type: None }) {
                    return vec![];
                }
            }

            [
                check_line_length(line, line_number, &file_path, LONG_LINE_THRESHOLD),
                check_nesting_level(line, line_number, &file_path, DEEP_NESTING_THRESHOLD),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
        })
        .collect()
}

fn check_line_length(
    line: &str,
    line_number: usize,
    file_path: &Path,
    threshold: usize,
) -> Option<DebtItem> {
    (line.len() > threshold).then(|| DebtItem {
        id: format!("long-line-{}-{}", file_path.display(), line_number),
        debt_type: DebtType::CodeSmell {
            smell_type: Some("long_line".to_string()),
        },
        priority: Priority::Low,
        file: file_path.to_path_buf(),
        line: line_number,
        column: None,
        message: format!("Line exceeds {} characters ({})", threshold, line.len()),
        context: None,
    })
}

fn check_nesting_level(
    line: &str,
    line_number: usize,
    file_path: &Path,
    threshold: usize,
) -> Option<DebtItem> {
    let indent_count = line.chars().take_while(|c| c.is_whitespace()).count() / 4;

    (indent_count > threshold).then(|| DebtItem {
        id: format!("deep-nesting-{}-{}", file_path.display(), line_number),
        debt_type: DebtType::CodeSmell {
            smell_type: Some("deep_nesting".to_string()),
        },
        priority: Priority::Medium,
        file: file_path.to_path_buf(),
        line: line_number,
        column: None,
        message: format!("Deep nesting level: {indent_count}"),
        context: Some(line.trim().to_string()),
    })
}

pub fn detect_duplicate_strings(content: &str, file: &Path) -> Vec<DebtItem> {
    let string_occurrences = extract_string_occurrences(content);
    let string_counts = group_string_occurrences(string_occurrences);

    string_counts
        .into_iter()
        .filter(|(_, lines)| lines.len() > 2)
        .map(|(string, lines)| create_duplicate_string_item(&string, &lines, file))
        .collect()
}

fn extract_string_occurrences(content: &str) -> Vec<(String, usize)> {
    let string_regex = Regex::new(r#"["']([^"']{20,})["']"#).unwrap();

    content
        .lines()
        .enumerate()
        .flat_map(|(line_num, line)| extract_strings_from_line(&string_regex, line, line_num + 1))
        .collect()
}

fn extract_strings_from_line(
    regex: &Regex,
    line: &str,
    line_number: usize,
) -> Vec<(String, usize)> {
    regex
        .captures_iter(line)
        .filter_map(|captures| captures.get(1))
        .map(|matched| (matched.as_str().to_string(), line_number))
        .collect()
}

fn group_string_occurrences(occurrences: Vec<(String, usize)>) -> HashMap<String, Vec<usize>> {
    occurrences
        .into_iter()
        .fold(HashMap::new(), |mut acc, (string, line)| {
            acc.entry(string).or_default().push(line);
            acc
        })
}

fn create_duplicate_string_item(string: &str, lines: &[usize], file: &Path) -> DebtItem {
    let truncated_string = if string.len() > 50 {
        &string[..50]
    } else {
        string
    };

    DebtItem {
        id: format!("duplicate-string-{}-{}", file.display(), lines[0]),
        debt_type: DebtType::Duplication {
            instances: lines.len() as u32,
            total_lines: lines.len() as u32,
        },
        priority: Priority::Low,
        file: file.to_path_buf(),
        line: lines[0],
        column: None,
        message: format!(
            "String '{}' appears {} times",
            truncated_string,
            lines.len()
        ),
        context: Some(format!("Lines: {lines:?}")),
    }
}

pub fn combine_debt_items(items: Vec<Vec<DebtItem>>) -> Vec<DebtItem> {
    items.into_iter().flatten().collect()
}

pub fn deduplicate_debt_items(items: Vec<DebtItem>) -> Vec<DebtItem> {
    use std::collections::HashSet;

    items
        .into_iter()
        .fold(
            (HashSet::new(), Vec::new()),
            |(mut seen, mut unique), item| {
                if seen.insert(item.id.clone()) {
                    unique.push(item);
                }
                (seen, unique)
            },
        )
        .1
}
