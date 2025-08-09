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

pub fn parse_lcov_file(path: &Path) -> Result<LcovData> {
    let file = File::open(path)
        .with_context(|| format!("Failed to open LCOV file: {}", path.display()))?;
    let reader = BufReader::new(file);

    let mut data = LcovData::default();
    let mut current_file: Option<PathBuf> = None;
    let mut function_lines: HashMap<String, usize> = HashMap::new();
    let mut function_hits: HashMap<String, u64> = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.starts_with("SF:") {
            // Source file
            let path = line.strip_prefix("SF:").unwrap();
            current_file = Some(PathBuf::from(path));
            function_lines.clear();
            function_hits.clear();
        } else if line.starts_with("FN:") {
            // Function definition
            if let Some(fn_data) = line.strip_prefix("FN:") {
                if let Some((line_str, name)) = fn_data.split_once(',') {
                    if let Ok(line_num) = line_str.parse::<usize>() {
                        function_lines.insert(name.to_string(), line_num);
                    }
                }
            }
        } else if line.starts_with("FNDA:") {
            // Function hit data
            if let Some(fnda_data) = line.strip_prefix("FNDA:") {
                if let Some((count_str, name)) = fnda_data.split_once(',') {
                    if let Ok(count) = count_str.parse::<u64>() {
                        function_hits.insert(name.to_string(), count);
                    }
                }
            }
        } else if line == "end_of_record" {
            // End of current file's data
            if let Some(ref file_path) = current_file {
                let mut functions = Vec::new();

                for (name, start_line) in &function_lines {
                    let execution_count = function_hits.get(name).copied().unwrap_or(0);
                    let coverage_percentage = if execution_count > 0 { 100.0 } else { 0.0 };

                    functions.push(FunctionCoverage {
                        name: name.clone(),
                        start_line: *start_line,
                        execution_count,
                        coverage_percentage,
                    });
                }

                if !functions.is_empty() {
                    data.functions.insert(file_path.clone(), functions);
                }
            }

            current_file = None;
            function_lines.clear();
            function_hits.clear();
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
