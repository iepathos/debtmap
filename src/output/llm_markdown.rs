//! LLM-optimized markdown output (Spec 264)
//!
//! This module provides the output function for LLM-optimized markdown format,
//! designed for AI agent consumption.
//!
//! ## Location Grouping
//!
//! Items at the same location (file, function, line) are grouped together with
//! a combined score, matching the TUI list view behavior. This provides LLMs
//! with the same prioritized view that users see in the TUI.

use crate::io::writers::llm_markdown::format;
use crate::output::unified::{
    convert_to_unified_format, FunctionDebtItemOutput, Priority, UnifiedOutput,
};
use crate::priority::UnifiedAnalysis;
use crate::tui::results::detail_pages::overview::format_debt_type_name;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// A group of function debt items at the same location (file, function, line).
/// Matches TUI's LocationGroup behavior for consistent output.
#[derive(Debug)]
pub struct LocationGroup {
    /// Combined score (sum of all item scores)
    pub combined_score: f64,
    /// All function items at this location
    pub items: Vec<FunctionDebtItemOutput>,
}

impl LocationGroup {
    /// Get the representative item (first item) for location/metrics
    pub fn representative(&self) -> &FunctionDebtItemOutput {
        &self.items[0]
    }

    /// Get the highest priority among items
    pub fn max_priority(&self) -> &Priority {
        // Priority order: Critical > High > Medium > Low
        self.items
            .iter()
            .map(|i| &i.priority)
            .max_by_key(|p| match p {
                Priority::Critical => 3,
                Priority::High => 2,
                Priority::Medium => 1,
                Priority::Low => 0,
            })
            .unwrap_or(&Priority::Low)
    }
}

/// Group function items by location (file, function, line).
/// Returns groups sorted by combined score descending.
fn group_by_location(items: Vec<FunctionDebtItemOutput>) -> Vec<LocationGroup> {
    // Group by (file, function, line)
    let mut groups: HashMap<(String, Option<String>, Option<usize>), Vec<FunctionDebtItemOutput>> =
        HashMap::new();

    for item in items {
        let key = (
            item.location.file.clone(),
            item.location.function.clone(),
            item.location.line,
        );
        groups.entry(key).or_default().push(item);
    }

    // Convert to LocationGroup and compute combined scores
    let mut result: Vec<LocationGroup> = groups
        .into_values()
        .map(|items| {
            let combined_score = items.iter().map(|i| i.score).sum::<f64>();
            LocationGroup {
                combined_score,
                items,
            }
        })
        .collect();

    // Sort by combined score descending
    result.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    result
}

/// Output analysis as LLM-optimized markdown (Spec 264)
///
/// This function produces markdown designed for AI agent consumption:
/// - Hierarchical with consistent heading levels
/// - No decorative elements (emoji, boxes, separators)
/// - Complete with all available data
/// - Stable item IDs for reference
pub fn output_llm_markdown(analysis: &UnifiedAnalysis, output_file: Option<PathBuf>) -> Result<()> {
    output_llm_markdown_with_filters(analysis, None, None, output_file)
}

/// Output analysis as LLM-optimized markdown with filters (Spec 264)
pub fn output_llm_markdown_with_filters(
    analysis: &UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    output_file: Option<PathBuf>,
) -> Result<()> {
    output_llm_markdown_with_format(analysis, top, tail, output_file, false)
}

/// Output analysis as LLM-optimized markdown with full options (Spec 264)
pub fn output_llm_markdown_with_format(
    analysis: &UnifiedAnalysis,
    top: Option<usize>,
    tail: Option<usize>,
    output_file: Option<PathBuf>,
    include_scoring_details: bool,
) -> Result<()> {
    // Convert to unified format (same as JSON for consistency)
    let unified_output = convert_to_unified_format(analysis, include_scoring_details);

    // Apply filtering and grouping
    let (groups, filtered_output) = apply_filters_and_group(unified_output, top, tail);

    if let Some(path) = output_file {
        if let Some(parent) = path.parent() {
            crate::io::ensure_dir(parent)?;
        }
        let mut file = fs::File::create(path)?;
        write_grouped_markdown(&mut file, &groups, &filtered_output)?;
    } else {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        write_grouped_markdown(&mut handle, &groups, &filtered_output)?;
    }
    Ok(())
}

