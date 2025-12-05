//! Enhanced markdown writer trait and implementation
//!
//! Contains the EnhancedMarkdownWriter trait and its implementation for MarkdownWriter,
//! providing unified analysis output capabilities.

use crate::priority::{UnifiedAnalysis, UnifiedAnalysisQueries, UnifiedDebtItem};
use std::io::Write;

use super::core::MarkdownWriter;
use super::formatters::{format_debt_issue, format_debt_type};

// Additional trait for enhanced markdown output
pub trait EnhancedMarkdownWriter {
    fn write_unified_analysis(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
    fn write_priority_section(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
    fn write_dead_code_section(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
    fn write_call_graph_insights(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
    fn write_testing_recommendations(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()>;
}

impl<W: Write> EnhancedMarkdownWriter for MarkdownWriter<W> {
    fn write_unified_analysis(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        self.write_priority_section(analysis)?;
        self.write_dead_code_section(analysis)?;
        self.write_call_graph_insights(analysis)?;
        self.write_testing_recommendations(analysis)?;
        Ok(())
    }

    fn write_priority_section(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        writeln!(self.writer(), "## Priority Technical Debt")?;
        writeln!(self.writer())?;

        // Get top 10 priorities
        let top_items = analysis.get_top_priorities(10);

        if top_items.is_empty() {
            writeln!(self.writer(), "_No priority items found._")?;
            writeln!(self.writer())?;
            return Ok(());
        }

        let items_vec: Vec<UnifiedDebtItem> = top_items.iter().cloned().collect();
        let table = build_priority_table(&items_vec);
        write!(self.writer(), "{}", table)?;

        // Add score breakdown if verbosity is enabled
        if self.verbosity() > 0 {
            self.write_score_breakdown(&items_vec)?;
        }

        Ok(())
    }

    fn write_dead_code_section(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        use super::dead_code::format_dead_code_section;

        let content = format_dead_code_section(analysis);
        if !content.is_empty() {
            write!(self.writer(), "{}", content)?;
        }
        Ok(())
    }

    fn write_call_graph_insights(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        if self.verbosity() < 2 {
            return Ok(());
        }

        writeln!(self.writer(), "## Call Graph Analysis")?;
        writeln!(self.writer())?;

        // Show module dependency statistics
        // The call graph structure uses different field names
        let total_functions = analysis.call_graph.node_count();
        let total_relationships = analysis
            .items
            .iter()
            .map(|item| item.downstream_dependencies)
            .sum::<usize>();

        writeln!(self.writer(), "### Module Statistics")?;
        writeln!(self.writer())?;
        writeln!(self.writer(), "- Total Functions: {}", total_functions)?;
        writeln!(
            self.writer(),
            "- Total Call Relationships: {}",
            total_relationships
        )?;
        writeln!(self.writer())?;

        Ok(())
    }

    fn write_testing_recommendations(&mut self, analysis: &UnifiedAnalysis) -> anyhow::Result<()> {
        use super::testing::{collect_testing_gaps, format_testing_recommendations};

        writeln!(self.writer(), "## Testing Recommendations")?;
        writeln!(self.writer())?;

        // Get top untested functions with high ROI
        // Convert im::Vector to slice for compatibility
        let items_vec: Vec<UnifiedDebtItem> = analysis.items.iter().cloned().collect();
        let testing_gaps = collect_testing_gaps(&items_vec);

        // Format and write the recommendations
        let recommendations = format_testing_recommendations(&testing_gaps);
        write!(self.writer(), "{}", recommendations)?;

        Ok(())
    }
}

impl<W: Write> MarkdownWriter<W> {
    pub(super) fn write_score_breakdown(
        &mut self,
        items: &[UnifiedDebtItem],
    ) -> anyhow::Result<()> {
        let breakdown = format_score_breakdown(items);
        write!(self.writer(), "{}", breakdown)?;
        Ok(())
    }
}

// Pure functions for formatting priority table
/// Builds the complete priority table from a list of debt items.
///
/// This function orchestrates the creation of a markdown-formatted table showing
/// the top priority technical debt items. It combines the header and all rows
/// into a single string output.
///
/// # Arguments
/// * `items` - Slice of unified debt items to display in the table
///
/// # Returns
/// A string containing the complete markdown table with header and all rows
fn build_priority_table(items: &[UnifiedDebtItem]) -> String {
    let mut table = format_priority_table_header(items.len());

    for (idx, item) in items.iter().enumerate() {
        let rank = idx + 1;
        table.push_str(&format_priority_table_row(rank, item));
    }

    table.push('\n');
    table
}

/// Formats the header section for the priority table.
///
/// Creates a markdown section header and table column headers for the priority
/// items table.
///
/// # Arguments
/// * `item_count` - Number of items to mention in the section header
///
/// # Returns
/// A string containing the markdown header and table column definitions
fn format_priority_table_header(item_count: usize) -> String {
    format!(
        "### Top {} Priority Items\n\n| Rank | Score | Function | Type | Issue |\n|------|-------|----------|------|-------|\n",
        item_count
    )
}

/// Formats a single row for the priority table.
///
/// Creates a markdown table row with the debt item's rank, priority score,
/// location, type, and issue description.
///
/// # Arguments
/// * `rank` - The ranking position of this item (1-indexed)
/// * `item` - The unified debt item to format
///
/// # Returns
/// A string containing the markdown table row with newline
fn format_priority_table_row(rank: usize, item: &UnifiedDebtItem) -> String {
    let score = format!("{:.1}", item.unified_score.final_score);
    let location = format!("{}:{}", item.location.file.display(), item.location.line);
    let debt_type = format_debt_type(&item.debt_type);
    let issue = format_debt_issue(&item.debt_type);

    format!(
        "| {} | {} | `{}` | {} | {} |\n",
        rank, score, location, debt_type, issue
    )
}

// Pure functions for formatting score breakdown
fn format_score_breakdown(items: &[UnifiedDebtItem]) -> String {
    let mut output = String::new();
    output.push_str("<details>\n");
    output.push_str("<summary>Score Breakdown (click to expand)</summary>\n\n");

    for (idx, item) in items.iter().enumerate().take(3) {
        output.push_str(&format_item_breakdown(idx + 1, item));
    }

    output.push_str("</details>\n\n");
    output
}

fn format_item_breakdown(number: usize, item: &UnifiedDebtItem) -> String {
    let mut result = format!(
        "#### {}. {}\n\n{}\n",
        number,
        item.location.function,
        format_score_factors(&item.unified_score)
    );

    // Add god object indicators if present
    if let Some(ref god_obj) = item.god_object_indicators {
        if god_obj.is_god_object {
            result.push_str(&format!(
                "- **God Object Warning**: {} methods, {} fields, {} responsibilities (score: {:.1}%)\n",
                god_obj.method_count,
                god_obj.field_count,
                god_obj.responsibility_count,
                god_obj.god_object_score
            ));
        }
    }

    result
}

fn format_score_factors(score: &crate::priority::unified_scorer::UnifiedScore) -> String {
    format!(
        "- **Priority Score**: {:.2}\n\
         - **Complexity Factor**: {:.2}\n\
         - **Coverage Factor**: {:.2}\n\
         - **Dependency Factor**: {:.2}\n\
",
        score.final_score, score.complexity_factor, score.coverage_factor, score.dependency_factor
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::unified_scorer::{Location, UnifiedDebtItem, UnifiedScore};
    use crate::priority::{ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics};

    fn create_test_item(function_name: &str, final_score: f64) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: std::path::PathBuf::from("test.rs"),
                line: 10,
                function: function_name.to_string(),
            },
            unified_score: UnifiedScore {
                final_score,
                complexity_factor: 0.8,
                coverage_factor: 0.6,
                dependency_factor: 0.5,
                role_multiplier: 1.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
                adjusted_cyclomatic: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Refactor to reduce complexity".to_string(),
                rationale: "Test recommendation".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 5.0,
                coverage_improvement: 0.1,
                lines_reduction: 10,
                risk_reduction: 0.2,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 1,
            downstream_dependencies: 2,
            upstream_callers: vec![],
            downstream_callees: vec![],
            nesting_depth: 0,
            function_length: 50,
            cyclomatic_complexity: 15,
            cognitive_complexity: 20,
            is_pure: None,
            purity_confidence: None,
            purity_level: None,
            entropy_details: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None, // spec 190
            detected_pattern: None,
            contextual_risk: None,
        }
    }

    #[test]
    fn test_build_priority_table_empty() {
        let items: Vec<UnifiedDebtItem> = vec![];
        let table = build_priority_table(&items);

        // Empty case should still have header
        assert!(table.contains("### Top 0 Priority Items"));
        assert!(table.contains("| Rank | Score | Function | Type | Issue |"));
    }

    #[test]
    fn test_build_priority_table_single_item() {
        let items = vec![create_test_item("function_one", 9.0)];
        let table = build_priority_table(&items);

        assert!(table.contains("### Top 1 Priority Items"));
        assert!(table.contains("| 1 |"));
        assert!(table.contains("| 9.0 |"));
        assert!(table.contains("| `test.rs:10` |"));
    }

    #[test]
    fn test_build_priority_table_multiple_items() {
        let items = vec![
            create_test_item("function_one", 9.0),
            create_test_item("function_two", 7.5),
            create_test_item("function_three", 6.0),
        ];
        let table = build_priority_table(&items);

        assert!(table.contains("### Top 3 Priority Items"));
        assert!(table.contains("| 1 |"));
        assert!(table.contains("| 9.0 |"));
        assert!(table.contains("| 2 |"));
        assert!(table.contains("| 7.5 |"));
        assert!(table.contains("| 3 |"));
        assert!(table.contains("| 6.0 |"));
    }

    #[test]
    fn test_format_priority_table_header() {
        let header = format_priority_table_header(5);

        assert!(header.contains("### Top 5 Priority Items"));
        assert!(header.contains("| Rank | Score | Function | Type | Issue |"));
        assert!(header.contains("|------|-------|----------|------|-------|"));
    }

    #[test]
    fn test_format_priority_table_header_single_item() {
        let header = format_priority_table_header(1);

        assert!(header.contains("### Top 1 Priority Items"));
    }

    #[test]
    fn test_format_priority_table_row() {
        let item = create_test_item("my_complex_function", 23.78);
        let row = format_priority_table_row(1, &item);

        assert!(row.contains("| 1 |"));
        assert!(row.contains("| 23.8 |"));
        assert!(row.contains("| `test.rs:10` |"));
        assert!(row.contains("Complexity"));
    }

    #[test]
    fn test_format_priority_table_row_formatting() {
        let item = create_test_item("another_function", 15.5);
        let row = format_priority_table_row(5, &item);

        // Should be a proper markdown table row
        assert!(row.starts_with("| 5 |"));
        assert!(row.contains("| 15.5 |"));
        assert!(row.ends_with("|\n"));
    }

    #[test]
    fn test_format_score_factors() {
        let score = UnifiedScore {
            final_score: 7.89,
            complexity_factor: 0.85,
            coverage_factor: 0.65,
            dependency_factor: 0.45,
            role_multiplier: 1.0,
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
        };

        let result = format_score_factors(&score);

        assert!(result.contains("Priority Score**: 7.89"));
        assert!(result.contains("Complexity Factor**: 0.85"));
        assert!(result.contains("Coverage Factor**: 0.65"));
        assert!(result.contains("Dependency Factor**: 0.45"));
    }

    #[test]
    fn test_format_item_breakdown() {
        let item = create_test_item("test_function", 8.5);
        let result = format_item_breakdown(1, &item);

        assert!(result.starts_with("#### 1. test_function\n"));
        assert!(result.contains("Priority Score**: 8.50"));
        assert!(result.contains("Complexity Factor**: 0.80"));
    }

    #[test]
    fn test_format_score_breakdown_empty() {
        let items: Vec<UnifiedDebtItem> = vec![];
        let result = format_score_breakdown(&items);

        assert!(result.starts_with("<details>\n"));
        assert!(result.contains("<summary>Score Breakdown (click to expand)</summary>"));
        assert!(result.ends_with("</details>\n\n"));
    }

    #[test]
    fn test_format_score_breakdown_single_item() {
        let items = vec![create_test_item("function_one", 9.0)];
        let result = format_score_breakdown(&items);

        assert!(result.contains("#### 1. function_one"));
        assert!(result.contains("Priority Score**: 9.00"));
        assert!(result.contains("<details>"));
        assert!(result.contains("</details>"));
    }

    #[test]
    fn test_format_score_breakdown_multiple_items() {
        let items = vec![
            create_test_item("function_one", 9.0),
            create_test_item("function_two", 7.5),
            create_test_item("function_three", 6.0),
        ];
        let result = format_score_breakdown(&items);

        assert!(result.contains("#### 1. function_one"));
        assert!(result.contains("#### 2. function_two"));
        assert!(result.contains("#### 3. function_three"));
        assert!(result.contains("Priority Score**: 9.00"));
        assert!(result.contains("Priority Score**: 7.50"));
        assert!(result.contains("Priority Score**: 6.00"));
    }

    #[test]
    fn test_format_score_breakdown_limits_to_three() {
        let items = vec![
            create_test_item("function_one", 9.0),
            create_test_item("function_two", 7.5),
            create_test_item("function_three", 6.0),
            create_test_item("function_four", 5.0),
            create_test_item("function_five", 4.0),
        ];
        let result = format_score_breakdown(&items);

        // Should only include first three
        assert!(result.contains("#### 1. function_one"));
        assert!(result.contains("#### 2. function_two"));
        assert!(result.contains("#### 3. function_three"));
        assert!(!result.contains("#### 4. function_four"));
        assert!(!result.contains("#### 5. function_five"));
    }

    #[test]
    fn test_format_score_factors_precision() {
        let score = UnifiedScore {
            final_score: 7.899999,
            complexity_factor: 0.855555,
            coverage_factor: 0.654321,
            dependency_factor: 0.456789,
            role_multiplier: 1.0,
            base_score: None,
            exponential_factor: None,
            risk_boost: None,
            pre_adjustment_score: None,
            adjustment_applied: None,
        };

        let result = format_score_factors(&score);

        // Check that all values are rounded to 2 decimal places
        assert!(result.contains("Priority Score**: 7.90"));
        assert!(result.contains("Complexity Factor**: 0.86"));
        assert!(result.contains("Coverage Factor**: 0.65"));
        assert!(result.contains("Dependency Factor**: 0.46"));
    }

    #[test]
    fn test_format_item_breakdown_escapes_special_chars() {
        let mut item = create_test_item("test_function_with_<special>&_chars", 8.5);
        item.location.function = "test_function_with_<special>&_chars".to_string();
        let result = format_item_breakdown(1, &item);

        // The function name should be included as-is in markdown
        assert!(result.contains("test_function_with_<special>&_chars"));
    }
}
