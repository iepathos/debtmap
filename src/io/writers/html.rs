use crate::core::AnalysisResults;
use crate::io::output::OutputWriter;
use crate::output::unified::convert_to_unified_format;
use crate::priority::UnifiedAnalysis;
use crate::risk::RiskInsight;
use anyhow::Result;
use html_escape::encode_text;
use serde_json;
use std::env;
use std::io::Write;
use std::path::Path;

pub struct HtmlWriter<W: Write> {
    writer: W,
    template: &'static str,
    unified_analysis: Option<UnifiedAnalysis>,
}

impl<W: Write> HtmlWriter<W> {
    pub fn with_unified_analysis(writer: W, analysis: UnifiedAnalysis) -> Self {
        Self {
            writer,
            template: include_str!("templates/dashboard.html"),
            unified_analysis: Some(analysis),
        }
    }
}

impl<W: Write> HtmlWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            template: include_str!("templates/dashboard.html"),
            unified_analysis: None,
        }
    }

    fn calculate_metrics(&self, results: &AnalysisResults) -> DashboardMetrics {
        // If unified analysis is available, use filtered data (excludes T4 items)
        if let Some(ref unified) = self.unified_analysis {
            use crate::output::unified::convert_to_unified_format;
            use crate::priority::tiers::TierConfig;
            use crate::priority::UnifiedAnalysisQueries;

            // Get filtered items (T4 excluded by default)
            let tier_config = TierConfig::default();
            let filtered_items = unified.get_top_mixed_priorities_tiered(usize::MAX, &tier_config);

            // Use unified output which has filtered totals and density
            let unified_output = convert_to_unified_format(unified, false);
            let critical_count = unified_output.summary.score_distribution.critical;
            let high_count = unified_output.summary.score_distribution.high;
            let medium_count = unified_output.summary.score_distribution.medium;
            let low_count = unified_output.summary.score_distribution.low;

            return DashboardMetrics {
                total_items: filtered_items.len(),
                critical_count,
                high_count,
                medium_count,
                low_count,
                total_functions: results.complexity.summary.total_functions,
                average_complexity: results.complexity.summary.average_complexity,
                // Use debt density from unified output (calculated from filtered items)
                debt_density: unified_output.summary.debt_density,
            };
        }

        // Legacy path: use raw results (includes all items)
        let critical_count = results
            .technical_debt
            .items
            .iter()
            .filter(|i| matches!(i.priority, crate::core::Priority::Critical))
            .count();

        let high_count = results
            .technical_debt
            .items
            .iter()
            .filter(|i| matches!(i.priority, crate::core::Priority::High))
            .count();

        let medium_count = results
            .technical_debt
            .items
            .iter()
            .filter(|i| matches!(i.priority, crate::core::Priority::Medium))
            .count();

        let low_count = results
            .technical_debt
            .items
            .iter()
            .filter(|i| matches!(i.priority, crate::core::Priority::Low))
            .count();

        DashboardMetrics {
            total_items: results.technical_debt.items.len(),
            critical_count,
            high_count,
            medium_count,
            low_count,
            total_functions: results.complexity.summary.total_functions,
            average_complexity: results.complexity.summary.average_complexity,
            debt_density: calculate_debt_density(results),
        }
    }

    fn render_html(&self, results: &AnalysisResults, metrics: &DashboardMetrics) -> Result<String> {
        // Use unified format if available, otherwise use legacy format
        let json_data = if let Some(ref unified) = self.unified_analysis {
            let unified_output = convert_to_unified_format(unified, true);
            serde_json::to_string(&unified_output)?
        } else {
            serde_json::to_string(results)?
        };
        let escaped_json = encode_text(&json_data);

        let html = self
            .template
            .replace("{{{JSON_DATA}}}", &escaped_json)
            .replace(
                "{{{TIMESTAMP}}}",
                &results.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            )
            .replace(
                "{{{PROJECT_NAME}}}",
                &format_project_name(&results.project_path),
            )
            .replace("{{{TOTAL_ITEMS}}}", &metrics.total_items.to_string())
            .replace("{{{CRITICAL_COUNT}}}", &metrics.critical_count.to_string())
            .replace("{{{HIGH_COUNT}}}", &metrics.high_count.to_string())
            .replace("{{{MEDIUM_COUNT}}}", &metrics.medium_count.to_string())
            .replace("{{{LOW_COUNT}}}", &metrics.low_count.to_string())
            .replace(
                "{{{DEBT_DENSITY}}}",
                &format!("{:.1}", metrics.debt_density),
            )
            .replace(
                "{{{TOTAL_FUNCTIONS}}}",
                &metrics.total_functions.to_string(),
            )
            .replace(
                "{{{AVG_COMPLEXITY}}}",
                &format!("{:.1}", metrics.average_complexity),
            );

        Ok(html)
    }
}

