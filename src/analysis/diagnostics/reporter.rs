use super::{
    generate_detailed_attribution, generate_summary, AnalysisPerformanceMetrics, DetailLevel,
    DetailedAttribution, DiagnosticReport, OutputFormat,
};
use crate::analysis::multi_pass::MultiPassResult;
use serde_json;
use serde_yaml;

/// Diagnostic report generator
pub struct DiagnosticReporter {
    output_format: OutputFormat,
    detail_level: DetailLevel,
}

impl DiagnosticReporter {
    pub fn new(output_format: OutputFormat, detail_level: DetailLevel) -> Self {
        Self {
            output_format,
            detail_level,
        }
    }

    pub fn generate_report(&self, result: &MultiPassResult) -> DiagnosticReport {
        let summary = generate_summary(result);

        let detailed_attribution = if self.detail_level.includes_attribution() {
            generate_detailed_attribution(&result.attribution)
        } else {
            DetailedAttribution {
                logical_breakdown: super::AttributionBreakdown {
                    category: String::new(),
                    total: 0,
                    percentage: 0.0,
                    components: vec![],
                },
                formatting_breakdown: super::AttributionBreakdown {
                    category: String::new(),
                    total: 0,
                    percentage: 0.0,
                    components: vec![],
                },
                pattern_breakdown: super::AttributionBreakdown {
                    category: String::new(),
                    total: 0,
                    percentage: 0.0,
                    components: vec![],
                },
                total_attribution: 0,
                confidence_level: 0.0,
            }
        };

        let recommendations = if self.detail_level.includes_recommendations() {
            result.recommendations.clone()
        } else {
            vec![]
        };

        let performance_metrics = if self.detail_level.includes_performance() {
            result.performance_metrics.as_ref().map(|perf| {
                AnalysisPerformanceMetrics {
                    total_time_ms: perf.total_time_ms,
                    raw_analysis_ms: perf.raw_analysis_time_ms,
                    normalized_analysis_ms: perf.normalized_analysis_time_ms,
                    attribution_ms: perf.attribution_time_ms,
                    reporting_ms: 0, // Not tracked yet
                    memory_used_mb: perf.memory_used_mb,
                }
            })
        } else {
            None
        };

        DiagnosticReport {
            summary,
            detailed_attribution,
            recommendations,
            comparative_analysis: None,
            performance_metrics,
        }
    }

    pub fn format_report(&self, report: &DiagnosticReport) -> String {
        match self.output_format {
            OutputFormat::Json => self.format_json(report),
            OutputFormat::Yaml => self.format_yaml(report),
            OutputFormat::Markdown => self.format_markdown(report),
            OutputFormat::Html => self.format_html(report),
            OutputFormat::Text => self.format_text(report),
        }
    }

    fn format_json(&self, report: &DiagnosticReport) -> String {
        serde_json::to_string_pretty(report)
            .unwrap_or_else(|e| format!("Failed to serialize report to JSON: {}", e))
    }

    fn format_yaml(&self, report: &DiagnosticReport) -> String {
        serde_yaml::to_string(report)
            .unwrap_or_else(|e| format!("Failed to serialize report to YAML: {}", e))
    }

