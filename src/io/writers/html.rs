use crate::core::AnalysisResults;
use crate::io::output::OutputWriter;
use crate::output::unified::convert_to_unified_format;
use crate::priority::UnifiedAnalysis;
use crate::risk::RiskInsight;
use anyhow::Result;
use html_escape::encode_text;
use serde_json;
use std::io::Write;

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
                &results.project_path.display().to_string(),
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
}
