use crate::core::AnalysisResults;
use crate::priority::UnifiedAnalysis;
use anyhow::Result;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use super::complexity_analyzer::*;
use super::formatters::*;
use super::risk_analyzer::*;
use super::statistics::calculate_complexity_distribution;

/// Write complexity distribution visualization
pub fn write_complexity_distribution<W: Write>(
    writer: &mut W,
    results: &AnalysisResults,
) -> Result<()> {
    writeln!(writer, "### Complexity Distribution\n")?;

    let distribution = calculate_complexity_distribution(results);

    writeln!(writer, "```")?;
    for (label, percentage) in distribution {
        let bar_length = (percentage / 2.0) as usize;
        let bar = "█".repeat(bar_length);
        writeln!(writer, "{:15} {} {:.1}%", label, bar, percentage)?;
    }
    writeln!(writer, "```\n")?;

    Ok(())
}

/// Write risk heat map visualization
pub fn write_risk_heat_map<W: Write>(writer: &mut W, results: &AnalysisResults) -> Result<()> {
    writeln!(writer, "### Risk Heat Map\n")?;

    let modules = get_top_risk_modules(results, 5);

    writeln!(writer, "| Module | Complexity | Coverage | Risk Level |")?;
    writeln!(writer, "|--------|------------|----------|------------|")?;

    for module in modules {
        writeln!(
            writer,
            "| {} | {} | {} | {} |",
            module.name,
            get_complexity_indicator(module.complexity),
            get_coverage_indicator(module.coverage),
            get_risk_indicator(module.risk)
        )?;
    }

    writeln!(writer)?;
    Ok(())
}

/// Write module dependency graph
pub fn write_dependency_graph<W: Write>(writer: &mut W, analysis: &UnifiedAnalysis) -> Result<()> {
    writeln!(writer, "### Module Dependencies\n")?;

    let items: Vec<_> = analysis.items.iter().cloned().collect();
    let deps = extract_module_dependencies(&items);

    writeln!(writer, "```mermaid")?;
    writeln!(writer, "graph LR")?;

    for (module, dependencies) in deps.iter().take(10) {
        for dep in dependencies {
            writeln!(writer, "    {} --> {}", module, dep)?;
        }
    }

    writeln!(writer, "```\n")?;
    Ok(())
}

/// Write complexity trends chart
pub fn write_distribution_charts<W: Write>(
    writer: &mut W,
    results: &AnalysisResults,
) -> Result<()> {
    writeln!(writer, "### Complexity Trends\n")?;

    let sample_values: Vec<u32> = results
        .complexity
        .metrics
        .iter()
        .take(20)
        .map(|m| m.cyclomatic)
        .collect();

    if !sample_values.is_empty() {
        writeln!(
            writer,
            "Recent complexity trend: {}\n",
            create_sparkline(&sample_values)
        )?;
    }

    Ok(())
}

/// Write complexity hotspots table
pub fn write_complexity_hotspots<W: Write>(
    writer: &mut W,
    results: &AnalysisResults,
) -> Result<()> {
    writeln!(writer, "### Complexity Hotspots\n")?;

    let mut file_complexities: HashMap<&Path, Vec<u32>> = HashMap::new();

    for metric in &results.complexity.metrics {
        file_complexities
            .entry(&metric.file)
            .or_default()
            .push(metric.cyclomatic);
    }

    let mut hotspots: Vec<_> = file_complexities
        .into_iter()
        .map(|(path, complexities)| {
            let avg = complexities.iter().sum::<u32>() as f64 / complexities.len() as f64;
            let max = *complexities.iter().max().unwrap_or(&0);
            (path, avg, max, complexities.len())
        })
        .collect();

    hotspots.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    writeln!(
        writer,
        "| File | Avg Complexity | Max | Functions | Trend |"
    )?;
    writeln!(
        writer,
        "|------|----------------|-----|-----------|-------|"
    )?;

    for (path, avg, max, count) in hotspots.iter().take(10) {
        writeln!(
            writer,
            "| {} | {:.1} | {} | {} | {} |",
            path.display(),
            avg,
            max,
            count,
            get_trend_indicator(0.0)
        )?;
    }

    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{ComplexityReport, ComplexitySummary, FunctionMetrics};
    use std::io::Cursor;
    use std::path::PathBuf;

    fn create_test_results() -> AnalysisResults {
        use crate::core::{DependencyReport, TechnicalDebtReport};
        use chrono::Utc;

        let metrics = vec![
            FunctionMetrics {
                file: PathBuf::from("test1.rs"),
                name: "func1".to_string(),
                line: 10,
                cyclomatic: 5,
                cognitive: 7,
                nesting: 2,
                length: 20,
                is_test: false,
                is_pure: Some(true),
                purity_confidence: Some(0.9),
                purity_reason: None,
                call_dependencies: None,
                visibility: Some("pub".to_string()),
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
            },
            FunctionMetrics {
                file: PathBuf::from("test2.rs"),
                name: "func2".to_string(),
                line: 20,
                cyclomatic: 15,
                cognitive: 20,
                nesting: 4,
                length: 50,
                is_test: false,
                is_pure: Some(false),
                purity_confidence: Some(0.7),
                purity_reason: None,
                call_dependencies: None,
                visibility: Some("pub".to_string()),
                is_trait_method: false,
                in_test_module: false,
                entropy_score: None,
                detected_patterns: None,
                upstream_callers: None,
                downstream_callees: None,
                mapping_pattern_result: None,
                adjusted_complexity: None,
                composition_metrics: None,
                language_specific: None,
            },
        ];

        AnalysisResults {
            project_path: PathBuf::from("/test"),
            timestamp: Utc::now(),
            complexity: ComplexityReport {
                summary: ComplexitySummary {
                    total_functions: 2,
                    average_complexity: 10.0,
                    high_complexity_count: 1,
                    max_complexity: 15,
                },
                metrics,
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
        }
    }

    #[test]
    fn test_write_complexity_distribution() {
        let results = create_test_results();
        let mut buffer = Cursor::new(Vec::new());

        let result = write_complexity_distribution(&mut buffer, &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Complexity Distribution"));
        assert!(output.contains("```"));
    }

    #[test]
    fn test_write_risk_heat_map() {
        let results = create_test_results();
        let mut buffer = Cursor::new(Vec::new());

        let result = write_risk_heat_map(&mut buffer, &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Risk Heat Map"));
        assert!(output.contains("| Module | Complexity | Coverage | Risk Level |"));
    }

    #[test]
    fn test_write_complexity_hotspots() {
        let results = create_test_results();
        let mut buffer = Cursor::new(Vec::new());

        let result = write_complexity_hotspots(&mut buffer, &results);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Complexity Hotspots"));
        assert!(output.contains("test1.rs"));
        assert!(output.contains("test2.rs"));
    }
}
