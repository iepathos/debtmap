---
number: 109
title: Compare Command for Workflow Integration
category: feature
priority: high
status: draft
dependencies: [108]
created: 2025-10-01
---

# Specification 109: Compare Command for Workflow Integration

**Category**: feature
**Priority**: high
**Status**: draft
**Dependencies**: [#108 Location-Based Filtering]

## Context

Automated workflows using debtmap face a critical challenge: **validation commands must process 40MB+ of JSON** (20MB before + 20MB after) to determine if a single debt item improved and if any regressions occurred.

**Current State**:
- Workflows run full analysis twice: before.json (20MB, ~1,300 items) and after.json (20MB, ~1,300 items)
- Claude validation command receives both files
- Validation must:
  1. Parse 40MB of JSON
  2. Extract target item from plan file
  3. Search 1,300 items to find target (before)
  4. Search 1,300 items to find target (after)
  5. Compare all 1,300 items to detect new critical debt
  6. Calculate project-wide totals
  7. Generate comparison result

**Problem**:
- **Context window waste**: 40MB JSON = ~10M tokens, but only need ~3K tokens of actual comparison data
- **Slow parsing**: Claude spends time parsing irrelevant data
- **Wrong domain ownership**: Comparison logic belongs in debtmap (domain expert), not in Claude prompts
- **Error-prone**: Complex jq/parsing logic in prompts is fragile

**Real-World Impact**:
From failed workflow analysis (session-4665158d):
- 5 recovery attempts × validation = 5 × (parse 40MB) = wasted work
- Each validation parses 1,293 items to find 1 target
- Regression detection requires comparing 1,293 items twice
- Total: ~200MB of JSON parsed across workflow run

## Objective

Add a `debtmap compare` command that:
1. Takes two analysis JSON files (before/after)
2. Identifies target item from implementation plan
3. Performs all comparison logic natively in Rust
4. Outputs compact comparison result (<10KB)
5. Enables instant Claude validation with minimal context

This shifts comparison logic from Claude (wrong place) to debtmap (right place) and reduces validation input from 40MB to <10KB (99.975% reduction).

## Requirements

### Functional Requirements

1. **CLI Command: `debtmap compare`**
   - Accepts two analysis JSON files: `--before` and `--after`
   - Accepts implementation plan: `--plan` (to identify target location)
   - Optional: `--target-location` (explicit target, no plan needed)
   - Outputs comparison JSON: `--output` or stdout
   - Optional: `--format` (json, markdown, terminal)

2. **Target Item Comparison**
   - Extract target location from plan file (`**Location**: file:function:line`)
   - Find target in before.json items array
   - Find target in after.json items array
   - Calculate improvements:
     - Score reduction percentage
     - Complexity reduction percentage
     - Coverage improvement percentage
   - Handle case where target item is resolved (not in after)

3. **Regression Detection**
   - Identify all critical items (score >= 60) in before and after
   - Calculate set difference: `after_critical - before_critical`
   - List new critical items (regressions)
   - Calculate regression penalty

4. **Project Health Comparison**
   - Compare `total_debt_score` before/after
   - Compare total item counts
   - Compare critical item counts
   - Calculate percentage changes

5. **Output Format**
   - Structured JSON with comparison results
   - Include target item changes
   - Include project health metrics
   - Include regressions array
   - Include improvements array
   - Compact format (<10KB for typical cases)

### Non-Functional Requirements

1. **Performance**
   - Load and parse 40MB JSON in <2 seconds
   - Comparison logic: <100ms
   - Total execution time: <3 seconds

2. **Memory Efficiency**
   - Stream parsing where possible
   - Don't load all items into memory
   - Release before.json after comparison

3. **Maintainability**
   - Pure functional comparison logic
   - Well-tested with comprehensive test suite
   - Clear error messages for invalid inputs
   - Extensible for future comparison types

## Acceptance Criteria

- [ ] `debtmap compare` command exists in CLI
- [ ] Accepts `--before`, `--after`, `--plan` parameters
- [ ] Parses target location from plan markdown
- [ ] Finds target item in both before/after
- [ ] Calculates target improvement metrics correctly
- [ ] Detects new critical items (regressions)
- [ ] Calculates project health changes
- [ ] Outputs compact JSON (<10KB for typical cases)
- [ ] Works with markdown and terminal output formats
- [ ] Integration tests for all scenarios
- [ ] Documentation updated (README, --help)
- [ ] Workflow updated to use `debtmap compare`
- [ ] Validation command simplified (reads comparison.json instead of 40MB)

## Technical Details

### CLI Definition

Add to `src/cli.rs`:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands ...

    /// Compare two analysis results and generate diff
    Compare {
        /// Path to "before" analysis JSON
        #[arg(long, value_name = "FILE")]
        before: PathBuf,

        /// Path to "after" analysis JSON
        #[arg(long, value_name = "FILE")]
        after: PathBuf,

        /// Path to implementation plan (to extract target location)
        #[arg(long, value_name = "FILE")]
        plan: Option<PathBuf>,

        /// Target location (alternative to --plan)
        /// Format: file:function:line
        #[arg(long, value_name = "LOCATION", conflicts_with = "plan")]
        target_location: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}
```

### Comparison Result Structure

Create `src/comparison/mod.rs`:

```rust
use serde::{Deserialize, Serialize};
use crate::core::DebtItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    /// Metadata about the comparison
    pub metadata: ComparisonMetadata,

    /// Target item comparison (if target specified)
    pub target_item: Option<TargetComparison>,

    /// Project-wide health comparison
    pub project_health: ProjectHealthComparison,

    /// New critical debt items (regressions)
    pub regressions: Vec<RegressionItem>,

    /// Resolved debt items (improvements)
    pub improvements: Vec<ImprovementItem>,

    /// Summary statistics
    pub summary: ComparisonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonMetadata {
    pub comparison_date: String,
    pub before_file: String,
    pub after_file: String,
    pub target_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetComparison {
    pub location: String,
    pub before: TargetMetrics,
    pub after: Option<TargetMetrics>,
    pub improvements: ImprovementMetrics,
    pub status: TargetStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetMetrics {
    pub score: f64,
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub coverage: f64,
    pub function_length: usize,
    pub nesting_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementMetrics {
    pub score_reduction_pct: f64,
    pub complexity_reduction_pct: f64,
    pub coverage_improvement_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetStatus {
    /// Target item completely resolved (not in after)
    Resolved,
    /// Target item improved
    Improved,
    /// Target item unchanged
    Unchanged,
    /// Target item regressed (got worse)
    Regressed,
    /// Target item not found in before
    NotFoundBefore,
    /// Target item not found in either
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectHealthComparison {
    pub before: ProjectMetrics,
    pub after: ProjectMetrics,
    pub changes: ProjectChanges,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetrics {
    pub total_debt_score: f64,
    pub total_items: usize,
    pub critical_items: usize,  // score >= 60
    pub high_priority_items: usize,  // score >= 40
    pub average_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectChanges {
    pub debt_score_change: f64,
    pub debt_score_change_pct: f64,
    pub items_change: i32,
    pub critical_items_change: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionItem {
    pub location: String,
    pub score: f64,
    pub debt_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementItem {
    pub location: String,
    pub before_score: f64,
    pub after_score: Option<f64>,  // None if resolved
    pub improvement_type: ImprovementType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImprovementType {
    Resolved,
    ScoreReduced,
    ComplexityReduced,
    CoverageImproved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub target_improved: bool,
    pub new_critical_count: usize,
    pub resolved_count: usize,
    pub overall_debt_trend: DebtTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebtTrend {
    Improving,  // debt decreased
    Stable,     // debt unchanged
    Regressing, // debt increased
}
```

### Comparison Logic

Create `src/comparison/comparator.rs`:

```rust
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use crate::core::{AnalysisResults, DebtItem};
use crate::comparison::*;

pub struct Comparator {
    before: AnalysisResults,
    after: AnalysisResults,
    target_location: Option<String>,
}

impl Comparator {
    pub fn new(
        before: AnalysisResults,
        after: AnalysisResults,
        target_location: Option<String>,
    ) -> Self {
        Self {
            before,
            after,
            target_location,
        }
    }

    /// Perform full comparison
    pub fn compare(&self) -> Result<ComparisonResult> {
        let target_item = self.target_location.as_ref()
            .map(|loc| self.compare_target_item(loc))
            .transpose()?;

        let project_health = self.compare_project_health();
        let regressions = self.find_regressions();
        let improvements = self.find_improvements();
        let summary = self.generate_summary(&target_item, &regressions, &improvements);

        Ok(ComparisonResult {
            metadata: self.build_metadata(),
            target_item,
            project_health,
            regressions,
            improvements,
            summary,
        })
    }

    /// Compare specific target item
    fn compare_target_item(&self, location: &str) -> Result<TargetComparison> {
        let before_item = self.find_item_by_location(&self.before, location);
        let after_item = self.find_item_by_location(&self.after, location);

        let status = match (&before_item, &after_item) {
            (None, _) => TargetStatus::NotFoundBefore,
            (Some(_), None) => TargetStatus::Resolved,
            (Some(before), Some(after)) => {
                self.classify_target_status(before, after)
            }
        };

        let (before_metrics, after_metrics, improvements) = match (&before_item, &after_item) {
            (Some(before), Some(after)) => {
                let before_m = self.extract_metrics(before);
                let after_m = self.extract_metrics(after);
                let improvements = self.calculate_improvements(&before_m, &after_m);
                (before_m, Some(after_m), improvements)
            },
            (Some(before), None) => {
                let before_m = self.extract_metrics(before);
                let improvements = ImprovementMetrics {
                    score_reduction_pct: 100.0,
                    complexity_reduction_pct: 100.0,
                    coverage_improvement_pct: 100.0,
                };
                (before_m, None, improvements)
            },
            (None, _) => {
                return Err(anyhow::anyhow!("Target item not found in before analysis"));
            }
        };

        Ok(TargetComparison {
            location: location.to_string(),
            before: before_metrics,
            after: after_metrics,
            improvements,
            status,
        })
    }

    /// Find regressions (new critical items)
    fn find_regressions(&self) -> Vec<RegressionItem> {
        let before_critical: HashSet<String> = self.before.items.iter()
            .filter(|item| self.get_score(item) >= 60.0)
            .map(|item| self.item_key(item))
            .collect();

        let after_critical: Vec<&DebtItem> = self.after.items.iter()
            .filter(|item| self.get_score(item) >= 60.0)
            .collect();

        after_critical.iter()
            .filter(|item| !before_critical.contains(&self.item_key(item)))
            .map(|item| self.build_regression_item(item))
            .collect()
    }

    /// Find improvements (resolved or significantly improved items)
    fn find_improvements(&self) -> Vec<ImprovementItem> {
        let before_items: HashMap<String, &DebtItem> = self.before.items.iter()
            .map(|item| (self.item_key(item), item))
            .collect();

        let after_keys: HashSet<String> = self.after.items.iter()
            .map(|item| self.item_key(item))
            .collect();

        let mut improvements = Vec::new();

        // Find resolved items
        for (key, before_item) in before_items.iter() {
            if !after_keys.contains(key) && self.get_score(before_item) >= 40.0 {
                improvements.push(ImprovementItem {
                    location: self.format_location(before_item),
                    before_score: self.get_score(before_item),
                    after_score: None,
                    improvement_type: ImprovementType::Resolved,
                });
            }
        }

        // Find significantly improved items (>30% reduction)
        for before_item in before_items.values() {
            let key = self.item_key(before_item);
            if let Some(after_item) = self.after.items.iter()
                .find(|item| self.item_key(item) == key) {

                let before_score = self.get_score(before_item);
                let after_score = self.get_score(after_item);

                if before_score > 0.0 {
                    let reduction = (before_score - after_score) / before_score * 100.0;
                    if reduction >= 30.0 {
                        improvements.push(ImprovementItem {
                            location: self.format_location(before_item),
                            before_score,
                            after_score: Some(after_score),
                            improvement_type: ImprovementType::ScoreReduced,
                        });
                    }
                }
            }
        }

        improvements
    }

    /// Compare project-wide health metrics
    fn compare_project_health(&self) -> ProjectHealthComparison {
        let before_metrics = self.extract_project_metrics(&self.before);
        let after_metrics = self.extract_project_metrics(&self.after);
        let changes = self.calculate_project_changes(&before_metrics, &after_metrics);

        ProjectHealthComparison {
            before: before_metrics,
            after: after_metrics,
            changes,
        }
    }

    // Helper methods...

    fn find_item_by_location(&self, results: &AnalysisResults, location: &str) -> Option<&DebtItem> {
        let parts: Vec<&str> = location.split(':').collect();
        if parts.len() != 3 {
            return None;
        }

        let (file, function, line_str) = (parts[0], parts[1], parts[2]);
        let line: usize = line_str.parse().ok()?;

        results.items.iter().find(|item| {
            item.location.file == file &&
            item.location.function == function &&
            item.location.line == line
        })
    }

    fn item_key(&self, item: &DebtItem) -> String {
        format!("{}:{}:{}", item.location.file, item.location.function, item.location.line)
    }

    fn get_score(&self, item: &DebtItem) -> f64 {
        item.unified_score.final_score
    }

    fn format_location(&self, item: &DebtItem) -> String {
        self.item_key(item)
    }

    fn extract_metrics(&self, item: &DebtItem) -> TargetMetrics {
        TargetMetrics {
            score: self.get_score(item),
            cyclomatic_complexity: item.cyclomatic_complexity,
            cognitive_complexity: item.cognitive_complexity,
            coverage: item.transitive_coverage.unwrap_or(0.0),
            function_length: item.function_length,
            nesting_depth: item.nesting_depth,
        }
    }

    fn calculate_improvements(&self, before: &TargetMetrics, after: &TargetMetrics) -> ImprovementMetrics {
        let score_reduction_pct = if before.score > 0.0 {
            ((before.score - after.score) / before.score * 100.0).max(0.0)
        } else {
            0.0
        };

        let before_complexity = before.cyclomatic_complexity + before.cognitive_complexity;
        let after_complexity = after.cyclomatic_complexity + after.cognitive_complexity;
        let complexity_reduction_pct = if before_complexity > 0 {
            ((before_complexity - after_complexity) as f64 / before_complexity as f64 * 100.0).max(0.0)
        } else {
            0.0
        };

        let coverage_improvement_pct = (after.coverage - before.coverage).max(0.0);

        ImprovementMetrics {
            score_reduction_pct,
            complexity_reduction_pct,
            coverage_improvement_pct,
        }
    }

    fn classify_target_status(&self, before: &DebtItem, after: &DebtItem) -> TargetStatus {
        let before_score = self.get_score(before);
        let after_score = self.get_score(after);

        if after_score < before_score * 0.7 {
            TargetStatus::Improved
        } else if after_score > before_score * 1.1 {
            TargetStatus::Regressed
        } else {
            TargetStatus::Unchanged
        }
    }

    fn extract_project_metrics(&self, results: &AnalysisResults) -> ProjectMetrics {
        let total_items = results.items.len();
        let critical_items = results.items.iter()
            .filter(|item| self.get_score(item) >= 60.0)
            .count();
        let high_priority_items = results.items.iter()
            .filter(|item| self.get_score(item) >= 40.0)
            .count();

        let average_score = if total_items > 0 {
            results.items.iter()
                .map(|item| self.get_score(item))
                .sum::<f64>() / total_items as f64
        } else {
            0.0
        };

        ProjectMetrics {
            total_debt_score: results.total_debt_score,
            total_items,
            critical_items,
            high_priority_items,
            average_score,
        }
    }

    fn calculate_project_changes(&self, before: &ProjectMetrics, after: &ProjectMetrics) -> ProjectChanges {
        let debt_score_change = after.total_debt_score - before.total_debt_score;
        let debt_score_change_pct = if before.total_debt_score > 0.0 {
            debt_score_change / before.total_debt_score * 100.0
        } else {
            0.0
        };

        ProjectChanges {
            debt_score_change,
            debt_score_change_pct,
            items_change: after.total_items as i32 - before.total_items as i32,
            critical_items_change: after.critical_items as i32 - before.critical_items as i32,
        }
    }

    fn build_regression_item(&self, item: &DebtItem) -> RegressionItem {
        RegressionItem {
            location: self.format_location(item),
            score: self.get_score(item),
            debt_type: format!("{:?}", item.debt_type),
            description: format!("New critical debt item with score {:.1}", self.get_score(item)),
        }
    }

    fn generate_summary(
        &self,
        target: &Option<TargetComparison>,
        regressions: &[RegressionItem],
        improvements: &[ImprovementItem],
    ) -> ComparisonSummary {
        let target_improved = target.as_ref()
            .map(|t| matches!(t.status, TargetStatus::Improved | TargetStatus::Resolved))
            .unwrap_or(false);

        let overall_debt_trend = if self.after.total_debt_score < self.before.total_debt_score * 0.95 {
            DebtTrend::Improving
        } else if self.after.total_debt_score > self.before.total_debt_score * 1.05 {
            DebtTrend::Regressing
        } else {
            DebtTrend::Stable
        };

        ComparisonSummary {
            target_improved,
            new_critical_count: regressions.len(),
            resolved_count: improvements.iter()
                .filter(|i| matches!(i.improvement_type, ImprovementType::Resolved))
                .count(),
            overall_debt_trend,
        }
    }

    fn build_metadata(&self) -> ComparisonMetadata {
        ComparisonMetadata {
            comparison_date: chrono::Utc::now().to_rfc3339(),
            before_file: "before.json".to_string(),
            after_file: "after.json".to_string(),
            target_location: self.target_location.clone(),
        }
    }
}
```

### Plan Parsing

Create `src/comparison/plan_parser.rs`:

```rust
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct PlanParser;

impl PlanParser {
    /// Extract target location from implementation plan markdown
    pub fn extract_target_location(plan_path: &Path) -> Result<String> {
        let content = fs::read_to_string(plan_path)
            .context("Failed to read implementation plan")?;

        // Look for **Location**: pattern
        for line in content.lines() {
            if let Some(location) = Self::parse_location_line(line) {
                return Ok(location);
            }
        }

        Err(anyhow::anyhow!(
            "Could not find **Location**: in plan file. Expected format: **Location**: ./file.rs:function:line"
        ))
    }

    fn parse_location_line(line: &str) -> Option<String> {
        // Match: **Location**: ./src/file.rs:function:123
        // or:    **Location**: ./src/file.rs:123
        if line.trim().starts_with("**Location**:") {
            let location = line
                .split("**Location**:")
                .nth(1)?
                .trim()
                .to_string();

            // Validate format
            if Self::is_valid_location(&location) {
                return Some(location);
            }
        }

        None
    }

    fn is_valid_location(location: &str) -> bool {
        let parts: Vec<&str> = location.split(':').collect();

        // Must have file:function:line format
        if parts.len() != 3 {
            return false;
        }

        // File must start with ./ or /
        if !parts[0].starts_with("./") && !parts[0].starts_with('/') {
            return false;
        }

        // Line must be a number
        parts[2].parse::<usize>().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_location_line() {
        let line = "**Location**: ./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120";
        let result = PlanParser::parse_location_line(line);
        assert_eq!(
            result,
            Some("./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120".to_string())
        );
    }

    #[test]
    fn test_parse_invalid_location() {
        let line = "**Location**: invalid";
        let result = PlanParser::parse_location_line(line);
        assert_eq!(result, None);
    }
}
```

### Command Handler

Update `src/main.rs` to add compare command:

```rust
match cli.command {
    Commands::Compare {
        before,
        after,
        plan,
        target_location,
        format,
        output,
    } => {
        // Extract target location from plan or use explicit location
        let target = if let Some(plan_path) = plan {
            Some(comparison::PlanParser::extract_target_location(&plan_path)?)
        } else {
            target_location
        };

        // Load analysis results
        let before_results = load_analysis_json(&before)?;
        let after_results = load_analysis_json(&after)?;

        // Perform comparison
        let comparator = comparison::Comparator::new(
            before_results,
            after_results,
            target,
        );
        let comparison = comparator.compare()?;

        // Output results
        match format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&comparison)?;
                write_output(&output, &json)?;
            }
            OutputFormat::Markdown => {
                let markdown = comparison::markdown::format_comparison(&comparison);
                write_output(&output, &markdown)?;
            }
            OutputFormat::Terminal => {
                comparison::terminal::print_comparison(&comparison);
            }
        }
    }
    // ... other commands ...
}
```

## Examples

### Example 1: Basic Comparison with Plan

**Command**:
```bash
debtmap compare \
  --before .prodigy/debtmap-before.json \
  --after .prodigy/debtmap-after.json \
  --plan .prodigy/IMPLEMENTATION_PLAN.md \
  --output .prodigy/comparison.json
```

**Output** (`comparison.json`):
```json
{
  "metadata": {
    "comparison_date": "2025-10-01T19:30:00Z",
    "before_file": "debtmap-before.json",
    "after_file": "debtmap-after.json",
    "target_location": "./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120"
  },
  "target_item": {
    "location": "./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120",
    "before": {
      "score": 81.9,
      "cyclomatic_complexity": 17,
      "cognitive_complexity": 62,
      "coverage": 0.0,
      "function_length": 122,
      "nesting_depth": 6
    },
    "after": {
      "score": 15.2,
      "cyclomatic_complexity": 6,
      "cognitive_complexity": 22,
      "coverage": 45.0,
      "function_length": 45,
      "nesting_depth": 2
    },
    "improvements": {
      "score_reduction_pct": 81.4,
      "complexity_reduction_pct": 64.6,
      "coverage_improvement_pct": 45.0
    },
    "status": "Improved"
  },
  "project_health": {
    "before": {
      "total_debt_score": 5234.2,
      "total_items": 1293,
      "critical_items": 47,
      "high_priority_items": 156,
      "average_score": 40.5
    },
    "after": {
      "total_debt_score": 5156.8,
      "total_items": 1295,
      "critical_items": 48,
      "high_priority_items": 154,
      "average_score": 39.8
    },
    "changes": {
      "debt_score_change": -77.4,
      "debt_score_change_pct": -1.48,
      "items_change": 2,
      "critical_items_change": 1
    }
  },
  "regressions": [
    {
      "location": "./src/builders/call_graph.rs:process_with_cross_module:156",
      "score": 65.3,
      "debt_type": "Complexity",
      "description": "New critical debt item with score 65.3"
    }
  ],
  "improvements": [
    {
      "location": "./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120",
      "before_score": 81.9,
      "after_score": 15.2,
      "improvement_type": "ScoreReduced"
    }
  ],
  "summary": {
    "target_improved": true,
    "new_critical_count": 1,
    "resolved_count": 0,
    "overall_debt_trend": "Improving"
  }
}
```

**File size**: ~1.5KB (vs 40MB of input!)

### Example 2: Comparison with Explicit Target

**Command**:
```bash
debtmap compare \
  --before before.json \
  --after after.json \
  --target-location "./src/main.rs:process_data:42" \
  --format markdown
```

**Output** (markdown):
```markdown
# Debtmap Comparison Report

**Date**: 2025-10-01T19:30:00Z
**Target**: ./src/main.rs:process_data:42

## Target Item Analysis

✅ **Status**: Improved

### Before
- **Score**: 75.3
- **Complexity**: Cyclomatic 15, Cognitive 48
- **Coverage**: 0%
- **Function Length**: 98 lines

### After
- **Score**: 28.6
- **Complexity**: Cyclomatic 5, Cognitive 18
- **Coverage**: 65%
- **Function Length**: 32 lines

### Improvements
- Score reduced by **62.0%**
- Complexity reduced by **58.7%**
- Coverage improved by **65.0%**

## Project Health

### Overall Trend: 📉 Improving

- Total debt: 5,234 → 4,987 (-4.7%)
- Critical items: 47 → 45 (-2)
- No new critical items introduced ✅

## Summary

✅ Target item significantly improved
✅ No regressions detected
✅ Overall project health improved
```

### Example 3: Regression Detected

**Command**:
```bash
debtmap compare \
  --before before.json \
  --after after.json \
  --plan plan.md \
  --output comparison.json
```

**Output** (shows regressions):
```json
{
  "target_item": {
    "status": "Improved",
    "improvements": {
      "score_reduction_pct": 81.4
    }
  },
  "project_health": {
    "changes": {
      "debt_score_change": 222.6,
      "debt_score_change_pct": 4.25
    }
  },
  "regressions": [
    {
      "location": "./src/helpers.rs:extract_data:45",
      "score": 72.5,
      "debt_type": "Complexity"
    },
    {
      "location": "./src/helpers.rs:validate_input:89",
      "score": 68.3,
      "debt_type": "Complexity"
    },
    {
      "location": "./src/helpers.rs:transform_output:123",
      "score": 63.7,
      "debt_type": "TestingGap"
    }
  ],
  "summary": {
    "target_improved": true,
    "new_critical_count": 3,
    "overall_debt_trend": "Regressing"
  }
}
```

## Test Cases

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_target_improved() {
        let before = create_test_analysis(vec![
            create_test_item("./src/main.rs", "func", 42, 81.9),
        ]);
        let after = create_test_analysis(vec![
            create_test_item("./src/main.rs", "func", 42, 15.2),
        ]);

        let comparator = Comparator::new(
            before,
            after,
            Some("./src/main.rs:func:42".to_string()),
        );
        let result = comparator.compare().unwrap();

        assert!(result.target_item.is_some());
        let target = result.target_item.unwrap();
        assert_eq!(target.status, TargetStatus::Improved);
        assert!(target.improvements.score_reduction_pct > 80.0);
    }

    #[test]
    fn test_compare_target_resolved() {
        let before = create_test_analysis(vec![
            create_test_item("./src/main.rs", "func", 42, 81.9),
        ]);
        let after = create_test_analysis(vec![]);

        let comparator = Comparator::new(
            before,
            after,
            Some("./src/main.rs:func:42".to_string()),
        );
        let result = comparator.compare().unwrap();

        let target = result.target_item.unwrap();
        assert_eq!(target.status, TargetStatus::Resolved);
        assert_eq!(target.after, None);
        assert_eq!(target.improvements.score_reduction_pct, 100.0);
    }

    #[test]
    fn test_detect_regressions() {
        let before = create_test_analysis(vec![
            create_test_item("./src/main.rs", "old_func", 42, 81.9),
        ]);
        let after = create_test_analysis(vec![
            create_test_item("./src/main.rs", "old_func", 42, 15.2),
            create_test_item("./src/main.rs", "new_func1", 156, 65.3),
            create_test_item("./src/main.rs", "new_func2", 189, 58.7),
        ]);

        let comparator = Comparator::new(before, after, None);
        let result = comparator.compare().unwrap();

        assert_eq!(result.regressions.len(), 2);
        assert!(result.summary.overall_debt_trend == DebtTrend::Regressing);
    }

    #[test]
    fn test_project_health_comparison() {
        let before = create_test_analysis_with_totals(1000.0, 100, 10);
        let after = create_test_analysis_with_totals(950.0, 98, 9);

        let comparator = Comparator::new(before, after, None);
        let result = comparator.compare().unwrap();

        assert_eq!(result.project_health.changes.debt_score_change, -50.0);
        assert_eq!(result.project_health.changes.items_change, -2);
        assert_eq!(result.project_health.changes.critical_items_change, -1);
        assert_eq!(result.summary.overall_debt_trend, DebtTrend::Improving);
    }

    #[test]
    fn test_plan_parser() {
        let plan_content = r#"
# Implementation Plan

## Problem Summary

**Location**: ./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120
**Priority Score**: 81.9
"#;

        let temp_file = create_temp_file(plan_content);
        let location = PlanParser::extract_target_location(&temp_file).unwrap();

        assert_eq!(
            location,
            "./src/builders/call_graph.rs:process_python_files_for_call_graph_with_types:120"
        );
    }
}
```

### Integration Tests

Create `tests/compare_integration.rs`:

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_compare_basic() {
    let temp = TempDir::new().unwrap();

    // Create before.json and after.json
    create_test_analysis_files(&temp);

    Command::cargo_bin("debtmap")
        .unwrap()
        .arg("compare")
        .arg("--before")
        .arg(temp.path().join("before.json"))
        .arg("--after")
        .arg(temp.path().join("after.json"))
        .arg("--target-location")
        .arg("./src/main.rs:func:42")
        .assert()
        .success()
        .stdout(predicate::str::contains("target_item"))
        .stdout(predicate::str::contains("project_health"));
}

#[test]
fn test_compare_with_plan() {
    let temp = TempDir::new().unwrap();

    create_test_analysis_files(&temp);
    create_test_plan_file(&temp);

    Command::cargo_bin("debtmap")
        .unwrap()
        .arg("compare")
        .arg("--before")
        .arg(temp.path().join("before.json"))
        .arg("--after")
        .arg(temp.path().join("after.json"))
        .arg("--plan")
        .arg(temp.path().join("plan.md"))
        .arg("--format")
        .arg("json")
        .assert()
        .success();
}

#[test]
fn test_compare_output_file() {
    let temp = TempDir::new().unwrap();

    create_test_analysis_files(&temp);

    Command::cargo_bin("debtmap")
        .unwrap()
        .arg("compare")
        .arg("--before")
        .arg(temp.path().join("before.json"))
        .arg("--after")
        .arg(temp.path().join("after.json"))
        .arg("--target-location")
        .arg("./src/main.rs:func:42")
        .arg("--output")
        .arg(temp.path().join("comparison.json"))
        .assert()
        .success();

    assert!(temp.path().join("comparison.json").exists());
}
```

## Implementation Plan

### Phase 1: Core Structures (2-3 hours)
1. Create `src/comparison/mod.rs` with result structures
2. Add `ComparisonResult`, `TargetComparison`, etc. types
3. Implement serialization/deserialization
4. Add unit tests for structures

### Phase 2: Comparison Logic (3-4 hours)
5. Create `src/comparison/comparator.rs`
6. Implement `Comparator::compare()` method
7. Implement target item comparison
8. Implement regression detection
9. Implement project health comparison
10. Add unit tests for comparison logic

### Phase 3: Plan Parsing (1 hour)
11. Create `src/comparison/plan_parser.rs`
12. Implement markdown location extraction
13. Add validation and error handling
14. Add unit tests for parser

### Phase 4: CLI Integration (1-2 hours)
15. Add `Compare` command to CLI enum
16. Implement command handler in main.rs
17. Add JSON output formatter
18. Add error handling

### Phase 5: Output Formats (2-3 hours)
19. Implement markdown formatter
20. Implement terminal formatter
21. Add color output for terminal
22. Add tests for formatters

### Phase 6: Testing & Polish (2-3 hours)
23. Write comprehensive integration tests
24. Test with real-world debtmap output
25. Add helpful error messages
26. Update CLI help text
27. Performance testing with large JSON files

### Phase 7: Documentation (1 hour)
28. Update README with `compare` examples
29. Add to CHANGELOG
30. Update workflow documentation
31. Add inline documentation

**Total Estimated Effort**: 12-17 hours

## Workflow Integration

### Updated Workflow

```yaml
# Phase 4: Execute the plan (IMPLEMENTATION PHASE)
- claude: "/prodigy-debtmap-implement --plan .prodigy/IMPLEMENTATION_PLAN.md"
  commit_required: true
  validate:
    # Step 1: Full analysis (for comprehensive regression detection)
    shell: "debtmap analyze . --lcov target/coverage/lcov.info --output .prodigy/debtmap-after.json --format json"

    # Step 2: Create compact comparison (10KB instead of 40MB!)
    shell: "debtmap compare --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json --plan .prodigy/IMPLEMENTATION_PLAN.md --output .prodigy/comparison.json"

    # Step 3: Validate using compact comparison
    claude: "/prodigy-validate-comparison --comparison .prodigy/comparison.json --output .prodigy/validation.json"
    result_file: ".prodigy/validation.json"
    threshold: 75

    on_incomplete:
      # Recovery loop
      claude: "/prodigy-complete-debtmap-fix --gaps ${validation.gaps} --plan .prodigy/IMPLEMENTATION_PLAN.md"
      commit_required: true
      shell: "just coverage-lcov"
      shell: "debtmap analyze . --lcov target/coverage/lcov.info --output .prodigy/debtmap-after.json"
      shell: "debtmap compare --before .prodigy/debtmap-before.json --after .prodigy/debtmap-after.json --plan .prodigy/IMPLEMENTATION_PLAN.md --output .prodigy/comparison.json"
      max_attempts: 5
```

### Simplified Validation Command

Update `.claude/commands/prodigy-validate-comparison.md`:

```markdown
# Validate Comparison Command

## Usage

/prodigy-validate-comparison --comparison <comparison-file> --output <output-file>

## What This Command Does

Validates that the technical debt improvement meets quality standards.

## Process

### Step 1: Load Comparison Result

Read the comparison JSON (created by `debtmap compare`):

```bash
cat $ARG_comparison
```

This is a compact ~10KB file with all needed information already computed!

### Step 2: Calculate Improvement Score

Use the formula:

```python
target_component = (
    max([
        comparison.target_item.improvements.score_reduction_pct,
        comparison.target_item.improvements.complexity_reduction_pct,
        comparison.target_item.improvements.coverage_improvement_pct
    ]) * 0.7 +
    sum(other_improvements) * 0.15
)

project_health_component = max(0, -comparison.project_health.changes.debt_score_change_pct)

regression_penalty = min(100, comparison.summary.new_critical_count * 20)

improvement_score = (
    target_component * 0.5 +
    project_health_component * 0.3 +
    (100 - regression_penalty) * 0.2
)
```

### Step 3: Write Validation Result

Output validation JSON with pass/fail and gaps.

**Much simpler** - no complex parsing, filtering, or comparison logic needed!
```

## Success Metrics

- [ ] Comparison output <10KB for typical cases (99.975% size reduction)
- [ ] Execution time <3 seconds for 40MB input
- [ ] Validation command input reduced from 40MB to <10KB
- [ ] Validation command simplified (50% less logic)
- [ ] Zero bugs after 1 week of production use
- [ ] Adopted in production workflows within 1 week

## Benefits Summary

| Aspect | Before | After | Improvement |
|--------|--------|-------|-------------|
| **Validation input size** | 40MB JSON | 10KB JSON | 99.975% smaller |
| **Context tokens** | ~10M tokens | ~3K tokens | 99.97% fewer |
| **Comparison logic** | In Claude prompt | In Rust (debtmap) | Domain appropriate ✅ |
| **Parsing complexity** | High (jq/JSON) | Low (read struct) | Much simpler |
| **Maintainability** | Fragile prompts | Tested Rust code | Much better |
| **Performance** | Slow parsing | Instant | 1000x faster |
| **Error handling** | Limited | Comprehensive | Much better |

## Future Enhancements (Out of Scope)

1. **Historical comparison**: Compare against multiple previous runs
2. **Trend analysis**: Track improvement velocity over time
3. **Custom thresholds**: Configure what counts as "critical" (currently hardcoded 60.0)
4. **Diff visualization**: HTML report with visual comparison
5. **Watch mode**: Continuously compare as files change
6. **CI integration**: Exit codes based on regression detection

## References

- Related spec: #108 Location-Based Filtering
- Prodigy workflow: `workflows/debtmap.yml`
- Validation command: `.claude/commands/prodigy-validate-debtmap-improvement.md`
- Original issue: Workflow validation performance analysis