impl<W: Write> OutputWriter for HtmlWriter<W> {
    fn write_results(&mut self, results: &AnalysisResults) -> Result<()> {
        let metrics = self.calculate_metrics(results);
        let html = self.render_html(results, &metrics)?;
        write!(self.writer, "{}", html)?;
        Ok(())
    }

    fn write_risk_insights(&mut self, _insights: &RiskInsight) -> Result<()> {
        Ok(())
    }
}

struct DashboardMetrics {
    total_items: usize,
    critical_count: usize,
    high_count: usize,
    medium_count: usize,
    low_count: usize,
    total_functions: usize,
    average_complexity: f64,
    debt_density: f64,
}

fn format_project_name(project_path: &Path) -> String {
    // Get the current directory for resolving relative paths
    let current_dir = env::current_dir().ok();

    // If the path is ".", return the current directory name
    if project_path.as_os_str() == "." {
        if let Some(cwd) = current_dir {
            if let Some(dir_name) = cwd.file_name() {
                return dir_name.to_string_lossy().to_string();
            }
        }
        return ".".to_string();
    }

    // If it's a relative path and we have current_dir, combine them for context
    if project_path.is_relative() {
        if let Some(cwd) = current_dir {
            if let Some(cwd_name) = cwd.file_name() {
                return format!("{}/{}", cwd_name.to_string_lossy(), project_path.display());
            }
        }
    }

    // Otherwise, just display the path as is
    project_path.display().to_string()
}

