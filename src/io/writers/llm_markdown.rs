//! LLM-optimized markdown writer (Spec 264)
//!
//! Produces machine-parseable markdown designed for AI agent consumption.
//! Key characteristics:
//! - Hierarchical with consistent heading levels
//! - No decorative elements (emoji, boxes, separators)
//! - Complete with all available data
//! - Stable item IDs for reference

use crate::core::AnalysisResults;
use crate::io::output::OutputWriter;
use crate::output::unified::{
    FileDebtItemOutput, FunctionDebtItemOutput, UnifiedDebtItemOutput, UnifiedOutput,
};
use crate::risk::RiskInsight;
use std::io::Write;

/// LLM-optimized markdown writer (Spec 264)
///
/// Produces markdown designed for AI agent consumption:
/// - Consistent structure across all items
/// - No decorative formatting
/// - Complete data with all scoring factors
/// - Stable item IDs for reliable reference
pub struct LlmMarkdownWriter<W: Write> {
    writer: W,
}

impl<W: Write> LlmMarkdownWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write a UnifiedOutput as LLM-optimized markdown
    pub fn write_unified_output(&mut self, output: &UnifiedOutput) -> anyhow::Result<()> {
        self.write_header(output)?;
        self.write_metadata(output)?;
        self.write_summary(output)?;
        self.write_items(output)?;
        Ok(())
    }

    fn write_header(&mut self, _output: &UnifiedOutput) -> anyhow::Result<()> {
        writeln!(self.writer, "# Debtmap Analysis Report")?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_metadata(&mut self, output: &UnifiedOutput) -> anyhow::Result<()> {
        writeln!(self.writer, "## Metadata")?;
        writeln!(
            self.writer,
            "- Version: {}",
            output.metadata.debtmap_version
        )?;
        writeln!(self.writer, "- Generated: {}", output.metadata.generated_at)?;
        if let Some(ref project_root) = output.metadata.project_root {
            writeln!(self.writer, "- Project: {}", project_root.display())?;
        }
        writeln!(
            self.writer,
            "- Total Items Analyzed: {}",
            output.summary.total_items
        )?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_summary(&mut self, output: &UnifiedOutput) -> anyhow::Result<()> {
        writeln!(self.writer, "## Summary")?;
        writeln!(
            self.writer,
            "- Total Debt Score: {}",
            output.summary.total_debt_score
        )?;
        writeln!(
            self.writer,
            "- Debt Density: {} per 1K LOC",
            output.summary.debt_density
        )?;
        writeln!(self.writer, "- Total LOC: {}", output.summary.total_loc)?;
        writeln!(self.writer, "- Items by Severity:")?;
        writeln!(
            self.writer,
            "  - Critical: {}",
            output.summary.score_distribution.critical
        )?;
        writeln!(
            self.writer,
            "  - High: {}",
            output.summary.score_distribution.high
        )?;
        writeln!(
            self.writer,
            "  - Medium: {}",
            output.summary.score_distribution.medium
        )?;
        writeln!(
            self.writer,
            "  - Low: {}",
            output.summary.score_distribution.low
        )?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_items(&mut self, output: &UnifiedOutput) -> anyhow::Result<()> {
        writeln!(self.writer, "## Debt Items")?;
        writeln!(self.writer)?;

        for (index, item) in output.items.iter().enumerate() {
            self.write_item(index + 1, item)?;
        }
        Ok(())
    }

    fn write_item(&mut self, index: usize, item: &UnifiedDebtItemOutput) -> anyhow::Result<()> {
        writeln!(self.writer, "### Item {}", index)?;
        writeln!(self.writer)?;

        match item {
            UnifiedDebtItemOutput::Function(func) => self.write_function_item(func),
            UnifiedDebtItemOutput::File(file) => self.write_file_item(file),
        }
    }

    fn write_function_item(&mut self, item: &FunctionDebtItemOutput) -> anyhow::Result<()> {
        // Identification section
        writeln!(self.writer, "#### Identification")?;
        writeln!(
            self.writer,
            "- ID: {}",
            generate_item_id(&item.location.file, item.location.line)
        )?;
        writeln!(self.writer, "- Type: Function")?;
        writeln!(
            self.writer,
            "- Location: {}:{}",
            item.location.file,
            item.location.line.unwrap_or(0)
        )?;
        if let Some(ref func_name) = item.location.function {
            writeln!(self.writer, "- Function: {}", func_name)?;
        }
        writeln!(self.writer, "- Category: {}", item.category)?;
        writeln!(self.writer)?;

        // Severity section
        writeln!(self.writer, "#### Severity")?;
        writeln!(self.writer, "- Score: {}", item.score)?;
        writeln!(self.writer, "- Priority: {:?}", item.priority)?;
        writeln!(self.writer, "- Tier: {}", priority_tier(item.score))?;
        writeln!(self.writer)?;

        // Metrics section
        writeln!(self.writer, "#### Metrics")?;
        writeln!(
            self.writer,
            "- Cyclomatic Complexity: {}",
            item.metrics.cyclomatic_complexity
        )?;
        writeln!(
            self.writer,
            "- Cognitive Complexity: {}",
            item.metrics.cognitive_complexity
        )?;
        writeln!(
            self.writer,
            "- Nesting Depth: {}",
            item.metrics.nesting_depth
        )?;
        writeln!(self.writer, "- Lines of Code: {}", item.metrics.length)?;
        if let Some(entropy) = item.metrics.entropy_score {
            writeln!(self.writer, "- Entropy Score: {:.2}", entropy)?;
        }
        if let Some(ref adjusted) = item.adjusted_complexity {
            writeln!(
                self.writer,
                "- Dampening Factor: {:.2}",
                adjusted.dampening_factor
            )?;
            writeln!(
                self.writer,
                "- Dampened Cyclomatic: {:.1}",
                adjusted.dampened_cyclomatic
            )?;
        }
        writeln!(self.writer)?;

        // Coverage section
        if item.metrics.coverage.is_some() {
            writeln!(self.writer, "#### Coverage")?;
            if let Some(coverage) = item.metrics.coverage {
                writeln!(self.writer, "- Direct Coverage: {:.0}%", coverage * 100.0)?;
            }
            writeln!(self.writer)?;
        }

        // Dependencies section
        writeln!(self.writer, "#### Dependencies")?;
        writeln!(
            self.writer,
            "- Upstream Callers: {}",
            item.dependencies.upstream_count
        )?;
        writeln!(
            self.writer,
            "- Downstream Callees: {}",
            item.dependencies.downstream_count
        )?;
        if !item.dependencies.upstream_callers.is_empty() {
            writeln!(self.writer, "- Top Callers:")?;
            for caller in item.dependencies.upstream_callers.iter().take(3) {
                writeln!(self.writer, "  - {}", caller)?;
            }
        }
        if !item.dependencies.downstream_callees.is_empty() {
            writeln!(self.writer, "- Top Callees:")?;
            for callee in item.dependencies.downstream_callees.iter().take(3) {
                writeln!(self.writer, "  - {}", callee)?;
            }
        }
        writeln!(self.writer)?;

        // Purity analysis section
        if let Some(ref purity) = item.purity_analysis {
            writeln!(self.writer, "#### Purity Analysis")?;
            writeln!(self.writer, "- Is Pure: {}", purity.is_pure)?;
            writeln!(self.writer, "- Confidence: {:.2}", purity.confidence)?;
            if let Some(ref side_effects) = purity.side_effects {
                if !side_effects.is_empty() {
                    writeln!(self.writer, "- Detected Side Effects:")?;
                    for effect in side_effects {
                        writeln!(self.writer, "  - {}", effect)?;
                    }
                }
            }
            writeln!(self.writer)?;
        }

        // Pattern analysis section
        if item.pattern_type.is_some() || item.pattern_confidence.is_some() {
            writeln!(self.writer, "#### Pattern Analysis")?;
            if let Some(ref pattern_type) = item.pattern_type {
                writeln!(self.writer, "- Pattern Type: {}", pattern_type)?;
            }
            if let Some(confidence) = item.pattern_confidence {
                writeln!(self.writer, "- Pattern Confidence: {:.2}", confidence)?;
            }
            writeln!(self.writer)?;
        }

        // Scoring breakdown section
        if let Some(ref scoring) = item.scoring_details {
            writeln!(self.writer, "#### Scoring Breakdown")?;
            writeln!(self.writer, "- Base Score: {}", scoring.base_score)?;
            writeln!(
                self.writer,
                "- Complexity Factor: {} (weight: 0.4)",
                scoring.complexity_score
            )?;
            writeln!(
                self.writer,
                "- Coverage Factor: {} (weight: 0.3)",
                scoring.coverage_score
            )?;
            writeln!(
                self.writer,
                "- Dependency Factor: {} (weight: 0.2)",
                scoring.dependency_score
            )?;
            writeln!(
                self.writer,
                "- Role Multiplier: {} ({:?})",
                scoring.role_multiplier, item.function_role
            )?;
            if let Some(purity_factor) = scoring.purity_factor {
                writeln!(self.writer, "- Purity Factor: {:.2}", purity_factor)?;
            }
            writeln!(self.writer)?;
        }

        // Context to read section (Spec 263)
        if let Some(ref ctx) = item.context {
            writeln!(self.writer, "#### Context to Read")?;
            writeln!(self.writer, "- Total Lines: {}", ctx.total_lines)?;
            writeln!(
                self.writer,
                "- Completeness Confidence: {:.2}",
                ctx.completeness_confidence
            )?;
            writeln!(self.writer, "- Primary:")?;
            writeln!(
                self.writer,
                "  - {}:{}-{} ({})",
                ctx.primary.file,
                ctx.primary.start_line,
                ctx.primary.end_line,
                ctx.primary.symbol.as_deref().unwrap_or("Unknown")
            )?;
            if !ctx.related.is_empty() {
                writeln!(self.writer, "- Related:")?;
                for related in &ctx.related {
                    writeln!(
                        self.writer,
                        "  - {}:{}-{} ({})",
                        related.range.file,
                        related.range.start_line,
                        related.range.end_line,
                        related.relationship
                    )?;
                }
            }
            writeln!(self.writer)?;
        }

        writeln!(self.writer, "---")?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_file_item(&mut self, item: &FileDebtItemOutput) -> anyhow::Result<()> {
        // Identification section
        writeln!(self.writer, "#### Identification")?;
        writeln!(
            self.writer,
            "- ID: {}",
            generate_item_id(&item.location.file, None)
        )?;
        writeln!(self.writer, "- Type: File")?;
        writeln!(self.writer, "- Location: {}", item.location.file)?;
        writeln!(self.writer, "- Category: {}", item.category)?;
        writeln!(self.writer)?;

        // Severity section
        writeln!(self.writer, "#### Severity")?;
        writeln!(self.writer, "- Score: {}", item.score)?;
        writeln!(self.writer, "- Priority: {:?}", item.priority)?;
        writeln!(self.writer, "- Tier: {}", priority_tier(item.score))?;
        writeln!(self.writer)?;

        // Metrics section
        writeln!(self.writer, "#### Metrics")?;
        writeln!(self.writer, "- Lines: {}", item.metrics.lines)?;
        writeln!(self.writer, "- Functions: {}", item.metrics.functions)?;
        writeln!(self.writer, "- Classes: {}", item.metrics.classes)?;
        writeln!(
            self.writer,
            "- Average Complexity: {:.1}",
            item.metrics.avg_complexity
        )?;
        writeln!(
            self.writer,
            "- Max Complexity: {}",
            item.metrics.max_complexity
        )?;
        writeln!(
            self.writer,
            "- Total Complexity: {}",
            item.metrics.total_complexity
        )?;
        writeln!(
            self.writer,
            "- Coverage: {:.0}%",
            item.metrics.coverage * 100.0
        )?;
        writeln!(
            self.writer,
            "- Uncovered Lines: {}",
            item.metrics.uncovered_lines
        )?;
        writeln!(self.writer)?;

        // God object indicators section
        if let Some(ref god) = item.god_object_indicators {
            writeln!(self.writer, "#### God Object Analysis")?;
            writeln!(self.writer, "- Is God Object: {}", god.is_god_object)?;
            writeln!(self.writer, "- Method Count: {}", god.methods_count)?;
            writeln!(self.writer, "- Field Count: {}", god.fields_count)?;
            writeln!(
                self.writer,
                "- Responsibility Count: {}",
                god.responsibilities
            )?;
            writeln!(
                self.writer,
                "- God Object Score: {:.2}",
                god.god_object_score
            )?;
            writeln!(self.writer)?;
        }

        // Cohesion section
        if let Some(ref cohesion) = item.cohesion {
            writeln!(self.writer, "#### Cohesion Analysis")?;
            writeln!(self.writer, "- Cohesion Score: {:.2}", cohesion.score)?;
            writeln!(
                self.writer,
                "- Classification: {:?}",
                cohesion.classification
            )?;
            writeln!(self.writer)?;
        }

        // Scoring details section
        if let Some(ref scoring) = item.scoring_details {
            writeln!(self.writer, "#### Scoring Breakdown")?;
            writeln!(
                self.writer,
                "- File Size Score: {}",
                scoring.file_size_score
            )?;
            writeln!(
                self.writer,
                "- Function Count Score: {}",
                scoring.function_count_score
            )?;
            writeln!(
                self.writer,
                "- Complexity Score: {}",
                scoring.complexity_score
            )?;
            writeln!(
                self.writer,
                "- Coverage Penalty: {}",
                scoring.coverage_penalty
            )?;
            writeln!(self.writer)?;
        }

        writeln!(self.writer, "---")?;
        writeln!(self.writer)?;
        Ok(())
    }
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

/// Determine the priority tier based on score
fn priority_tier(score: f64) -> &'static str {
    if score >= 100.0 {
        "Critical (>=100)"
    } else if score >= 50.0 {
        "High (>=50)"
    } else if score >= 20.0 {
        "Medium (>=20)"
    } else {
        "Low (<20)"
    }
}

// Implement OutputWriter trait for legacy compatibility
impl<W: Write> OutputWriter for LlmMarkdownWriter<W> {
    fn write_results(&mut self, results: &AnalysisResults) -> anyhow::Result<()> {
        // For legacy AnalysisResults, write basic markdown
        // This is a fallback - the preferred path is write_unified_output
        writeln!(self.writer, "# Debtmap Analysis Report")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "## Metadata")?;
        writeln!(
            self.writer,
            "- Generated: {}",
            results.timestamp.format("%Y-%m-%dT%H:%M:%SZ")
        )?;
        writeln!(self.writer, "- Project: {}", results.project_path.display())?;
        writeln!(self.writer)?;
        writeln!(self.writer, "## Summary")?;
        writeln!(
            self.writer,
            "- Files Analyzed: {}",
            results.complexity.metrics.len()
        )?;
        writeln!(
            self.writer,
            "- Total Functions: {}",
            results.complexity.summary.total_functions
        )?;
        writeln!(
            self.writer,
            "- Average Complexity: {:.1}",
            results.complexity.summary.average_complexity
        )?;
        writeln!(
            self.writer,
            "- Technical Debt Items: {}",
            results.technical_debt.items.len()
        )?;
        writeln!(self.writer)?;
        Ok(())
    }

    fn write_risk_insights(&mut self, insights: &RiskInsight) -> anyhow::Result<()> {
        writeln!(self.writer, "## Risk Analysis")?;
        writeln!(self.writer)?;
        writeln!(self.writer, "### Risk Summary")?;
        writeln!(
            self.writer,
            "- Codebase Risk Score: {:.1}",
            insights.codebase_risk_score
        )?;
        if let Some(correlation) = insights.complexity_coverage_correlation {
            writeln!(
                self.writer,
                "- Complexity-Coverage Correlation: {:.2}",
                correlation
            )?;
        }
        writeln!(self.writer)?;
        writeln!(self.writer, "### Risk Distribution")?;
        writeln!(
            self.writer,
            "- Critical: {}",
            insights.risk_distribution.critical_count
        )?;
        writeln!(
            self.writer,
            "- High: {}",
            insights.risk_distribution.high_count
        )?;
        writeln!(
            self.writer,
            "- Medium: {}",
            insights.risk_distribution.medium_count
        )?;
        writeln!(
            self.writer,
            "- Low: {}",
            insights.risk_distribution.low_count
        )?;
        writeln!(
            self.writer,
            "- Well Tested: {}",
            insights.risk_distribution.well_tested_count
        )?;
        writeln!(self.writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_item_id() {
        assert_eq!(generate_item_id("src/main.rs", Some(42)), "src_main_rs_42");
        assert_eq!(generate_item_id("src/lib.rs", None), "src_lib_rs");
        assert_eq!(
            generate_item_id("path/to/file.rs", Some(100)),
            "path_to_file_rs_100"
        );
    }

    #[test]
    fn test_priority_tier() {
        assert_eq!(priority_tier(150.0), "Critical (>=100)");
        assert_eq!(priority_tier(100.0), "Critical (>=100)");
        assert_eq!(priority_tier(75.0), "High (>=50)");
        assert_eq!(priority_tier(50.0), "High (>=50)");
        assert_eq!(priority_tier(30.0), "Medium (>=20)");
        assert_eq!(priority_tier(20.0), "Medium (>=20)");
        assert_eq!(priority_tier(10.0), "Low (<20)");
        assert_eq!(priority_tier(0.0), "Low (<20)");
    }

    #[test]
    fn test_llm_markdown_writer_basic() {
        let mut buffer = Vec::new();
        let mut writer = LlmMarkdownWriter::new(&mut buffer);

        // Create a minimal unified output for testing
        let output = UnifiedOutput {
            format_version: "3.0".to_string(),
            metadata: crate::output::unified::OutputMetadata {
                debtmap_version: "0.9.2".to_string(),
                generated_at: "2024-12-19T10:30:00Z".to_string(),
                project_root: None,
                analysis_type: "unified".to_string(),
            },
            summary: crate::output::unified::DebtSummary {
                total_items: 0,
                total_debt_score: 0.0,
                debt_density: 0.0,
                total_loc: 0,
                by_type: crate::output::unified::TypeBreakdown {
                    file: 0,
                    function: 0,
                },
                by_category: std::collections::HashMap::new(),
                score_distribution: crate::output::unified::ScoreDistribution {
                    critical: 0,
                    high: 0,
                    medium: 0,
                    low: 0,
                },
                cohesion: None,
            },
            items: vec![],
        };

        let result = writer.write_unified_output(&output);
        assert!(result.is_ok());

        let markdown = String::from_utf8(buffer).unwrap();
        assert!(markdown.contains("# Debtmap Analysis Report"));
        assert!(markdown.contains("## Metadata"));
        assert!(markdown.contains("## Summary"));
        assert!(markdown.contains("## Debt Items"));
        // No decorative elements - check for common ASCII decorative chars
        assert!(!markdown.contains("```"), "Should not contain code blocks");
        // Verify it's clean markdown without special characters
        assert!(
            !markdown.contains("===") && !markdown.contains("---\n---"),
            "Should not contain decorative separators"
        );
    }
}
