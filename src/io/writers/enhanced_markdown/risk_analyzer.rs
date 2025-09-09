use crate::core::{AnalysisResults, FunctionMetrics};
use crate::risk::RiskDistribution;
use std::path::Path;

pub struct ModuleInfo {
    pub name: String,
    pub complexity: f64,
    pub coverage: f64,
    pub risk: f64,
}

/// Calculate health score based on coverage and complexity
pub fn calculate_health_score(results: &AnalysisResults, coverage_percentage: Option<f64>) -> u32 {
    let coverage_score = coverage_percentage.unwrap_or(0.0);

    let avg_complexity = if results.complexity.metrics.is_empty() {
        0.0
    } else {
        let total: u32 = results
            .complexity
            .metrics
            .iter()
            .map(|m| m.cyclomatic)
            .sum();
        total as f64 / results.complexity.metrics.len() as f64
    };

    let complexity_score = match avg_complexity {
        x if x <= 5.0 => 100.0,
        x if x <= 10.0 => 80.0,
        x if x <= 15.0 => 60.0,
        x if x <= 20.0 => 40.0,
        _ => 20.0,
    };

    let debt_score = match results.technical_debt.items.len() {
        0..=5 => 100.0,
        6..=15 => 80.0,
        16..=30 => 60.0,
        31..=50 => 40.0,
        _ => 20.0,
    };

    ((coverage_score * 0.4 + complexity_score * 0.3 + debt_score * 0.3) as u32).min(100)
}

/// Get top risk modules (simplified implementation)
pub fn get_top_risk_modules(_results: &AnalysisResults, limit: usize) -> Vec<ModuleInfo> {
    // Simplified module risk calculation
    // In a real implementation, would aggregate by module
    let mut modules = vec![
        ModuleInfo {
            name: "core/auth".to_string(),
            complexity: 15.0,
            coverage: 0.3,
            risk: 8.5,
        },
        ModuleInfo {
            name: "api/handlers".to_string(),
            complexity: 10.0,
            coverage: 0.5,
            risk: 6.0,
        },
        ModuleInfo {
            name: "utils/helpers".to_string(),
            complexity: 3.0,
            coverage: 0.8,
            risk: 2.0,
        },
    ];

    modules.truncate(limit);
    modules
}

/// Get critical risk functions based on complexity and coverage
pub fn get_critical_risk_functions(
    metrics: &[FunctionMetrics],
    limit: usize,
) -> Vec<&FunctionMetrics> {
    let mut risk_functions: Vec<&FunctionMetrics> = metrics
        .iter()
        .filter(|m| m.cyclomatic > 10 || m.name.contains("critical"))
        .collect();

    risk_functions.sort_by(|a, b| b.cyclomatic.cmp(&a.cyclomatic));
    risk_functions.truncate(limit);
    risk_functions
}

/// Estimate complexity reduction potential for a function
pub fn estimate_complexity_reduction(func: &FunctionMetrics) -> f64 {
    match func.cyclomatic {
        0..=5 => 0.0,
        6..=10 => 20.0,
        11..=20 => 40.0,
        21..=30 => 60.0,
        _ => 80.0,
    }
}

/// Extract module name from file path
pub fn get_module_from_path(path: &Path) -> String {
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Extract module from function name (simplified)
pub fn get_module_from_function(_function: &str) -> String {
    "module".to_string() // Simplified implementation
}

/// Analyze risk distribution data
pub fn analyze_risk_distribution(distribution: &RiskDistribution) -> String {
    let total = distribution.low_count
        + distribution.medium_count
        + distribution.high_count
        + distribution.critical_count;

    if total == 0 {
        return "No risk data available".to_string();
    }

    let critical_percentage = (distribution.critical_count as f64 / total as f64) * 100.0;
    let high_percentage = (distribution.high_count as f64 / total as f64) * 100.0;

    format!(
        "Risk profile: {:.1}% critical, {:.1}% high risk items",
        critical_percentage, high_percentage
    )
}
