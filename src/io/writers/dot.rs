//! DOT/Graphviz output format for dependency visualization (Spec 204)
//!
//! Generates Graphviz DOT format output for visualizing file dependencies
//! and technical debt. The output can be rendered using Graphviz tools:
//!
//! ```bash
//! # Generate SVG
//! dot -Tsvg deps.dot -o deps.svg
//!
//! # Generate PNG
//! dot -Tpng deps.dot -o deps.png
//!
//! # Interactive exploration
//! xdot deps.dot
//! ```

use crate::output::unified::{
    CouplingClassification, FileDebtItemOutput, UnifiedDebtItemOutput, UnifiedLocation,
};
use crate::priority::UnifiedAnalysis;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};

/// Configuration for DOT output generation
#[derive(Debug, Clone)]
pub struct DotConfig {
    /// Minimum score threshold for including files (default: None = include all)
    pub min_score: Option<f64>,
    /// Maximum dependency depth to traverse (default: None = unlimited)
    pub max_depth: Option<usize>,
    /// Whether to include external crate dependencies (default: false)
    pub include_external: bool,
    /// Whether to cluster files by module/directory (default: true)
    pub cluster_by_module: bool,
    /// Graph layout direction
    pub rankdir: RankDir,
}

impl Default for DotConfig {
    fn default() -> Self {
        Self {
            min_score: None,
            max_depth: None,
            include_external: false,
            cluster_by_module: true,
            rankdir: RankDir::TopBottom,
        }
    }
}

/// Graph layout direction
#[derive(Debug, Clone, Copy, Default)]
pub enum RankDir {
    /// Top to bottom layout (default)
    #[default]
    TopBottom,
    /// Left to right layout
    LeftRight,
}

impl RankDir {
    fn as_str(&self) -> &'static str {
        match self {
            RankDir::TopBottom => "TB",
            RankDir::LeftRight => "LR",
        }
    }
}

/// DOT format writer for technical debt visualization
pub struct DotWriter {
    config: DotConfig,
}

impl DotWriter {
    /// Create a new DOT writer with default configuration
    pub fn new() -> Self {
        Self {
            config: DotConfig::default(),
        }
    }

    /// Create a new DOT writer with custom configuration
    pub fn with_config(config: DotConfig) -> Self {
        Self { config }
    }

    /// Write DOT output from unified analysis
    pub fn write<W: Write>(&self, analysis: &UnifiedAnalysis, out: &mut W) -> io::Result<()> {
        // Convert to output format
        let items = self.collect_file_items(analysis);

        // Build dependency graph
        let graph = self.build_dependency_graph(&items);

        // Write DOT output
        self.write_dot(&items, &graph, out)
    }

    /// Collect file-level debt items with their dependencies
    fn collect_file_items(&self, analysis: &UnifiedAnalysis) -> Vec<FileDebtItemOutput> {
        use crate::output::unified::convert_to_unified_format;

        let unified = convert_to_unified_format(analysis, false);

        unified
            .items
            .into_iter()
            .filter_map(|item| match item {
                UnifiedDebtItemOutput::File(file_item) => {
                    // Apply minimum score filter
                    if let Some(min) = self.config.min_score {
                        if file_item.score < min {
                            return None;
                        }
                    }
                    Some(*file_item)
                }
                UnifiedDebtItemOutput::Function(_) => None,
            })
            .collect()
    }

    /// Build dependency graph from file items
    fn build_dependency_graph(&self, items: &[FileDebtItemOutput]) -> HashMap<String, Vec<String>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Get set of files that we have data for
        let known_files: HashSet<String> = items
            .iter()
            .map(|item| item.location.file.clone())
            .collect();

        for item in items {
            let file_path = &item.location.file;

            if let Some(deps) = &item.dependencies {
                // Add dependencies (edges from this file to its dependencies)
                let dependencies: Vec<String> = deps
                    .top_dependencies
                    .iter()
                    .filter(|dep| {
                        // Only include if it's a known file (internal) or external is enabled
                        self.config.include_external || known_files.contains(*dep)
                    })
                    .cloned()
                    .collect();

                graph.insert(file_path.clone(), dependencies);
            } else {
                graph.insert(file_path.clone(), vec![]);
            }
        }