/// Apply filters to unified output and group by location for LLM markdown
///
/// Processing steps:
/// 1. Excludes File-level items (only shows Function items, matching TUI behavior)
/// 2. Groups items by location (file, function, line) with combined scores
/// 3. Applies top/tail limits to groups (not individual items)
fn apply_filters_and_group(
    mut output: UnifiedOutput,
    top: Option<usize>,
    tail: Option<usize>,
) -> (Vec<LocationGroup>, UnifiedOutput) {
    use crate::output::unified::UnifiedDebtItemOutput;

    // Filter to only Function items (matching TUI behavior which excludes File items)
    let function_items: Vec<FunctionDebtItemOutput> = output
        .items
        .into_iter()
        .filter_map(|item| match item {
            UnifiedDebtItemOutput::Function(f) => Some(*f),
            _ => None,
        })
        .collect();

    // Group by location (matching TUI's group_by_location behavior)
    let mut groups = group_by_location(function_items);

    // Apply top/tail to groups (not individual items)
    if let Some(n) = top {
        groups.truncate(n);
    } else if let Some(n) = tail {
        let total = groups.len();
        let skip = total.saturating_sub(n);
        groups = groups.into_iter().skip(skip).collect();
    }

    // Count total items across all groups for summary
    let total_items: usize = groups.iter().map(|g| g.items.len()).sum();

    // Update output.items to be empty (we use groups instead)
    output.items = vec![];
    output.summary.total_items = total_items;

    (groups, output)
}

/// Write grouped markdown output to a writer.
///
/// This produces markdown matching the TUI's LLM copy format with location grouping.
fn write_grouped_markdown<W: Write>(
    writer: &mut W,
    groups: &[LocationGroup],
    output: &UnifiedOutput,
) -> Result<()> {
    // Header
    writeln!(writer, "# Debtmap Analysis Report")?;
    writeln!(writer)?;

    // Metadata
    writeln!(writer, "## Metadata")?;
    writeln!(writer, "- Version: {}", output.metadata.debtmap_version)?;
    writeln!(writer, "- Generated: {}", output.metadata.generated_at)?;
    if let Some(ref project_root) = output.metadata.project_root {
        writeln!(writer, "- Project: {}", project_root.display())?;
    }
    writeln!(
        writer,
        "- Total Items Analyzed: {}",
        output.summary.total_items
    )?;
    writeln!(writer, "- Location Groups: {}", groups.len())?;
    writeln!(writer)?;

    // Summary
    writeln!(writer, "## Summary")?;
    writeln!(
        writer,
        "- Total Debt Score: {}",
        output.summary.total_debt_score
    )?;
    writeln!(
        writer,
        "- Debt Density: {} per 1K LOC",
        output.summary.debt_density
    )?;
    writeln!(writer, "- Total LOC: {}", output.summary.total_loc)?;
    writeln!(writer, "- Items by Severity:")?;
    writeln!(
        writer,
        "  - Critical: {}",
        output.summary.score_distribution.critical
    )?;
    writeln!(
        writer,
        "  - High: {}",
        output.summary.score_distribution.high
    )?;
    writeln!(
        writer,
        "  - Medium: {}",
        output.summary.score_distribution.medium
    )?;
    writeln!(writer, "  - Low: {}", output.summary.score_distribution.low)?;
    writeln!(writer)?;

    // Debt Items (grouped by location)
    writeln!(writer, "## Debt Items")?;
    writeln!(writer)?;

    for (index, group) in groups.iter().enumerate() {
        write_location_group(writer, index + 1, group)?;
    }

    Ok(())
}

