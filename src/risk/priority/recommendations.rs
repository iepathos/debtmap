use super::{module_detection::ModuleType, TestTarget};
use crate::core::ComplexityMetrics;

pub struct RationaleBuilder;

impl RationaleBuilder {
    pub fn coverage_description(coverage: f64) -> String {
        match coverage {
            0.0 => "NO test coverage".to_string(),
            c => format!("{c:.0}% coverage"),
        }
    }

    pub fn complexity_level(cognitive: u32) -> &'static str {
        match cognitive {
            0..=7 => "Simple",
            8..=15 => "Moderate",
            16..=30 => "Complex",
            _ => "Very complex",
        }
    }

    pub fn effort_description(cyclomatic: u32, cognitive: u32) -> &'static str {
        match (cyclomatic, cognitive) {
            (1..=3, 1..=7) => " - easy win",
            (1..=5, 1..=10) => " - quick test",
            (6..=10, _) => " - moderate effort",
            _ => " - requires effort",
        }
    }

    pub fn roi_description(target: &TestTarget) -> &'static str {
        match (
            target.dependents.len(),
            target.complexity.cognitive_complexity,
            &target.module_type,
        ) {
            (3.., 0..=10, _) => " - maximum ROI",
            (_, _, ModuleType::EntryPoint) => " - critical path",
            _ => "",
        }
    }

    pub fn module_description(module_type: &ModuleType, has_coverage: bool) -> String {
        let base = match module_type {
            ModuleType::EntryPoint => "Critical entry point",
            ModuleType::Core => "Core module",
            ModuleType::Api => "API module",
            ModuleType::IO => "I/O module",
            ModuleType::Model => "Data model",
            ModuleType::Utility => "Utility module",
            ModuleType::Test => "Test module",
            ModuleType::Unknown => "Module",
        };

        if has_coverage {
            base.to_string()
        } else {
            format!("{base} completely untested")
        }
    }
}

#[allow(dead_code)]
pub fn describe_coverage_status(target: &TestTarget) -> &'static str {
    match (target.current_coverage, &target.module_type) {
        (0.0, ModuleType::EntryPoint) => "Critical entry point with NO test coverage",
        (0.0, ModuleType::Core) => "Core module completely untested",
        (0.0, ModuleType::Api) => "API handler with zero coverage",
        (0.0, ModuleType::IO) => "I/O module without any tests",
        (0.0, _) => "Module has no test coverage",
        (c, _) if c < 30.0 => "Poorly tested",
        (c, _) if c < 60.0 => "Moderately tested",
        _ => "Well tested",
    }
}

#[allow(dead_code)]
pub fn describe_complexity(metrics: &ComplexityMetrics) -> &'static str {
    use std::cmp::max;
    let max_complexity = max(
        metrics.cyclomatic_complexity / 2,
        metrics.cognitive_complexity / 4,
    );

    match max_complexity {
        0..=2 => "simple",
        3..=5 => "moderately complex",
        6..=10 => "highly complex",
        _ => "extremely complex",
    }
}

#[allow(dead_code)]
pub fn describe_impact(dependents: &[String]) -> String {
    match dependents.len() {
        0 => String::new(),
        1..=5 => format!(" - affects {} other modules", dependents.len()),
        n => format!(" - critical dependency for {n} modules"),
    }
}

pub fn generate_enhanced_rationale_v2(target: &TestTarget, _roi: &crate::risk::roi::ROI) -> String {
    let coverage_str = RationaleBuilder::coverage_description(target.current_coverage);
    let complexity_desc =
        RationaleBuilder::complexity_level(target.complexity.cognitive_complexity);
    let complexity_str = format!(
        "cyclo={}, cognitive={}",
        target.complexity.cyclomatic_complexity, target.complexity.cognitive_complexity
    );
    let effort_desc = RationaleBuilder::effort_description(
        target.complexity.cyclomatic_complexity,
        target.complexity.cognitive_complexity,
    );
    let roi_desc = RationaleBuilder::roi_description(target);
    let module_desc =
        RationaleBuilder::module_description(&target.module_type, target.current_coverage > 0.0);

    format!(
        "{module_desc} with {coverage_str}\n            {complexity_desc} code ({complexity_str}){effort_desc}{roi_desc}"
    )
}

#[allow(dead_code)]
pub fn generate_enhanced_rationale(target: &TestTarget, roi: &super::ROI) -> String {
    let coverage_status = describe_coverage_status(target);
    let complexity_desc = describe_complexity(&target.complexity);
    let impact_desc = describe_impact(&target.dependents);

    format!(
        "{} - {} code (cyclo={}, cognitive={}){}. ROI: {:.1}x with {:.1}% risk reduction",
        coverage_status,
        complexity_desc,
        target.complexity.cyclomatic_complexity,
        target.complexity.cognitive_complexity,
        impact_desc,
        roi.value,
        roi.risk_reduction
    )
}

#[derive(Clone, Debug)]
pub struct TestRecommendation {
    pub target: TestTarget,
    pub priority: f64,
    pub roi: super::ROI,
    pub effort: TestEffortDetails,
    pub impact: ImpactAnalysis,
    pub rationale: String,
    pub suggested_approach: TestApproach,
}

#[derive(Clone, Debug)]
pub struct TestEffortDetails {
    pub estimated_cases: usize,
    pub estimated_hours: f64,
    pub complexity_level: ComplexityLevel,
    pub setup_requirements: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum ComplexityLevel {
    Trivial,
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

#[derive(Clone, Debug)]
pub enum TestApproach {
    UnitTest,
    IntegrationTest,
    ModuleTest,
    EndToEndTest,
}

#[derive(Clone, Debug)]
pub struct ImpactAnalysis {
    pub direct_risk_reduction: f64,
    pub cascade_effect: f64,
    pub affected_modules: Vec<String>,
    pub coverage_improvement: f64,
}
