//! DOT format output for unified analysis (Spec 204)
//!
//! Provides DOT/Graphviz format output for file dependency visualization.

use crate::io::writers::{DotConfig, DotWriter, RankDir};
use crate::priority::UnifiedAnalysis;
use anyhow::Result;
use std::path::PathBuf;

/// Output unified analysis in DOT format
///
/// # Arguments
/// * `analysis` - The unified analysis results
/// * `output_file` - Optional output file path (stdout if None)
/// * `config` - DOT output configuration
pub fn output_dot(
    analysis: &UnifiedAnalysis,
    output_file: Option<PathBuf>,
    config: DotConfig,
) -> Result<()> {
    let writer = DotWriter::with_config(config);

    match output_file {
        Some(path) => {
            let file = std::fs::File::create(&path)?;
            let mut buf_writer = std::io::BufWriter::new(file);
            writer.write(analysis, &mut buf_writer)?;
        }
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            writer.write(analysis, &mut handle)?;
        }
    }

    Ok(())
}

/// Output DOT format with default configuration
pub fn output_dot_default(analysis: &UnifiedAnalysis, output_file: Option<PathBuf>) -> Result<()> {
    output_dot(analysis, output_file, DotConfig::default())
}

/// Output DOT format with minimum score filter
pub fn output_dot_with_min_score(
    analysis: &UnifiedAnalysis,
    output_file: Option<PathBuf>,
    min_score: f64,
) -> Result<()> {
    let config = DotConfig {
        min_score: Some(min_score),
        ..DotConfig::default()
    };
    output_dot(analysis, output_file, config)
}

/// Output DOT format with left-to-right layout
pub fn output_dot_lr(analysis: &UnifiedAnalysis, output_file: Option<PathBuf>) -> Result<()> {
    let config = DotConfig {
        rankdir: RankDir::LeftRight,
        ..DotConfig::default()
    };
    output_dot(analysis, output_file, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_dot_config_defaults() {
        let config = DotConfig::default();
        assert!(config.min_score.is_none());
        assert!(config.cluster_by_module);
        assert!(!config.include_external);
    }
}
