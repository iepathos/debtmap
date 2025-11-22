use crate::core::{AnalysisResults, FunctionMetrics};
use crate::io::writers::{HtmlWriter, JsonWriter, MarkdownWriter, TerminalWriter};
use crate::risk::RiskInsight;
use std::io;

#[derive(Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Markdown,
    Terminal,
    Html,
}

pub trait OutputWriter {
    fn write_results(&mut self, results: &AnalysisResults) -> anyhow::Result<()>;
    fn write_risk_insights(&mut self, insights: &RiskInsight) -> anyhow::Result<()>;
}

pub fn create_writer(format: OutputFormat) -> Box<dyn OutputWriter> {
    match format {
        OutputFormat::Json => Box::new(JsonWriter::new(io::stdout())),
        OutputFormat::Markdown => Box::new(MarkdownWriter::new(io::stdout())),
        OutputFormat::Terminal => Box::new(TerminalWriter::default()),
        OutputFormat::Html => Box::new(HtmlWriter::new(io::stdout())),
    }
}

// Helper functions shared by multiple writers
pub fn complexity_status(avg: f64) -> &'static str {
    match avg {
        a if a > 15.0 => "[ERROR] High",
        a if a > 10.0 => "[WARN] Medium",
        a if a > 5.0 => "[INFO] Moderate",
        _ => "[OK] Low",
    }
}

pub fn debt_status(count: usize) -> &'static str {
    match count {
        c if c > 50 => "[ERROR] High",
        c if c > 20 => "[WARN] Medium",
        c if c > 10 => "[INFO] Moderate",
        _ => "[OK] Low",
    }
}

pub fn high_complexity_status(count: usize) -> &'static str {
    match count {
        0 => "[OK] Good",
        1..=5 => "[INFO] Fair",
        _ => "[ERROR] Poor",
    }
}

pub fn debt_score_status(score: u32, threshold: u32) -> &'static str {
    match score {
        s if s > threshold => "[ERROR] High",
        s if s > threshold / 2 => "[WARN] Medium",
        _ => "[OK] Good",
    }
}

pub fn complexity_header_lines() -> Vec<&'static str> {
    vec![
        "## Complexity Analysis",
        "",
        "| Location | Function | Cyclomatic | Cognitive | Recommendation |",
        "|----------|----------|------------|-----------|----------------|",
    ]
}

pub fn build_summary_rows(
    results: &AnalysisResults,
    debt_score: u32,
    debt_threshold: u32,
) -> Vec<(&'static str, String, String)> {
    vec![
        (
            "Files Analyzed",
            results.complexity.metrics.len().to_string(),
            "-".to_string(),
        ),
        (
            "Total Functions",
            results.complexity.summary.total_functions.to_string(),
            "-".to_string(),
        ),
        (
            "Average Complexity",
            format!("{:.1}", results.complexity.summary.average_complexity),
            complexity_status(results.complexity.summary.average_complexity).to_string(),
        ),
        (
            "High Complexity Functions",
            results.complexity.summary.high_complexity_count.to_string(),
            high_complexity_status(results.complexity.summary.high_complexity_count).to_string(),
        ),
        (
            "Technical Debt Items",
            results.technical_debt.items.len().to_string(),
            debt_status(results.technical_debt.items.len()).to_string(),
        ),
        (
            "Total Debt Score",
            format!("{debt_score} / {debt_threshold}"),
            debt_score_status(debt_score, debt_threshold).to_string(),
        ),
    ]
}

pub fn get_top_complex_functions(
    metrics: &[FunctionMetrics],
    count: usize,
) -> Vec<&FunctionMetrics> {
    let mut sorted = metrics.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|m| std::cmp::Reverse(m.cyclomatic.max(m.cognitive)));
    sorted.into_iter().take(count).collect()
}

