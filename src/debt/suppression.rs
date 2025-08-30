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
        // Check all suppression sources using lazy evaluation
        [
            self.is_in_suppression_block(line, debt_type),
            self.has_line_suppression(line, debt_type),
            self.has_next_line_suppression(line, debt_type),
        ]
        .into_iter()
        .any(|suppressed| suppressed)
    }

    fn is_in_suppression_block(&self, line: usize, debt_type: &DebtType) -> bool {
        self.active_blocks
            .iter()
            .filter(|block| line_within_block(line, block))
            .any(|block| debt_type_matches(debt_type, &block.debt_types))
    }

    fn has_line_suppression(&self, line: usize, debt_type: &DebtType) -> bool {
        self.line_suppressions
            .get(&line)
            .is_some_and(|rule| debt_type_matches(debt_type, &rule.debt_types))
    }

    fn has_next_line_suppression(&self, line: usize, debt_type: &DebtType) -> bool {
        (line > 0)
            .then(|| self.line_suppressions.get(&(line - 1)))
            .flatten()
            .is_some_and(|rule| {
                rule.applies_to_next_line && debt_type_matches(debt_type, &rule.debt_types)
            })
    }

    pub fn get_stats(&self) -> SuppressionStats {
        let completed_blocks = self.count_completed_blocks();
        let all_debt_types = self.collect_all_debt_types();
        let total_suppressions = completed_blocks + self.line_suppressions.len();
        let suppressions_by_type = self.count_suppressions_by_type(all_debt_types);

        SuppressionStats {
            total_suppressions,
            suppressions_by_type,
            unclosed_blocks: self.unclosed_blocks.clone(),
        }
    }

    fn count_completed_blocks(&self) -> usize {
        self.active_blocks
            .iter()
            .filter(|block| block.end_line.is_some())
            .count()
    }

    fn collect_all_debt_types(&self) -> Vec<DebtType> {
        let block_types = self
            .active_blocks
            .iter()
            .filter(|block| block.end_line.is_some())
            .flat_map(|block| block.debt_types.iter().copied());

        let line_types = self
            .line_suppressions
            .values()
            .flat_map(|rule| rule.debt_types.iter().copied());

        block_types.chain(line_types).collect()
    }

    fn count_suppressions_by_type(&self, debt_types: Vec<DebtType>) -> HashMap<DebtType, usize> {
        debt_types
            .into_iter()
            .fold(HashMap::new(), |mut acc, debt_type| {
                *acc.entry(debt_type).or_insert(0) += 1;
                acc
            })
    }
}