        graph
    }

    /// Write DOT format output
    fn write_dot<W: Write>(
        &self,
        items: &[FileDebtItemOutput],
        graph: &HashMap<String, Vec<String>>,
        out: &mut W,
    ) -> io::Result<()> {
        writeln!(out, "digraph debtmap {{")?;
        writeln!(out, "  rankdir={};", self.config.rankdir.as_str())?;
        writeln!(
            out,
            "  node [shape=box, style=filled, fontname=\"Helvetica\"];"
        )?;
        writeln!(out, "  edge [fontname=\"Helvetica\", fontsize=10];")?;
        writeln!(out)?;

        // Write legend
        self.write_legend(out)?;
        writeln!(out)?;

        // Group files by module if clustering is enabled
        if self.config.cluster_by_module {
            let modules = self.group_by_module(items);
            for (module, module_items) in &modules {
                self.write_cluster(out, module, module_items)?;
            }
        } else {
            // Write all nodes without clustering
            for item in items {
                self.write_node(out, item, "  ")?;
            }
        }

        writeln!(out)?;

        // Write edges
        self.write_edges(out, graph)?;

        writeln!(out, "}}")?;
        Ok(())
    }

    /// Write legend subgraph
    fn write_legend<W: Write>(&self, out: &mut W) -> io::Result<()> {
        writeln!(out, "  subgraph cluster_legend {{")?;
        writeln!(out, "    label=\"Debt Score Legend\";")?;
        writeln!(out, "    style=rounded;")?;
        writeln!(out, "    bgcolor=\"#F0F0F0\";")?;
        writeln!(out, "    fontname=\"Helvetica\";")?;
        writeln!(out)?;
        writeln!(out, "    legend_critical [label=\"Critical (>=100)\", fillcolor=\"#FF6B6B\", fontcolor=\"white\"];")?;
        writeln!(
            out,
            "    legend_high [label=\"High (>=50)\", fillcolor=\"#FF8C00\"];"
        )?;
        writeln!(
            out,
            "    legend_medium [label=\"Medium (>=20)\", fillcolor=\"#FFD93D\"];"
        )?;
        writeln!(
            out,
            "    legend_low [label=\"Low (<20)\", fillcolor=\"#6BCB77\"];"
        )?;
        writeln!(
            out,
            "    legend_critical -> legend_high -> legend_medium -> legend_low [style=invis];"
        )?;
        writeln!(out, "  }}")?;
        Ok(())
    }

    /// Group files by their parent directory/module
    fn group_by_module<'a>(
        &self,
        items: &'a [FileDebtItemOutput],
    ) -> HashMap<String, Vec<&'a FileDebtItemOutput>> {
        let mut modules: HashMap<String, Vec<&'a FileDebtItemOutput>> = HashMap::new();

        for item in items {
            let module = extract_module(&item.location);
            modules.entry(module).or_default().push(item);
        }

        modules
    }

    /// Write a cluster (subgraph) for a module
    fn write_cluster<W: Write>(
        &self,
        out: &mut W,
        module: &str,
        items: &[&FileDebtItemOutput],
    ) -> io::Result<()> {
        let cluster_id = sanitize_id(module);
        writeln!(out, "  subgraph cluster_{} {{", cluster_id)?;
        writeln!(out, "    label=\"{}\";", escape_label(module))?;
        writeln!(out, "    style=rounded;")?;
        writeln!(out, "    bgcolor=\"#F5F5F5\";")?;
        writeln!(out, "    fontname=\"Helvetica\";")?;
        writeln!(out)?;

        for item in items {
            self.write_node(out, item, "    ")?;
        }

        writeln!(out, "  }}")?;
        Ok(())
    }

    /// Write a single node
    fn write_node<W: Write>(
        &self,
        out: &mut W,
        item: &FileDebtItemOutput,
        indent: &str,
    ) -> io::Result<()> {
        let id = path_to_id(&item.location.file);
        let label = path_to_label(&item.location.file);
        let color = score_to_color(item.score);
        let font_color = score_to_font_color(item.score);
        let tooltip = format!(
            "{}\\nScore: {:.1}\\nFunctions: {}\\nLines: {}",
            item.location.file, item.score, item.metrics.functions, item.metrics.lines
        );

        // Add coupling info to tooltip if available
        let tooltip = if let Some(deps) = &item.dependencies {
            format!(
                "{}\\nCoupling: {} (Ca={}, Ce={})\\nInstability: {:.2}",
                tooltip,
                deps.coupling_classification,
                deps.afferent_coupling,
                deps.efferent_coupling,
                deps.instability
            )
        } else {
            tooltip
        };

        writeln!(
            out,
            "{}\"{}\" [label=\"{}\", fillcolor=\"{}\", fontcolor=\"{}\", tooltip=\"{}\"];",
            indent, id, label, color, font_color, tooltip
        )?;
        Ok(())
    }

    /// Write all edges (dependencies)
    fn write_edges<W: Write>(
        &self,
        out: &mut W,
        graph: &HashMap<String, Vec<String>>,
    ) -> io::Result<()> {
        writeln!(out, "  // Dependencies")?;

        for (from_file, to_files) in graph {
            let from_id = path_to_id(from_file);
            for to_file in to_files {
                let to_id = path_to_id(to_file);
                writeln!(out, "  \"{}\" -> \"{}\";", from_id, to_id)?;
            }
        }

        Ok(())
    }
}

