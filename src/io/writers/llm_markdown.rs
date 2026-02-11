//! LLM-optimized markdown writer (Spec 264)
//!
//! Produces machine-parseable markdown designed for AI agent consumption.
//! Key characteristics:
//! - Hierarchical with consistent heading levels
//! - No decorative elements (emoji, boxes, separators)
//! - Complete with all available data
//! - Stable item IDs for reference
//!
//! Architecture follows Stillwater's "Pure Core, Imperative Shell" pattern:
//! - Pure formatting functions return Strings (the "still" core)
//! - Writer methods handle I/O (the "flowing" shell)

use crate::core::AnalysisResults;
use crate::io::output::OutputWriter;
use crate::output::unified::{
    Dependencies, FileDebtItemOutput, FileScoringDetails, FunctionDebtItemOutput,
    FunctionMetricsOutput, FunctionScoringDetails, GitHistoryOutput, UnifiedDebtItemOutput,
    UnifiedOutput,
};
use crate::priority::GodObjectIndicators;
use crate::risk::RiskInsight;
use std::fmt::Write as FmtWrite;
use std::io::Write;

// =============================================================================
// PURE FORMATTING FUNCTIONS (the "still" core)
// =============================================================================
// These functions are pure: they take data, return strings, no side effects.
// Easy to test, easy to reason about, composable.
//
// This module is public so the TUI can reuse it for clipboard copy (Spec 001).

pub mod format {
    use super::*;
    use crate::output::unified::{ContextSuggestionOutput, PurityAnalysis, UnifiedLocation};
    use crate::priority::FunctionRole;

    /// Format identification section for a function item
    pub fn identification(location: &UnifiedLocation, category: &str) -> String {
        let mut out = String::new();
        writeln!(out, "#### Identification").unwrap();
        writeln!(
            out,
            "- ID: {}",
            super::generate_item_id(&location.file, location.line)
        )
        .unwrap();
        writeln!(out, "- Type: Function").unwrap();
        writeln!(
            out,
            "- Location: {}:{}",
            location.file,
            location.line.unwrap_or(0)
        )
        .unwrap();
        if let Some(ref func_name) = location.function {
            writeln!(out, "- Function: {}", func_name).unwrap();
        }
        writeln!(out, "- Category: {}", category).unwrap();
        out
    }

    /// Format severity section
    pub fn severity(score: f64, priority: &crate::output::unified::Priority) -> String {
        let mut out = String::new();
        writeln!(out, "#### Severity").unwrap();
        writeln!(out, "- Score: {}", score).unwrap();
        writeln!(out, "- Priority: {:?}", priority).unwrap();
        writeln!(out, "- Tier: {}", super::priority_tier(score)).unwrap();
        out
    }

    /// Format metrics section
    pub fn metrics(
        m: &FunctionMetricsOutput,
        adj: Option<&crate::output::unified::AdjustedComplexity>,
    ) -> String {
        let mut out = String::new();
        writeln!(out, "#### Metrics").unwrap();
        writeln!(out, "- Cyclomatic Complexity: {}", m.cyclomatic_complexity).unwrap();

        // Cognitive complexity with entropy-adjusted notation
        match (m.entropy_adjusted_cognitive, m.cognitive_complexity) {
            (Some(adjusted), raw) if adjusted != raw => {
                writeln!(
                    out,
                    "- Cognitive Complexity: {} → {} (entropy-adjusted)",
                    raw, adjusted
                )
                .unwrap();
            }
            _ => {
                writeln!(out, "- Cognitive Complexity: {}", m.cognitive_complexity).unwrap();
            }
        }

        writeln!(out, "- Nesting Depth: {}", m.nesting_depth).unwrap();
        writeln!(out, "- Lines of Code: {}", m.length).unwrap();

        if let Some(entropy) = m.entropy_score {
            writeln!(out, "- Entropy Score: {:.2}", entropy).unwrap();
        }
        if let Some(repetition) = m.pattern_repetition {
            writeln!(out, "- Pattern Repetition: {:.2}", repetition).unwrap();
        }
        if let Some(similarity) = m.branch_similarity {
            writeln!(out, "- Branch Similarity: {:.2}", similarity).unwrap();
        }
        if let Some(adjusted) = adj {
            writeln!(out, "- Dampening Factor: {:.2}", adjusted.dampening_factor).unwrap();
            writeln!(
                out,
                "- Dampened Cyclomatic: {:.1}",
                adjusted.dampened_cyclomatic
            )
            .unwrap();
        }
        out
    }