impl Default for SuppressionContext {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions extracted as pure functions for better testability
fn line_within_block(line: usize, block: &SuppressionBlock) -> bool {
    // Special case: line 0 means "unknown line" and should be suppressed if the block starts at line 1
    // This handles cases where the AST doesn't provide exact line numbers
    if line == 0 && block.start_line == 1 {
        return true;
    }
    line >= block.start_line && block.end_line.is_some_and(|end| line <= end)
}

fn debt_type_matches(debt_type: &DebtType, allowed_types: &[DebtType]) -> bool {
    allowed_types.is_empty() || allowed_types.contains(debt_type)
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
    // Try each pattern in order and return the first match
    try_parse_block_start(line, line_number, patterns)
        .or_else(|| try_parse_block_end(line, line_number, patterns))
        .or_else(|| try_parse_next_line(line, line_number, patterns))
        .or_else(|| try_parse_line_suppression(line, line_number, patterns))
        .unwrap_or(LineParseResult::None)
}

fn try_parse_block_start(
    line: &str,
    line_number: usize,
    patterns: &SuppressionPatterns,
) -> Option<LineParseResult> {
    patterns.block_start.captures(line).map(|captures| {
        LineParseResult::BlockStart(
            line_number,
            parse_debt_types(captures.get(1).map(|m| m.as_str())),
            captures.get(2).map(|m| m.as_str().to_string()),
        )
    })
}

fn try_parse_block_end(
    line: &str,
    line_number: usize,
    patterns: &SuppressionPatterns,
) -> Option<LineParseResult> {
    patterns
        .block_end
        .is_match(line)
        .then_some(LineParseResult::BlockEnd(line_number))
}

fn try_parse_next_line(
    line: &str,
    line_number: usize,
    patterns: &SuppressionPatterns,
) -> Option<LineParseResult> {
    patterns.next_line.captures(line).map(|captures| {
        LineParseResult::NextLineSuppression(
            line_number,
            parse_debt_types(captures.get(1).map(|m| m.as_str())),
            captures.get(2).map(|m| m.as_str().to_string()),
        )
    })
}

fn try_parse_line_suppression(
    line: &str,
    line_number: usize,
    patterns: &SuppressionPatterns,
) -> Option<LineParseResult> {
    patterns.line.captures(line).map(|captures| {
        LineParseResult::LineSuppression(
            line_number,
            parse_debt_types(captures.get(1).map(|m| m.as_str())),
            captures.get(2).map(|m| m.as_str().to_string()),
        )
    })
}

fn process_parsed_line(
    result: LineParseResult,
    context: &mut SuppressionContext,
    open_blocks: &mut Vec<(usize, Vec<DebtType>, Option<String>)>,
) {
    use LineParseResult::*;

    match result {
        BlockStart(ln, types, reason) => open_blocks.push((ln, types, reason)),
        BlockEnd(end_line) => handle_block_end(context, open_blocks, end_line),
        NextLineSuppression(ln, types, reason) => {
            add_line_suppression(context, ln, types, reason, true)
        }
        LineSuppression(ln, types, reason) => {
            add_line_suppression(context, ln, types, reason, false)
        }
        None => {}
    }
}

fn handle_block_end(
    context: &mut SuppressionContext,
    open_blocks: &mut Vec<(usize, Vec<DebtType>, Option<String>)>,
    end_line: usize,
) {
    if let Some((start_line, debt_types, reason)) = open_blocks.pop() {
        context.active_blocks.push(SuppressionBlock {
            start_line,
            end_line: Some(end_line),
            debt_types,
            reason,
        });
    }
}

fn add_line_suppression(
    context: &mut SuppressionContext,
    line: usize,
    debt_types: Vec<DebtType>,
    reason: Option<String>,
    applies_to_next_line: bool,
) {
    context.line_suppressions.insert(
        line,
        SuppressionRule {
            debt_types,
            reason,
            applies_to_next_line,
        },
    );
}

/// Transforms open blocks into unclosed block markers
fn create_unclosed_blocks(
    open_blocks: Vec<(usize, Vec<DebtType>, Option<String>)>,
    file: &Path,
) -> Vec<UnclosedBlock> {
    open_blocks
        .into_iter()
        .map(|(start_line, _, _)| UnclosedBlock {
            file: file.to_path_buf(),
            start_line,
        })
        .collect()
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
        .for_each(|(line_number, line)| {
            let result = parse_line(line, line_number, &patterns);
            process_parsed_line(result, &mut context, &mut open_blocks);
        });

    // Record any unclosed blocks
    context.unclosed_blocks = create_unclosed_blocks(open_blocks, file);

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

    #[test]
    fn test_create_unclosed_blocks() {
        let open_blocks = vec![
            (10, vec![DebtType::Todo], Some("reason1".to_string())),
            (25, vec![DebtType::Fixme], None),
            (
                42,
                vec![DebtType::CodeSmell, DebtType::Complexity],
                Some("reason2".to_string()),
            ),
        ];
        let file = Path::new("test_file.rs");

        let unclosed = create_unclosed_blocks(open_blocks, file);

        assert_eq!(unclosed.len(), 3);
        assert_eq!(unclosed[0].start_line, 10);
        assert_eq!(unclosed[0].file, PathBuf::from("test_file.rs"));
        assert_eq!(unclosed[1].start_line, 25);
        assert_eq!(unclosed[2].start_line, 42);
    }

    #[test]
    fn test_create_unclosed_blocks_empty() {
        let open_blocks = vec![];
        let file = Path::new("empty.rs");

        let unclosed = create_unclosed_blocks(open_blocks, file);

        assert!(unclosed.is_empty());
    }
}