/// Write a single location group to the output.
fn write_location_group<W: Write>(
    writer: &mut W,
    index: usize,
    group: &LocationGroup,
) -> Result<()> {
    let rep = group.representative();

    // Group header with combined score
    writeln!(writer, "### Item {}", index)?;
    writeln!(writer)?;

    // Identification section (shared across all items at this location)
    writeln!(writer, "#### Identification")?;
    writeln!(
        writer,
        "- ID: {}",
        generate_item_id(&rep.location.file, rep.location.line)
    )?;
    writeln!(writer, "- Type: Function")?;
    writeln!(
        writer,
        "- Location: {}:{}",
        rep.location.file,
        rep.location.line.unwrap_or(0)
    )?;
    if let Some(ref func_name) = rep.location.function {
        writeln!(writer, "- Function: {}", func_name)?;
    }
    writeln!(writer, "- Items at Location: {}", group.items.len())?;
    writeln!(writer)?;

    // Severity section with combined score
    writeln!(writer, "#### Severity")?;
    writeln!(writer, "- Combined Score: {:.2}", group.combined_score)?;
    writeln!(writer, "- Max Priority: {:?}", group.max_priority())?;
    writeln!(
        writer,
        "- Tier: {}",
        crate::io::writers::llm_markdown::priority_tier(group.combined_score)
    )?;
    writeln!(writer)?;

    // Debt types at this location
    writeln!(writer, "#### Debt Types")?;
    for item in &group.items {
        writeln!(
            writer,
            "- {} (score: {:.2})",
            format_debt_type_name(&item.debt_type),
            item.score
        )?;
    }
    writeln!(writer)?;

    // Metrics section (same for all items at location)
    write!(
        writer,
        "{}",
        format::metrics(&rep.metrics, rep.adjusted_complexity.as_ref())
    )?;
    writeln!(writer)?;

    // Coverage section (optional)
    if let Some(cov) = format::coverage(&rep.metrics) {
        write!(writer, "{}", cov)?;
        writeln!(writer)?;
    }

    // Dependencies section
    write!(writer, "{}", format::dependencies(&rep.dependencies))?;
    writeln!(writer)?;

    // Purity analysis section (optional)
    if let Some(pur) = format::purity(rep.purity_analysis.as_ref()) {
        write!(writer, "{}", pur)?;
        writeln!(writer)?;
    }

    // Pattern analysis section (optional)
    if let Some(pat) = format::pattern_analysis(rep.pattern_type.as_ref(), rep.pattern_confidence) {
        write!(writer, "{}", pat)?;
        writeln!(writer)?;
    }

    // Scoring breakdown (optional, show for first item with scoring details)
    if let Some(ref scoring) = rep.scoring_details {
        if let Some(scr) = format::scoring(Some(scoring), &rep.function_role) {
            write!(writer, "{}", scr)?;
            writeln!(writer)?;
        }
    }

    // Context section (critical for LLM agents)
    if let Some(ctx) = format::context(rep.context.as_ref()) {
        write!(writer, "{}", ctx)?;
        writeln!(writer)?;
    }

    // Git history section (optional)
    if let Some(git) = format::git_history(rep.git_history.as_ref()) {
        write!(writer, "{}", git)?;
        writeln!(writer)?;
    }

    writeln!(writer, "---")?;
    writeln!(writer)?;
    Ok(())
}