    /// Format coverage section (returns None if no coverage data)
    pub fn coverage(m: &FunctionMetricsOutput) -> Option<String> {
        if m.coverage.is_none() && m.transitive_coverage.is_none() {
            return None;
        }
        let mut out = String::new();
        writeln!(out, "#### Coverage").unwrap();
        if let Some(cov) = m.coverage {
            writeln!(out, "- Direct Coverage: {:.0}%", cov * 100.0).unwrap();
        }
        if let Some(trans) = m.transitive_coverage {
            writeln!(out, "- Transitive Coverage: {:.0}%", trans * 100.0).unwrap();
        }
        Some(out)
    }

    /// Format dependencies section
    pub fn dependencies(deps: &Dependencies) -> String {
        let mut out = String::new();
        writeln!(out, "#### Dependencies").unwrap();

        // Spec 267: Show both production and test caller counts
        if deps.production_upstream_count > 0 || deps.test_upstream_count > 0 {
            // Show detailed breakdown when we have the data
            writeln!(
                out,
                "- Upstream Callers: {} ({} production, {} test)",
                deps.upstream_count, deps.production_upstream_count, deps.test_upstream_count
            )
            .unwrap();
        } else {
            // Fallback to simple count for backward compatibility
            writeln!(out, "- Upstream Callers: {}", deps.upstream_count).unwrap();
        }
        writeln!(out, "- Downstream Callees: {}", deps.downstream_count).unwrap();

        // Spec 267: Show production-only blast radius
        if deps.production_blast_radius > 0 {
            let impact = classify_blast_radius(deps.production_blast_radius);
            writeln!(
                out,
                "- Production Blast Radius: {} ({})",
                deps.production_blast_radius, impact
            )
            .unwrap();
        } else if deps.blast_radius > 0 {
            // Fallback to legacy blast radius
            let impact = classify_blast_radius(deps.blast_radius);
            writeln!(out, "- Blast Radius: {} ({})", deps.blast_radius, impact).unwrap();
        }
        if deps.critical_path {
            writeln!(out, "- Critical Path: Yes").unwrap();
        }
        if let Some(ref class) = deps.coupling_classification {
            writeln!(out, "- Coupling Classification: {}", class).unwrap();

            // Spec 269: Add architectural insight for stable-by-design modules
            if let Some(insight) = architectural_insight(class) {
                writeln!(out, "- Architectural Insight: {}", insight).unwrap();
            }
        }
        if let Some(inst) = deps.instability {
            writeln!(out, "- Instability: {:.2} (I=Ce/(Ca+Ce))", inst).unwrap();
        }

        // Spec 267: Show production and test callers separately
        format_caller_list(
            &mut out,
            "Production Callers",
            &deps.upstream_production_callers,
        );
        format_caller_list(&mut out, "Test Callers", &deps.upstream_test_callers);
        // Fallback to legacy callers if new fields are empty
        if deps.upstream_production_callers.is_empty() && deps.upstream_test_callers.is_empty() {
            format_caller_list(&mut out, "Top Callers", &deps.upstream_callers);
        }
        format_caller_list(&mut out, "Top Callees", &deps.downstream_callees);
        out
    }

    fn format_caller_list(out: &mut String, label: &str, items: &[String]) {
        if items.is_empty() {
            return;
        }
        writeln!(out, "- {}:", label).unwrap();
        for item in items.iter().take(5) {
            writeln!(out, "  - {}", item).unwrap();
        }
        if items.len() > 5 {
            writeln!(out, "  - (+{} more)", items.len() - 5).unwrap();
        }
    }

