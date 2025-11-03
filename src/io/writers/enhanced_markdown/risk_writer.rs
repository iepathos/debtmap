use crate::core::{AnalysisResults, FunctionMetrics};
use crate::risk::RiskDistribution;
use anyhow::Result;
use std::io::Write;

use super::formatters::*;

/// Write risk distribution table
pub fn write_risk_distribution<W: Write>(
    writer: &mut W,
    distribution: &RiskDistribution,
) -> Result<()> {
    writeln!(writer, "### Risk Distribution\n")?;

    let total = distribution.low_count
        + distribution.medium_count
        + distribution.high_count
        + distribution.critical_count;

    if total > 0 {
        writeln!(writer, "| Risk Level | Count | Percentage |")?;
        writeln!(writer, "|------------|-------|------------|")?;
        writeln!(
            writer,
            "| [OK] Low | {} | {:.1}% |",
            distribution.low_count,
            (distribution.low_count as f64 / total as f64) * 100.0
        )?;
        writeln!(
            writer,
            "| [WARN] Medium | {} | {:.1}% |",
            distribution.medium_count,
            (distribution.medium_count as f64 / total as f64) * 100.0
        )?;
        writeln!(
            writer,
            "| [WARN] High | {} | {:.1}% |",
            distribution.high_count,
            (distribution.high_count as f64 / total as f64) * 100.0
        )?;
        writeln!(
            writer,
            "| [ERROR] Critical | {} | {:.1}% |",
            distribution.critical_count,
            (distribution.critical_count as f64 / total as f64) * 100.0
        )?;
    }

    writeln!(writer)?;
    Ok(())
}

/// Get critical risk functions sorted by complexity
pub fn get_critical_risk_functions(
    metrics: &[FunctionMetrics],
    limit: usize,
) -> Vec<&FunctionMetrics> {
    let mut critical: Vec<_> = metrics.iter().filter(|m| m.cyclomatic >= 10).collect();

    critical.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic));
    critical.into_iter().take(limit).collect()
}

/// Write critical risk functions table
pub fn write_critical_risks<W: Write>(writer: &mut W, results: &AnalysisResults) -> Result<()> {
    writeln!(writer, "### Critical Risk Functions\n")?;

    let critical_functions = get_critical_risk_functions(&results.complexity.metrics, 5);

    if !critical_functions.is_empty() {
        writeln!(writer, "| Function | Complexity | Priority |")?;
        writeln!(writer, "|----------|------------|----------|")?;

        for func in critical_functions {
            writeln!(
                writer,
                "| {} | {} | {} |",
                func.name,
                func.cyclomatic,
                get_priority_label(0)
            )?;
        }
    }

    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::RiskDistribution;
    use std::io::Cursor;

    #[test]
    fn test_write_risk_distribution() {
        let distribution = RiskDistribution {
            low_count: 10,
            medium_count: 5,
            high_count: 3,
            critical_count: 2,
            well_tested_count: 8,
            total_functions: 20,
        };

        let mut buffer = Cursor::new(Vec::new());
        let result = write_risk_distribution(&mut buffer, &distribution);
        assert!(result.is_ok());

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("Risk Distribution"));
        assert!(output.contains("[OK] Low"));
        assert!(output.contains("[WARN] Medium"));
        assert!(output.contains("[WARN] High"));
        assert!(output.contains("[ERROR] Critical"));
    }

    #[test]
    fn test_write_risk_distribution_percentages() {
        let distribution = RiskDistribution {
            low_count: 80,
            medium_count: 15,
            high_count: 4,
            critical_count: 1,
            well_tested_count: 70,
            total_functions: 100,
        };

        let mut buffer = Cursor::new(Vec::new());
        write_risk_distribution(&mut buffer, &distribution).unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        assert!(output.contains("80.0%"));
        assert!(output.contains("15.0%"));
        assert!(output.contains("4.0%"));
        assert!(output.contains("1.0%"));
    }

    #[test]
    fn test_get_critical_risk_functions() {
        use crate::core::FunctionMetrics;
        use std::path::PathBuf;

        let metrics = vec![
            FunctionMetrics {
                file: PathBuf::from("test1.rs"),
                name: "low_complexity".to_string(),
                line: 10,
                cyclomatic: 5,
                cognitive: 3,
                nesting: 1,
                length: 10,
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
                purity_level: None,
            },
            FunctionMetrics {
                file: PathBuf::from("test2.rs"),
                name: "high_complexity".to_string(),
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
                purity_level: None,
            },
            FunctionMetrics {
                file: PathBuf::from("test3.rs"),
                name: "critical_complexity".to_string(),
                line: 30,
                cyclomatic: 25,
                cognitive: 30,
                nesting: 5,
                length: 100,
                is_test: false,
                is_pure: Some(false),
                purity_confidence: Some(0.6),
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
                purity_level: None,
            },
        ];

        let critical = get_critical_risk_functions(&metrics, 2);
        assert_eq!(critical.len(), 2);
        assert_eq!(critical[0].name, "critical_complexity");
        assert_eq!(critical[1].name, "high_complexity");
    }
}
