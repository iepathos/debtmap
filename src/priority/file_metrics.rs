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
    /// The specific type of god object detected (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub god_object_type: Option<crate::organization::GodObjectType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GodObjectIndicators {
    pub methods_count: usize,
    pub fields_count: usize,
    pub responsibilities: usize,
    pub is_god_object: bool,
    pub god_object_score: f64,
    /// Detailed list of identified responsibilities (e.g., "Data Access", "Validation")
    #[serde(default)]
    pub responsibility_names: Vec<String>,
    /// Recommended module splits with methods to move
    #[serde(default)]
    pub recommended_splits: Vec<ModuleSplit>,
    /// Detailed module structure analysis (for enhanced reporting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_structure: Option<crate::analysis::ModuleStructure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSplit {
    pub suggested_name: String,
    pub methods_to_move: Vec<String>,
    #[serde(default)]
    pub structs_to_move: Vec<String>,
    pub responsibility: String,
    pub estimated_lines: usize,
    #[serde(default)]
    pub method_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default)]
    pub priority: Priority,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Priority {
    High,
    #[default]
    Medium,
    Low,
}

impl From<crate::organization::Priority> for Priority {
    fn from(p: crate::organization::Priority) -> Self {
        match p {
            crate::organization::Priority::High => Priority::High,
            crate::organization::Priority::Medium => Priority::Medium,
            crate::organization::Priority::Low => Priority::Low,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDebtItem {
    pub metrics: FileDebtMetrics,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub priority_rank: usize,
    #[serde(default = "default_recommendation")]
    pub recommendation: String,
    #[serde(default)]
    pub impact: FileImpact,
}

fn default_recommendation() -> String {
    "Refactor for better maintainability".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileImpact {
    pub complexity_reduction: f64,
    pub maintainability_improvement: f64,
    pub test_effort: f64,
}

impl Default for FileImpact {
    fn default() -> Self {
        Self {
            complexity_reduction: 0.0,
            maintainability_improvement: 0.0,
            test_effort: 0.0,
        }
    }
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
        // First check for boilerplate pattern - highest priority
        if let Some(crate::organization::GodObjectType::BoilerplatePattern {
            recommendation, ..
        }) = &self.god_object_type
        {
            return recommendation.clone();
        }

        if self.god_object_indicators.is_god_object {
            // Analyze the file path to provide context-specific recommendations
            let file_name = self
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("module");

            let is_parser = file_name.contains("pars") || file_name.contains("lexer");
            let is_cache = file_name.contains("cache");
            let is_writer = file_name.contains("writer") || file_name.contains("format");
            let is_analyzer = file_name.contains("analyz") || file_name.contains("detect");

            // Generate specific splitting recommendations based on file type
            if is_parser {
                format!(
                    "Split parser into {} modules: 1) Tokenizer/Lexer 2) AST builder 3) Visitor/Walker 4) Error handling. Group by parsing phase, not by node type.",
                    (self.function_count / 20).clamp(3, 5)
                )
            } else if is_cache {
                "Split cache into: 1) Storage backend 2) Eviction policy 3) Serialization 4) Cache operations. Separate policy from mechanism.".to_string()
            } else if is_writer {
                "Split into: 1) Core formatter 2) Section writers (one per major section) 3) Style/theme handling. Max 20 functions per writer module.".to_string()
            } else if is_analyzer {
                "Split by analysis phase: 1) Data collection 2) Pattern detection 3) Scoring/metrics 4) Reporting. Keep related analyses together.".to_string()
            } else {
                // Generic but more specific recommendation
                format!(
                    "URGENT: {} lines, {} functions! Split by data flow: 1) Input/parsing functions 2) Core logic/transformation 3) Output/formatting. Create {} focused modules with <30 functions each.",
                    self.total_lines, self.function_count,
                    (self.function_count / 25).clamp(3, 8)
                )
            }
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
            god_object_type: None,
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
            responsibility_names: Vec::new(),
            recommended_splits: Vec::new(),
            module_structure: None,
        }
    }
}

// Extension to support legacy JSON format that only has metrics
impl FileDebtItem {
    pub fn from_metrics(metrics: FileDebtMetrics) -> Self {
        let score = metrics.calculate_score();
        let recommendation = metrics.generate_recommendation();
        let impact = FileImpact {
            complexity_reduction: metrics.avg_complexity * metrics.function_count as f64 * 0.2,
            maintainability_improvement: (metrics.max_complexity as f64 - metrics.avg_complexity)
                * 10.0,
            test_effort: metrics.uncovered_lines as f64 * 0.1,
        };

        FileDebtItem {
            metrics,
            score,
            priority_rank: 0,
            recommendation,
            impact,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_score_basic() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 100,
            function_count: 5,
            class_count: 1,
            avg_complexity: 5.0,
            max_complexity: 10,
            total_complexity: 25,
            coverage_percent: 0.8,
            uncovered_lines: 20,
            god_object_indicators: GodObjectIndicators::default(),
            function_scores: vec![1.0, 2.0, 3.0],
            god_object_type: None,
        };

        let score = metrics.calculate_score();
        assert!(score > 0.0);
        assert!(score < 10.0);
    }

    #[test]
    fn test_calculate_score_with_god_object() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("god.rs"),
            total_lines: 1000,
            function_count: 60,
            class_count: 1,
            avg_complexity: 15.0,
            max_complexity: 50,
            total_complexity: 900,
            coverage_percent: 0.3,
            uncovered_lines: 700,
            god_object_indicators: GodObjectIndicators {
                methods_count: 60,
                fields_count: 30,
                responsibilities: 10,
                is_god_object: true,
                god_object_score: 0.8,
                responsibility_names: Vec::new(),
                recommended_splits: Vec::new(),
                module_structure: None,
            },
            function_scores: vec![5.0; 60],
            god_object_type: None,
        };

        let score = metrics.calculate_score();
        assert!(score > 50.0, "God object should have high score");
    }

    #[test]
    fn test_calculate_score_low_coverage() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("untested.rs"),
            total_lines: 200,
            function_count: 10,
            class_count: 2,
            avg_complexity: 3.0,
            max_complexity: 5,
            total_complexity: 30,
            coverage_percent: 0.1,
            uncovered_lines: 180,
            god_object_indicators: GodObjectIndicators::default(),
            function_scores: vec![1.0; 10],
            god_object_type: None,
        };

