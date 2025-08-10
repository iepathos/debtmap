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

fn parse_lcov_line(line: &str) -> LcovLine {
    let line = line.trim();

    if let Some(path) = line.strip_prefix("SF:") {
        return LcovLine::SourceFile(PathBuf::from(path));
    }

    if let Some(fn_data) = line.strip_prefix("FN:") {
        if let Some((line_str, name)) = fn_data.split_once(',') {
            if let Ok(line_num) = line_str.parse::<usize>() {
                return LcovLine::Function {
                    line: line_num,
                    name: name.to_string(),
                };
            }
        }
    }

    if let Some(fnda_data) = line.strip_prefix("FNDA:") {
        if let Some((count_str, name)) = fnda_data.split_once(',') {
            if let Ok(count) = count_str.parse::<u64>() {
                return LcovLine::FunctionData {
                    count,
                    name: name.to_string(),
                };
            }
        }
    }

    if line == "end_of_record" {
        return LcovLine::EndOfRecord;
    }

    LcovLine::Other
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

pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open LCOV file: {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut data = LcovData::default();
    let mut current_record = FileRecord::default();

    for line in reader.lines() {
        let line = line?;

        match parse_lcov_line(&line) {
            LcovLine::SourceFile(path) => {
                current_record = FileRecord::new(path);
            }
            LcovLine::Function { line, name } => {
                current_record.add_function(line, name);
            }
            LcovLine::FunctionData { count, name } => {
                current_record.add_function_data(count, name);
            }
            LcovLine::EndOfRecord => {
                if let Some(file_path) = current_record.path.take() {
                    let functions = current_record.to_function_coverage();
                    if !functions.is_empty() {
                        data.functions.insert(file_path, functions);
                    }
                }
                current_record = FileRecord::default();
            }
            LcovLine::Other => {}
        }
    }

    Ok(data)
}

impl LcovData {
    pub fn get_function_coverage(&self, file: &Path, function_name: &str) -> Option<f64> {
        self.functions.get(file).and_then(|funcs| {
            funcs
                .iter()
                .find(|f| f.name == function_name || f.name.contains(function_name))
                .map(|f| f.coverage_percentage)
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
}
