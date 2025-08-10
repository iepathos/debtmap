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
        // Try multiple path variations to handle relative vs absolute paths
        let paths_to_try = vec![
            file.to_path_buf(),
            // Convert relative path to absolute by prepending current dir
            std::env::current_dir()
                .ok()?
                .join(file.strip_prefix("./").unwrap_or(file)),
            // Try with src/ prefix
            std::env::current_dir()
                .ok()?
                .join("src")
                .join(file.strip_prefix("./src/").unwrap_or(file)),
        ];

        for path in paths_to_try {
            if let Some(funcs) = self.functions.get(&path) {
                // First try exact match
                if let Some(func) = funcs.iter().find(|f| f.name == function_name) {
                    return Some(func.coverage_percentage);
                }

                // If no exact match, try partial match for qualified names (e.g., "Struct::method")
                if let Some(func) = funcs
                    .iter()
                    .find(|f| f.name.ends_with(&format!("::{function_name}")))
                {
                    return Some(func.coverage_percentage);
                }

                // Fallback: contains match only if function_name is reasonably specific
                if function_name.len() > 3 {
                    if let Some(func) = funcs.iter().find(|f| f.name.contains(function_name)) {
                        return Some(func.coverage_percentage);
                    }
                }
            }
        }

        None
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
}
