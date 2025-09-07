use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDebtMetrics {
    pub path: PathBuf,
    pub total_lines: usize,
    pub function_count: usize,
    pub class_count: usize,
    pub avg_complexity: f64,
    pub max_complexity: u32,
    pub total_complexity: u32,
    pub coverage_percent: f64,
    pub uncovered_lines: usize,
    pub god_object_indicators: GodObjectIndicators,
    pub function_scores: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectIndicators {
    pub methods_count: usize,
    pub fields_count: usize,
    pub responsibilities: usize,
    pub is_god_object: bool,
    pub god_object_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDebtItem {
    pub metrics: FileDebtMetrics,
    pub score: f64,
    pub priority_rank: usize,
    pub recommendation: String,
    pub impact: FileImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileImpact {
    pub complexity_reduction: f64,
    pub maintainability_improvement: f64,
    pub test_effort: f64,
}

impl FileDebtMetrics {
    pub fn calculate_score(&self) -> f64 {
        // Size factor: larger files have higher impact
        let size_factor = (self.total_lines as f64 / 100.0).sqrt();

        // Complexity factor: average and total complexity
        let avg_complexity_factor = (self.avg_complexity / 5.0).min(3.0);
        let total_complexity_factor = (self.total_complexity as f64 / 50.0).sqrt();
        let complexity_factor = avg_complexity_factor * total_complexity_factor;

        // Coverage factor: lower coverage = higher score
        let coverage_gap = 1.0 - self.coverage_percent;
        let coverage_factor = (coverage_gap * 2.0) + 1.0;

        // Function density: too many functions = god object
        let density_factor = if self.function_count > 50 {
            1.0 + ((self.function_count - 50) as f64 * 0.02)
        } else {
            1.0
        };

        // God object multiplier
        let god_object_multiplier = if self.god_object_indicators.is_god_object {
            2.0 + self.god_object_indicators.god_object_score
        } else {
            1.0
        };

        // Aggregate function scores
        let function_score_sum: f64 = self.function_scores.iter().sum();
        let function_factor = (function_score_sum / 10.0).max(1.0);

        // Calculate final score
        size_factor
            * complexity_factor
            * coverage_factor
            * density_factor
            * god_object_multiplier
            * function_factor
    }

    pub fn generate_recommendation(&self) -> String {
        if self.god_object_indicators.is_god_object {
            let module_count = (self.function_count / 10).clamp(3, 8);
            format!(
                "Break into {} modules based on responsibilities. Extract related functions into cohesive units.",
                module_count
            )
        } else if self.total_lines > 500 {
            format!(
                "Extract complex functions, reduce file to <500 lines. Current: {} lines",
                self.total_lines
            )
        } else if self.avg_complexity > 10.0 {
            "Simplify complex functions. Consider extracting helper functions or breaking down logic.".to_string()
        } else if self.coverage_percent < 0.5 {
            format!(
                "Increase test coverage from {:.1}% to at least 80%",
                self.coverage_percent * 100.0
            )
        } else {
            "Refactor for better maintainability and testability".to_string()
        }
    }
}

impl Default for FileDebtMetrics {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            total_lines: 0,
            function_count: 0,
            class_count: 0,
            avg_complexity: 0.0,
            max_complexity: 0,
            total_complexity: 0,
            coverage_percent: 0.0,
            uncovered_lines: 0,
            god_object_indicators: GodObjectIndicators::default(),
            function_scores: Vec::new(),
        }
    }
}

impl Default for GodObjectIndicators {
    fn default() -> Self {
        Self {
            methods_count: 0,
            fields_count: 0,
            responsibilities: 0,
            is_god_object: false,
            god_object_score: 0.0,
        }
    }
}
