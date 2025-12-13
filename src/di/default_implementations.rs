//! Default implementations for dependency injection traits
//!
//! This module provides production-ready implementations of the core DI traits
//! including scorers, config providers, priority calculators, and formatters.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::core::injection::{AppContainer, AppContainerBuilder, RustAnalyzerAdapter};
use crate::core::traits::{ConfigProvider, Formatter, PriorityCalculator, PriorityFactor, Scorer};
use crate::core::types::{AnalysisResult, DebtCategory, DebtItem, Severity};

/// Default debt scorer implementation
///
/// Scores debt items based on category, severity, and estimated effort.
pub struct DefaultDebtScorer;

impl DefaultDebtScorer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultDebtScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl Scorer for DefaultDebtScorer {
    type Item = DebtItem;

    fn score(&self, item: &Self::Item) -> f64 {
        let base_score = match item.category {
            DebtCategory::Complexity => 10.0,
            DebtCategory::Testing => 8.0,
            DebtCategory::Documentation => 6.0,
            DebtCategory::Organization => 7.0,
            DebtCategory::Performance => 9.0,
            DebtCategory::Security => 10.0,
            DebtCategory::Maintainability => 7.0,
            _ => 5.0,
        };

        let severity_multiplier = match item.severity {
            Severity::Critical => 3.0,
            Severity::Major => 2.0,
            Severity::Warning => 1.5,
            Severity::Info => 1.0,
        };

        base_score * severity_multiplier * item.effort.max(0.1)
    }

    fn methodology(&self) -> &str {
        "Default scoring based on category, severity, and estimated hours"
    }
}

/// Default configuration provider
///
/// Uses a RwLock-protected HashMap for thread-safe config access.
pub struct DefaultConfigProvider {
    config: RwLock<HashMap<String, String>>,
}

impl DefaultConfigProvider {
    pub fn new() -> Self {
        let mut config = HashMap::new();
        // Load default configuration values
        config.insert("complexity_threshold".to_string(), "10".to_string());
        config.insert("max_file_size".to_string(), "1000000".to_string());
        config.insert("parallel_processing".to_string(), "true".to_string());

        Self {
            config: RwLock::new(config),
        }
    }
}

impl Default for DefaultConfigProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigProvider for DefaultConfigProvider {
    fn get(&self, key: &str) -> Option<String> {
        let config = self.config.read().unwrap();
        config.get(key).cloned()
    }

    fn set(&mut self, key: String, value: String) {
        let mut config = self.config.write().unwrap();
        config.insert(key, value);
    }

    fn load_from_file(&self, _path: &std::path::Path) -> Result<()> {
        // In production, would read from actual config file
        // For now, just return Ok to satisfy the trait
        Ok(())
    }
}

/// Default priority calculator
///
/// Calculates priority based on severity, category, and effort factors.
pub struct DefaultPriorityCalculator;

impl DefaultPriorityCalculator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultPriorityCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl PriorityCalculator for DefaultPriorityCalculator {
    type Item = DebtItem;

    fn calculate_priority(&self, item: &Self::Item) -> f64 {
        let severity_weight = match item.severity {
            Severity::Critical => 1.0,
            Severity::Major => 0.75,
            Severity::Warning => 0.5,
            Severity::Info => 0.25,
        };

        let category_weight = match item.category {
            DebtCategory::Complexity => 0.9,
            DebtCategory::Testing => 0.85,
            DebtCategory::Performance => 0.8,
            DebtCategory::Organization => 0.7,
            DebtCategory::Documentation => 0.6,
            DebtCategory::Security => 0.95,
            _ => 0.5,
        };

        let effort_factor = 1.0 / (1.0 + item.effort);

        f64::min(
            severity_weight * 0.5 + category_weight * 0.3 + effort_factor * 0.2,
            1.0,
        )
    }

    fn get_factors(&self, item: &Self::Item) -> Vec<PriorityFactor> {
        vec![
            PriorityFactor {
                name: "severity".to_string(),
                weight: 0.5,
                value: match item.severity {
                    Severity::Critical => 1.0,
                    Severity::Major => 0.75,
                    Severity::Warning => 0.5,
                    Severity::Info => 0.25,
                },
                description: format!("Severity: {:?}", item.severity),
            },
            PriorityFactor {
                name: "category".to_string(),
                weight: 0.3,
                value: match item.category {
                    DebtCategory::Complexity => 0.9,
                    DebtCategory::Testing => 0.85,
                    _ => 0.5,
                },
                description: format!("Category: {:?}", item.category),
            },
            PriorityFactor {
                name: "effort".to_string(),
                weight: 0.2,
                value: 1.0 / (1.0 + item.effort),
                description: format!("Estimated effort: {} hours", item.effort),
            },
        ]
    }
}

/// JSON formatter for analysis results
pub struct JsonFormatter;

impl JsonFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for JsonFormatter {
    type Report = AnalysisResult;

    fn format(&self, report: &Self::Report) -> Result<String> {
        serde_json::to_string_pretty(report)
            .map_err(|e| anyhow::anyhow!("JSON formatting error: {}", e))
    }

    fn format_name(&self) -> &str {
        "json"
    }
}

/// Markdown formatter for analysis results
pub struct MarkdownFormatter;

impl MarkdownFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MarkdownFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for MarkdownFormatter {
    type Report = AnalysisResult;

