use crate::core::{DebtType, Language};
use once_cell::sync::Lazy;
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
        let in_block = self.active_blocks.iter().any(|block| {
            line >= block.start_line
                && block.end_line.is_some_and(|end| line <= end)
                && (block.debt_types.is_empty() || block.debt_types.contains(debt_type))
        });

        if in_block {
            return true;
        }

        // Check line-specific suppressions
        let line_suppressed = self
            .line_suppressions
            .get(&line)
            .is_some_and(|rule| rule.debt_types.is_empty() || rule.debt_types.contains(debt_type));

        if line_suppressed {
            return true;
        }

        // Check if previous line has a next-line suppression
        line > 0
            && self.line_suppressions.get(&(line - 1)).is_some_and(|rule| {
                rule.applies_to_next_line
                    && (rule.debt_types.is_empty() || rule.debt_types.contains(debt_type))
            })
    }

    pub fn get_stats(&self) -> SuppressionStats {
        let block_stats = self
            .active_blocks
            .iter()
            .filter(|block| block.end_line.is_some())
            .flat_map(|block| block.debt_types.iter().copied());

        let line_stats = self
            .line_suppressions
            .values()
            .flat_map(|rule| rule.debt_types.iter().copied());

        let all_debt_types: Vec<DebtType> = block_stats.chain(line_stats).collect();

        let total_suppressions = self
            .active_blocks
            .iter()
            .filter(|block| block.end_line.is_some())
            .count()
            + self.line_suppressions.len();

        let mut suppressions_by_type = HashMap::new();
        for debt_type in all_debt_types {
            *suppressions_by_type.entry(debt_type).or_insert(0) += 1;
        }

        SuppressionStats {
            total_suppressions,
            suppressions_by_type,
            unclosed_blocks: self.unclosed_blocks.clone(),
        }
    }
}

impl Default for SuppressionContext {
    fn default() -> Self {
        Self::new()
    }
}

struct SuppressionPatterns {
    block_start: Regex,
    block_end: Regex,
    line: Regex,
    next_line: Regex,
}

impl SuppressionPatterns {
    fn new(language: Language) -> Self {
        let comment_prefix = get_comment_prefix(language);
        let escaped_prefix = regex::escape(comment_prefix);

        Self {
            block_start: Regex::new(&format!(
                r"(?m)^\s*{escaped_prefix}\s*debtmap:ignore-start(?:\s*\[([\w,*]+)\])?(?:\s*--\s*(.*))?$"
            )).unwrap(),
            block_end: Regex::new(&format!(
                r"(?m)^\s*{escaped_prefix}\s*debtmap:ignore-end\s*$"
            )).unwrap(),
            line: Regex::new(&format!(
                r"(?m){escaped_prefix}\s*debtmap:ignore(?:\s*\[([\w,*]+)\])?(?:\s*--\s*(.*))?$"
            )).unwrap(),
            next_line: Regex::new(&format!(
                r"(?m)^\s*{escaped_prefix}\s*debtmap:ignore-next-line(?:\s*\[([\w,*]+)\])?(?:\s*--\s*(.*))?$"
            )).unwrap(),
        }
    }
}

fn get_comment_prefix(language: Language) -> &'static str {
    match language {
        Language::Python => "#",
        Language::Rust | Language::JavaScript | Language::TypeScript => "//",
        _ => "//",
    }
}

enum LineParseResult {
    BlockStart(usize, Vec<DebtType>, Option<String>),
    BlockEnd(usize),
    NextLineSuppression(usize, Vec<DebtType>, Option<String>),
    LineSuppression(usize, Vec<DebtType>, Option<String>),
    None,
}

fn parse_line(line: &str, line_number: usize, patterns: &SuppressionPatterns) -> LineParseResult {
    if let Some(captures) = patterns.block_start.captures(line) {
        return LineParseResult::BlockStart(
            line_number,
            parse_debt_types(captures.get(1).map(|m| m.as_str())),
            captures.get(2).map(|m| m.as_str().to_string()),
        );
    }

    if patterns.block_end.is_match(line) {
        return LineParseResult::BlockEnd(line_number);
    }

    if let Some(captures) = patterns.next_line.captures(line) {
        return LineParseResult::NextLineSuppression(
            line_number,
            parse_debt_types(captures.get(1).map(|m| m.as_str())),
            captures.get(2).map(|m| m.as_str().to_string()),
        );
    }

    if let Some(captures) = patterns.line.captures(line) {
        return LineParseResult::LineSuppression(
            line_number,
            parse_debt_types(captures.get(1).map(|m| m.as_str())),
            captures.get(2).map(|m| m.as_str().to_string()),
        );
    }

    LineParseResult::None
}