    fn format_markdown(&self, report: &DiagnosticReport) -> String {
        let mut output = String::new();

        // Title
        output.push_str("# Multi-Pass Complexity Analysis Report\n\n");

        // Summary section
        output.push_str("## Summary\n\n");
        output.push_str(&format!(
            "- **Raw Complexity**: {}\n",
            report.summary.raw_complexity
        ));
        output.push_str(&format!(
            "- **Normalized Complexity**: {}\n",
            report.summary.normalized_complexity
        ));
        output.push_str(&format!(
            "- **Complexity Reduction**: {:.1}%\n",
            report.summary.complexity_reduction
        ));
        output.push_str(&format!(
            "- **Formatting Impact**: {:.1}%\n",
            report.summary.formatting_impact
        ));
        output.push_str(&format!(
            "- **Pattern Recognition**: {:.1}%\n\n",
            report.summary.pattern_recognition
        ));

        // Key findings
        if !report.summary.key_findings.is_empty() {
            output.push_str("### Key Findings\n\n");
            for finding in &report.summary.key_findings {
                output.push_str(&format!("- {}\n", finding));
            }
            output.push_str("\n");
        }

        // Attribution details
        if self.detail_level.includes_attribution() {
            output.push_str("## Attribution Analysis\n\n");

            // Logical complexity
            output.push_str(&format!(
                "### Logical Structure ({:.1}%)\n",
                report.detailed_attribution.logical_breakdown.percentage
            ));
            output.push_str(&format!(
                "Total contribution: {}\n\n",
                report.detailed_attribution.logical_breakdown.total
            ));

            if !report
                .detailed_attribution
                .logical_breakdown
                .components
                .is_empty()
            {
                output.push_str("**Components:**\n");
                for component in &report.detailed_attribution.logical_breakdown.components {
                    output.push_str(&format!(
                        "- {} ({}): {} at {}\n",
                        component.name,
                        component.contribution,
                        component.suggestions.first().unwrap_or(&String::new()),
                        component.location
                    ));
                }
                output.push_str("\n");
            }

            // Formatting artifacts
            output.push_str(&format!(
                "### Formatting Artifacts ({:.1}%)\n",
                report.detailed_attribution.formatting_breakdown.percentage
            ));
            output.push_str(&format!(
                "Total contribution: {}\n\n",
                report.detailed_attribution.formatting_breakdown.total
            ));

            // Pattern recognition
            output.push_str(&format!(
                "### Pattern Recognition ({:.1}%)\n",
                report.detailed_attribution.pattern_breakdown.percentage
            ));
            output.push_str(&format!(
                "Total contribution: {}\n",
                report.detailed_attribution.pattern_breakdown.total
            ));
            output.push_str(&format!(
                "Confidence level: {:.1}%\n\n",
                report.detailed_attribution.confidence_level * 100.0
            ));
        }

        // Recommendations
        if !report.recommendations.is_empty() {
            output.push_str("## Recommendations\n\n");
            for (i, rec) in report.recommendations.iter().enumerate() {
                output.push_str(&format!("### {}. {}\n", i + 1, rec.title));
                output.push_str(&format!("{}\n", rec.description));
                output.push_str(&format!("**Priority**: {:?}\n", rec.priority));
                output.push_str(&format!("**Category**: {:?}\n", rec.category));
                if rec.estimated_impact > 0 {
                    output.push_str(&format!(
                        "**Estimated Impact**: -{}\n",
                        rec.estimated_impact
                    ));
                }
                if !rec.suggested_actions.is_empty() {
                    output.push_str("**Actions**:\n");
                    for action in &rec.suggested_actions {
                        output.push_str(&format!("- {}\n", action));
                    }
                }
                output.push_str("\n");
            }
        }

        // Performance metrics
        if let Some(perf) = &report.performance_metrics {
            output.push_str("## Performance Metrics\n\n");
            output.push_str(&format!("- Total time: {}ms\n", perf.total_time_ms));
            output.push_str(&format!("- Raw analysis: {}ms\n", perf.raw_analysis_ms));
            output.push_str(&format!(
                "- Normalized analysis: {}ms\n",
                perf.normalized_analysis_ms
            ));
            output.push_str(&format!("- Attribution: {}ms\n", perf.attribution_ms));
            output.push_str(&format!("- Report generation: {}ms\n", perf.reporting_ms));
            output.push_str(&format!("- Memory used: {:.1}MB\n", perf.memory_used_mb));
        }

        output
    }

