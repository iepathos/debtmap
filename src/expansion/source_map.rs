//! Source mapping between expanded and original code

use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Maps expanded code locations back to original source
#[derive(Debug, Clone)]
pub struct SourceMap {
    mappings: Vec<SourceMapping>,
}

/// A single source location mapping
#[derive(Debug, Clone)]
pub struct SourceMapping {
    /// Line number in expanded code
    pub expanded_line: usize,
    /// Original source file
    pub original_file: PathBuf,
    /// Line number in original source
    pub original_line: usize,
    /// Whether this line is macro-generated (not in original)
    pub is_macro_generated: bool,
}

impl SourceMap {
    /// Create a source map from expanded code
    pub fn from_expanded(expanded: &str, original_path: &Path) -> Result<Self> {
        let mut mappings = Vec::new();

        // Regex to match line directives from cargo expand
        // Format: #[line = 123] or # 123 "file.rs"
        let line_directive_re = Regex::new(r#"^#(?:\[line\s*=\s*(\d+)\]|\s+(\d+)\s+"([^"]+)")"#)
            .context("Failed to compile line directive regex")?;

        let mut current_file = original_path.to_path_buf();
        let mut current_original_line = 1;
        let mut is_macro_generated = false;

        for (expanded_line_idx, line) in expanded.lines().enumerate() {
            let expanded_line = expanded_line_idx + 1;

            // Check for line directives
            if let Some(captures) = line_directive_re.captures(line) {
                if let Some(line_match) = captures.get(1) {
                    // Format: #[line = 123]
                    current_original_line = line_match
                        .as_str()
                        .parse()
                        .context("Failed to parse line number")?;
                    is_macro_generated = false;
                } else if let (Some(line_match), Some(file_match)) =
                    (captures.get(2), captures.get(3))
                {
                    // Format: # 123 "file.rs"
                    current_original_line = line_match
                        .as_str()
                        .parse()
                        .context("Failed to parse line number")?;
                    current_file = PathBuf::from(file_match.as_str());
                    is_macro_generated = false;
                }
                continue; // Don't map the directive itself
            }

            // Check for macro-generated code markers
            if line.contains("// macro-generated") || line.contains("__macro_expanded") {
                is_macro_generated = true;
            }

            // Create mapping for this line
            mappings.push(SourceMapping {
                expanded_line,
                original_file: current_file.clone(),
                original_line: current_original_line,
                is_macro_generated,
            });

            // Increment original line unless we're in macro-generated code
            if !is_macro_generated {
                current_original_line += 1;
            }
        }

        Ok(Self { mappings })
    }

    /// Create a source map from existing mappings
    pub fn from_mappings(mappings: Vec<SourceMapping>) -> Self {
        Self { mappings }
    }

    /// Get all mappings
    pub fn mappings(&self) -> &[SourceMapping] {
        &self.mappings
    }

    /// Find the original location for an expanded line
    pub fn get_original(&self, expanded_line: usize) -> Option<&SourceMapping> {
        self.mappings
            .iter()
            .find(|m| m.expanded_line == expanded_line)
    }

    /// Find all expanded lines for an original location
    pub fn get_expanded(&self, original_file: &Path, original_line: usize) -> Vec<usize> {
        self.mappings
            .iter()
            .filter(|m| {
                m.original_file == original_file
                    && m.original_line == original_line
                    && !m.is_macro_generated
            })
            .map(|m| m.expanded_line)
            .collect()
    }

    /// Check if a line is macro-generated
    pub fn is_macro_generated(&self, expanded_line: usize) -> bool {
        self.get_original(expanded_line)
            .map(|m| m.is_macro_generated)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_map_parsing() {
        let expanded = r#"
#[line = 10]
fn foo() {
    println!("hello");
}
#[line = 15]
fn bar() {
    // macro-generated
    format!("world");
}
"#;

        let original_path = Path::new("src/lib.rs");
        let source_map = SourceMap::from_expanded(expanded, original_path).unwrap();

        // Check that we have the right number of mappings
        assert!(!source_map.mappings.is_empty());

        // Check that macro-generated lines are detected
        let macro_lines: Vec<_> = source_map
            .mappings
            .iter()
            .filter(|m| m.is_macro_generated)
            .collect();
        assert!(!macro_lines.is_empty());
    }
}
