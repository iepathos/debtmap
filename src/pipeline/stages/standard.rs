//! Standard pipeline stages for technical debt analysis.
//!
//! This module implements the 9 core stages of the analysis pipeline as
//! reusable, composable units following Spec 209.
//!
//! NOTE: This is a simplified implementation that provides the stage structure.
//! Full integration with the existing analysis code will be completed in future iterations.

use crate::core::Language;
use crate::errors::AnalysisError;
use crate::pipeline::data::{CoverageData, PipelineData, ProjectContext};
use crate::pipeline::stage::Stage;
use std::path::{Path, PathBuf};

/// Stage 1: Discover project files
///
/// Scans the project directory for source files matching the specified languages.
pub struct FileDiscoveryStage {
    path: PathBuf,
    languages: Vec<Language>,
}

impl FileDiscoveryStage {
    pub fn new(path: &Path, languages: &[Language]) -> Self {
        Self {
            path: path.to_path_buf(),
            languages: languages.to_vec(),
        }
    }
}

impl Stage for FileDiscoveryStage {
    type Input = ();
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, _input: Self::Input) -> Result<Self::Output, Self::Error> {
        // Discover files using file system walk
        let files = discover_files(&self.path, &self.languages)?;
        Ok(PipelineData::new(files))
    }

    fn name(&self) -> &str {
        "File Discovery"
    }
}

/// Stage 2: Parse files to extract metrics
///
/// Analyzes discovered files using language-specific parsers to extract
/// function metrics (complexity, LOC, parameters, etc.).
pub struct ParsingStage;

impl ParsingStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ParsingStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for ParsingStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, data: Self::Input) -> Result<Self::Output, Self::Error> {
        // TODO: Integrate with existing analysis code
        // For now, return empty metrics to allow pipeline to compile
        log::warn!("ParsingStage not fully implemented - returning empty metrics");
        Ok(data)
    }

    fn name(&self) -> &str {
        "Parsing"
    }
}

/// Stage 3: Build call graph
///
/// Constructs function call relationships from the parsed metrics.
pub struct CallGraphStage;

impl CallGraphStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CallGraphStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for CallGraphStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = std::convert::Infallible;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        let graph = super::call_graph::build_call_graph(&data.metrics);
        data.call_graph = Some(graph);
        Ok(data)
    }

    fn name(&self) -> &str {
        "Call Graph Construction"
    }
}

/// Stage 4: Resolve trait calls
///
/// Resolves trait implementations and method calls for better call graph accuracy.
pub struct TraitResolutionStage {
    project_path: PathBuf,
}

impl TraitResolutionStage {
    pub fn new(project_path: &Path) -> Self {
        Self {
            project_path: project_path.to_path_buf(),
        }
    }
}

impl Stage for TraitResolutionStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, data: Self::Input) -> Result<Self::Output, Self::Error> {
        // Trait resolution currently integrated into call graph construction
        // This stage is a placeholder for future trait resolution logic
        Ok(data)
    }

    fn name(&self) -> &str {
        "Trait Resolution"
    }
}

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

/// Stage 6: Analyze function purity
///
/// Determines which functions are pure (no side effects) vs impure (I/O operations).
pub struct PurityAnalysisStage;

impl PurityAnalysisStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PurityAnalysisStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for PurityAnalysisStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = std::convert::Infallible;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        if let Some(ref call_graph) = data.call_graph {
            let purity = super::purity::analyze_purity(&data.metrics, call_graph);
            data.purity_scores = Some(purity);
        }
        Ok(data)
    }

    fn name(&self) -> &str {
        "Purity Analysis"
    }
}

/// Stage 7: Load project context (optional)
///
/// Extracts project information from README, Cargo.toml, etc.
pub struct ContextLoadingStage {
    project_path: PathBuf,
}

impl ContextLoadingStage {
    pub fn new(project_path: &Path) -> Self {
        Self {
            project_path: project_path.to_path_buf(),
        }
    }
}

impl Stage for ContextLoadingStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = AnalysisError;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        let context = load_project_context(&self.project_path)?;
        data.context = Some(context);
        Ok(data)
    }

    fn name(&self) -> &str {
        "Context Loading"
    }
}

/// Stage 8: Detect technical debt
///
/// Identifies technical debt patterns in the analyzed code.
pub struct DebtDetectionStage;

impl DebtDetectionStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DebtDetectionStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for DebtDetectionStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = std::convert::Infallible;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        let debt_items =
            super::debt::detect_debt_from_pipeline(&data.metrics, data.call_graph.as_ref());
        data.debt_items = debt_items;
        Ok(data)
    }

    fn name(&self) -> &str {
        "Debt Detection"
    }
}