/// Generate a stable ID for an item based on file and line
fn generate_item_id(file: &str, line: Option<usize>) -> String {
    let file_part: String = file
        .chars()
        .map(|c| match c {
            '/' | '\\' | '.' | ' ' => '_',
            other => other,
        })
        .collect();
    match line {
        Some(l) => format!("{}_{}", file_part, l),
        None => file_part,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::unified::{
        Dependencies, FunctionImpactOutput, FunctionMetricsOutput, Priority, UnifiedLocation,
    };
    use crate::priority::call_graph::CallGraph;
    use crate::priority::{DebtType, FunctionRole};
    use tempfile::TempDir;

    fn create_test_function_item(
        file: &str,
        function: &str,
        line: usize,
        score: f64,
        debt_type: DebtType,
    ) -> FunctionDebtItemOutput {
        FunctionDebtItemOutput {
            score,
            category: "Testing".to_string(),
            priority: Priority::from_score(score),
            location: UnifiedLocation {
                file: file.to_string(),
                line: Some(line),
                function: Some(function.to_string()),
                file_context_label: None,
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: 10,
                cognitive_complexity: 15,
                length: 50,
                nesting_depth: 3,
                coverage: Some(0.5),
                ..Default::default()
            },
            debt_type,
            function_role: FunctionRole::Unknown,
            purity_analysis: None,
            dependencies: Dependencies::default(),
            impact: FunctionImpactOutput {
                coverage_improvement: 0.1,
                complexity_reduction: 0.1,
                risk_reduction: 0.1,
            },
            scoring_details: None,
            adjusted_complexity: None,
            complexity_pattern: None,
            pattern_type: None,
            pattern_confidence: None,
            pattern_details: None,
            context: None,
            git_history: None,
        }
    }

    #[test]
    fn test_output_llm_markdown_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("output.md");

        let call_graph = CallGraph::new();
        let analysis = UnifiedAnalysis::new(call_graph);

        let result = output_llm_markdown(&analysis, Some(output_path.clone()));
        assert!(
            result.is_ok(),
            "Failed to write LLM markdown: {:?}",
            result.err()
        );
        assert!(output_path.exists(), "Output file was not created");

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("# Debtmap Analysis Report"));
        assert!(content.contains("## Metadata"));
        assert!(content.contains("## Summary"));
        assert!(content.contains("## Debt Items"));
        assert!(content.contains("Location Groups:"));
    }

    #[test]
    fn test_output_llm_markdown_with_filters() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("filtered.md");

        let call_graph = CallGraph::new();
        let analysis = UnifiedAnalysis::new(call_graph);

        // Test with top=5
        let result =
            output_llm_markdown_with_filters(&analysis, Some(5), None, Some(output_path.clone()));
        assert!(result.is_ok());

        let content = fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("# Debtmap Analysis Report"));
    }

    #[test]
    fn test_output_llm_markdown_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir
            .path()
            .join("nested")
            .join("subdirs")
            .join("output.md");

        let call_graph = CallGraph::new();
        let analysis = UnifiedAnalysis::new(call_graph);

        let result = output_llm_markdown(&analysis, Some(nested_path.clone()));
        assert!(
            result.is_ok(),
            "Failed to write to nested path: {:?}",
            result.err()
        );
        assert!(
            nested_path.exists(),
            "Output file was not created at nested path"
        );
    }

    #[test]
    fn test_group_by_location_single_item() {
        let items = vec![create_test_function_item(
            "test.rs",
            "test_fn",
            10,
            50.0,
            DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 10,
                cognitive: 15,
            },
        )];

        let groups = group_by_location(items);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].items.len(), 1);
        assert_eq!(groups[0].combined_score, 50.0);
    }

    #[test]
    fn test_group_by_location_multiple_items_same_location() {
        let items = vec![
            create_test_function_item(
                "test.rs",
                "test_fn",
                10,
                30.0,
                DebtType::TestingGap {
                    coverage: 0.5,
                    cyclomatic: 10,
                    cognitive: 15,
                },
            ),
            create_test_function_item(
                "test.rs",
                "test_fn",
                10,
                20.0,
                DebtType::ComplexityHotspot {
                    cyclomatic: 25,
                    cognitive: 30,
                },
            ),
        ];

        let groups = group_by_location(items);
        assert_eq!(groups.len(), 1, "Items at same location should be grouped");
        assert_eq!(groups[0].items.len(), 2);
        assert_eq!(
            groups[0].combined_score, 50.0,
            "Combined score should be sum of individual scores"
        );
    }

    #[test]
    fn test_group_by_location_different_locations() {
        let items = vec![
            create_test_function_item(
                "test.rs",
                "test_fn",
                10,
                30.0,
                DebtType::TestingGap {
                    coverage: 0.5,
                    cyclomatic: 10,
                    cognitive: 15,
                },
            ),
            create_test_function_item(
                "other.rs",
                "other_fn",
                20,
                40.0,
                DebtType::ComplexityHotspot {
                    cyclomatic: 25,
                    cognitive: 30,
                },
            ),
        ];

        let groups = group_by_location(items);
        assert_eq!(
            groups.len(),
            2,
            "Items at different locations should be separate groups"
        );
        // Should be sorted by combined score descending
        assert_eq!(
            groups[0].combined_score, 40.0,
            "Higher score group should be first"
        );
        assert_eq!(groups[1].combined_score, 30.0);
    }

    #[test]
    fn test_group_by_location_sorts_by_combined_score() {
        let items = vec![
            create_test_function_item(
                "low.rs",
                "low_fn",
                10,
                10.0,
                DebtType::TestingGap {
                    coverage: 0.5,
                    cyclomatic: 10,
                    cognitive: 15,
                },
            ),
            create_test_function_item(
                "high.rs",
                "high_fn",
                10,
                20.0,
                DebtType::TestingGap {
                    coverage: 0.5,
                    cyclomatic: 10,
                    cognitive: 15,
                },
            ),
            create_test_function_item(
                "high.rs",
                "high_fn",
                10,
                20.0,
                DebtType::ComplexityHotspot {
                    cyclomatic: 25,
                    cognitive: 30,
                },
            ),
        ];

        let groups = group_by_location(items);
        assert_eq!(groups.len(), 2);
        // high.rs has combined score of 40, low.rs has 10
        assert_eq!(groups[0].combined_score, 40.0);
        assert_eq!(groups[1].combined_score, 10.0);
    }

    #[test]
    fn test_format_debt_type_name() {
        assert_eq!(
            format_debt_type_name(&DebtType::TestingGap {
                coverage: 0.5,
                cyclomatic: 10,
                cognitive: 15
            }),
            "Testing Gap"
        );
        assert_eq!(
            format_debt_type_name(&DebtType::ComplexityHotspot {
                cyclomatic: 25,
                cognitive: 30
            }),
            "High Complexity"
        );
        assert_eq!(
            format_debt_type_name(&DebtType::GodObject {
                methods: 50,
                fields: Some(20),
                responsibilities: 5,
                lines: 1000,
                god_object_score: 100.0
            }),
            "God Object"
        );
    }
}
