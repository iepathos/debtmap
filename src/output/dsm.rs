//! DSM format output for unified analysis (Spec 205)
//!
//! Provides Dependency Structure Matrix output for module dependency visualization.

use crate::io::writers::{DsmConfig, DsmWriter};
use crate::priority::UnifiedAnalysis;
use anyhow::Result;
use std::path::PathBuf;

/// Output unified analysis in DSM format
///
/// # Arguments
/// * `analysis` - The unified analysis results
/// * `output_file` - Optional output file path (stdout if None)
/// * `config` - DSM output configuration
pub fn output_dsm(
    analysis: &UnifiedAnalysis,
    output_file: Option<PathBuf>,
    config: DsmConfig,
) -> Result<()> {
    let writer = DsmWriter::with_config(config);

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

/// Output DSM format with default configuration
pub fn output_dsm_default(analysis: &UnifiedAnalysis, output_file: Option<PathBuf>) -> Result<()> {
    output_dsm(analysis, output_file, DsmConfig::default())
}

/// Output DSM format with minimum score filter
pub fn output_dsm_with_min_score(
    analysis: &UnifiedAnalysis,
    output_file: Option<PathBuf>,
    min_score: f64,
) -> Result<()> {
    let config = DsmConfig {
        min_score: Some(min_score),
        ..DsmConfig::default()
    };
    output_dsm(analysis, output_file, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_dsm_config_defaults() {
        let config = DsmConfig::default();
        assert!(config.min_score.is_none());
        assert!(config.module_level);
        assert!(config.optimize_ordering);
    }
}
