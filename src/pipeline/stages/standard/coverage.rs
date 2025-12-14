//! Coverage loading stage with LCOV parsing.
//!
//! This module demonstrates the Stillwater "pure core, imperative shell" pattern:
//! - Pure functions: `parse_lcov_line`, `calculate_coverage_percentage`, `parse_lcov_content`
//! - I/O wrapper: `load_coverage_from_file`

use crate::errors::AnalysisError;
use crate::pipeline::data::{CoverageData, PipelineData};
use crate::pipeline::stage::Stage;
use std::path::{Path, PathBuf};

/// Stage 5: Load coverage data (optional)
///
/// Loads test coverage information from an LCOV file.
pub struct CoverageLoadingStage {
    coverage_path: PathBuf,
}

impl CoverageLoadingStage {
    pub fn new(coverage_path: &Path) -> Self {
        Self {
            coverage_path: coverage_path.to_path_buf(),
        }
    }
}

impl Stage for CoverageLoadingStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        let coverage = load_coverage_from_file(&self.coverage_path)?;
        data.coverage = Some(coverage);
        Ok(data)
    }

    fn name(&self) -> &str {
        "Coverage Loading"
    }
}

// =============================================================================
// LCOV Parsing - Pure Core
// =============================================================================

/// Represents a parsed LCOV line type.
#[derive(Debug, Clone, PartialEq)]
pub enum LcovLine {
    /// Source file declaration (SF:path)
    SourceFile(PathBuf),
    /// Data line with hit count (DA:line_number,hit_count)
    DataLine { hit: bool },
    /// End of record marker
    EndOfRecord,
    /// Line we don't care about (comments, other markers)
    Ignored,
}

/// Parse a single LCOV line into its type.
///
/// This is a pure function - no I/O, deterministic output.
pub fn parse_lcov_line(line: &str) -> LcovLine {
    if let Some(sf) = line.strip_prefix("SF:") {
        LcovLine::SourceFile(PathBuf::from(sf))
    } else if let Some(da) = line.strip_prefix("DA:") {
        let hit = da
            .split_once(',')
            .and_then(|(_, hit_str)| hit_str.parse::<i32>().ok())
            .map(|count| count > 0)
            .unwrap_or(false);
        LcovLine::DataLine { hit }
    } else if line == "end_of_record" {
        LcovLine::EndOfRecord
    } else {
        LcovLine::Ignored
    }
}

/// Calculate coverage percentage from line hits.
///
/// Pure function: (covered_lines / total_lines) * 100
pub fn calculate_coverage_percentage(line_hits: &[bool]) -> f64 {
    if line_hits.is_empty() {
        return 0.0;
    }
    let covered = line_hits.iter().filter(|&&hit| hit).count();
    (covered as f64 / line_hits.len() as f64) * 100.0
}

/// State accumulator for LCOV parsing.
#[derive(Default)]
struct LcovParseState {
    coverage: CoverageData,
    current_file: Option<PathBuf>,
    line_hits: Vec<bool>,
}

impl LcovParseState {
    /// Process a single parsed LCOV line, returning updated state.
    fn process_line(mut self, line: LcovLine) -> Self {
        match line {
            LcovLine::SourceFile(path) => {
                self.current_file = Some(path);
                self.line_hits.clear();
            }
            LcovLine::DataLine { hit } => {
                self.line_hits.push(hit);
            }
            LcovLine::EndOfRecord => {
                if let Some(file) = self.current_file.take() {
                    let pct = calculate_coverage_percentage(&self.line_hits);
                    self.coverage.file_coverage.insert(file.clone(), pct);
                    self.coverage
                        .line_coverage
                        .insert(file, std::mem::take(&mut self.line_hits));
                }
            }
            LcovLine::Ignored => {}
        }
        self
    }
}

/// Parse LCOV content into coverage data using functional composition.
///
/// Pure function: content in, coverage data out.
pub fn parse_lcov_content(content: &str) -> CoverageData {
    content
        .lines()
        .map(parse_lcov_line)
        .fold(LcovParseState::default(), LcovParseState::process_line)
        .coverage
}

