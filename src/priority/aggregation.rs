use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::priority::UnifiedDebtItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAggregateScore {
    pub file_path: PathBuf,
    pub total_score: f64,
    pub function_count: usize,
    pub problematic_functions: usize,            // Score > threshold
    pub top_function_scores: Vec<(String, f64)>, // Top 5
    pub aggregate_score: f64,
    pub aggregation_method: AggregationMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationMethod {
    Sum,
    WeightedSum,
    LogarithmicSum,
    MaxPlusAverage,
}

impl FileAggregateScore {
    pub fn calculate_aggregate(&mut self) {
        self.aggregate_score = match self.aggregation_method {
            AggregationMethod::Sum => {
                // Simple sum with count scaling
                self.total_score * (1.0 + (self.function_count as f64).ln() / 10.0)
            }

            AggregationMethod::WeightedSum => {
                // Weight by problem density
                let density = if self.function_count > 0 {
                    self.problematic_functions as f64 / self.function_count as f64
                } else {
                    0.0
                };
                self.total_score * (1.0 + density) * (self.function_count as f64).sqrt() / 10.0
            }

            AggregationMethod::LogarithmicSum => {
                // Logarithmic scaling to prevent runaway scores
                self.total_score * (1.0 + (self.function_count as f64).ln())
            }

            AggregationMethod::MaxPlusAverage => {
                // Max function score plus average of others
                let max_score = self
                    .top_function_scores
                    .first()
                    .map(|(_, s)| *s)
                    .unwrap_or(0.0);
                let avg_score = if self.function_count > 0 {
                    self.total_score / self.function_count as f64
                } else {
                    0.0
                };
                max_score + (avg_score * self.function_count as f64 * 0.5)
            }
        };
    }

    pub fn new(file_path: PathBuf, aggregation_method: AggregationMethod) -> Self {
        Self {
            file_path,
            total_score: 0.0,
            function_count: 0,
            problematic_functions: 0,
            top_function_scores: Vec::new(),
            aggregate_score: 0.0,
            aggregation_method,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AggregationPipeline {
    function_scores: HashMap<PathBuf, Vec<FunctionScore>>,
    file_aggregates: HashMap<PathBuf, FileAggregateScore>,
    config: AggregationConfig,
}

#[derive(Debug, Clone)]
pub struct FunctionScore {
    pub name: String,
    pub score: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AggregationConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_method")]
    pub method: AggregationMethod,
    #[serde(default = "default_problem_threshold")]
    pub problem_threshold: f64, // Functions scoring above this are "problematic"
    #[serde(default = "default_min_functions")]
    pub min_functions_for_aggregation: usize, // Don't aggregate files with few functions
    #[serde(default = "default_display_top_functions")]
    pub display_top_functions: usize, // How many top functions to show
}

fn default_enabled() -> bool {
    true
}

fn default_method() -> AggregationMethod {
    AggregationMethod::WeightedSum
}

fn default_problem_threshold() -> f64 {
    5.0
}

fn default_min_functions() -> usize {
    2
}

fn default_display_top_functions() -> usize {
    5
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            method: default_method(),
            problem_threshold: default_problem_threshold(),
            min_functions_for_aggregation: default_min_functions(),
            display_top_functions: default_display_top_functions(),
        }
    }
}

impl AggregationPipeline {
    pub fn new(config: AggregationConfig) -> Self {
        Self {
            function_scores: HashMap::new(),
            file_aggregates: HashMap::new(),
            config,
        }
    }

    pub fn add_function_score(&mut self, file_path: PathBuf, name: String, score: f64) {
        let entry = self.function_scores.entry(file_path).or_default();
        entry.push(FunctionScore { name, score });
    }

    pub fn aggregate_file_scores(&mut self) -> Vec<FileAggregateScore> {
        for (path, functions) in &self.function_scores {
            // Skip files with too few functions if configured
            if functions.len() < self.config.min_functions_for_aggregation {
                continue;
            }

            let total_score: f64 = functions.iter().map(|f| f.score).sum();
            let problematic = functions
                .iter()
                .filter(|f| f.score > self.config.problem_threshold)
                .count();

            let mut top_functions: Vec<_> = functions
                .iter()
                .map(|f| (f.name.clone(), f.score))
                .collect();
            top_functions
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            top_functions.truncate(self.config.display_top_functions);

            let mut aggregate = FileAggregateScore {
                file_path: path.clone(),
                total_score,
                function_count: functions.len(),
                problematic_functions: problematic,
                top_function_scores: top_functions,
                aggregate_score: 0.0,
                aggregation_method: self.config.method.clone(),
            };

            aggregate.calculate_aggregate();
            self.file_aggregates.insert(path.clone(), aggregate);
        }

        let mut results: Vec<_> = self.file_aggregates.values().cloned().collect();
        results.sort_by(|a, b| {
            b.aggregate_score
                .partial_cmp(&a.aggregate_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results
    }

    pub fn aggregate_from_debt_items(
        items: &[UnifiedDebtItem],
        config: &AggregationConfig,
    ) -> Vec<FileAggregateScore> {
        let mut pipeline = AggregationPipeline::new(config.clone());

        for item in items {
            pipeline.add_function_score(
                item.location.file.clone(),
                item.location.function.clone(),
                item.unified_score.final_score,
            );
        }

        pipeline.aggregate_file_scores()
    }

    pub fn aggregate_from_metrics(
        metrics: &[crate::core::FunctionMetrics],
        config: &AggregationConfig,
    ) -> Vec<FileAggregateScore> {
        let mut pipeline = AggregationPipeline::new(config.clone());

        for metric in metrics {
            // Calculate a basic score from the metric's complexity
            let score = (metric.cyclomatic as f64 * 0.5) + (metric.cognitive as f64 * 0.5);
            pipeline.add_function_score(metric.file.clone(), metric.name.clone(), score);
        }

        pipeline.aggregate_file_scores()
    }

    pub fn update_file_aggregate(&mut self, path: &PathBuf) {
        if let Some(functions) = self.function_scores.get(path) {
            if functions.len() < self.config.min_functions_for_aggregation {
                self.file_aggregates.remove(path);
                return;
            }

            let total_score: f64 = functions.iter().map(|f| f.score).sum();
            let problematic = functions
                .iter()
                .filter(|f| f.score > self.config.problem_threshold)
                .count();

            let mut top_functions: Vec<_> = functions
                .iter()
                .map(|f| (f.name.clone(), f.score))
                .collect();
            top_functions
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            top_functions.truncate(self.config.display_top_functions);

            let mut aggregate = FileAggregateScore {
                file_path: path.clone(),
                total_score,
                function_count: functions.len(),
                problematic_functions: problematic,
                top_function_scores: top_functions,
                aggregate_score: 0.0,
                aggregation_method: self.config.method.clone(),
            };

            aggregate.calculate_aggregate();
            self.file_aggregates.insert(path.clone(), aggregate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregation_methods() {
        let mut aggregate = FileAggregateScore {
            file_path: PathBuf::from("test.rs"),
            total_score: 100.0,
            function_count: 10,
            problematic_functions: 5,
            top_function_scores: vec![
                ("func1".to_string(), 20.0),
                ("func2".to_string(), 15.0),
                ("func3".to_string(), 10.0),
            ],
            aggregate_score: 0.0,
            aggregation_method: AggregationMethod::Sum,
        };

        // Test Sum method
        aggregate.calculate_aggregate();
        assert!(aggregate.aggregate_score > 100.0);

        // Test WeightedSum method
        aggregate.aggregation_method = AggregationMethod::WeightedSum;
        aggregate.calculate_aggregate();
        assert!(aggregate.aggregate_score > 0.0);

        // Test LogarithmicSum method
        aggregate.aggregation_method = AggregationMethod::LogarithmicSum;
        aggregate.calculate_aggregate();
        assert!(aggregate.aggregate_score > 100.0);

        // Test MaxPlusAverage method
        aggregate.aggregation_method = AggregationMethod::MaxPlusAverage;
        aggregate.calculate_aggregate();
        assert!(aggregate.aggregate_score > 20.0); // Should be at least the max score
    }

    #[test]
    fn test_pipeline_aggregation() {
        let config = AggregationConfig {
            enabled: true,
            method: AggregationMethod::WeightedSum,
            problem_threshold: 5.0,
            min_functions_for_aggregation: 2,
            display_top_functions: 3,
        };

        let mut pipeline = AggregationPipeline::new(config);

        // Add scores for file1
        pipeline.add_function_score(PathBuf::from("file1.rs"), "func1".to_string(), 10.0);
        pipeline.add_function_score(PathBuf::from("file1.rs"), "func2".to_string(), 8.0);
        pipeline.add_function_score(PathBuf::from("file1.rs"), "func3".to_string(), 3.0);

        // Add scores for file2
        pipeline.add_function_score(PathBuf::from("file2.rs"), "func4".to_string(), 15.0);
        pipeline.add_function_score(PathBuf::from("file2.rs"), "func5".to_string(), 2.0);

        let aggregates = pipeline.aggregate_file_scores();

        assert_eq!(aggregates.len(), 2);
        assert!(aggregates[0].aggregate_score > 0.0);
        assert_eq!(aggregates[0].function_count, 3);
        assert_eq!(aggregates[0].problematic_functions, 2); // func1 and func2 > 5.0
    }
}
