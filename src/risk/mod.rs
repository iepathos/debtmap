pub mod correlation;
pub mod insights;
pub mod lcov;
pub mod priority;

use crate::core::ComplexityMetrics;
use im::Vector;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionRisk {
    pub file: PathBuf,
    pub function_name: String,
    pub line_range: (usize, usize),
    pub cyclomatic_complexity: u32,
    pub cognitive_complexity: u32,
    pub coverage_percentage: Option<f64>,
    pub risk_score: f64,
    pub test_effort: TestEffort,
    pub risk_category: RiskCategory,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskCategory {
    Critical,   // High complexity (>15), low coverage (<30%)
    High,       // High complexity (>10), moderate coverage (<60%)
    Medium,     // Moderate complexity (>5), low coverage (<50%)
    Low,        // Low complexity or high coverage
    WellTested, // High complexity with high coverage (good examples)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestEffort {
    pub estimated_difficulty: Difficulty,
    pub cognitive_load: u32,
    pub branch_count: u32,
    pub recommended_test_cases: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Difficulty {
    Trivial,     // Cognitive < 5
    Simple,      // Cognitive 5-10
    Moderate,    // Cognitive 10-20
    Complex,     // Cognitive 20-40
    VeryComplex, // Cognitive > 40
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskInsight {
    pub top_risks: Vector<FunctionRisk>,
    pub risk_reduction_opportunities: Vector<TestingRecommendation>,
    pub codebase_risk_score: f64,
    pub complexity_coverage_correlation: Option<f64>,
    pub risk_distribution: RiskDistribution,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestingRecommendation {
    pub function: String,
    pub file: PathBuf,
    pub current_risk: f64,
    pub potential_risk_reduction: f64,
    pub test_effort_estimate: TestEffort,
    pub rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskDistribution {
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub well_tested_count: usize,
    pub total_functions: usize,
}

pub struct RiskAnalyzer {
    pub complexity_weight: f64, // Default: 1.0
    pub coverage_weight: f64,   // Default: 1.0
    pub cognitive_weight: f64,  // Default: 1.5 (cognitive is harder to test)
}

impl Default for RiskAnalyzer {
    fn default() -> Self {
        Self {
            complexity_weight: 1.0,
            coverage_weight: 1.0,
            cognitive_weight: 1.5,
        }
    }
}

impl RiskAnalyzer {
    pub fn analyze_function(
        &self,
        file: PathBuf,
        function_name: String,
        line_range: (usize, usize),
        complexity: &ComplexityMetrics,
        coverage: Option<f64>,
    ) -> FunctionRisk {
        let cyclomatic = complexity.cyclomatic_complexity;
        let cognitive = complexity.cognitive_complexity;

        let risk_score = self.calculate_risk_score(cyclomatic, cognitive, coverage);
        let test_effort = self.estimate_test_effort(cognitive, cyclomatic);
        let risk_category = self.categorize_risk(cyclomatic, cognitive, coverage);

        FunctionRisk {
            file,
            function_name,
            line_range,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
            coverage_percentage: coverage,
            risk_score,
            test_effort,
            risk_category,
        }
    }

    pub fn calculate_risk_score(
        &self,
        cyclomatic: u32,
        cognitive: u32,
        coverage: Option<f64>,
    ) -> f64 {
        let complexity_factor = (cyclomatic as f64 * self.complexity_weight
            + cognitive as f64 * self.cognitive_weight)
            / 2.0;

        match coverage {
            Some(cov) => {
                let coverage_gap = (100.0 - cov) / 100.0;
                coverage_gap * complexity_factor * self.coverage_weight
            }
            None => {
                // When no coverage data, use complexity as proxy for risk
                complexity_factor
            }
        }
    }

    pub fn estimate_test_effort(&self, cognitive: u32, cyclomatic: u32) -> TestEffort {
        TestEffort {
            estimated_difficulty: Self::classify_difficulty(cognitive),
            cognitive_load: cognitive,
            branch_count: cyclomatic,
            recommended_test_cases: Self::calculate_test_cases(cyclomatic),
        }
    }

    fn classify_difficulty(cognitive: u32) -> Difficulty {
        const THRESHOLDS: [(u32, Difficulty); 5] = [
            (4, Difficulty::Trivial),
            (10, Difficulty::Simple),
            (20, Difficulty::Moderate),
            (40, Difficulty::Complex),
            (u32::MAX, Difficulty::VeryComplex),
        ];

        THRESHOLDS
            .iter()
            .find(|(threshold, _)| cognitive <= *threshold)
            .map(|(_, difficulty)| difficulty.clone())
            .unwrap_or(Difficulty::VeryComplex)
    }

    fn calculate_test_cases(cyclomatic: u32) -> u32 {
        const MAPPINGS: [(u32, u32); 6] =
            [(3, 1), (7, 2), (10, 3), (15, 5), (20, 7), (u32::MAX, 10)];

        MAPPINGS
            .iter()
            .find(|(threshold, _)| cyclomatic <= *threshold)
            .map(|(_, cases)| *cases)
            .unwrap_or(10)
    }

    fn categorize_risk(
        &self,
        cyclomatic: u32,
        cognitive: u32,
        coverage: Option<f64>,
    ) -> RiskCategory {
        let avg_complexity = (cyclomatic + cognitive) / 2;

        // Define risk rules as data
        struct RiskRule {
            min_complexity: u32,
            max_coverage: Option<f64>,
            category: RiskCategory,
        }

        let rules = [
            RiskRule {
                min_complexity: 15,
                max_coverage: Some(30.0),
                category: RiskCategory::Critical,
            },
            RiskRule {
                min_complexity: 10,
                max_coverage: Some(60.0),
                category: RiskCategory::High,
            },
            RiskRule {
                min_complexity: 5,
                max_coverage: Some(50.0),
                category: RiskCategory::Medium,
            },
        ];

        // Check for well-tested case first
        if let Some(cov) = coverage {
            if avg_complexity > 10 && cov > 80.0 {
                return RiskCategory::WellTested;
            }
        }

        // Apply risk rules
        for rule in &rules {
            if avg_complexity > rule.min_complexity {
                match (coverage, rule.max_coverage) {
                    (Some(cov), Some(max_cov)) if cov < max_cov => return rule.category.clone(),
                    (None, _) => return rule.category.clone(),
                    _ => continue,
                }
            }
        }

        RiskCategory::Low
    }

    pub fn calculate_risk_reduction(
        &self,
        current_risk: f64,
        _complexity: u32,
        target_coverage: f64,
    ) -> f64 {
        // How much risk would be eliminated by achieving target coverage
        current_risk * (target_coverage / 100.0)
    }
}
