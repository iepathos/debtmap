use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FunctionCoverage {
    pub name: String,
    pub start_line: usize,
    pub execution_count: u64,
    pub coverage_percentage: f64,
    pub uncovered_lines: Vec<usize>,
}

#[derive(Debug, Default, Clone)]
pub struct LcovData {
    pub functions: HashMap<PathBuf, Vec<FunctionCoverage>>,
    pub total_lines: usize,
    pub lines_hit: usize,
}

pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
    use lcov::{Reader, Record};

    let reader = Reader::open_file(path)
        .with_context(|| format!("Failed to open LCOV file: {}", path.display()))?;

    let mut data = LcovData::default();
    let mut current_file: Option<PathBuf> = None;
    let mut file_functions: HashMap<String, FunctionCoverage> = HashMap::new();
    let mut file_lines: HashMap<usize, u64> = HashMap::new();
    let mut _lines_found = 0;
    let mut _lines_hit = 0;

    for record in reader {
        let record = record.with_context(|| "Failed to parse LCOV record")?;

        match record {
            Record::SourceFile { path } => {
                // Save previous file's data if any
                if let Some(file) = current_file.take() {
                    if !file_functions.is_empty() {
                        let mut funcs: Vec<FunctionCoverage> =
                            file_functions.drain().map(|(_, v)| v).collect();
                        funcs.sort_by_key(|f| f.start_line);
                        data.functions.insert(file, funcs);
                    }
                }
                current_file = Some(path);
                file_functions.clear();
                file_lines.clear();
            }

            Record::FunctionName { start_line, name } => {
                file_functions
                    .entry(name.clone())
                    .or_insert_with(|| FunctionCoverage {
                        name,
                        start_line: start_line as usize,
                        execution_count: 0,
                        coverage_percentage: 0.0,
                        uncovered_lines: Vec::new(),
                    });
            }

            Record::FunctionData { name, count } => {
                if let Some(func) = file_functions.get_mut(&name) {
                    func.execution_count = count;
                    // If no line data is available, use execution count to determine coverage
                    // Functions with count > 0 are considered 100% covered, 0 means 0% covered
                    if func.coverage_percentage == 0.0 && count > 0 {
                        func.coverage_percentage = 100.0;
                    }
                }
            }

            Record::LineData { line, count, .. } => {
                file_lines.insert(line as usize, count);
                if count > 0 {
                    _lines_hit += 1;
                }
            }

            Record::LinesFound { found } => {
                _lines_found = found as usize;
                data.total_lines += found as usize;
            }

            Record::LinesHit { hit } => {
                _lines_hit = hit as usize;
                data.lines_hit += hit as usize;
            }

            Record::EndOfRecord => {
                // Collect all function start lines for boundary detection
                let func_start_lines: Vec<usize> =
                    file_functions.values().map(|f| f.start_line).collect();

                // Calculate coverage percentage for functions based on line data
                for func in file_functions.values_mut() {
                    // Find the next function start line to determine this function's boundary
                    let next_func_line = func_start_lines
                        .iter()
                        .filter(|line| **line > func.start_line)
                        .min()
                        .copied();

                    let func_lines: Vec<_> = file_lines
                        .iter()
                        .filter(|(line, _)| {
                            **line >= func.start_line
                                && next_func_line.is_none_or(|next| **line < next)
                        })
                        .collect();

                    if !func_lines.is_empty() {
                        let covered = func_lines.iter().filter(|(_, count)| **count > 0).count();
                        func.coverage_percentage =
                            (covered as f64 / func_lines.len() as f64) * 100.0;

                        // Collect uncovered lines
                        func.uncovered_lines = func_lines
                            .iter()
                            .filter(|(_, count)| **count == 0)
                            .map(|(line, _)| **line)
                            .collect();
                    } else if func.execution_count > 0 {
                        // If we have execution count but no line data, assume it's covered
                        func.coverage_percentage = 100.0;
                        func.uncovered_lines.clear();
                    }
                    // If no line data and no execution count, coverage remains 0.0 with no specific uncovered lines
                }

                // Save the file's data
                if let Some(file) = current_file.take() {
                    if !file_functions.is_empty() {
                        let mut funcs: Vec<FunctionCoverage> =
                            file_functions.drain().map(|(_, v)| v).collect();
                        funcs.sort_by_key(|f| f.start_line);
                        data.functions.insert(file, funcs);
                    }
                }

                file_functions.clear();
                file_lines.clear();
            }

            _ => {} // Ignore other record types
        }
    }

    // Handle case where file doesn't end with EndOfRecord
    if let Some(file) = current_file {
        if !file_functions.is_empty() {
            let mut funcs: Vec<FunctionCoverage> = file_functions.drain().map(|(_, v)| v).collect();
            funcs.sort_by_key(|f| f.start_line);
            data.functions.insert(file, funcs);
        }
    }

    Ok(data)
}

