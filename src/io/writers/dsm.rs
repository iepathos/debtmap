//! DSM (Dependency Structure Matrix) output format (Spec 205)
//!
//! Generates ASCII/text matrix output for visualizing module dependencies.
//! The DSM provides a scalable view of dependencies that:
//! - Reveals cycles immediately (cells above diagonal)
//! - Shows layered architecture (triangular matrices = clean layers)
//! - Enables pattern recognition (clusters, fan-out, fan-in)

use crate::analysis::dsm::{DependencyMatrix, DsmCell};
use crate::priority::UnifiedAnalysis;
use std::io::{self, Write};

/// Configuration for DSM text output
#[derive(Debug, Clone)]
pub struct DsmConfig {
    /// Minimum score threshold for including files (default: None = include all)
    pub min_score: Option<f64>,
    /// Whether to use module-level grouping (default: true)
    pub module_level: bool,
    /// Whether to optimize ordering to minimize above-diagonal deps (default: true)
    pub optimize_ordering: bool,
    /// Maximum module name width in output (default: 20)
    pub max_name_width: usize,
}

impl Default for DsmConfig {
    fn default() -> Self {
        Self {
            min_score: None,
            module_level: true,
            optimize_ordering: true,
            max_name_width: 20,
        }
    }
}

/// DSM text format writer
pub struct DsmWriter {
    config: DsmConfig,
}

impl DsmWriter {
    /// Create a new DSM writer with default configuration
    pub fn new() -> Self {
        Self {
            config: DsmConfig::default(),
        }
    }

    /// Create a new DSM writer with custom configuration
    pub fn with_config(config: DsmConfig) -> Self {
        Self { config }
    }

    /// Write DSM output from unified analysis
    pub fn write<W: Write>(&self, analysis: &UnifiedAnalysis, out: &mut W) -> io::Result<()> {
        use crate::output::unified::{convert_to_unified_format, UnifiedDebtItemOutput};

        // Convert to output format and filter file items
        let unified = convert_to_unified_format(analysis, false);

        let file_items: Vec<_> = unified
            .items
            .into_iter()
            .filter_map(|item| match item {
                UnifiedDebtItemOutput::File(file_item) => {
                    if let Some(min) = self.config.min_score {
                        if file_item.score < min {
                            return None;
                        }
                    }
                    Some(*file_item)
                }
                UnifiedDebtItemOutput::Function(_) => None,
            })
            .collect();

        if file_items.is_empty() {
            writeln!(out, "No files to analyze for DSM.")?;
            return Ok(());
        }

        // Build dependency matrix
        let mut matrix = DependencyMatrix::from_file_dependencies(&file_items);

        // Optionally optimize ordering
        if self.config.optimize_ordering {
            matrix.optimize_ordering();
        }

        // Write the DSM output
        self.write_dsm(&matrix, out)
    }

    /// Write DSM from a pre-built matrix
    pub fn write_dsm<W: Write>(&self, matrix: &DependencyMatrix, out: &mut W) -> io::Result<()> {
        // Header
        writeln!(out, "DEPENDENCY STRUCTURE MATRIX")?;
        writeln!(out, "===========================")?;
        writeln!(out)?;

        // Metrics summary
        let metrics = &matrix.metrics;
        writeln!(out, "Modules: {}", metrics.module_count)?;
        writeln!(out, "Dependencies: {}", metrics.dependency_count)?;
        writeln!(
            out,
            "Cycles: {} (cells above diagonal)",
            metrics.cycle_count
        )?;
        writeln!(
            out,
            "Layering Score: {:.0}%",
            metrics.layering_score * 100.0
        )?;
        writeln!(out, "Density: {:.1}%", metrics.density * 100.0)?;
        writeln!(out, "Propagation Cost: {:.1}", metrics.propagation_cost)?;
        writeln!(out)?;

        if matrix.modules.is_empty() {
            writeln!(out, "(No modules to display)")?;
            return Ok(());
        }

        // Print column indices header
        let name_width = self.config.max_name_width;
        write!(out, "{:>width$} ", "", width = name_width)?;
        for (i, _) in matrix.modules.iter().enumerate() {
            write!(out, "{:>2} ", i)?;
        }
        writeln!(out)?;

        // Print matrix rows
        for (row_idx, module) in matrix.modules.iter().enumerate() {
            // Truncate module name if needed
            let short_name: String = if module.len() > name_width {
                format!("...{}", &module[module.len() - (name_width - 3)..])
            } else {
                module.clone()
            };

            write!(out, "{:>width$} ", short_name, width = name_width)?;

            for col_idx in 0..matrix.modules.len() {
                let cell = &matrix.matrix[row_idx][col_idx];
                let symbol = Self::cell_symbol(cell, row_idx, col_idx);
                write!(out, "{:>2} ", symbol)?;
            }
            writeln!(out)?;
        }

        writeln!(out)?;
        writeln!(out, "Legend: × = dependency, ● = cycle, ■ = self, · = none")?;
        writeln!(
            out,
            "        Lower triangle (good) | Upper triangle (cycles)"
        )?;

        // Print cycle details if any
        if !matrix.cycles.is_empty() {
            writeln!(out)?;
            writeln!(out, "Detected Cycles:")?;
            for (i, cycle) in matrix.cycles.iter().enumerate() {
                writeln!(
                    out,
                    "  {}. [{}] {} -> ...",
                    i + 1,
                    format!("{:?}", cycle.severity).to_uppercase(),
                    cycle.modules.join(" -> ")
                )?;
            }
        }

        Ok(())
    }