    /// Classify blast radius into impact severity level.
    /// Pure function for consistent classification across production and legacy paths.
    pub(crate) fn classify_blast_radius(radius: usize) -> &'static str {
        match radius {
            r if r >= 20 => "critical",
            r if r >= 10 => "high",
            _ => "moderate",
        }
    }

    /// Map coupling classification to architectural insight.
    /// Returns None if the classification has no specific insight.
    pub(crate) fn architectural_insight(classification: &str) -> Option<&'static str> {
        match classification {
            "Well-Tested Core" | "well_tested_core" => {
                Some("Stable foundation with high test coverage - not actual debt")
            }
            "Stable Foundation" | "stable_foundation" => {
                Some("Intentionally stable module - many callers is by design")
            }
            "Stable Core" | "stable_core" => {
                Some("Stable dependency - high callers indicates good architecture")
            }
            "Unstable High Coupling" | "unstable_high_coupling" => {
                Some("Actual architectural debt - unstable module with many dependents")
            }
            "Architectural Hub" | "architectural_hub" => {
                Some("Central connector - review for potential refactoring opportunities")
            }
            _ => None,
        }
    }

    /// Format purity analysis section (returns None if no purity data)
    pub fn purity(purity: Option<&PurityAnalysis>) -> Option<String> {
        let p = purity?;
        let mut out = String::new();
        writeln!(out, "#### Purity Analysis").unwrap();
        writeln!(out, "- Is Pure: {}", p.is_pure).unwrap();
        if let Some(ref level) = p.purity_level {
            writeln!(out, "- Purity Level: {}", level).unwrap();
        }
        writeln!(out, "- Confidence: {:.2}", p.confidence).unwrap();
        if let Some(ref effects) = &p.side_effects {
            if !effects.is_empty() {
                writeln!(out, "- Side Effects:").unwrap();
                for effect in effects {
                    writeln!(out, "  - {}", effect).unwrap();
                }
            }
        }
        Some(out)
    }

    /// Format pattern analysis section (returns None if no pattern data)
    pub fn pattern_analysis(
        pattern_type: Option<&String>,
        confidence: Option<f64>,
    ) -> Option<String> {
        if pattern_type.is_none() && confidence.is_none() {
            return None;
        }
        let mut out = String::new();
        writeln!(out, "#### Pattern Analysis").unwrap();
        if let Some(pt) = pattern_type {
            writeln!(out, "- Pattern Type: {}", pt).unwrap();
        }
        if let Some(conf) = confidence {
            writeln!(out, "- Pattern Confidence: {:.2}", conf).unwrap();
        }
        Some(out)
    }

    /// Format scoring breakdown section (returns None if no scoring data)
    pub fn scoring(
        scoring: Option<&FunctionScoringDetails>,
        role: &FunctionRole,
    ) -> Option<String> {
        let s = scoring?;
        let mut out = String::new();
        writeln!(out, "#### Scoring Breakdown").unwrap();
        writeln!(out, "- Base Score: {:.2}", s.base_score).unwrap();
        writeln!(
            out,
            "- Complexity Factor: {:.2} (weight: 0.4)",
            s.complexity_score
        )
        .unwrap();
        writeln!(
            out,
            "- Coverage Factor: {:.2} (weight: 0.3)",
            s.coverage_score
        )
        .unwrap();
        writeln!(
            out,
            "- Dependency Factor: {:.2} (weight: 0.2)",
            s.dependency_score
        )
        .unwrap();
        writeln!(
            out,
            "- Role Multiplier: {:.2} ({:?})",
            s.role_multiplier, role
        )
        .unwrap();

        // Additional multipliers only if they differ from 1.0
        format_optional_multiplier(&mut out, "Structural Multiplier", s.structural_multiplier);
        format_optional_multiplier(&mut out, "Context Multiplier", s.context_multiplier);
        format_optional_multiplier(
            &mut out,
            "Contextual Risk Multiplier",
            s.contextual_risk_multiplier,
        );

        if let Some(pf) = s.purity_factor {
            writeln!(out, "- Purity Factor: {:.2}", pf).unwrap();
        }
        format_optional_multiplier(&mut out, "Refactorability Factor", s.refactorability_factor);
        format_optional_multiplier(&mut out, "Pattern Factor", s.pattern_factor);

        // Pre-normalization score if clamping occurred
        if let Some(pre) = s.pre_normalization_score {
            if (pre - s.final_score).abs() > 0.1 {
                writeln!(
                    out,
                    "- Pre-normalization Score: {:.2} (clamped to {:.2})",
                    pre, s.final_score
                )
                .unwrap();
            }
        }
        writeln!(out, "- Final Score: {:.2}", s.final_score).unwrap();
        Some(out)
    }

    fn format_optional_multiplier(out: &mut String, label: &str, value: Option<f64>) {
        if let Some(v) = value {
            if (v - 1.0).abs() > 0.01 {
                writeln!(out, "- {}: {:.2}", label, v).unwrap();
            }
        }
    }

    /// Format context section (returns None if no context data)
    pub fn context(ctx: Option<&ContextSuggestionOutput>) -> Option<String> {
        let c = ctx?;
        let mut out = String::new();
        writeln!(out, "#### Context to Read").unwrap();
        writeln!(out, "- Total Lines: {}", c.total_lines).unwrap();
        writeln!(
            out,
            "- Completeness Confidence: {:.2}",
            c.completeness_confidence
        )
        .unwrap();
        writeln!(out, "- Primary:").unwrap();
        writeln!(
            out,
            "  - {}:{}-{} ({})",
            c.primary.file,
            c.primary.start_line,
            c.primary.end_line,
            c.primary.symbol.as_deref().unwrap_or("Unknown")
        )
        .unwrap();
        if !c.related.is_empty() {
            writeln!(out, "- Related:").unwrap();
            for rel in &c.related {
                writeln!(
                    out,
                    "  - {}:{}-{} ({})",
                    rel.range.file, rel.range.start_line, rel.range.end_line, rel.relationship
                )
                .unwrap();
            }
        }
        Some(out)
    }

    /// Format git history section (returns None if no git data)
    pub fn git_history(git: Option<&GitHistoryOutput>) -> Option<String> {
        let g = git?;
        let mut out = String::new();
        writeln!(out, "#### Git History").unwrap();
        // Show commits with frequency for clarity
        let commit_label = if g.total_commits == 1 {
            "commit"
        } else {
            "commits"
        };
        writeln!(
            out,
            "- Change Frequency: {} {} ({:.2}/month)",
            g.total_commits, commit_label, g.change_frequency
        )
        .unwrap();
        // Show fix rate as "N fixes / M changes" for clarity
        let changes = g.total_commits.saturating_sub(1);
        if changes == 0 {
            writeln!(out, "- Bug Density: 0%").unwrap();
        } else {
            writeln!(
                out,
                "- Bug Density: {:.0}% ({} fix{} / {} change{})",
                g.bug_density * 100.0,
                g.bug_fix_count,
                if g.bug_fix_count == 1 { "" } else { "es" },
                changes,
                if changes == 1 { "" } else { "s" }
            )
            .unwrap();
        }
        writeln!(out, "- Age: {} days", g.age_days).unwrap();
        writeln!(out, "- Authors: {}", g.author_count).unwrap();
        writeln!(out, "- Stability: {}", g.stability).unwrap();
        Some(out)
    }

    // =========================================================================
    // FILE ITEM FORMATTERS
    // =========================================================================

    /// Format identification section for a file item
    pub fn file_identification(file: &str, category: &str) -> String {
        let mut out = String::new();
        writeln!(out, "#### Identification").unwrap();
        writeln!(out, "- ID: {}", super::generate_item_id(file, None)).unwrap();
        writeln!(out, "- Type: File").unwrap();
        writeln!(out, "- Location: {}", file).unwrap();
        writeln!(out, "- Category: {}", category).unwrap();
        out
    }

    /// Format file metrics section
    pub fn file_metrics(m: &crate::output::unified::FileMetricsOutput) -> String {
        let mut out = String::new();
        writeln!(out, "#### Metrics").unwrap();
        writeln!(out, "- Total Cyclomatic Complexity: {}", m.total_complexity).unwrap();

        // Show distribution metrics if available (Spec 268)
        if let Some(ref dist) = m.distribution {
            writeln!(
                out,
                "- Complexity Distribution: {} (max: {}, avg: {:.1}, median: {})",
                dist.distribution_type,
                dist.max_function_complexity,
                dist.avg_function_complexity,
                dist.median_complexity
            )
            .unwrap();
            writeln!(
                out,
                "- Functions: {} total, {} exceeding threshold",
                dist.function_count, dist.exceeding_threshold
            )
            .unwrap();
            writeln!(out, "- Production LOC: {}", dist.production_loc).unwrap();
            if dist.test_loc > 0 {
                writeln!(out, "- Test LOC: {}", dist.test_loc).unwrap();
            }
            writeln!(
                out,
                "- Distribution Classification: {}",
                dist.classification_explanation
            )
            .unwrap();
        } else {
            // Fallback to basic metrics when distribution not available
            writeln!(out, "- Lines: {}", m.lines).unwrap();
            writeln!(out, "- Functions: {}", m.functions).unwrap();
            writeln!(out, "- Average Complexity: {:.1}", m.avg_complexity).unwrap();
            writeln!(out, "- Max Complexity: {}", m.max_complexity).unwrap();
        }

        writeln!(out, "- Classes: {}", m.classes).unwrap();
        writeln!(out, "- Coverage: {:.0}%", m.coverage * 100.0).unwrap();
        writeln!(out, "- Uncovered Lines: {}", m.uncovered_lines).unwrap();
        out
    }

    /// Format god object analysis section (returns None if no god object data)
    pub fn god_object(god: Option<&GodObjectIndicators>) -> Option<String> {
        let g = god?;
        let mut out = String::new();
        writeln!(out, "#### God Object Analysis").unwrap();
        writeln!(out, "- Is God Object: {}", g.is_god_object).unwrap();
        writeln!(out, "- Method Count: {}", g.methods_count).unwrap();
        writeln!(out, "- Field Count: {}", g.fields_count).unwrap();
        writeln!(out, "- Responsibility Count: {}", g.responsibilities).unwrap();
        writeln!(out, "- God Object Score: {:.2}", g.god_object_score).unwrap();
        Some(out)
    }

    /// Format cohesion analysis section (returns None if no cohesion data)
    pub fn cohesion(cohesion: Option<&crate::output::unified::CohesionOutput>) -> Option<String> {
        let c = cohesion?;
        let mut out = String::new();
        writeln!(out, "#### Cohesion Analysis").unwrap();
        writeln!(out, "- Cohesion Score: {:.2}", c.score).unwrap();
        writeln!(out, "- Classification: {:?}", c.classification).unwrap();
        writeln!(out, "- Functions Analyzed: {}", c.functions_analyzed).unwrap();
        writeln!(out, "- Internal Calls: {}", c.internal_calls).unwrap();
        writeln!(out, "- External Calls: {}", c.external_calls).unwrap();
        Some(out)
    }

    /// Format file scoring breakdown section (returns None if no scoring data)
    pub fn file_scoring(scoring: Option<&FileScoringDetails>) -> Option<String> {
        let s = scoring?;
        let mut out = String::new();
        writeln!(out, "#### Scoring Breakdown").unwrap();
        writeln!(out, "- File Size Score: {}", s.file_size_score).unwrap();
        writeln!(out, "- Function Count Score: {}", s.function_count_score).unwrap();
        writeln!(out, "- Complexity Score: {}", s.complexity_score).unwrap();
        writeln!(out, "- Coverage Penalty: {}", s.coverage_penalty).unwrap();
        Some(out)
    }
}

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

    /// Write a function debt item using composed pure formatters.
    ///
    /// This is the "imperative shell" - thin I/O that composes pure formatters.
    fn write_function_item(&mut self, item: &FunctionDebtItemOutput) -> anyhow::Result<()> {
        // Compose all sections from pure formatters
        write!(
            self.writer,
            "{}",
            format::identification(&item.location, &item.category)
        )?;
        writeln!(self.writer)?;

        write!(
            self.writer,
            "{}",
            format::severity(item.score, &item.priority)
        )?;
        writeln!(self.writer)?;

        write!(
            self.writer,
            "{}",
            format::metrics(&item.metrics, item.adjusted_complexity.as_ref())
        )?;
        writeln!(self.writer)?;

        if let Some(cov) = format::coverage(&item.metrics) {
            write!(self.writer, "{}", cov)?;
            writeln!(self.writer)?;
        }

        write!(self.writer, "{}", format::dependencies(&item.dependencies))?;
        writeln!(self.writer)?;

        if let Some(pur) = format::purity(item.purity_analysis.as_ref()) {
            write!(self.writer, "{}", pur)?;
            writeln!(self.writer)?;
        }

        if let Some(pat) =
            format::pattern_analysis(item.pattern_type.as_ref(), item.pattern_confidence)
        {
            write!(self.writer, "{}", pat)?;
            writeln!(self.writer)?;
        }

        if let Some(scr) = format::scoring(item.scoring_details.as_ref(), &item.function_role) {
            write!(self.writer, "{}", scr)?;
            writeln!(self.writer)?;
        }

        if let Some(ctx) = format::context(item.context.as_ref()) {
            write!(self.writer, "{}", ctx)?;
            writeln!(self.writer)?;
        }

        if let Some(git) = format::git_history(item.git_history.as_ref()) {
            write!(self.writer, "{}", git)?;
            writeln!(self.writer)?;
        }

        writeln!(self.writer, "---")?;
        writeln!(self.writer)?;
        Ok(())
    }

    /// Write a file debt item using composed pure formatters.
    ///
    /// This is the "imperative shell" - thin I/O that composes pure formatters.
    fn write_file_item(&mut self, item: &FileDebtItemOutput) -> anyhow::Result<()> {
        // Compose all sections from pure formatters
        write!(
            self.writer,
            "{}",
            format::file_identification(&item.location.file, &item.category)
        )?;
        writeln!(self.writer)?;

        write!(
            self.writer,
            "{}",
            format::severity(item.score, &item.priority)
        )?;
        writeln!(self.writer)?;

        write!(self.writer, "{}", format::file_metrics(&item.metrics))?;
        writeln!(self.writer)?;

        if let Some(god) = format::god_object(item.god_object_indicators.as_ref()) {
            write!(self.writer, "{}", god)?;
            writeln!(self.writer)?;
        }

        if let Some(coh) = format::cohesion(item.cohesion.as_ref()) {
            write!(self.writer, "{}", coh)?;
            writeln!(self.writer)?;
        }

        if let Some(scr) = format::file_scoring(item.scoring_details.as_ref()) {
            write!(self.writer, "{}", scr)?;
            writeln!(self.writer)?;
        }

        writeln!(self.writer, "---")?;
        writeln!(self.writer)?;
        Ok(())
    }
}