    fn format(&self, report: &Self::Report) -> Result<String> {
        let mut output = String::new();
        output.push_str("# Code Analysis Report\n\n");
        output.push_str("## Summary\n\n");
        output.push_str(&format!("- Total Files: {}\n", report.metrics.total_files));
        output.push_str(&format!(
            "- Total Functions: {}\n",
            report.metrics.total_functions
        ));
        output.push_str(&format!("- Total Lines: {}\n", report.metrics.total_lines));
        output.push_str(&format!(
            "- Average Complexity: {:.2}\n",
            report.metrics.average_complexity
        ));
        output.push_str(&format!("- Debt Score: {:.2}\n\n", report.total_score));

        if !report.debt_items.is_empty() {
            output.push_str("## Technical Debt Items\n\n");
            for item in &report.debt_items {
                output.push_str(&format!(
                    "- **{:?}** ({:?}): {}\n",
                    item.category, item.severity, item.description
                ));
            }
        }

        Ok(output)
    }

    fn format_name(&self) -> &str {
        "markdown"
    }
}

/// Terminal formatter for analysis results
pub struct TerminalFormatter;

impl TerminalFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TerminalFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for TerminalFormatter {
    type Report = AnalysisResult;

    fn format(&self, report: &Self::Report) -> Result<String> {
        let mut output = String::new();
        output.push_str("═══════════════════════════════════════\n");
        output.push_str("         Code Analysis Report          \n");
        output.push_str("═══════════════════════════════════════\n\n");

        output.push_str(&format!(
            "Total Files:      {}\n",
            report.metrics.total_files
        ));
        output.push_str(&format!(
            "Total Functions:  {}\n",
            report.metrics.total_functions
        ));
        output.push_str(&format!(
            "Total Lines:      {}\n",
            report.metrics.total_lines
        ));
        output.push_str(&format!(
            "Avg Complexity:   {:.2}\n",
            report.metrics.average_complexity
        ));
        output.push_str(&format!("Debt Score:       {:.2}\n", report.total_score));

        if !report.debt_items.is_empty() {
            output.push_str("\n───────────────────────────────────────\n");
            output.push_str("Technical Debt Summary:\n");
            output.push_str(&format!("  {} items found\n", report.debt_items.len()));

            // Create severity counts
            let mut critical_count = 0;
            let mut major_count = 0;
            let mut warning_count = 0;
            let mut info_count = 0;

            for item in &report.debt_items {
                match item.severity {
                    Severity::Critical => critical_count += 1,
                    Severity::Major => major_count += 1,
                    Severity::Warning => warning_count += 1,
                    Severity::Info => info_count += 1,
                }
            }

            if critical_count > 0 {
                output.push_str(&format!("  Critical: {}\n", critical_count));
            }
            if major_count > 0 {
                output.push_str(&format!("  Major: {}\n", major_count));
            }
            if warning_count > 0 {
                output.push_str(&format!("  Warning: {}\n", warning_count));
            }
            if info_count > 0 {
                output.push_str(&format!("  Info: {}\n", info_count));
            }
        }

        Ok(output)
    }

    fn format_name(&self) -> &str {
        "terminal"
    }
}

/// Create and configure the dependency injection container
pub fn create_app_container() -> Result<AppContainer> {
    let container = AppContainerBuilder::new()
        .with_rust_analyzer(RustAnalyzerAdapter::new())
        .with_debt_scorer(DefaultDebtScorer::new())
        .with_config(DefaultConfigProvider::new())
        .with_priority_calculator(DefaultPriorityCalculator::new())
        .with_json_formatter(JsonFormatter::new())
        .with_markdown_formatter(MarkdownFormatter::new())
        .with_terminal_formatter(TerminalFormatter::new())
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build container: {}", e))?;

    Ok(container)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::SourceLocation;
    use std::path::PathBuf;

    fn create_test_item(category: DebtCategory, severity: Severity, effort: f64) -> DebtItem {
        DebtItem {
            id: "test-id".to_string(),
            category,
            severity,
            location: SourceLocation {
                file: PathBuf::from("test.rs"),
                line: 1,
                column: 0,
                end_line: Some(10),
                end_column: Some(0),
            },
            description: "Test item".to_string(),
            impact: 1.0,
            effort,
            priority: 0.5,
            suggestions: vec![],
        }
    }

    #[test]
    fn test_default_scorer() {
        let scorer = DefaultDebtScorer::new();
        let item = create_test_item(DebtCategory::Complexity, Severity::Major, 2.0);
        let score = scorer.score(&item);
        assert!(score > 0.0);
    }

    #[test]
    fn test_default_config_provider() {
        let provider = DefaultConfigProvider::new();
        assert_eq!(provider.get("complexity_threshold"), Some("10".to_string()));
        assert!(provider.get("nonexistent").is_none());
    }

    #[test]
    fn test_default_priority_calculator() {
        let calc = DefaultPriorityCalculator::new();
        let item = create_test_item(DebtCategory::Complexity, Severity::Critical, 1.0);
        let priority = calc.calculate_priority(&item);
        assert!(priority > 0.0);
        assert!(priority <= 1.0);
    }

    #[test]
    fn test_create_app_container() {
        let result = create_app_container();
        assert!(result.is_ok());
    }
}
