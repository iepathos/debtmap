use crate::core::{DebtType, Language};
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SuppressionContext {
    pub active_blocks: Vec<SuppressionBlock>,
    pub line_suppressions: HashMap<usize, SuppressionRule>,
    pub unclosed_blocks: Vec<UnclosedBlock>,
}

#[derive(Debug, Clone)]
pub struct SuppressionBlock {
    pub start_line: usize,
    pub end_line: Option<usize>,
    pub debt_types: Vec<DebtType>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SuppressionRule {
    pub debt_types: Vec<DebtType>,
    pub reason: Option<String>,
    pub applies_to_next_line: bool,
}

#[derive(Debug, Clone)]
pub struct UnclosedBlock {
    pub file: PathBuf,
    pub start_line: usize,
}

#[derive(Debug, Clone)]
pub struct SuppressionStats {
    pub total_suppressions: usize,
    pub suppressions_by_type: HashMap<DebtType, usize>,
    pub unclosed_blocks: Vec<UnclosedBlock>,
}

impl SuppressionContext {
    pub fn new() -> Self {
        Self {
            active_blocks: Vec::new(),
            line_suppressions: HashMap::new(),
            unclosed_blocks: Vec::new(),
        }
    }

    pub fn is_suppressed(&self, line: usize, debt_type: &DebtType) -> bool {
        // Check if line is within any active suppression block
        for block in &self.active_blocks {
            if line >= block.start_line {
                if let Some(end_line) = block.end_line {
                    if line <= end_line
                        && (block.debt_types.is_empty() || block.debt_types.contains(debt_type))
                    {
                        return true;
                    }
                }
            }
        }

        // Check line-specific suppressions
        if let Some(rule) = self.line_suppressions.get(&line) {
            if rule.debt_types.is_empty() || rule.debt_types.contains(debt_type) {
                return true;
            }
        }

        // Check if previous line has a next-line suppression
        if line > 0 {
            if let Some(rule) = self.line_suppressions.get(&(line - 1)) {
                if rule.applies_to_next_line
                    && (rule.debt_types.is_empty() || rule.debt_types.contains(debt_type))
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_stats(&self) -> SuppressionStats {
        let mut total = 0;
        let mut by_type = HashMap::new();

        for block in &self.active_blocks {
            if block.end_line.is_some() {
                total += 1;
                for debt_type in &block.debt_types {
                    *by_type.entry(*debt_type).or_insert(0) += 1;
                }
            }
        }

        for rule in self.line_suppressions.values() {
            total += 1;
            for debt_type in &rule.debt_types {
                *by_type.entry(*debt_type).or_insert(0) += 1;
            }
        }

        SuppressionStats {
            total_suppressions: total,
            suppressions_by_type: by_type,
            unclosed_blocks: self.unclosed_blocks.clone(),
        }
    }
}

impl Default for SuppressionContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn parse_suppression_comments(
    content: &str,
    language: Language,
    file: &Path,
) -> SuppressionContext {
    let mut context = SuppressionContext::new();

    // Determine comment prefix based on language
    let comment_prefix = match language {
        Language::Python => "#",
        Language::Rust => "//",
        _ => "//",
    };

    // Build regex patterns
    let block_start_pattern = format!(
        r"(?m)^\s*{}\s*debtmap:ignore-start(?:\[([\w,*]+)\])?(?:\s*--\s*(.*))?$",
        regex::escape(comment_prefix)
    );
    let block_end_pattern = format!(
        r"(?m)^\s*{}\s*debtmap:ignore-end\s*$",
        regex::escape(comment_prefix)
    );
    let line_pattern = format!(
        r"(?m){}\s*debtmap:ignore(?:\[([\w,*]+)\])?(?:\s*--\s*(.*))?$",
        regex::escape(comment_prefix)
    );
    let next_line_pattern = format!(
        r"(?m)^\s*{}\s*debtmap:ignore-next-line(?:\[([\w,*]+)\])?(?:\s*--\s*(.*))?$",
        regex::escape(comment_prefix)
    );

    let block_start_re = Regex::new(&block_start_pattern).unwrap();
    let block_end_re = Regex::new(&block_end_pattern).unwrap();
    let line_re = Regex::new(&line_pattern).unwrap();
    let next_line_re = Regex::new(&next_line_pattern).unwrap();

    let mut open_blocks: Vec<(usize, Vec<DebtType>, Option<String>)> = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        let line_number = line_num + 1; // 1-indexed

        // Check for block start
        if let Some(captures) = block_start_re.captures(line) {
            let debt_types = parse_debt_types(captures.get(1).map(|m| m.as_str()));
            let reason = captures.get(2).map(|m| m.as_str().to_string());
            open_blocks.push((line_number, debt_types, reason));
            continue;
        }

        // Check for block end
        if block_end_re.is_match(line) {
            if let Some((start_line, debt_types, reason)) = open_blocks.pop() {
                context.active_blocks.push(SuppressionBlock {
                    start_line,
                    end_line: Some(line_number),
                    debt_types,
                    reason,
                });
            }
            continue;
        }

        // Check for next-line suppression
        if let Some(captures) = next_line_re.captures(line) {
            let debt_types = parse_debt_types(captures.get(1).map(|m| m.as_str()));
            let reason = captures.get(2).map(|m| m.as_str().to_string());
            context.line_suppressions.insert(
                line_number,
                SuppressionRule {
                    debt_types,
                    reason,
                    applies_to_next_line: true,
                },
            );
            continue;
        }

        // Check for same-line suppression
        if let Some(captures) = line_re.captures(line) {
            let debt_types = parse_debt_types(captures.get(1).map(|m| m.as_str()));
            let reason = captures.get(2).map(|m| m.as_str().to_string());
            context.line_suppressions.insert(
                line_number,
                SuppressionRule {
                    debt_types,
                    reason,
                    applies_to_next_line: false,
                },
            );
        }
    }

    // Record any unclosed blocks
    for (start_line, _, _) in open_blocks {
        context.unclosed_blocks.push(UnclosedBlock {
            file: file.to_path_buf(),
            start_line,
        });
    }

    context
}

fn parse_debt_types(types_str: Option<&str>) -> Vec<DebtType> {
    match types_str {
        Some("*") => vec![], // Empty vector means all types
        Some(types) => types
            .split(',')
            .filter_map(|t| match t.trim().to_lowercase().as_str() {
                "todo" => Some(DebtType::Todo),
                "fixme" => Some(DebtType::Fixme),
                "smell" | "codesmell" => Some(DebtType::CodeSmell),
                "duplication" | "duplicate" => Some(DebtType::Duplication),
                "complexity" => Some(DebtType::Complexity),
                "dependency" => Some(DebtType::Dependency),
                _ => None,
            })
            .collect(),
        None => vec![], // No types specified means all types
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_block_suppression() {
        let content = r#"
// debtmap:ignore-start
// TODO: This should be suppressed
// FIXME: This too
// debtmap:ignore-end
// TODO: This should not be suppressed
"#;
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert_eq!(context.active_blocks.len(), 1);
        assert_eq!(context.active_blocks[0].start_line, 2);
        assert_eq!(context.active_blocks[0].end_line, Some(5));
        assert!(context.is_suppressed(3, &DebtType::Todo));
        assert!(context.is_suppressed(4, &DebtType::Fixme));
        assert!(!context.is_suppressed(6, &DebtType::Todo));
    }

    #[test]
    fn test_parse_line_suppression() {
        let content = r#"
// TODO: Not suppressed
// TODO: Suppressed // debtmap:ignore
// FIXME: Also not suppressed
"#;
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert!(!context.is_suppressed(2, &DebtType::Todo));
        assert!(context.is_suppressed(3, &DebtType::Todo));
        assert!(!context.is_suppressed(4, &DebtType::Fixme));
    }

    #[test]
    fn test_parse_next_line_suppression() {
        let content = r#"
// debtmap:ignore-next-line
// TODO: This should be suppressed
// TODO: This should not be suppressed
"#;
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert!(context.is_suppressed(3, &DebtType::Todo));
        assert!(!context.is_suppressed(4, &DebtType::Todo));
    }

    #[test]
    fn test_type_specific_suppression() {
        let content = r#"
// debtmap:ignore-start[todo]
// TODO: Suppressed
// FIXME: Not suppressed
// debtmap:ignore-end
"#;
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert!(context.is_suppressed(3, &DebtType::Todo));
        assert!(!context.is_suppressed(4, &DebtType::Fixme));
    }

    #[test]
    fn test_suppression_with_reason() {
        let content = r#"
// debtmap:ignore-start -- Test fixture
// TODO: Suppressed with reason
// debtmap:ignore-end
"#;
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert_eq!(
            context.active_blocks[0].reason,
            Some("Test fixture".to_string())
        );
    }

    #[test]
    fn test_unclosed_block_detection() {
        let content = r#"
// debtmap:ignore-start
// TODO: In unclosed block
"#;
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert_eq!(context.unclosed_blocks.len(), 1);
        assert_eq!(context.unclosed_blocks[0].start_line, 2);
    }

    #[test]
    fn test_python_comment_syntax() {
        let content = r#"
# debtmap:ignore-start
# TODO: Python TODO
# debtmap:ignore-end
"#;
        let file = PathBuf::from("test.py");
        let context = parse_suppression_comments(content, Language::Python, &file);

        assert_eq!(context.active_blocks.len(), 1);
        assert!(context.is_suppressed(3, &DebtType::Todo));
    }

    #[test]
    fn test_wildcard_suppression() {
        let content = "// TODO: Test // debtmap:ignore[*]";
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        // Line 1 has the wildcard suppression that applies to the same line
        assert!(context.is_suppressed(1, &DebtType::Todo));
        assert!(context.is_suppressed(1, &DebtType::Fixme));
        assert!(context.is_suppressed(1, &DebtType::CodeSmell));
    }
}
