use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FunctionCoverage {
    pub name: String,
    pub start_line: usize,
    pub execution_count: u64,
    pub coverage_percentage: f64,
}

#[derive(Debug, Default)]
pub struct LcovData {
    pub functions: HashMap<PathBuf, Vec<FunctionCoverage>>,
}

// Pure function to parse a line and return its type
#[derive(Debug, Clone)]
enum LcovLine {
    SourceFile(PathBuf),
    Function { line: usize, name: String },
    FunctionData { count: u64, name: String },
    EndOfRecord,
    Other,
}

// Helper function to parse comma-separated data
fn parse_comma_data<T>(data: &str) -> Option<(T, String)>
where
    T: std::str::FromStr,
{
    data.split_once(',')
        .and_then(|(first, second)| first.parse::<T>().ok().map(|val| (val, second.to_string())))
}

// Parse source file line
fn parse_source_file(line: &str) -> Option<LcovLine> {
    line.strip_prefix("SF:")
        .map(|path| LcovLine::SourceFile(PathBuf::from(path)))
}

// Parse function definition line
fn parse_function(line: &str) -> Option<LcovLine> {
    line.strip_prefix("FN:")
        .and_then(parse_comma_data::<usize>)
        .map(|(line_num, name)| LcovLine::Function {
            line: line_num,
            name,
        })
}

// Parse function data line
fn parse_function_data(line: &str) -> Option<LcovLine> {
    line.strip_prefix("FNDA:")
        .and_then(parse_comma_data::<u64>)
        .map(|(count, name)| LcovLine::FunctionData { count, name })
}

fn parse_lcov_line(line: &str) -> LcovLine {
    let line = line.trim();

    parse_source_file(line)
        .or_else(|| parse_function(line))
        .or_else(|| parse_function_data(line))
        .or_else(|| {
            if line == "end_of_record" {
                Some(LcovLine::EndOfRecord)
            } else {
                None
            }
        })
        .unwrap_or(LcovLine::Other)
}

// State for processing a single file's records
#[derive(Debug, Default)]
struct FileRecord {
    path: Option<PathBuf>,
    function_lines: HashMap<String, usize>,
    function_hits: HashMap<String, u64>,
}

impl FileRecord {
    fn new(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            function_lines: HashMap::new(),
            function_hits: HashMap::new(),
        }
    }

    fn add_function(&mut self, line: usize, name: String) {
        self.function_lines.insert(name, line);
    }

    fn add_function_data(&mut self, count: u64, name: String) {
        self.function_hits.insert(name, count);
    }

    fn to_function_coverage(&self) -> Vec<FunctionCoverage> {
        self.function_lines
            .iter()
            .map(|(name, &start_line)| {
                let execution_count = self.function_hits.get(name).copied().unwrap_or(0);
                let coverage_percentage = if execution_count > 0 { 100.0 } else { 0.0 };

                FunctionCoverage {
                    name: name.clone(),
                    start_line,
                    execution_count,
                    coverage_percentage,
                }
            })
            .collect()
    }
}

// Process a single parsed line and update the record state
fn process_lcov_line(line_type: LcovLine, current_record: &mut FileRecord, data: &mut LcovData) {
    match line_type {
        LcovLine::SourceFile(path) => {
            *current_record = FileRecord::new(path);
        }
        LcovLine::Function { line, name } => {
            current_record.add_function(line, name);
        }
        LcovLine::FunctionData { count, name } => {
            current_record.add_function_data(count, name);
        }
        LcovLine::EndOfRecord => {
            finalize_record(current_record, data);
        }
        LcovLine::Other => {}
    }
}

// Finalize the current record and add it to the data
fn finalize_record(current_record: &mut FileRecord, data: &mut LcovData) {
    if let Some(file_path) = current_record.path.take() {
        let functions = current_record.to_function_coverage();
        if !functions.is_empty() {
            data.functions.insert(file_path, functions);
        }
    }
    *current_record = FileRecord::default();
}