/// Generate a stable ID for an item based on file and line
pub fn generate_item_id(file: &str, line: Option<usize>) -> String {
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
pub fn priority_tier(score: f64) -> &'static str {
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

    #[test]
    fn test_llm_markdown_outputs_new_dependency_fields() {
        use crate::output::unified::{
            Dependencies, FunctionDebtItemOutput, FunctionImpactOutput, FunctionMetricsOutput,
            Priority, UnifiedLocation,
        };
        use crate::priority::{DebtType, FunctionRole};

        let mut buffer = Vec::new();
        let mut writer = LlmMarkdownWriter::new(&mut buffer);

        // Create a function item with enhanced dependency data
        let item = FunctionDebtItemOutput {
            score: 85.5,
            category: "Complexity".to_string(),
            priority: Priority::High,
            location: UnifiedLocation {
                file: "src/test.rs".to_string(),
                line: Some(100),
                function: Some("complex_fn".to_string()),
                file_context_label: None,
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: 25,
                cognitive_complexity: 30,
                length: 150,
                nesting_depth: 5,
                coverage: Some(0.6),
                uncovered_lines: None,
                entropy_score: Some(0.7),
                pattern_repetition: Some(0.6),
                branch_similarity: Some(0.4),
                entropy_adjusted_cognitive: Some(24),
                transitive_coverage: Some(0.75),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 25,
                cognitive: 30,
            },
            function_role: FunctionRole::Unknown,
            purity_analysis: None,
            dependencies: Dependencies {
                upstream_count: 8,
                downstream_count: 12,
                upstream_callers: vec!["caller1".to_string(), "caller2".to_string()],
                downstream_callees: vec!["callee1".to_string(), "callee2".to_string()],
                blast_radius: 20,
                critical_path: true,
                coupling_classification: Some("Hub".to_string()),
                instability: Some(0.6),
                // Spec 267: Production/test caller separation
                upstream_production_callers: vec!["caller1".to_string()],
                upstream_test_callers: vec!["caller2".to_string()],
                production_upstream_count: 1,
                test_upstream_count: 1,
                production_blast_radius: 13,
            },
            impact: FunctionImpactOutput {
                coverage_improvement: 0.1,
                complexity_reduction: 0.2,
                risk_reduction: 0.15,
            },
            scoring_details: None,
            adjusted_complexity: None,
            complexity_pattern: None,
            pattern_type: None,
            pattern_confidence: None,
            pattern_details: None,
            context: None,
            git_history: None,
        };

        let result = writer.write_function_item(&item);
        assert!(result.is_ok());

        let markdown = String::from_utf8(buffer).unwrap();

        // Check entropy-adjusted cognitive complexity notation
        assert!(
            markdown.contains("30 → 24 (entropy-adjusted)"),
            "Should show entropy-adjusted cognitive: {}",
            markdown
        );

        // Check new entropy fields: pattern_repetition and branch_similarity
        assert!(
            markdown.contains("Pattern Repetition: 0.60"),
            "Should show pattern repetition: {}",
            markdown
        );
        assert!(
            markdown.contains("Branch Similarity: 0.40"),
            "Should show branch similarity: {}",
            markdown
        );

        // Check transitive coverage
        assert!(
            markdown.contains("Transitive Coverage: 75%"),
            "Should show transitive coverage"
        );

        // Check new dependency fields (Spec 267: Production Blast Radius)
        assert!(
            markdown.contains("Production Blast Radius: 13 (high)"),
            "Should show production blast radius: {}",
            markdown
        );
        assert!(
            markdown.contains("Critical Path: Yes"),
            "Should show critical path"
        );
        assert!(
            markdown.contains("Coupling Classification: Hub"),
            "Should show coupling classification"
        );
        assert!(
            markdown.contains("Instability: 0.60 (I=Ce/(Ca+Ce))"),
            "Should show instability metric"
        );
    }

    #[test]
    fn test_llm_markdown_outputs_enhanced_scoring() {
        use crate::output::unified::{
            Dependencies, FunctionDebtItemOutput, FunctionImpactOutput, FunctionMetricsOutput,
            FunctionScoringDetails, Priority, UnifiedLocation,
        };
        use crate::priority::{DebtType, FunctionRole};

        let mut buffer = Vec::new();
        let mut writer = LlmMarkdownWriter::new(&mut buffer);

        let item = FunctionDebtItemOutput {
            score: 100.0,
            category: "Complexity".to_string(),
            priority: Priority::Critical,
            location: UnifiedLocation {
                file: "src/test.rs".to_string(),
                line: Some(50),
                function: Some("test_fn".to_string()),
                file_context_label: None,
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: 20,
                cognitive_complexity: 25,
                length: 100,
                nesting_depth: 4,
                coverage: Some(0.5),
                ..Default::default()
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 20,
                cognitive: 25,
            },
            function_role: FunctionRole::Unknown,
            purity_analysis: None,
            dependencies: Dependencies::default(),
            impact: FunctionImpactOutput {
                coverage_improvement: 0.1,
                complexity_reduction: 0.2,
                risk_reduction: 0.15,
            },
            scoring_details: Some(FunctionScoringDetails {
                coverage_score: 5.0,
                complexity_score: 8.0,
                dependency_score: 3.0,
                base_score: 16.0,
                entropy_dampening: Some(0.8),
                role_multiplier: 1.2,
                final_score: 100.0,
                purity_factor: Some(0.9),
                refactorability_factor: Some(1.1),
                pattern_factor: Some(0.85),
                structural_multiplier: Some(1.3),
                context_multiplier: Some(0.9),
                contextual_risk_multiplier: Some(1.15),
                pre_normalization_score: Some(150.0),
            }),
            adjusted_complexity: None,
            complexity_pattern: None,
            pattern_type: None,
            pattern_confidence: None,
            pattern_details: None,
            context: None,
            git_history: None,
        };

        let result = writer.write_function_item(&item);
        assert!(result.is_ok());

        let markdown = String::from_utf8(buffer).unwrap();

        // Check new scoring multipliers are present
        assert!(
            markdown.contains("Structural Multiplier: 1.30"),
            "Should show structural multiplier: {}",
            markdown
        );
        assert!(
            markdown.contains("Context Multiplier: 0.90"),
            "Should show context multiplier"
        );
        assert!(
            markdown.contains("Contextual Risk Multiplier: 1.15"),
            "Should show contextual risk multiplier"
        );
        assert!(
            markdown.contains("Refactorability Factor: 1.10"),
            "Should show refactorability factor"
        );
        assert!(
            markdown.contains("Pre-normalization Score: 150.00 (clamped to 100.00)"),
            "Should show pre-normalization score when clamped"
        );
        assert!(
            markdown.contains("Final Score: 100.00"),
            "Should show final score"
        );
    }

    #[test]
    fn test_llm_markdown_outputs_git_history() {
        use crate::output::unified::{
            Dependencies, FunctionDebtItemOutput, FunctionImpactOutput, FunctionMetricsOutput,
            GitHistoryOutput, Priority, UnifiedLocation,
        };
        use crate::priority::{DebtType, FunctionRole};

        let mut buffer = Vec::new();
        let mut writer = LlmMarkdownWriter::new(&mut buffer);

        let item = FunctionDebtItemOutput {
            score: 75.0,
            category: "Complexity".to_string(),
            priority: Priority::High,
            location: UnifiedLocation {
                file: "src/test.rs".to_string(),
                line: Some(50),
                function: Some("test_fn".to_string()),
                file_context_label: None,
            },
            metrics: FunctionMetricsOutput {
                cyclomatic_complexity: 15,
                cognitive_complexity: 20,
                length: 80,
                nesting_depth: 3,
                coverage: Some(0.4),
                ..Default::default()
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 20,
            },
            function_role: FunctionRole::Unknown,
            purity_analysis: None,
            dependencies: Dependencies::default(),
            impact: FunctionImpactOutput {
                coverage_improvement: 0.1,
                complexity_reduction: 0.2,
                risk_reduction: 0.15,
            },
            scoring_details: None,
            adjusted_complexity: None,
            complexity_pattern: None,
            pattern_type: None,
            pattern_confidence: None,
            pattern_details: None,
            context: None,
            git_history: Some(GitHistoryOutput {
                change_frequency: 3.5,
                bug_density: 0.25,
                age_days: 180,
                author_count: 4,
                total_commits: 21,
                bug_fix_count: 5,
                stability: "Frequently Changed".to_string(),
            }),
        };

        let result = writer.write_function_item(&item);
        assert!(result.is_ok());

        let markdown = String::from_utf8(buffer).unwrap();

        // Check git history section is present
        assert!(
            markdown.contains("#### Git History"),
            "Should have Git History header: {}",
            markdown
        );
        assert!(
            markdown.contains("Change Frequency: 21 commits (3.50/month)"),
            "Should show change frequency with commits: {}",
            markdown
        );
        assert!(
            markdown.contains("Bug Density: 25% (5 fixes / 20 changes)"),
            "Should show bug density with fix counts: {}",
            markdown
        );
        assert!(
            markdown.contains("Age: 180 days"),
            "Should show age: {}",
            markdown
        );
        assert!(
            markdown.contains("Authors: 4"),
            "Should show author count: {}",
            markdown
        );
        assert!(
            markdown.contains("Stability: Frequently Changed"),
            "Should show stability: {}",
            markdown
        );
    }

    #[test]
    fn test_classify_blast_radius() {
        // Critical threshold (>= 20)
        assert_eq!(format::classify_blast_radius(20_usize), "critical");
        assert_eq!(format::classify_blast_radius(25_usize), "critical");
        assert_eq!(format::classify_blast_radius(100_usize), "critical");

        // High threshold (>= 10, < 20)
        assert_eq!(format::classify_blast_radius(10_usize), "high");
        assert_eq!(format::classify_blast_radius(15_usize), "high");
        assert_eq!(format::classify_blast_radius(19_usize), "high");

        // Moderate (< 10)
        assert_eq!(format::classify_blast_radius(0_usize), "moderate");
        assert_eq!(format::classify_blast_radius(5_usize), "moderate");
        assert_eq!(format::classify_blast_radius(9_usize), "moderate");
    }

    #[test]
    fn test_architectural_insight() {
        // Well-Tested Core variants
        assert_eq!(
            format::architectural_insight("Well-Tested Core"),
            Some("Stable foundation with high test coverage - not actual debt")
        );
        assert_eq!(
            format::architectural_insight("well_tested_core"),
            Some("Stable foundation with high test coverage - not actual debt")
        );

        // Stable Foundation variants
        assert_eq!(
            format::architectural_insight("Stable Foundation"),
            Some("Intentionally stable module - many callers is by design")
        );
        assert_eq!(
            format::architectural_insight("stable_foundation"),
            Some("Intentionally stable module - many callers is by design")
        );

        // Stable Core variants
        assert_eq!(
            format::architectural_insight("Stable Core"),
            Some("Stable dependency - high callers indicates good architecture")
        );
        assert_eq!(
            format::architectural_insight("stable_core"),
            Some("Stable dependency - high callers indicates good architecture")
        );

        // Unstable High Coupling variants
        assert_eq!(
            format::architectural_insight("Unstable High Coupling"),
            Some("Actual architectural debt - unstable module with many dependents")
        );
        assert_eq!(
            format::architectural_insight("unstable_high_coupling"),
            Some("Actual architectural debt - unstable module with many dependents")
        );

        // Architectural Hub variants
        assert_eq!(
            format::architectural_insight("Architectural Hub"),
            Some("Central connector - review for potential refactoring opportunities")
        );
        assert_eq!(
            format::architectural_insight("architectural_hub"),
            Some("Central connector - review for potential refactoring opportunities")
        );

        // Unknown classifications
        assert_eq!(format::architectural_insight("Unknown"), None);
        assert_eq!(format::architectural_insight("SomeOtherClass"), None);
        assert_eq!(format::architectural_insight(""), None);
    }
}