impl Default for DotWriter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a file path to a DOT-safe node ID
fn path_to_id(path: &str) -> String {
    path.replace(['/', '\\', '.', '-', ' '], "_")
}

/// Convert a file path to a display label (filename only)
fn path_to_label(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(path)
        .to_string()
}

/// Extract module name from file location
fn extract_module(location: &UnifiedLocation) -> String {
    let path = std::path::Path::new(&location.file);

    // Get parent directory as module name
    path.parent()
        .and_then(|p| p.to_str())
        .map(|s| {
            // Remove leading "./" or "src/" if present
            let s = s.strip_prefix("./").unwrap_or(s);
            let s = s.strip_prefix("src/").unwrap_or(s);
            // Handle the case where the parent is just "src" (no trailing path)
            let s = if s == "src" { "" } else { s };
            if s.is_empty() {
                "root".to_string()
            } else {
                s.to_string()
            }
        })
        .unwrap_or_else(|| "root".to_string())
}

/// Sanitize a string for use as a DOT identifier
fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Escape a string for use as a DOT label
fn escape_label(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Convert a debt score to a fill color
fn score_to_color(score: f64) -> &'static str {
    if score >= 100.0 {
        "#FF6B6B" // Red for critical
    } else if score >= 50.0 {
        "#FF8C00" // Orange for high
    } else if score >= 20.0 {
        "#FFD93D" // Yellow for medium
    } else {
        "#6BCB77" // Green for low
    }
}

/// Get appropriate font color for contrast
fn score_to_font_color(score: f64) -> &'static str {
    if score >= 100.0 {
        "white" // White text on red background
    } else {
        "black" // Black text for others
    }
}

/// Get edge style based on coupling classification
#[allow(dead_code)]
fn coupling_to_edge_style(classification: &CouplingClassification) -> &'static str {
    match classification {
        CouplingClassification::StableCore => "bold",
        CouplingClassification::HighlyCoupled => "dashed",
        CouplingClassification::Isolated => "dotted",
        _ => "solid",
    }
}