/// Stage 9: Score and prioritize debt
///
/// Assigns priority scores to debt items based on impact, risk, and context.
pub struct ScoringStage;

impl ScoringStage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ScoringStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Stage for ScoringStage {
    type Input = PipelineData;
    type Output = PipelineData;
    type Error = std::convert::Infallible;

    fn execute(&self, mut data: Self::Input) -> Result<Self::Output, Self::Error> {
        let scored_items = super::scoring::score_debt_items(
            &data.debt_items,
            data.call_graph.as_ref(),
            data.coverage.as_ref(),
            data.purity_scores.as_ref(),
        );
        data.scored_items = scored_items;
        Ok(data)
    }

    fn name(&self) -> &str {
        "Scoring & Prioritization"
    }
}

// Helper functions

fn discover_files(path: &Path, languages: &[Language]) -> Result<Vec<PathBuf>, AnalysisError> {
    use walkdir::WalkDir;

    let mut files = Vec::new();

    // For now, only support Rust files
    let extensions = vec!["rs"];

    for entry in WalkDir::new(path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let file_path = entry.path();
            if let Some(ext) = file_path.extension() {
                if extensions.iter().any(|&e| e == ext.to_str().unwrap_or("")) {
                    files.push(file_path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}


fn load_coverage_from_file(path: &Path) -> Result<CoverageData, AnalysisError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        AnalysisError::other(format!("Failed to read coverage file {:?}: {}", path, e))
    })?;

    // Parse LCOV format
    let mut coverage = CoverageData::new();

    let mut current_file: Option<PathBuf> = None;
    let mut line_hits: Vec<bool> = Vec::new();

    for line in content.lines() {
        if let Some(sf) = line.strip_prefix("SF:") {
            current_file = Some(PathBuf::from(sf));
            line_hits.clear();
        } else if let Some(da) = line.strip_prefix("DA:") {
            // DA:line_number,hit_count
            if let Some((_, hit_str)) = da.split_once(',') {
                let hits = hit_str.parse::<i32>().unwrap_or(0);
                line_hits.push(hits > 0);
            }
        } else if line == "end_of_record" {
            if let Some(ref file) = current_file {
                let total_lines = line_hits.len();
                let covered_lines = line_hits.iter().filter(|&&hit| hit).count();
                let coverage_pct = if total_lines > 0 {
                    (covered_lines as f64 / total_lines as f64) * 100.0
                } else {
                    0.0
                };
                coverage.file_coverage.insert(file.clone(), coverage_pct);
                coverage.line_coverage.insert(file.clone(), line_hits.clone());
            }
            current_file = None;
            line_hits.clear();
        }
    }

    Ok(coverage)
}

fn load_project_context(path: &Path) -> Result<ProjectContext, AnalysisError> {
    let mut context = ProjectContext::new();

    // Try to read README
    for readme_name in &["README.md", "README", "README.txt"] {
        let readme_path = path.join(readme_name);
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            // Extract first paragraph as description
            if let Some(first_para) = content.split("\n\n").next() {
                context.description = Some(first_para.trim().to_string());
            }
            break;
        }
    }

    // Try to read Cargo.toml for Rust projects
    let cargo_toml = path.join("Cargo.toml");
    if cargo_toml.exists() {
        context.technologies.push("Rust".to_string());
        // Could parse Cargo.toml for more info
    }

    // Try to read package.json for JS/TS projects
    let package_json = path.join("package.json");
    if package_json.exists() {
        context
            .technologies
            .push("JavaScript/TypeScript".to_string());
    }

    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_discovery_stage_creation() {
        let stage = FileDiscoveryStage::new(Path::new("."), &[Language::Rust]);
        assert_eq!(stage.name(), "File Discovery");
    }

    #[test]
    fn test_parsing_stage_creation() {
        let stage = ParsingStage::new();
        assert_eq!(stage.name(), "Parsing");
    }

    #[test]
    fn test_call_graph_stage_creation() {
        let stage = CallGraphStage::new();
        assert_eq!(stage.name(), "Call Graph Construction");
    }

    #[test]
    fn test_purity_analysis_stage_creation() {
        let stage = PurityAnalysisStage::new();
        assert_eq!(stage.name(), "Purity Analysis");
    }

    #[test]
    fn test_debt_detection_stage_creation() {
        let stage = DebtDetectionStage::new();
        assert_eq!(stage.name(), "Debt Detection");
    }

    #[test]
    fn test_scoring_stage_creation() {
        let stage = ScoringStage::new();
        assert_eq!(stage.name(), "Scoring & Prioritization");
    }
}
