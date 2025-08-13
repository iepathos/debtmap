use crate::priority::semantic_classifier::FunctionRole;
use crate::risk::evidence::ModuleType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityThresholds {
    pub low: f64,
    pub moderate: f64,
    pub high: f64,
    pub critical: f64,
}

impl Default for ComplexityThresholds {
    fn default() -> Self {
        Self {
            low: 5.0,
            moderate: 10.0,
            high: 15.0,
            critical: 20.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageThresholds {
    pub excellent: f64,
    pub good: f64,
    pub moderate: f64,
    pub poor: f64,
    pub critical: f64,
}

impl Default for CoverageThresholds {
    fn default() -> Self {
        Self {
            excellent: 90.0,
            good: 75.0,
            moderate: 50.0,
            poor: 25.0,
            critical: 10.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CouplingThresholds {
    pub low: u32,
    pub moderate: u32,
    pub high: u32,
    pub critical: u32,
}

impl Default for CouplingThresholds {
    fn default() -> Self {
        Self {
            low: 3,
            moderate: 7,
            high: 12,
            critical: 20,
        }
    }
}

pub struct StatisticalDistribution {
    percentiles: Vec<(f64, f64)>, // (percentile, value)
}

impl StatisticalDistribution {
    pub fn new(mut values: Vec<f64>) -> Self {
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let percentiles = vec![
            (10.0, Self::calculate_percentile(&values, 0.10)),
            (25.0, Self::calculate_percentile(&values, 0.25)),
            (50.0, Self::calculate_percentile(&values, 0.50)),
            (75.0, Self::calculate_percentile(&values, 0.75)),
            (90.0, Self::calculate_percentile(&values, 0.90)),
            (95.0, Self::calculate_percentile(&values, 0.95)),
            (99.0, Self::calculate_percentile(&values, 0.99)),
        ];

        Self { percentiles }
    }

    pub fn percentile(&self, p: f64) -> f64 {
        // Find the closest percentile or interpolate
        for i in 0..self.percentiles.len() {
            if self.percentiles[i].0 >= p {
                if i == 0 {
                    return self.percentiles[0].1;
                }
                // Linear interpolation between percentiles
                let (p1, v1) = self.percentiles[i - 1];
                let (p2, v2) = self.percentiles[i];
                let ratio = (p - p1) / (p2 - p1);
                return v1 + ratio * (v2 - v1);
            }
        }
        self.percentiles.last().unwrap().1
    }

    fn calculate_percentile(values: &[f64], percentile: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }

        let index = (percentile * (values.len() - 1) as f64) as usize;
        values[index]
    }
}

pub struct BaselineDatabase {
    complexity_distributions: HashMap<FunctionRole, StatisticalDistribution>,
    coverage_distributions: HashMap<FunctionRole, StatisticalDistribution>,
    coupling_distributions: HashMap<ModuleType, StatisticalDistribution>,
}

impl Default for BaselineDatabase {
    fn default() -> Self {
        // Initialize with typical baseline values for Rust codebases
        let mut complexity_distributions = HashMap::new();
        let mut coverage_distributions = HashMap::new();
        let mut coupling_distributions = HashMap::new();

        // Complexity baselines by role (cyclomatic complexity values)
        complexity_distributions.insert(
            FunctionRole::PureLogic,
            StatisticalDistribution::new(vec![
                1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 15.0,
                18.0, 22.0, 28.0, 35.0,
            ]),
        );

        complexity_distributions.insert(
            FunctionRole::Orchestrator,
            StatisticalDistribution::new(vec![
                2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0,
                24.0, 28.0, 32.0, 38.0, 45.0,
            ]),
        );

        complexity_distributions.insert(
            FunctionRole::IOWrapper,
            StatisticalDistribution::new(vec![
                1.0, 2.0, 3.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 10.0, 12.0, 15.0, 18.0, 22.0, 26.0,
                30.0, 35.0, 40.0, 48.0, 55.0,
            ]),
        );

        complexity_distributions.insert(
            FunctionRole::EntryPoint,
            StatisticalDistribution::new(vec![
                3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 23.0,
                26.0, 30.0, 35.0, 42.0, 50.0,
            ]),
        );

        complexity_distributions.insert(
            FunctionRole::Unknown,
            StatisticalDistribution::new(vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 14.0, 16.0, 19.0, 22.0,
                26.0, 31.0, 36.0, 43.0, 50.0,
            ]),
        );

        // Coverage baselines by role (percentage values)
        coverage_distributions.insert(
            FunctionRole::PureLogic,
            StatisticalDistribution::new(vec![
                0.0, 0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 75.0, 80.0, 85.0, 88.0, 90.0,
                92.0, 94.0, 96.0, 98.0, 99.0, 100.0,
            ]),
        );

        coverage_distributions.insert(
            FunctionRole::Orchestrator,
            StatisticalDistribution::new(vec![
                0.0, 0.0, 5.0, 15.0, 25.0, 35.0, 45.0, 55.0, 65.0, 70.0, 75.0, 80.0, 83.0, 85.0,
                87.0, 89.0, 91.0, 93.0, 95.0, 98.0,
            ]),
        );

        coverage_distributions.insert(
            FunctionRole::IOWrapper,
            StatisticalDistribution::new(vec![
                0.0, 0.0, 0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 65.0, 70.0, 75.0, 78.0, 80.0,
                82.0, 84.0, 86.0, 88.0, 90.0, 95.0,
            ]),
        );

        coverage_distributions.insert(
            FunctionRole::EntryPoint,
            StatisticalDistribution::new(vec![
                0.0, 0.0, 5.0, 15.0, 25.0, 35.0, 45.0, 55.0, 65.0, 70.0, 75.0, 80.0, 83.0, 85.0,
                87.0, 89.0, 91.0, 93.0, 95.0, 98.0,
            ]),
        );

        coverage_distributions.insert(
            FunctionRole::Unknown,
            StatisticalDistribution::new(vec![
                0.0, 0.0, 5.0, 15.0, 25.0, 35.0, 45.0, 55.0, 65.0, 70.0, 75.0, 80.0, 83.0, 85.0,
                87.0, 89.0, 91.0, 93.0, 95.0, 98.0,
            ]),
        );

        // Coupling baselines by module type (dependency count)
        coupling_distributions.insert(
            ModuleType::Core,
            StatisticalDistribution::new(vec![
                0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 14.0, 16.0, 18.0,
                20.0, 23.0, 26.0, 30.0, 35.0,
            ]),
        );

        coupling_distributions.insert(
            ModuleType::Api,
            StatisticalDistribution::new(vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 13.0, 15.0, 17.0, 19.0,
                21.0, 24.0, 27.0, 31.0, 36.0,
            ]),
        );

        coupling_distributions.insert(
            ModuleType::Util,
            StatisticalDistribution::new(vec![
                0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0,
                12.0, 14.0, 16.0, 20.0,
            ]),
        );

        coupling_distributions.insert(
            ModuleType::Test,
            StatisticalDistribution::new(vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0,
                22.0, 25.0, 28.0, 32.0, 38.0,
            ]),
        );

        coupling_distributions.insert(
            ModuleType::Infrastructure,
            StatisticalDistribution::new(vec![
                2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0,
                23.0, 26.0, 29.0, 33.0, 40.0,
            ]),
        );

        Self {
            complexity_distributions,
            coverage_distributions,
            coupling_distributions,
        }
    }
}

impl BaselineDatabase {
    pub fn get_complexity_distribution(&self, role: &FunctionRole) -> &StatisticalDistribution {
        self.complexity_distributions
            .get(role)
            .unwrap_or_else(|| &self.complexity_distributions[&FunctionRole::Unknown])
    }

    pub fn get_coverage_distribution(&self, role: &FunctionRole) -> &StatisticalDistribution {
        self.coverage_distributions
            .get(role)
            .unwrap_or_else(|| &self.coverage_distributions[&FunctionRole::Unknown])
    }

    pub fn get_coupling_distribution(&self, module_type: &ModuleType) -> &StatisticalDistribution {
        self.coupling_distributions
            .get(module_type)
            .unwrap_or_else(|| &self.coupling_distributions[&ModuleType::Util])
    }
}

pub struct ProjectContext {
    pub language: String,
    pub project_type: String,
    pub team_size: usize,
}

impl Default for ProjectContext {
    fn default() -> Self {
        Self {
            language: "rust".to_string(),
            project_type: "library".to_string(),
            team_size: 1,
        }
    }
}

#[derive(Default)]
pub struct StatisticalThresholdProvider {
    baseline_data: BaselineDatabase,
    #[allow(dead_code)]
    project_context: ProjectContext,
}

impl StatisticalThresholdProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_complexity_thresholds(&self, role: &FunctionRole) -> ComplexityThresholds {
        let baseline = self.baseline_data.get_complexity_distribution(role);

        ComplexityThresholds {
            low: baseline.percentile(50.0),      // P50 - median
            moderate: baseline.percentile(75.0), // P75 - above average
            high: baseline.percentile(90.0),     // P90 - high
            critical: baseline.percentile(95.0), // P95 - very high
        }
    }

    pub fn get_coverage_thresholds(&self, role: &FunctionRole) -> CoverageThresholds {
        let baseline = self.baseline_data.get_coverage_distribution(role);

        CoverageThresholds {
            excellent: baseline.percentile(90.0), // P90 - well tested
            good: baseline.percentile(75.0),      // P75 - adequately tested
            moderate: baseline.percentile(50.0),  // P50 - some testing
            poor: baseline.percentile(25.0),      // P25 - minimal testing
            critical: baseline.percentile(10.0),  // P10 - essentially untested
        }
    }

    pub fn get_coupling_thresholds(&self, module_type: &ModuleType) -> CouplingThresholds {
        let baseline = self.baseline_data.get_coupling_distribution(module_type);

        CouplingThresholds {
            low: baseline.percentile(50.0) as u32,      // P50 - median
            moderate: baseline.percentile(75.0) as u32, // P75 - above average
            high: baseline.percentile(90.0) as u32,     // P90 - high
            critical: baseline.percentile(95.0) as u32, // P95 - very high
        }
    }
}
