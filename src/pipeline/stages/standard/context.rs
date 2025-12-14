//! Project context loading stage.
//!
//! Extracts project information from README, Cargo.toml, package.json, etc.
//!
//! # Design
//!
//! Pure functions extract data from content:
//! - `extract_description`: Gets first paragraph from README
//! - `detect_technologies`: Determines tech stack from file presence
//!
//! I/O wrapper coordinates filesystem access.

use crate::errors::AnalysisError;
use crate::pipeline::data::{PipelineData, ProjectContext};
use crate::pipeline::stage::Stage;
use std::path::{Path, PathBuf};

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

// =============================================================================
// Pure Functions
// =============================================================================

/// Extract first paragraph from README content as description.
///
/// Pure function: content in, optional description out.
fn extract_description(content: &str) -> Option<String> {
    content
        .split("\n\n")
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Determine technologies from file existence flags.
///
/// Pure function: booleans in, tech list out.
fn detect_technologies(has_cargo_toml: bool, has_package_json: bool) -> Vec<String> {
    let mut techs = Vec::new();
    if has_cargo_toml {
        techs.push("Rust".to_string());
    }
    if has_package_json {
        techs.push("JavaScript/TypeScript".to_string());
    }
    techs
}

// =============================================================================
// I/O - Context Loading
// =============================================================================

/// Try to read a README file from the project.
fn read_readme(path: &Path) -> Option<String> {
    for readme_name in &["README.md", "README", "README.txt"] {
        let readme_path = path.join(readme_name);
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            return Some(content);
        }
    }
    None
}

/// Load project context from filesystem.
///
/// Thin I/O wrapper that delegates to pure functions.
fn load_project_context(path: &Path) -> Result<ProjectContext, AnalysisError> {
    let mut context = ProjectContext::new();

    // Extract description from README (pure function)
    if let Some(readme_content) = read_readme(path) {
        context.description = extract_description(&readme_content);
    }

    // Detect technologies (pure function)
    let has_cargo_toml = path.join("Cargo.toml").exists();
    let has_package_json = path.join("package.json").exists();
    context.technologies = detect_technologies(has_cargo_toml, has_package_json);

    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_description_first_paragraph() {
        let content = "# Project Name\n\nThis is the description.\n\nMore content here.";
        assert_eq!(
            extract_description(content),
            Some("# Project Name".to_string())
        );
    }

    #[test]
    fn extract_description_single_paragraph() {
        let content = "Just one paragraph";
        assert_eq!(
            extract_description(content),
            Some("Just one paragraph".to_string())
        );
    }

    #[test]
    fn extract_description_empty() {
        assert_eq!(extract_description(""), None);
        assert_eq!(extract_description("   \n\n   "), None);
    }

    #[test]
    fn detect_technologies_rust_only() {
        let techs = detect_technologies(true, false);
        assert_eq!(techs, vec!["Rust".to_string()]);
    }

    #[test]
    fn detect_technologies_js_only() {
        let techs = detect_technologies(false, true);
        assert_eq!(techs, vec!["JavaScript/TypeScript".to_string()]);
    }

    #[test]
    fn detect_technologies_both() {
        let techs = detect_technologies(true, true);
        assert_eq!(
            techs,
            vec!["Rust".to_string(), "JavaScript/TypeScript".to_string()]
        );
    }

    #[test]
    fn detect_technologies_neither() {
        let techs = detect_technologies(false, false);
        assert!(techs.is_empty());
    }
}