fn calculate_debt_density(results: &AnalysisResults) -> f64 {
    let total_loc: usize = results.complexity.metrics.iter().map(|m| m.length).sum();

    if total_loc > 0 {
        (results.technical_debt.items.len() as f64 / total_loc as f64) * 1000.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        AnalysisResults, ComplexityReport, ComplexitySummary, DebtItem, DebtType, DependencyReport,
        FunctionMetrics, Priority, TechnicalDebtReport,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_test_results() -> AnalysisResults {
        let items = vec![DebtItem {
            id: "test-1".to_string(),
            debt_type: DebtType::Complexity,
            priority: Priority::High,
            file: PathBuf::from("test.rs"),
            line: 5,
            column: None,
            message: "High complexity function".to_string(),
            context: None,
        }];

        let metrics = vec![FunctionMetrics {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 15,
            cognitive: 20,
            nesting: 3,
            length: 50,
            is_test: false,
            visibility: None,
            is_trait_method: false,
            in_test_module: false,
            entropy_score: None,
            is_pure: None,
            purity_confidence: None,
            purity_reason: None,
            call_dependencies: None,
            detected_patterns: None,
            upstream_callers: None,
            downstream_callees: None,
            mapping_pattern_result: None,
            adjusted_complexity: None,
            composition_metrics: None,
            language_specific: None,
            purity_level: None,
        }];

        AnalysisResults {
            project_path: PathBuf::from("/test/project"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: metrics.clone(),
                summary: ComplexitySummary {
                    total_functions: 1,
                    average_complexity: 15.0,
                    max_complexity: 20,
                    high_complexity_count: 1,
                },
            },
            technical_debt: TechnicalDebtReport {
                items,
                by_type: HashMap::new(),
                priorities: vec![Priority::High],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
            file_contexts: HashMap::new(),
        }
    }

    #[test]
    fn test_html_writer_generates_valid_html() {
        let results = create_test_results();
        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("<!DOCTYPE html>"));
        assert!(output.contains("</html>"));
        assert!(output.contains("Debtmap Analysis Dashboard"));
    }

    #[test]
    fn test_html_contains_metrics() {
        let results = create_test_results();
        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("1 items analyzed"));
        assert!(output.contains("1 functions"));
    }

    #[test]
    fn test_html_escapes_json_data() {
        let mut results = create_test_results();
        results.technical_debt.items[0].message = "<script>alert('xss')</script>".to_string();

        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("&lt;script&gt;") || !output.contains("<script>alert"));
    }

    #[test]
    fn test_calculate_debt_density() {
        let results = create_test_results();
        let density = calculate_debt_density(&results);
        assert_eq!(density, 20.0);
    }

    #[test]
    fn test_calculate_debt_density_zero_loc() {
        let mut results = create_test_results();
        results.complexity.metrics[0].length = 0;

        let density = calculate_debt_density(&results);
        assert_eq!(density, 0.0);
    }

    #[test]
    fn test_all_template_variables_substituted() {
        let results = create_test_results();
        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        // Verify all template variables are replaced (none should remain)
        assert!(!output.contains("{{{PROJECT_NAME}}}"));
        assert!(!output.contains("{{{TIMESTAMP}}}"));
        assert!(!output.contains("{{{TOTAL_ITEMS}}}"));
        assert!(!output.contains("{{{CRITICAL_COUNT}}}"));
        assert!(!output.contains("{{{HIGH_COUNT}}}"));
        assert!(!output.contains("{{{MEDIUM_COUNT}}}"));
        assert!(!output.contains("{{{LOW_COUNT}}}"));
        assert!(!output.contains("{{{DEBT_DENSITY}}}"));
        assert!(!output.contains("{{{TOTAL_FUNCTIONS}}}"));
        assert!(!output.contains("{{{AVG_COMPLEXITY}}}"));

        // Verify actual values are present
        assert!(output.contains("/test/project"));
        assert!(output.contains("1 items analyzed"));
        assert!(output.contains("1 functions"));
    }

    #[test]
    fn test_priority_counts_all_levels() {
        let items = vec![
            DebtItem {
                id: "critical-1".to_string(),
                debt_type: DebtType::Complexity,
                priority: Priority::Critical,
                file: PathBuf::from("test1.rs"),
                line: 1,
                column: None,
                message: "Critical issue".to_string(),
                context: None,
            },
            DebtItem {
                id: "critical-2".to_string(),
                debt_type: DebtType::ErrorSwallowing,
                priority: Priority::Critical,
                file: PathBuf::from("test2.rs"),
                line: 2,
                column: None,
                message: "Another critical".to_string(),
                context: None,
            },
            DebtItem {
                id: "high-1".to_string(),
                debt_type: DebtType::CodeSmell,
                priority: Priority::High,
                file: PathBuf::from("test3.rs"),
                line: 3,
                column: None,
                message: "High priority".to_string(),
                context: None,
            },
            DebtItem {
                id: "medium-1".to_string(),
                debt_type: DebtType::Todo,
                priority: Priority::Medium,
                file: PathBuf::from("test4.rs"),
                line: 4,
                column: None,
                message: "Medium priority".to_string(),
                context: None,
            },
            DebtItem {
                id: "low-1".to_string(),
                debt_type: DebtType::Duplication,
                priority: Priority::Low,
                file: PathBuf::from("test5.rs"),
                line: 5,
                column: None,
                message: "Low priority".to_string(),
                context: None,
            },
        ];

        let results = AnalysisResults {
            project_path: PathBuf::from("/test/project"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 0,
                    average_complexity: 0.0,
                    max_complexity: 0,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: items.clone(),
                by_type: HashMap::new(),
                priorities: vec![
                    Priority::Critical,
                    Priority::High,
                    Priority::Medium,
                    Priority::Low,
                ],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
            file_contexts: HashMap::new(),
        };

        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);
        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        // Check counts in metric cards
        assert!(output.contains(">2</div>")); // Critical count
        assert!(output.contains(">1</div>")); // High, Medium, Low counts
        assert!(output.contains("5 items analyzed"));
    }

    #[test]
    fn test_empty_results() {
        let results = AnalysisResults {
            project_path: PathBuf::from("/empty/project"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: vec![],
                summary: ComplexitySummary {
                    total_functions: 0,
                    average_complexity: 0.0,
                    max_complexity: 0,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
            file_contexts: HashMap::new(),
        };

        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        // Should not panic with empty data
        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("0 items analyzed"));
        assert!(output.contains("0 functions"));
        assert!(output.contains("Debt Density: 0.0 per 1K LOC"));
    }

    #[test]
    fn test_with_unified_analysis_constructor() {
        // Simple test to verify with_unified_analysis constructor doesn't panic
        // and produces valid HTML output
        use crate::priority::call_graph::CallGraph;
        use crate::priority::UnifiedAnalysis;

        let call_graph = CallGraph::new();
        let analysis = UnifiedAnalysis::new(call_graph);
        let results = create_test_results();
        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::with_unified_analysis(&mut buffer, analysis);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        // Verify HTML structure is present
        assert!(output.contains("<!DOCTYPE html>"));
        assert!(output.contains("<html"));
        // Verify unified format markers are present
        assert!(output.contains("format_version"));
    }

    #[test]
    fn test_special_characters_in_paths() {
        let mut results = create_test_results();
        results.project_path = PathBuf::from("/path/with spaces/and'quotes\"");
        results.technical_debt.items[0].file = PathBuf::from("file with spaces.rs");
        results.technical_debt.items[0].message = "Message with <html> & \"quotes\"".to_string();

        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        // Verify special characters are escaped in JSON
        assert!(output.contains("&lt;html&gt;") || output.contains("\\u003c"));
        assert!(output.contains("file with spaces.rs"));
        // Should not contain unescaped quotes that would break JSON
        let json_start = output.find("debt-data").unwrap();
        let json_section = &output[json_start..json_start + 1000];
        // JSON should be properly escaped
        assert!(!json_section.contains("Message with <html>"));
    }

    #[test]
    fn test_multiple_debt_items_with_entropy() {
        let metrics = vec![
            FunctionMetrics {
                name: "high_entropy_func".to_string(),
                file: PathBuf::from("test1.rs"),
                line: 10,
                cyclomatic: 15,
                cognitive: 20,
                nesting: 3,
                length: 50,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: Some(crate::complexity::entropy_core::EntropyScore {
                    token_entropy: 0.8,
                    pattern_repetition: 0.2,
                    branch_similarity: 0.3,
                    effective_complexity: 0.7,
                    unique_variables: 15,
                    max_nesting: 3,
                    dampening_applied: 0.8,
                }),
                is_pure: Some(true),
                purity_confidence: Some(0.95),
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: Some(12.0),
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
            },
            FunctionMetrics {
                name: "low_entropy_func".to_string(),
                file: PathBuf::from("test2.rs"),
                line: 20,
                cyclomatic: 10,
                cognitive: 12,
                nesting: 2,
                length: 30,
                is_test: false,
                visibility: None,
                is_trait_method: false,
                in_test_module: false,
                entropy_score: Some(crate::complexity::entropy_core::EntropyScore {
                    token_entropy: 0.3,
                    pattern_repetition: 0.7,
                    branch_similarity: 0.8,
                    effective_complexity: 0.2,
                    unique_variables: 8,
                    max_nesting: 2,
                    dampening_applied: 0.3,
                }),
                is_pure: Some(false),
                purity_confidence: Some(0.6),
                purity_reason: None,
                call_dependencies: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: Some(3.0),
                composition_metrics: None,
                language_specific: None,
                purity_level: None,
            },
        ];

        let results = AnalysisResults {
            project_path: PathBuf::from("/test/project"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                metrics: metrics.clone(),
                summary: ComplexitySummary {
                    total_functions: 2,
                    average_complexity: 12.5,
                    max_complexity: 20,
                    high_complexity_count: 1,
                },
            },
            technical_debt: TechnicalDebtReport {
                items: vec![],
                by_type: HashMap::new(),
                priorities: vec![],
                duplications: vec![],
            },
            dependencies: DependencyReport {
                modules: vec![],
                circular: vec![],
            },
            duplications: vec![],
            file_contexts: HashMap::new(),
        };

        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        // Verify entropy data is in JSON
        assert!(output.contains("token_entropy"));
        assert!(output.contains("pattern_repetition"));
        assert!(output.contains("effective_complexity"));
        assert!(output.contains("high_entropy_func"));
        assert!(output.contains("low_entropy_func"));
    }

    #[test]
    fn test_format_project_name_current_dir() {
        let path = Path::new(".");
        let formatted = format_project_name(path);

        // Should return the actual directory name, not "."
        assert_ne!(formatted, ".");
        // Should contain some directory name
        assert!(!formatted.is_empty());
    }

    #[test]
    fn test_format_project_name_relative_path() {
        let path = Path::new("src");
        let formatted = format_project_name(path);

        // Should contain a slash indicating it's showing context
        assert!(formatted.contains('/'));
        // Should end with "src"
        assert!(formatted.ends_with("src"));
    }

    #[test]
    fn test_format_project_name_absolute_path() {
        let path = Path::new("/absolute/path/to/project");
        let formatted = format_project_name(path);

        // Should return the path as-is for absolute paths
        assert_eq!(formatted, "/absolute/path/to/project");
    }

    #[test]
    fn test_format_project_name_nested_relative() {
        let path = Path::new("src/analyzers");
        let formatted = format_project_name(path);

        // Should contain the relative path
        assert!(formatted.contains("src/analyzers"));
        // Should have context from current directory
        assert!(formatted.contains('/'));
    }

    #[test]
    fn test_very_large_numbers() {
        let mut results = create_test_results();

        // Add many items to create high density
        for i in 0..1000 {
            results.technical_debt.items.push(DebtItem {
                id: format!("item-{}", i),
                debt_type: DebtType::Complexity,
                priority: Priority::Medium,
                file: PathBuf::from(format!("test{}.rs", i)),
                line: i,
                column: None,
                message: "Test item".to_string(),
                context: None,
            });
        }

        // Very large complexity numbers
        results.complexity.metrics[0].cyclomatic = 999;
        results.complexity.metrics[0].cognitive = 1500;
        results.complexity.summary.average_complexity = 999.5;
        results.complexity.summary.max_complexity = 1500;

        let mut buffer = Vec::new();
        let mut writer = HtmlWriter::new(&mut buffer);

        writer.write_results(&results).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        // Should handle large numbers without crashing
        assert!(output.contains("1001 items analyzed")); // 1000 + original 1
        assert!(output.contains("cyclomatic"));
        // Debt density should be very high
        assert!(output.contains("Debt Density:"));
    }
}