// =============================================================================
// I/O Wrapper - Imperative Shell
// =============================================================================

/// Load coverage data from an LCOV file.
///
/// This is the thin I/O wrapper that delegates to pure parsing functions.
fn load_coverage_from_file(path: &Path) -> Result<CoverageData, AnalysisError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        AnalysisError::other(format!("Failed to read coverage file {:?}: {}", path, e))
    })?;

    Ok(parse_lcov_content(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    // === LCOV Parsing Tests ===

    #[test]
    fn parse_lcov_line_source_file() {
        let line = "SF:/path/to/file.rs";
        assert_eq!(
            parse_lcov_line(line),
            LcovLine::SourceFile(PathBuf::from("/path/to/file.rs"))
        );
    }

    #[test]
    fn parse_lcov_line_data_hit() {
        let line = "DA:10,5";
        assert_eq!(parse_lcov_line(line), LcovLine::DataLine { hit: true });
    }

    #[test]
    fn parse_lcov_line_data_no_hit() {
        let line = "DA:10,0";
        assert_eq!(parse_lcov_line(line), LcovLine::DataLine { hit: false });
    }

    #[test]
    fn parse_lcov_line_data_malformed() {
        let line = "DA:malformed";
        assert_eq!(parse_lcov_line(line), LcovLine::DataLine { hit: false });
    }

    #[test]
    fn parse_lcov_line_end_of_record() {
        assert_eq!(parse_lcov_line("end_of_record"), LcovLine::EndOfRecord);
    }

    #[test]
    fn parse_lcov_line_ignored() {
        assert_eq!(parse_lcov_line("TN:testname"), LcovLine::Ignored);
        assert_eq!(parse_lcov_line(""), LcovLine::Ignored);
        assert_eq!(parse_lcov_line("# comment"), LcovLine::Ignored);
    }

    #[test]
    fn calculate_coverage_empty() {
        assert_eq!(calculate_coverage_percentage(&[]), 0.0);
    }

    #[test]
    fn calculate_coverage_all_hit() {
        assert_eq!(calculate_coverage_percentage(&[true, true, true]), 100.0);
    }

    #[test]
    fn calculate_coverage_none_hit() {
        assert_eq!(calculate_coverage_percentage(&[false, false, false]), 0.0);
    }

    #[test]
    fn calculate_coverage_mixed() {
        assert_eq!(
            calculate_coverage_percentage(&[true, false, true, false]),
            50.0
        );
    }

    #[test]
    fn parse_lcov_content_single_file() {
        let content = "SF:/path/to/file.rs\nDA:1,1\nDA:2,0\nDA:3,1\nend_of_record\n";
        let coverage = parse_lcov_content(content);

        let path = PathBuf::from("/path/to/file.rs");
        assert!(coverage.file_coverage.contains_key(&path));

        let pct = coverage.file_coverage.get(&path).unwrap();
        assert!((pct - 66.666).abs() < 0.01);

        let lines = coverage.line_coverage.get(&path).unwrap();
        assert_eq!(lines, &vec![true, false, true]);
    }

    #[test]
    fn parse_lcov_content_multiple_files() {
        let content = "\
SF:/a.rs
DA:1,1
DA:2,1
end_of_record
SF:/b.rs
DA:1,0
end_of_record
";
        let coverage = parse_lcov_content(content);

        assert_eq!(coverage.file_coverage.len(), 2);
        assert_eq!(
            *coverage.file_coverage.get(&PathBuf::from("/a.rs")).unwrap(),
            100.0
        );
        assert_eq!(
            *coverage.file_coverage.get(&PathBuf::from("/b.rs")).unwrap(),
            0.0
        );
    }

    #[test]
    fn parse_lcov_content_empty() {
        let coverage = parse_lcov_content("");
        assert!(coverage.file_coverage.is_empty());
        assert!(coverage.line_coverage.is_empty());
    }
}