/// Render DOT output from unified analysis to a string
pub fn render_dot(analysis: &UnifiedAnalysis, config: DotConfig) -> io::Result<String> {
    let mut buffer = Vec::new();
    let writer = DotWriter::with_config(config);
    writer.write(analysis, &mut buffer)?;
    String::from_utf8(buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_id() {
        assert_eq!(path_to_id("src/main.rs"), "src_main_rs");
        assert_eq!(path_to_id("foo/bar-baz.rs"), "foo_bar_baz_rs");
        assert_eq!(path_to_id("simple.rs"), "simple_rs");
    }

    #[test]
    fn test_path_to_label() {
        assert_eq!(path_to_label("src/main.rs"), "main.rs");
        assert_eq!(path_to_label("foo/bar/baz.rs"), "baz.rs");
        assert_eq!(path_to_label("simple.rs"), "simple.rs");
    }

    #[test]
    fn test_extract_module() {
        let loc = UnifiedLocation {
            file: "src/io/writers/dot.rs".to_string(),
            line: None,
            function: None,
            file_context_label: None,
        };
        assert_eq!(extract_module(&loc), "io/writers");

        let loc = UnifiedLocation {
            file: "./src/main.rs".to_string(),
            line: None,
            function: None,
            file_context_label: None,
        };
        assert_eq!(extract_module(&loc), "root");
    }

    #[test]
    fn test_sanitize_id() {
        assert_eq!(sanitize_id("foo/bar"), "foo_bar");
        assert_eq!(sanitize_id("hello-world"), "hello_world");
        assert_eq!(sanitize_id("test_123"), "test_123");
    }

    #[test]
    fn test_escape_label() {
        assert_eq!(escape_label("foo\"bar"), "foo\\\"bar");
        assert_eq!(escape_label("foo\\bar"), "foo\\\\bar");
        assert_eq!(escape_label("foo\nbar"), "foo\\nbar");
    }

    #[test]
    fn test_score_to_color() {
        assert_eq!(score_to_color(150.0), "#FF6B6B"); // Critical
        assert_eq!(score_to_color(100.0), "#FF6B6B"); // Critical boundary
        assert_eq!(score_to_color(75.0), "#FF8C00"); // High
        assert_eq!(score_to_color(50.0), "#FF8C00"); // High boundary
        assert_eq!(score_to_color(35.0), "#FFD93D"); // Medium
        assert_eq!(score_to_color(20.0), "#FFD93D"); // Medium boundary
        assert_eq!(score_to_color(10.0), "#6BCB77"); // Low
    }

    #[test]
    fn test_score_to_font_color() {
        assert_eq!(score_to_font_color(100.0), "white"); // Critical gets white
        assert_eq!(score_to_font_color(50.0), "black"); // Others get black
    }

    #[test]
    fn test_rankdir_as_str() {
        assert_eq!(RankDir::TopBottom.as_str(), "TB");
        assert_eq!(RankDir::LeftRight.as_str(), "LR");
    }

    #[test]
    fn test_dot_config_default() {
        let config = DotConfig::default();
        assert!(config.min_score.is_none());
        assert!(config.max_depth.is_none());
        assert!(!config.include_external);
        assert!(config.cluster_by_module);
        assert!(matches!(config.rankdir, RankDir::TopBottom));
    }

    #[test]
    fn test_write_legend() {
        let writer = DotWriter::new();
        let mut buffer = Vec::new();
        writer.write_legend(&mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();

        assert!(output.contains("cluster_legend"));
        assert!(output.contains("Debt Score Legend"));
        assert!(output.contains("Critical (>=100)"));
        assert!(output.contains("High (>=50)"));
        assert!(output.contains("Medium (>=20)"));
        assert!(output.contains("Low (<20)"));
        assert!(output.contains("#FF6B6B")); // Critical color
        assert!(output.contains("#6BCB77")); // Low color
    }

    #[test]
    fn test_coupling_to_edge_style() {
        assert_eq!(
            coupling_to_edge_style(&CouplingClassification::StableCore),
            "bold"
        );
        assert_eq!(
            coupling_to_edge_style(&CouplingClassification::HighlyCoupled),
            "dashed"
        );
        assert_eq!(
            coupling_to_edge_style(&CouplingClassification::Isolated),
            "dotted"
        );
        assert_eq!(
            coupling_to_edge_style(&CouplingClassification::UtilityModule),
            "solid"
        );
        assert_eq!(
            coupling_to_edge_style(&CouplingClassification::LeafModule),
            "solid"
        );
    }
}