pub fn get_recommendation(func: &FunctionMetrics) -> &'static str {
    match func.cyclomatic.max(func.cognitive) {
        c if c > 20 => "Urgent refactoring needed",
        c if c > 15 => "Refactor recommended",
        c if c > 10 => "Consider simplifying",
        _ => "Acceptable",
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
            debt_type: DebtType::Todo,
            priority: Priority::Medium,
            file: PathBuf::from("test.rs"),
            line: 5,
            column: None,
            message: "TODO: Implement feature".to_string(),
            context: None,
        }];

        let metrics = vec![FunctionMetrics {
            name: "test_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            cyclomatic: 5,
            cognitive: 7,
            nesting: 2,
            length: 25,
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
                    average_complexity: 5.0,
                    max_complexity: 5,
                    high_complexity_count: 0,
                },
            },
            technical_debt: TechnicalDebtReport {
                items,
                by_type: HashMap::new(),
                priorities: vec![Priority::Medium],
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
    fn test_output_json_format() {
        let results = create_test_results();
        let mut buffer = Vec::new();
        let mut writer = crate::io::writers::JsonWriter::new(&mut buffer);
        writer.write_results(&results).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("test_func"));
        assert!(output.contains("TODO: Implement feature"));
    }

    #[test]
    fn test_output_markdown_format() {
        let results = create_test_results();
        let mut buffer = Vec::new();
        let mut writer = crate::io::writers::MarkdownWriter::new(&mut buffer);
        writer.write_results(&results).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("# Debtmap Analysis Report"));
        assert!(output.contains("Executive Summary"));
    }

    #[test]
    fn test_complexity_status_low() {
        assert_eq!(complexity_status(0.0), "[OK] Low");
        assert_eq!(complexity_status(2.5), "[OK] Low");
        assert_eq!(complexity_status(5.0), "[OK] Low");
    }

    #[test]
    fn test_complexity_status_moderate() {
        assert_eq!(complexity_status(5.1), "[INFO] Moderate");
        assert_eq!(complexity_status(7.5), "[INFO] Moderate");
        assert_eq!(complexity_status(10.0), "[INFO] Moderate");
    }

    #[test]
    fn test_complexity_status_medium() {
        assert_eq!(complexity_status(10.1), "[WARN] Medium");
        assert_eq!(complexity_status(12.5), "[WARN] Medium");
        assert_eq!(complexity_status(15.0), "[WARN] Medium");
    }

    #[test]
    fn test_complexity_status_high() {
        assert_eq!(complexity_status(15.1), "[ERROR] High");
        assert_eq!(complexity_status(20.0), "[ERROR] High");
        assert_eq!(complexity_status(100.0), "[ERROR] High");
    }

    #[test]
    fn test_debt_status_low() {
        assert_eq!(debt_status(0), "[OK] Low");
        assert_eq!(debt_status(5), "[OK] Low");
        assert_eq!(debt_status(10), "[OK] Low");
    }

    #[test]
    fn test_debt_status_moderate() {
        assert_eq!(debt_status(11), "[INFO] Moderate");
        assert_eq!(debt_status(15), "[INFO] Moderate");
        assert_eq!(debt_status(20), "[INFO] Moderate");
    }

    #[test]
    fn test_debt_status_medium() {
        assert_eq!(debt_status(21), "[WARN] Medium");
        assert_eq!(debt_status(35), "[WARN] Medium");
        assert_eq!(debt_status(50), "[WARN] Medium");
    }

    #[test]
    fn test_debt_status_high() {
        assert_eq!(debt_status(51), "[ERROR] High");
        assert_eq!(debt_status(100), "[ERROR] High");
        assert_eq!(debt_status(1000), "[ERROR] High");
    }

    #[test]
    fn test_high_complexity_status_good() {
        assert_eq!(high_complexity_status(0), "[OK] Good");
    }

    #[test]
    fn test_high_complexity_status_fair() {
        assert_eq!(high_complexity_status(1), "[INFO] Fair");
        assert_eq!(high_complexity_status(3), "[INFO] Fair");
        assert_eq!(high_complexity_status(5), "[INFO] Fair");
    }

    #[test]
    fn test_high_complexity_status_poor() {
        assert_eq!(high_complexity_status(6), "[ERROR] Poor");
        assert_eq!(high_complexity_status(10), "[ERROR] Poor");
        assert_eq!(high_complexity_status(100), "[ERROR] Poor");
    }

    #[test]
    fn test_debt_score_status_good() {
        assert_eq!(debt_score_status(25, 100), "[OK] Good");
        assert_eq!(debt_score_status(49, 100), "[OK] Good");
        assert_eq!(debt_score_status(0, 100), "[OK] Good");
    }

    #[test]
    fn test_debt_score_status_medium() {
        assert_eq!(debt_score_status(50, 100), "[OK] Good");
        assert_eq!(debt_score_status(51, 100), "[WARN] Medium");
        assert_eq!(debt_score_status(75, 100), "[WARN] Medium");
        assert_eq!(debt_score_status(100, 100), "[WARN] Medium");
    }

    #[test]
    fn test_debt_score_status_high() {
        assert_eq!(debt_score_status(101, 100), "[ERROR] High");
        assert_eq!(debt_score_status(150, 100), "[ERROR] High");
        assert_eq!(debt_score_status(1000, 100), "[ERROR] High");
    }

    #[test]
    fn test_debt_score_status_boundary_conditions() {
        // Test exact boundary values
        assert_eq!(debt_score_status(50, 100), "[OK] Good"); // Exactly half
        assert_eq!(debt_score_status(100, 100), "[WARN] Medium"); // Exactly at threshold
        assert_eq!(debt_score_status(101, 100), "[ERROR] High"); // Just over threshold
    }

    #[test]
    fn test_get_recommendation_acceptable() {
        let func = FunctionMetrics {
            name: "simple_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 5,
            cognitive: 8,
            nesting: 1,
            length: 15,
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
        };
        assert_eq!(get_recommendation(&func), "Acceptable");
    }

    #[test]
    fn test_get_recommendation_consider_simplifying() {
        let func = FunctionMetrics {
            name: "moderate_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 11,
            cognitive: 9,
            nesting: 2,
            length: 30,
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
        };
        assert_eq!(get_recommendation(&func), "Consider simplifying");
    }

    #[test]
    fn test_get_recommendation_refactor_recommended() {
        let func = FunctionMetrics {
            name: "complex_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 16,
            cognitive: 18,
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
        };
        assert_eq!(get_recommendation(&func), "Refactor recommended");
    }

    #[test]
    fn test_get_recommendation_urgent_refactoring() {
        let func = FunctionMetrics {
            name: "very_complex_func".to_string(),
            file: PathBuf::from("test.rs"),
            line: 1,
            cyclomatic: 25,
            cognitive: 30,
            nesting: 4,
            length: 100,
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
        };
        assert_eq!(get_recommendation(&func), "Urgent refactoring needed");
    }

    #[test]
    fn test_get_top_complex_functions() {
        let mut metrics = vec![];

        // Add functions with varying complexity
        for i in 1..=10 {
            metrics.push(FunctionMetrics {
                name: format!("func_{}", i),
                file: PathBuf::from("test.rs"),
                line: i * 10,
                cyclomatic: i as u32 * 2,
                cognitive: i as u32 * 3,
                nesting: 1,
                length: 20,
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
            });
        }

        let top_3 = get_top_complex_functions(&metrics, 3);
        assert_eq!(top_3.len(), 3);

        // Should get functions with highest complexity (cognitive is higher)
        assert_eq!(top_3[0].name, "func_10"); // cognitive: 30
        assert_eq!(top_3[1].name, "func_9"); // cognitive: 27
        assert_eq!(top_3[2].name, "func_8"); // cognitive: 24
    }

    #[test]
    fn test_get_top_complex_functions_empty() {
        let metrics = vec![];
        let top_5 = get_top_complex_functions(&metrics, 5);
        assert_eq!(top_5.len(), 0);
    }

    #[test]
    fn test_get_top_complex_functions_fewer_than_requested() {
        let metrics = vec![
            FunctionMetrics {
                name: "func_1".to_string(),
                file: PathBuf::from("test.rs"),
                line: 1,
                cyclomatic: 5,
                cognitive: 7,
                nesting: 1,
                length: 20,
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
            },
            FunctionMetrics {
                name: "func_2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 10,
                cyclomatic: 10,
                cognitive: 8,
                nesting: 1,
                length: 20,
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
            },
        ];

        let top_5 = get_top_complex_functions(&metrics, 5);
        assert_eq!(top_5.len(), 2); // Only 2 functions available
        assert_eq!(top_5[0].name, "func_2"); // cyclomatic: 10 is higher
        assert_eq!(top_5[1].name, "func_1"); // cyclomatic: 5, cognitive: 7
    }
}