pub fn parse_suppression_comments(
    content: &str,
    language: Language,
    file: &Path,
) -> SuppressionContext {
    let patterns = SuppressionPatterns::new(language);
    let mut context = SuppressionContext::new();
    let mut open_blocks: Vec<(usize, Vec<DebtType>, Option<String>)> = Vec::new();

    content
        .lines()
        .enumerate()
        .map(|(idx, line)| (idx + 1, line))
        .for_each(
            |(line_number, line)| match parse_line(line, line_number, &patterns) {
                LineParseResult::BlockStart(ln, types, reason) => {
                    open_blocks.push((ln, types, reason));
                }
                LineParseResult::BlockEnd(end_line) => {
                    if let Some((start_line, debt_types, reason)) = open_blocks.pop() {
                        context.active_blocks.push(SuppressionBlock {
                            start_line,
                            end_line: Some(end_line),
                            debt_types,
                            reason,
                        });
                    }
                }
                LineParseResult::NextLineSuppression(ln, types, reason) => {
                    context.line_suppressions.insert(
                        ln,
                        SuppressionRule {
                            debt_types: types,
                            reason,
                            applies_to_next_line: true,
                        },
                    );
                }
                LineParseResult::LineSuppression(ln, types, reason) => {
                    context.line_suppressions.insert(
                        ln,
                        SuppressionRule {
                            debt_types: types,
                            reason,
                            applies_to_next_line: false,
                        },
                    );
                }
                LineParseResult::None => {}
            },
        );

    // Record any unclosed blocks
    context.unclosed_blocks = open_blocks
        .into_iter()
        .map(|(start_line, _, _)| UnclosedBlock {
            file: file.to_path_buf(),
            start_line,
        })
        .collect();

    context
}

static DEBT_TYPE_MAP: Lazy<HashMap<&'static str, DebtType>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("todo", DebtType::Todo);
    map.insert("fixme", DebtType::Fixme);
    map.insert("smell", DebtType::CodeSmell);
    map.insert("codesmell", DebtType::CodeSmell);
    map.insert("duplication", DebtType::Duplication);
    map.insert("duplicate", DebtType::Duplication);
    map.insert("complexity", DebtType::Complexity);
    map.insert("dependency", DebtType::Dependency);
    map
});

fn parse_debt_types(types_str: Option<&str>) -> Vec<DebtType> {
    // Early return for special cases
    let Some(types) = types_str else {
        return vec![]; // No types specified means all types
    };

    if types == "*" {
        return vec![]; // Empty vector means all types
    }

    // Use a static mapping for type conversion
    types
        .split(',')
        .filter_map(|t| parse_single_debt_type(t.trim()))
        .collect()
}

fn parse_single_debt_type(type_str: &str) -> Option<DebtType> {
    DEBT_TYPE_MAP.get(type_str.to_lowercase().as_str()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_block_suppression() {
        // debtmap:ignore-start -- Test fixture data
        let content = r#"
// debtmap:ignore-start
// TODO: This should be suppressed
// FIXME: This too
// debtmap:ignore-end
// TODO: This should not be suppressed
"#;
        // debtmap:ignore-end
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
        // debtmap:ignore-start -- Test fixture data
        let content = r#"
// TODO: Not suppressed
// TODO: Suppressed // debtmap:ignore
// FIXME: Also not suppressed
"#;
        // debtmap:ignore-end
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert!(!context.is_suppressed(2, &DebtType::Todo));
        assert!(context.is_suppressed(3, &DebtType::Todo));
        assert!(!context.is_suppressed(4, &DebtType::Fixme));
    }

    #[test]
    fn test_parse_next_line_suppression() {
        // debtmap:ignore-start -- Test fixture data
        let content = r#"
// debtmap:ignore-next-line
// TODO: This should be suppressed
// TODO: This should not be suppressed
"#;
        // debtmap:ignore-end
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert!(context.is_suppressed(3, &DebtType::Todo));
        assert!(!context.is_suppressed(4, &DebtType::Todo));
    }

    #[test]
    fn test_type_specific_suppression() {
        // debtmap:ignore-start -- Test fixture data
        let content = r#"
// debtmap:ignore-start[todo]
// TODO: Suppressed
// FIXME: Not suppressed
// debtmap:ignore-end
"#;
        // debtmap:ignore-end
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert!(context.is_suppressed(3, &DebtType::Todo));
        assert!(!context.is_suppressed(4, &DebtType::Fixme));
    }

    #[test]
    fn test_suppression_with_reason() {
        // debtmap:ignore-start -- Test fixture data
        let content = r#"
// debtmap:ignore-start -- Test fixture
// TODO: Suppressed with reason
// debtmap:ignore-end
"#;
        // debtmap:ignore-end
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(content, Language::Rust, &file);

        assert_eq!(
            context.active_blocks[0].reason,
            Some("Test fixture".to_string())
        );
    }

    #[test]
    fn test_unclosed_block_detection() {
        // Test content with intentionally unclosed block for testing
        let content = format!(
            "{}{}{}",
            "// debtmap:", "ignore-start\n", "// TODO: In unclosed block\n"
        );
        let file = PathBuf::from("test.rs");
        let context = parse_suppression_comments(&content, Language::Rust, &file);

        assert_eq!(context.unclosed_blocks.len(), 1);
        assert_eq!(context.unclosed_blocks[0].start_line, 1);
    }

    #[test]
    fn test_python_comment_syntax() {
        // debtmap:ignore-start -- Test fixture data
        let content = r#"
# debtmap:ignore-start
# TODO: Python TODO
# debtmap:ignore-end
"#;
        // debtmap:ignore-end
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