impl LcovData {
    pub fn get_function_coverage(&self, file: &Path, function_name: &str) -> Option<f64> {
        // Try exact match first, then use path normalization
        self.functions
            .get(file)
            .or_else(|| find_functions_by_path(&self.functions, file))
            .and_then(|funcs| {
                // Use a functional approach with multiple matching strategies
                find_function_by_name(funcs, function_name).map(|f| f.coverage_percentage / 100.0)
                // Convert percentage to fraction
            })
    }

    pub fn get_function_coverage_with_line(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<f64> {
        // Try exact match first, then use path normalization
        self.functions
            .get(file)
            .or_else(|| find_functions_by_path(&self.functions, file))
            .and_then(|funcs| {
                // Try multiple strategies in order of preference
                find_function_by_name(funcs, function_name)
                    .or_else(|| find_function_by_line_with_tolerance(funcs, line, 2))
                    .map(|f| f.coverage_percentage / 100.0)
            })
    }

    /// Get function coverage using exact boundaries from AST analysis
    /// This is more accurate than guessing boundaries from LCOV data alone
    pub fn get_function_coverage_with_bounds(
        &self,
        file: &Path,
        function_name: &str,
        start_line: usize,
        _end_line: usize,
    ) -> Option<f64> {
        // Try exact match first, then use path normalization
        self.functions
            .get(file)
            .or_else(|| find_functions_by_path(&self.functions, file))
            .and_then(|funcs| {
                // Chain multiple matching strategies using functional composition
                find_function_by_name(funcs, function_name)
                    .or_else(|| find_function_by_line_with_tolerance(funcs, start_line, 2))
                    .map(|f| f.coverage_percentage / 100.0)
            })
    }

    pub fn get_overall_coverage(&self) -> f64 {
        if self.total_lines == 0 {
            0.0
        } else {
            (self.lines_hit as f64 / self.total_lines as f64) * 100.0
        }
    }

    pub fn get_file_coverage(&self, file: &Path) -> Option<f64> {
        self.functions.get(file).map(|funcs| {
            if funcs.is_empty() {
                0.0
            } else {
                let sum: f64 = funcs.iter().map(|f| f.coverage_percentage).sum();
                sum / funcs.len() as f64 / 100.0 // Convert to fraction
            }
        })
    }

    /// Get uncovered lines for a specific function
    pub fn get_function_uncovered_lines(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<Vec<usize>> {
        // Try exact match first, then use path normalization
        self.functions
            .get(file)
            .or_else(|| find_functions_by_path(&self.functions, file))
            .and_then(|funcs| {
                // Try multiple matching strategies
                find_function_by_name(funcs, function_name)
                    .or_else(|| find_function_by_line_with_tolerance(funcs, line, 2))
                    .map(|f| f.uncovered_lines.clone())
            })
    }
}

/// Find a function by name using multiple matching strategies
/// Returns the first match found using these strategies in order:
/// 1. Exact match
/// 2. Normalized match (handling generics and special characters)
/// 3. Suffix match (for module-qualified names)
fn find_function_by_name<'a>(
    funcs: &'a [FunctionCoverage],
    target_name: &str,
) -> Option<&'a FunctionCoverage> {
    // Strategy 1: Exact match
    funcs
        .iter()
        .find(|f| f.name == target_name)
        // Strategy 2: Normalized match
        .or_else(|| {
            let normalized_target = normalize_function_name(target_name);
            funcs
                .iter()
                .find(|f| normalize_function_name(&f.name) == normalized_target)
        })
        // Strategy 3: Suffix match (e.g., "module::function" matches "function")
        .or_else(|| {
            funcs
                .iter()
                .find(|f| f.name.ends_with(target_name) || target_name.ends_with(&f.name))
        })
}

/// Find a function by line number with tolerance for AST/LCOV discrepancies
/// Pure function that searches for functions within a line range
fn find_function_by_line_with_tolerance(
    funcs: &[FunctionCoverage],
    target_line: usize,
    tolerance: usize,
) -> Option<&FunctionCoverage> {
    funcs
        .iter()
        .filter(|f| {
            let line_diff = (f.start_line as i32 - target_line as i32).abs();
            line_diff <= tolerance as i32
        })
        // If multiple matches within tolerance, prefer the closest one
        .min_by_key(|f| (f.start_line as i32 - target_line as i32).abs())
}

/// Normalize function names for matching
/// This is now a pure function used internally by the matching strategies
fn normalize_function_name(name: &str) -> String {
    // Handle Rust function names with generics and impl blocks
    name.replace(['<', '>'], "_")
        .replace("::", "_")
        .replace(' ', "_")
        .replace('\'', "")
}

