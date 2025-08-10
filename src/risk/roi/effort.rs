use super::super::priority::{ModuleType, TestTarget};

pub trait EffortModel: Send + Sync {
    fn estimate(&self, target: &TestTarget) -> EffortEstimate;
    fn explain(&self, estimate: &EffortEstimate) -> String;
}

#[derive(Clone, Debug)]
pub struct EffortEstimate {
    pub hours: f64,
    pub test_cases: usize,
    pub complexity: ComplexityLevel,
    pub breakdown: EffortBreakdown,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ComplexityLevel {
    Trivial,
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

#[derive(Clone, Debug)]
pub struct EffortBreakdown {
    pub base: f64,
    pub setup: f64,
    pub mocking: f64,
    pub understanding: f64,
}

pub struct AdvancedEffortModel {
    base_rates: EffortRates,
    complexity_factors: ComplexityFactors,
}

#[derive(Clone, Debug)]
struct EffortRates {
    per_test_case: f64,
    per_dependency: f64,
    cognitive_penalty: f64,
}

impl Default for EffortRates {
    fn default() -> Self {
        Self {
            per_test_case: 0.25,
            per_dependency: 0.15,
            cognitive_penalty: 0.1,
        }
    }
}

#[derive(Clone, Debug)]
struct ComplexityFactors {
    cyclomatic_base: f64,
    cognitive_weight: f64,
    nesting_penalty: f64,
}

impl Default for ComplexityFactors {
    fn default() -> Self {
        Self {
            cyclomatic_base: 1.0,
            cognitive_weight: 0.1,
            nesting_penalty: 0.2,
        }
    }
}

impl Default for AdvancedEffortModel {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedEffortModel {
    pub fn new() -> Self {
        Self {
            base_rates: EffortRates::default(),
            complexity_factors: ComplexityFactors::default(),
        }
    }

    fn calculate_base_effort(&self, target: &TestTarget) -> f64 {
        let min_cases = (target.complexity.cyclomatic_complexity + 1) as f64;
        let cognitive_factor = (target.complexity.cognitive_complexity as f64 / 7.0).max(1.0);
        let case_hours = min_cases * self.base_rates.per_test_case;

        case_hours * cognitive_factor
    }

    fn estimate_setup_effort(&self, target: &TestTarget) -> f64 {
        let dependency_count = target.dependencies.len();
        let module_factor = match target.module_type {
            ModuleType::EntryPoint => 2.0,
            ModuleType::IO => 1.5,
            ModuleType::Api => 1.3,
            ModuleType::Core => 1.0,
            _ => 0.5,
        };

        match dependency_count {
            0 => 0.0,
            1..=3 => 0.5 * module_factor,
            4..=7 => 1.0 * module_factor,
            8..=12 => 1.5 * module_factor,
            _ => 2.0 * module_factor,
        }
    }

    fn estimate_mocking_effort(&self, target: &TestTarget) -> f64 {
        let external_deps = target
            .dependencies
            .iter()
            .filter(|d| {
                d.contains("io")
                    || d.contains("net")
                    || d.contains("fs")
                    || d.contains("db")
                    || d.contains("http")
            })
            .count();

        match external_deps {
            0 => 0.0,
            1 => 0.5,
            2 => 1.0,
            3 => 1.5,
            _ => 2.0 + (external_deps as f64 - 3.0) * 0.25,
        }
    }

    fn estimate_understanding_effort(&self, target: &TestTarget) -> f64 {
        let cognitive = target.complexity.cognitive_complexity;
        let lines = target.lines;

        let cognitive_hours = match cognitive {
            0..=7 => 0.0,
            8..=15 => 0.5,
            16..=30 => 1.0,
            31..=50 => 2.0,
            _ => 3.0,
        };

        let size_factor = match lines {
            0..=50 => 1.0,
            51..=100 => 1.2,
            101..=200 => 1.5,
            201..=500 => 2.0,
            _ => 2.5,
        };

        cognitive_hours * size_factor
    }

    fn estimate_test_cases(&self, target: &TestTarget) -> usize {
        let min_cases = target.complexity.cyclomatic_complexity + 1;

        let edge_cases = match target.module_type {
            ModuleType::Api | ModuleType::IO => 3,
            ModuleType::Core | ModuleType::EntryPoint => 2,
            _ => 1,
        };

        let error_cases = if !target.dependencies.is_empty() {
            (target.dependencies.len() / 2).max(1) as u32
        } else {
            0
        };

        (min_cases + edge_cases + error_cases) as usize
    }

    fn categorize_complexity(&self, hours: f64) -> ComplexityLevel {
        match hours {
            h if h <= 0.5 => ComplexityLevel::Trivial,
            h if h <= 2.0 => ComplexityLevel::Simple,
            h if h <= 5.0 => ComplexityLevel::Moderate,
            h if h <= 10.0 => ComplexityLevel::Complex,
            _ => ComplexityLevel::VeryComplex,
        }
    }
}

impl EffortModel for AdvancedEffortModel {
    fn estimate(&self, target: &TestTarget) -> EffortEstimate {
        let base = self.calculate_base_effort(target);
        let setup = self.estimate_setup_effort(target);
        let mocking = self.estimate_mocking_effort(target);
        let understanding = self.estimate_understanding_effort(target);

        let total_hours = base + setup + mocking + understanding;

        EffortEstimate {
            hours: total_hours,
            test_cases: self.estimate_test_cases(target),
            complexity: self.categorize_complexity(total_hours),
            breakdown: EffortBreakdown {
                base,
                setup,
                mocking,
                understanding,
            },
        }
    }

    fn explain(&self, estimate: &EffortEstimate) -> String {
        format!(
            "Estimated effort: {:.1} hours ({} test cases)\n\
             - Base testing: {:.1}h\n\
             - Setup/teardown: {:.1}h\n\
             - Mocking dependencies: {:.1}h\n\
             - Understanding code: {:.1}h\n\
             Complexity level: {:?}",
            estimate.hours,
            estimate.test_cases,
            estimate.breakdown.base,
            estimate.breakdown.setup,
            estimate.breakdown.mocking,
            estimate.breakdown.understanding,
            estimate.complexity
        )
    }
}