// Read and parse lines from the LCOV file
fn read_lcov_lines(path: &Path) -> Result<Vec<String>> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open LCOV file: {}", path.display()))?;
    let reader = BufReader::new(file);
    reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
}

pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
    let lines = read_lcov_lines(path)?;
    let mut data = LcovData::default();
    let mut current_record = FileRecord::default();

    lines
        .iter()
        .map(|line| parse_lcov_line(line))
        .for_each(|line_type| process_lcov_line(line_type, &mut current_record, &mut data));

    Ok(data)
}

impl LcovData {
    pub fn get_function_coverage(&self, file: &Path, function_name: &str) -> Option<f64> {
        self.generate_path_variations(file)
            .into_iter()
            .filter_map(|path| self.functions.get(&path))
            .find_map(|funcs| self.find_matching_function(funcs, function_name))
            .map(|func| func.coverage_percentage)
    }

    pub fn get_function_coverage_with_line(
        &self,
        file: &Path,
        function_name: &str,
        line: usize,
    ) -> Option<f64> {
        // Special handling for closures: they inherit coverage from their parent function
        if function_name.starts_with("<closure@") {
            return self.find_closure_coverage(file, line);
        }

        // For regular functions, use the standard matching
        self.get_function_coverage(file, function_name)
    }

    fn find_closure_coverage(&self, file: &Path, line: usize) -> Option<f64> {
        self.generate_path_variations(file)
            .into_iter()
            .filter_map(|path| self.functions.get(&path))
            .find_map(|funcs| self.find_containing_function(funcs, line))
    }

    fn find_containing_function(&self, functions: &[FunctionCoverage], line: usize) -> Option<f64> {
        functions
            .iter()
            .rev()
            .find(|func| func.start_line <= line)
            .map(|func| func.coverage_percentage)
    }

    // Extract path generation logic to a pure function
    fn generate_path_variations(&self, file: &Path) -> Vec<PathBuf> {
        let current_dir = std::env::current_dir().ok();

        vec![
            Some(file.to_path_buf()),
            current_dir
                .as_ref()
                .map(|dir| dir.join(file.strip_prefix("./").unwrap_or(file))),
            current_dir.as_ref().map(|dir| {
                dir.join("src")
                    .join(file.strip_prefix("./src/").unwrap_or(file))
            }),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    // Extract function matching logic to a pure function
    fn find_matching_function<'a>(
        &self,
        funcs: &'a [FunctionCoverage],
        function_name: &str,
    ) -> Option<&'a FunctionCoverage> {
        // Chain matchers in order of preference
        funcs
            .iter()
            .find(|f| f.name == function_name)
            .or_else(|| {
                funcs
                    .iter()
                    .find(|f| f.name.ends_with(&format!("::{function_name}")))
            })
            .or_else(|| {
                if function_name.len() > 3 {
                    funcs.iter().find(|f| f.name.contains(function_name))
                } else {
                    None
                }
            })
    }

    pub fn get_file_coverage(&self, file: &Path) -> Option<f64> {
        self.functions.get(file).map(|funcs| {
            if funcs.is_empty() {
                0.0
            } else {
                let covered = funcs.iter().filter(|f| f.execution_count > 0).count();
                (covered as f64 / funcs.len() as f64) * 100.0
            }
        })
    }