    fn format_html(&self, report: &DiagnosticReport) -> String {
        let mut output = String::new();

        output.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        output.push_str("<title>Multi-Pass Complexity Analysis Report</title>\n");
        output.push_str("<style>\n");
        output.push_str("body { font-family: Arial, sans-serif; margin: 20px; }\n");
        output.push_str("h1 { color: #333; }\n");
        output
            .push_str("h2 { color: #666; border-bottom: 1px solid #ddd; padding-bottom: 5px; }\n");
        output.push_str("h3 { color: #888; }\n");
        output.push_str(".metric { margin: 10px 0; }\n");
        output.push_str(".metric-label { font-weight: bold; }\n");
        output.push_str(".recommendation { background: #f5f5f5; padding: 10px; margin: 10px 0; border-radius: 5px; }\n");
        output.push_str("</style>\n");
        output.push_str("</head>\n<body>\n");

        output.push_str("<h1>Multi-Pass Complexity Analysis Report</h1>\n");

        // Summary
        output.push_str("<h2>Summary</h2>\n");
        output.push_str(&format!(
            "<div class='metric'><span class='metric-label'>Raw Complexity:</span> {}</div>\n",
            report.summary.raw_complexity
        ));
        output.push_str(&format!(
            "<div class='metric'><span class='metric-label'>Normalized Complexity:</span> {}</div>\n",
            report.summary.normalized_complexity
        ));
        output.push_str(&format!(
            "<div class='metric'><span class='metric-label'>Complexity Reduction:</span> {:.1}%</div>\n",
            report.summary.complexity_reduction
        ));

        // Key findings
        if !report.summary.key_findings.is_empty() {
            output.push_str("<h3>Key Findings</h3>\n<ul>\n");
            for finding in &report.summary.key_findings {
                output.push_str(&format!("<li>{}</li>\n", finding));
            }
            output.push_str("</ul>\n");
        }

        // Recommendations
        if !report.recommendations.is_empty() {
            output.push_str("<h2>Recommendations</h2>\n");
            for rec in &report.recommendations {
                output.push_str("<div class='recommendation'>\n");
                output.push_str(&format!("<h3>{}</h3>\n", rec.title));
                output.push_str(&format!("<p>{}</p>\n", rec.description));
                output.push_str("</div>\n");
            }
        }

        output.push_str("</body>\n</html>");
        output
    }

    fn format_text(&self, report: &DiagnosticReport) -> String {
        let mut output = String::new();

        output.push_str("MULTI-PASS COMPLEXITY ANALYSIS REPORT\n");
        output.push_str("=====================================\n\n");

        output.push_str("SUMMARY\n");
        output.push_str("-------\n");
        output.push_str(&format!(
            "Raw Complexity: {}\n",
            report.summary.raw_complexity
        ));
        output.push_str(&format!(
            "Normalized Complexity: {}\n",
            report.summary.normalized_complexity
        ));
        output.push_str(&format!(
            "Complexity Reduction: {:.1}%\n",
            report.summary.complexity_reduction
        ));
        output.push_str(&format!(
            "Formatting Impact: {:.1}%\n",
            report.summary.formatting_impact
        ));
        output.push_str(&format!(
            "Pattern Recognition: {:.1}%\n\n",
            report.summary.pattern_recognition
        ));

        if !report.summary.key_findings.is_empty() {
            output.push_str("KEY FINDINGS\n");
            output.push_str("------------\n");
            for finding in &report.summary.key_findings {
                output.push_str(&format!("* {}\n", finding));
            }
            output.push_str("\n");
        }

        if !report.recommendations.is_empty() {
            output.push_str("RECOMMENDATIONS\n");
            output.push_str("---------------\n");
            for (i, rec) in report.recommendations.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, rec.title));
                output.push_str(&format!("   {}\n", rec.description));
                output.push_str(&format!("   Priority: {:?}\n", rec.priority));
                output.push_str(&format!("   Category: {:?}\n\n", rec.category));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::attribution::{AttributedComplexity, ComplexityAttribution};
    use crate::analysis::multi_pass::{AnalysisType, ComplexityResult};

    #[test]
    fn test_reporter_new() {
        let reporter = DiagnosticReporter::new(OutputFormat::Json, DetailLevel::Standard);
        assert_eq!(reporter.output_format, OutputFormat::Json);
        assert_eq!(reporter.detail_level, DetailLevel::Standard);
    }