    fn cell_symbol(cell: &DsmCell, row: usize, col: usize) -> &'static str {
        DependencyMatrix::cell_symbol(cell, row, col)
    }
}

impl Default for DsmWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Render DSM output to a string
pub fn render_dsm(analysis: &UnifiedAnalysis, config: DsmConfig) -> io::Result<String> {
    let mut buffer = Vec::new();
    let writer = DsmWriter::with_config(config);
    writer.write(analysis, &mut buffer)?;
    String::from_utf8(buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsm_config_default() {
        let config = DsmConfig::default();
        assert!(config.min_score.is_none());
        assert!(config.module_level);
        assert!(config.optimize_ordering);
        assert_eq!(config.max_name_width, 20);
    }

    #[test]
    fn test_dsm_writer_creation() {
        let writer = DsmWriter::new();
        assert!(writer.config.min_score.is_none());

        let config = DsmConfig {
            min_score: Some(10.0),
            ..Default::default()
        };
        let writer = DsmWriter::with_config(config);
        assert_eq!(writer.config.min_score, Some(10.0));
    }

    #[test]
    fn test_cell_symbol() {
        let cell = DsmCell::default();
        assert_eq!(DsmWriter::cell_symbol(&cell, 0, 0), "■");
        assert_eq!(DsmWriter::cell_symbol(&cell, 0, 1), "·");

        let dep_cell = DsmCell {
            has_dependency: true,
            dependency_count: 1,
            is_cycle: false,
        };
        assert_eq!(DsmWriter::cell_symbol(&dep_cell, 1, 0), "×");

        let cycle_cell = DsmCell {
            has_dependency: true,
            dependency_count: 1,
            is_cycle: true,
        };
        assert_eq!(DsmWriter::cell_symbol(&cycle_cell, 0, 1), "●");
    }

    #[test]
    fn test_write_empty_matrix() {
        use crate::analysis::dsm::DependencyMatrix;

        let modules = Vec::new();
        let matrix = DependencyMatrix {
            modules: modules.clone(),
            matrix: Vec::new(),
            cycles: Vec::new(),
            metrics: crate::analysis::dsm::DsmMetrics {
                module_count: 0,
                dependency_count: 0,
                cycle_count: 0,
                density: 0.0,
                layering_score: 1.0,
                propagation_cost: 0.0,
            },
        };

        let writer = DsmWriter::new();
        let mut buffer = Vec::new();
        writer.write_dsm(&matrix, &mut buffer).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("DEPENDENCY STRUCTURE MATRIX"));
        assert!(output.contains("Modules: 0"));
        assert!(output.contains("(No modules to display)"));
    }

    #[test]
    fn test_write_simple_matrix() {
        use crate::analysis::dsm::{DependencyMatrix, DsmCell, DsmMetrics};

        let modules = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        let mut matrix_cells = vec![vec![DsmCell::default(); 3]; 3];

        // B depends on A
        matrix_cells[1][0].has_dependency = true;
        matrix_cells[1][0].dependency_count = 1;

        // C depends on B
        matrix_cells[2][1].has_dependency = true;
        matrix_cells[2][1].dependency_count = 1;

        let matrix = DependencyMatrix {
            modules,
            matrix: matrix_cells,
            cycles: Vec::new(),
            metrics: DsmMetrics {
                module_count: 3,
                dependency_count: 2,
                cycle_count: 0,
                density: 0.33,
                layering_score: 1.0,
                propagation_cost: 0.67,
            },
        };

        let writer = DsmWriter::new();
        let mut buffer = Vec::new();
        writer.write_dsm(&matrix, &mut buffer).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("DEPENDENCY STRUCTURE MATRIX"));
        assert!(output.contains("Modules: 3"));
        assert!(output.contains("Dependencies: 2"));
        assert!(output.contains("Layering Score: 100%"));
        assert!(output.contains("Legend:"));
    }
}