        let score = metrics.calculate_score();
        let base_metrics = FileDebtMetrics {
            coverage_percent: 0.9,
            uncovered_lines: 20,
            ..metrics
        };
        let base_score = base_metrics.calculate_score();

        assert!(score > base_score, "Low coverage should increase score");
    }

    #[test]
    fn test_calculate_score_high_complexity() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("complex.rs"),
            total_lines: 300,
            function_count: 15,
            class_count: 1,
            avg_complexity: 20.0,
            max_complexity: 40,
            total_complexity: 300,
            coverage_percent: 0.7,
            uncovered_lines: 90,
            god_object_indicators: GodObjectIndicators::default(),
            function_scores: vec![3.0; 15],
            god_object_type: None,
        };

        let score = metrics.calculate_score();
        assert!(score > 10.0, "High complexity should produce high score");
    }

    #[test]
    fn test_calculate_score_many_functions() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("dense.rs"),
            total_lines: 500,
            function_count: 75,
            class_count: 1,
            avg_complexity: 4.0,
            max_complexity: 8,
            total_complexity: 300,
            coverage_percent: 0.6,
            uncovered_lines: 200,
            god_object_indicators: GodObjectIndicators::default(),
            function_scores: vec![2.0; 75],
            god_object_type: None,
        };

        let score = metrics.calculate_score();
        assert!(score > 15.0, "Dense files should have higher scores");
    }

    #[test]
    fn test_generate_recommendation_god_object() {
        let metrics = FileDebtMetrics {
            god_object_indicators: GodObjectIndicators {
                is_god_object: true,
                ..Default::default()
            },
            function_count: 50,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        // With the new format, the generic case says "Split by data flow"
        assert!(rec.contains("Split by data flow") || rec.contains("Split"));
        assert!(rec.contains("modules") || rec.contains("functions"));
    }

    #[test]
    fn test_generate_recommendation_large_file() {
        let metrics = FileDebtMetrics {
            total_lines: 800,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(rec.contains("Extract complex functions"));
        assert!(rec.contains("800 lines"));
    }

    #[test]
    fn test_generate_recommendation_high_complexity() {
        let metrics = FileDebtMetrics {
            avg_complexity: 15.0,
            total_lines: 200,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(rec.contains("Simplify complex functions"));
    }

    #[test]
    fn test_generate_recommendation_low_coverage() {
        let metrics = FileDebtMetrics {
            coverage_percent: 0.25,
            total_lines: 200,
            avg_complexity: 3.0,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(rec.contains("Increase test coverage"));
        assert!(rec.contains("25.0%"));
    }

    #[test]
    fn test_generate_recommendation_general() {
        let metrics = FileDebtMetrics {
            total_lines: 100,
            avg_complexity: 3.0,
            coverage_percent: 0.85,
            ..Default::default()
        };

        let rec = metrics.generate_recommendation();
        assert!(rec.contains("Refactor for better maintainability"));
    }

    #[test]
    fn test_default_file_debt_metrics() {
        let metrics = FileDebtMetrics::default();
        assert_eq!(metrics.total_lines, 0);
        assert_eq!(metrics.function_count, 0);
        assert_eq!(metrics.avg_complexity, 0.0);
        assert_eq!(metrics.coverage_percent, 0.0);
        assert!(!metrics.god_object_indicators.is_god_object);
    }

    #[test]
    fn test_score_factors_multiplication() {
        let mut metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 400,
            function_count: 20,
            avg_complexity: 10.0,
            total_complexity: 200,
            coverage_percent: 0.5,
            ..Default::default()
        };

        let score1 = metrics.calculate_score();

        metrics.god_object_indicators.is_god_object = true;
        metrics.god_object_indicators.god_object_score = 0.5;
        let score2 = metrics.calculate_score();

        assert!(
            score2 > score1 * 2.0,
            "God object should multiply score significantly"
        );
    }

    #[test]
    fn test_function_scores_aggregation() {
        let metrics = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 100,
            function_count: 3,
            avg_complexity: 2.0,
            total_complexity: 6,
            coverage_percent: 0.5,
            function_scores: vec![10.0, 20.0, 30.0],
            god_object_type: None,
            ..Default::default()
        };

        let score = metrics.calculate_score();
        assert!(score > 0.0);

        let metrics_no_functions = FileDebtMetrics {
            path: PathBuf::from("test.rs"),
            total_lines: 100,
            function_count: 3,
            avg_complexity: 2.0,
            total_complexity: 6,
            coverage_percent: 0.5,
            function_scores: vec![],
            god_object_type: None,
            ..Default::default()
        };

        let score_no_functions = metrics_no_functions.calculate_score();
        assert!(
            score > score_no_functions,
            "Function scores should increase total score"
        );
    }

    #[test]
    fn test_boilerplate_recommendation_used() {
        use crate::organization::boilerplate_detector::BoilerplatePattern;
        use crate::organization::GodObjectType;

        let boilerplate_type = GodObjectType::BoilerplatePattern {
            pattern: BoilerplatePattern::TraitImplementation {
                trait_name: "Flag".to_string(),
                impl_count: 104,
                shared_methods: vec!["name_long".to_string()],
                method_uniformity: 1.0,
            },
            confidence: 0.878,
            recommendation: "BOILERPLATE DETECTED: Create declarative macro to generate Flag implementations. This is NOT a god object requiring module splitting.".to_string(),
        };

        let metrics = FileDebtMetrics {
            god_object_indicators: GodObjectIndicators {
                is_god_object: true,
                methods_count: 888,
                fields_count: 0,
                responsibilities: 1,
                god_object_score: 0.878,
                responsibility_names: Vec::new(),
                recommended_splits: Vec::new(),
                module_structure: None,
            },
            god_object_type: Some(boilerplate_type),
            total_lines: 7775,
            function_count: 888,
            ..Default::default()
        };

        let recommendation = metrics.generate_recommendation();

        assert!(recommendation.contains("BOILERPLATE DETECTED"));
        assert!(recommendation.contains("declarative macro"));
        assert!(recommendation.contains("NOT a god object requiring module splitting"));
    }

    #[test]
    fn test_regular_god_object_still_gets_splitting_advice() {
        use crate::organization::GodObjectType;

        let god_file_type = GodObjectType::GodModule {
            total_structs: 20,
            total_methods: 100,
            largest_struct: crate::organization::StructMetrics {
                name: "Config".to_string(),
                method_count: 50,
                field_count: 30,
                responsibilities: vec!["Data Access".to_string()],
                line_span: (0, 1000),
            },
            suggested_splits: vec![],
        };

        let metrics = FileDebtMetrics {
            god_object_indicators: GodObjectIndicators {
                is_god_object: true,
                methods_count: 100,
                fields_count: 30,
                responsibilities: 5,
                god_object_score: 0.8,
                responsibility_names: Vec::new(),
                recommended_splits: Vec::new(),
                module_structure: None,
            },
            god_object_type: Some(god_file_type),
            total_lines: 2000,
            function_count: 100,
            ..Default::default()
        };

        let recommendation = metrics.generate_recommendation();

        assert!(recommendation.contains("Split") || recommendation.contains("URGENT"));
        assert!(!recommendation.contains("BOILERPLATE"));
        assert!(!recommendation.contains("macro"));
    }

    #[test]
    fn test_boilerplate_takes_precedence_over_god_object() {
        use crate::organization::boilerplate_detector::BoilerplatePattern;
        use crate::organization::GodObjectType;

        let boilerplate_type = GodObjectType::BoilerplatePattern {
            pattern: BoilerplatePattern::TestBoilerplate {
                test_count: 50,
                shared_structure: "similar test structure".to_string(),
            },
            confidence: 0.92,
            recommendation: "Use a macro to generate these test functions".to_string(),
        };

        let metrics = FileDebtMetrics {
            god_object_indicators: GodObjectIndicators {
                is_god_object: true,
                methods_count: 200,
                fields_count: 100,
                responsibilities: 10,
                god_object_score: 0.95,
                responsibility_names: Vec::new(),
                recommended_splits: Vec::new(),
                module_structure: None,
            },
            god_object_type: Some(boilerplate_type),
            total_lines: 5000,
            function_count: 200,
            ..Default::default()
        };

        let recommendation = metrics.generate_recommendation();

        // Should use boilerplate recommendation, not god object recommendation
        assert!(recommendation.contains("macro"));
        assert!(!recommendation.contains("Split"));
        assert!(!recommendation.contains("URGENT"));
    }
}