    #[test]
    fn test_generate_report_summary_level() {
        let reporter = DiagnosticReporter::new(OutputFormat::Json, DetailLevel::Summary);
        let result = create_test_result();
        let report = reporter.generate_report(&result);

        assert_eq!(report.summary.raw_complexity, 20);
        assert!(report.recommendations.is_empty());
        assert!(report.performance_metrics.is_none());
    }

    #[test]
    fn test_generate_report_debug_level() {
        let reporter = DiagnosticReporter::new(OutputFormat::Json, DetailLevel::Debug);
        let result = create_test_result();
        let report = reporter.generate_report(&result);

        assert!(!report.recommendations.is_empty());
        assert!(report.performance_metrics.is_some());
    }

    #[test]
    fn test_format_json() {
        let reporter = DiagnosticReporter::new(OutputFormat::Json, DetailLevel::Summary);
        let result = create_test_result();
        let report = reporter.generate_report(&result);
        let formatted = reporter.format_report(&report);

        assert!(formatted.contains("\"raw_complexity\""));
        assert!(formatted.contains("\"normalized_complexity\""));
    }

    #[test]
    fn test_format_markdown() {
        let reporter = DiagnosticReporter::new(OutputFormat::Markdown, DetailLevel::Standard);
        let result = create_test_result();
        let report = reporter.generate_report(&result);
        let formatted = reporter.format_report(&report);

        assert!(formatted.contains("# Multi-Pass Complexity Analysis Report"));
        assert!(formatted.contains("## Summary"));
    }

    #[test]
    fn test_format_text() {
        let reporter = DiagnosticReporter::new(OutputFormat::Text, DetailLevel::Standard);
        let result = create_test_result();
        let report = reporter.generate_report(&result);
        let formatted = reporter.format_report(&report);

        assert!(formatted.contains("MULTI-PASS COMPLEXITY ANALYSIS REPORT"));
        assert!(formatted.contains("SUMMARY"));
    }

    fn create_test_result() -> MultiPassResult {
        use crate::analysis::multi_pass::{
            ComplexityInsight, ComplexityRecommendation, ImpactLevel, InsightType,
            RecommendationCategory, RecommendationPriority,
        };

        MultiPassResult {
            raw_complexity: ComplexityResult {
                total_complexity: 20,
                cognitive_complexity: 15,
                functions: vec![],
                analysis_type: AnalysisType::Raw,
            },
            normalized_complexity: ComplexityResult {
                total_complexity: 15,
                cognitive_complexity: 12,
                functions: vec![],
                analysis_type: AnalysisType::Normalized,
            },
            attribution: ComplexityAttribution {
                logical_complexity: AttributedComplexity {
                    total: 12,
                    breakdown: vec![],
                    confidence: 0.9,
                },
                formatting_artifacts: AttributedComplexity {
                    total: 5,
                    breakdown: vec![],
                    confidence: 0.8,
                },
                pattern_complexity: AttributedComplexity {
                    total: 3,
                    breakdown: vec![],
                    confidence: 0.7,
                },
                source_mappings: vec![],
            },
            insights: vec![ComplexityInsight {
                insight_type: InsightType::FormattingImpact,
                description: "Formatting contributes significantly".to_string(),
                impact_level: ImpactLevel::Medium,
                actionable_steps: vec![],
            }],
            recommendations: vec![ComplexityRecommendation {
                priority: RecommendationPriority::High,
                category: RecommendationCategory::Refactoring,
                title: "Simplify complex function".to_string(),
                description: "Break down into smaller functions".to_string(),
                estimated_impact: 5,
                suggested_actions: vec!["Extract helper functions".to_string()],
            }],
            performance_metrics: Some(crate::analysis::multi_pass::AnalysisPerformanceMetrics {
                raw_analysis_time_ms: 100,
                normalized_analysis_time_ms: 80,
                attribution_time_ms: 50,
                total_time_ms: 230,
                memory_used_mb: 15.5,
            }),
        }
    }
}