/// Find functions by normalizing and matching paths
/// This handles cases where LCOV has relative paths but queries use absolute paths, or vice versa
fn find_functions_by_path<'a>(
    functions: &'a HashMap<PathBuf, Vec<FunctionCoverage>>,
    query_path: &Path,
) -> Option<&'a Vec<FunctionCoverage>> {
    // Strategy 1: Direct lookup (already tried by caller)

    // Strategy 2: Check if query path ends with any LCOV path (absolute query, relative LCOV)
    functions
        .iter()
        .find(|(lcov_path, _)| query_path.ends_with(lcov_path))
        .map(|(_, funcs)| funcs)
        .or_else(|| {
            // Strategy 3: Check if any LCOV path ends with query path (relative query, absolute LCOV)
            functions
                .iter()
                .find(|(lcov_path, _)| lcov_path.ends_with(query_path))
                .map(|(_, funcs)| funcs)
        })
        .or_else(|| {
            // Strategy 4: Strip leading ./ from either path and compare
            let normalized_query = normalize_path(query_path);
            functions
                .iter()
                .find(|(lcov_path, _)| normalize_path(lcov_path) == normalized_query)
                .map(|(_, funcs)| funcs)
        })
}

/// Normalize a path by removing leading ./ and resolving components
fn normalize_path(path: &Path) -> PathBuf {
    // Convert to string, remove leading ./
    let path_str = path.to_string_lossy();
    let cleaned = path_str.strip_prefix("./").unwrap_or(&path_str);
    PathBuf::from(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_lcov_file() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,test_function
FNDA:5,test_function
FNF:1
FNH:1
DA:10,5
DA:11,5
DA:12,0
LF:3
LH:2
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        assert_eq!(data.total_lines, 3);
        assert_eq!(data.lines_hit, 2);

        let file_path = PathBuf::from("/path/to/file.rs");
        assert!(data.functions.contains_key(&file_path));

        let funcs = &data.functions[&file_path];
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "test_function");
        assert_eq!(funcs[0].execution_count, 5);
    }

    #[test]
    fn test_function_name_normalization() {
        assert_eq!(
            normalize_function_name("MyStruct::my_function"),
            normalize_function_name("MyStruct_my_function")
        );

        assert_eq!(
            normalize_function_name("function<T>"),
            normalize_function_name("function_T_")
        );
    }

    #[test]
    fn test_coverage_percentage_calculation() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,fully_covered
FN:20,partially_covered
FN:30,not_covered
FNDA:10,fully_covered
FNDA:5,partially_covered
FNDA:0,not_covered
DA:10,10
DA:11,10
DA:12,10
DA:20,5
DA:21,5
DA:22,0
DA:23,0
DA:30,0
DA:31,0
DA:32,0
LF:10
LH:5
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Test fully covered function (100%)
        let coverage = data.get_function_coverage(&file_path, "fully_covered");
        assert_eq!(coverage, Some(1.0));

        // Test partially covered function (50%)
        let coverage = data.get_function_coverage(&file_path, "partially_covered");
        assert_eq!(coverage, Some(0.5));

        // Test uncovered function (0%)
        let coverage = data.get_function_coverage(&file_path, "not_covered");
        assert_eq!(coverage, Some(0.0));
    }

    #[test]
    fn test_get_function_coverage_with_line() {
        let lcov_content = r#"TN:
SF:/path/to/file.rs
FN:10,function_at_line_10
FN:50,function_at_line_50
FNDA:5,function_at_line_10
FNDA:0,function_at_line_50
DA:10,5
DA:11,5
DA:50,0
DA:51,0
LF:4
LH:2
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();
        let file_path = PathBuf::from("/path/to/file.rs");

        // Test finding function by line number
        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 10);
        assert_eq!(coverage, Some(1.0)); // Should find function_at_line_10

        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 51);
        assert_eq!(coverage, Some(0.0)); // Should find function_at_line_50

        // Test line outside any function range
        let coverage = data.get_function_coverage_with_line(&file_path, "unknown_name", 200);
        assert_eq!(coverage, None);
    }

    #[test]
    fn test_overall_coverage_calculation() {
        let lcov_content = r#"TN:
SF:/path/to/file1.rs
DA:1,1
DA:2,1
DA:3,0
DA:4,0
LF:4
LH:2
end_of_record
SF:/path/to/file2.rs
DA:1,5
DA:2,5
DA:3,5
DA:4,5
LF:4
LH:4
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        // Overall coverage: 6 lines hit out of 8 total = 75%
        assert_eq!(data.get_overall_coverage(), 75.0);
    }

    #[test]
    fn test_empty_coverage_data() {
        let lcov_content = r#"TN:
SF:/path/to/empty.rs
end_of_record
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(lcov_content.as_bytes()).unwrap();

        let data = parse_lcov_file(temp_file.path()).unwrap();

        assert_eq!(data.get_overall_coverage(), 0.0);
        assert_eq!(data.functions.len(), 0);
    }
}