    /// Calculate the overall project coverage percentage
    pub fn get_overall_coverage(&self) -> f64 {
        let mut total_functions = 0;
        let mut covered_functions = 0;

        for funcs in self.functions.values() {
            for func in funcs {
                total_functions += 1;
                if func.execution_count > 0 {
                    covered_functions += 1;
                }
            }
        }

        if total_functions == 0 {
            0.0
        } else {
            (covered_functions as f64 / total_functions as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_matching_function_with_qualified_name() {
        let lcov = LcovData::default();
        let functions = vec![FunctionCoverage {
            name: "CallGraphExtractor::classify_call_type".to_string(),
            start_line: 24,
            execution_count: 31,
            coverage_percentage: 100.0,
        }];

        // Should match when searching for short name
        let result = lcov.find_matching_function(&functions, "classify_call_type");
        assert!(result.is_some());
        assert_eq!(result.unwrap().coverage_percentage, 100.0);

        // Should also match with full name
        let result =
            lcov.find_matching_function(&functions, "CallGraphExtractor::classify_call_type");
        assert!(result.is_some());
        assert_eq!(result.unwrap().coverage_percentage, 100.0);
    }

    #[test]
    fn test_get_overall_coverage_empty() {
        let lcov = LcovData::default();
        assert_eq!(lcov.get_overall_coverage(), 0.0);
    }

    #[test]
    fn test_get_overall_coverage_single_covered_function() {
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            Path::new("src/lib.rs").to_path_buf(),
            vec![FunctionCoverage {
                name: "test_func".to_string(),
                start_line: 10,
                execution_count: 5,
                coverage_percentage: 100.0,
            }],
        );
        assert_eq!(lcov.get_overall_coverage(), 100.0);
    }

    #[test]
    fn test_get_overall_coverage_single_uncovered_function() {
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            Path::new("src/lib.rs").to_path_buf(),
            vec![FunctionCoverage {
                name: "test_func".to_string(),
                start_line: 10,
                execution_count: 0,
                coverage_percentage: 0.0,
            }],
        );
        assert_eq!(lcov.get_overall_coverage(), 0.0);
    }

    #[test]
    fn test_get_overall_coverage_mixed_coverage() {
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            Path::new("src/main.rs").to_path_buf(),
            vec![
                FunctionCoverage {
                    name: "main".to_string(),
                    start_line: 10,
                    execution_count: 1,
                    coverage_percentage: 100.0,
                },
                FunctionCoverage {
                    name: "helper".to_string(),
                    start_line: 20,
                    execution_count: 0,
                    coverage_percentage: 0.0,
                },
            ],
        );
        lcov.functions.insert(
            Path::new("src/lib.rs").to_path_buf(),
            vec![
                FunctionCoverage {
                    name: "process".to_string(),
                    start_line: 5,
                    execution_count: 10,
                    coverage_percentage: 100.0,
                },
                FunctionCoverage {
                    name: "validate".to_string(),
                    start_line: 15,
                    execution_count: 0,
                    coverage_percentage: 0.0,
                },
            ],
        );
        // 2 covered out of 4 total = 50%
        assert_eq!(lcov.get_overall_coverage(), 50.0);
    }

    #[test]
    fn test_get_overall_coverage_all_covered() {
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            Path::new("src/main.rs").to_path_buf(),
            vec![
                FunctionCoverage {
                    name: "main".to_string(),
                    start_line: 10,
                    execution_count: 1,
                    coverage_percentage: 100.0,
                },
                FunctionCoverage {
                    name: "helper".to_string(),
                    start_line: 20,
                    execution_count: 5,
                    coverage_percentage: 100.0,
                },
            ],
        );
        lcov.functions.insert(
            Path::new("src/lib.rs").to_path_buf(),
            vec![FunctionCoverage {
                name: "process".to_string(),
                start_line: 5,
                execution_count: 10,
                coverage_percentage: 100.0,
            }],
        );
        assert_eq!(lcov.get_overall_coverage(), 100.0);
    }

    #[test]
    fn test_get_overall_coverage_none_covered() {
        let mut lcov = LcovData::default();
        lcov.functions.insert(
            Path::new("src/main.rs").to_path_buf(),
            vec![
                FunctionCoverage {
                    name: "main".to_string(),
                    start_line: 10,
                    execution_count: 0,
                    coverage_percentage: 0.0,
                },
                FunctionCoverage {
                    name: "helper".to_string(),
                    start_line: 20,
                    execution_count: 0,
                    coverage_percentage: 0.0,
                },
            ],
        );
        lcov.functions.insert(
            Path::new("src/lib.rs").to_path_buf(),
            vec![FunctionCoverage {
                name: "process".to_string(),
                start_line: 5,
                execution_count: 0,
                coverage_percentage: 0.0,
            }],
        );
        assert_eq!(lcov.get_overall_coverage(), 0.0);
    }
}
