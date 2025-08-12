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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComplexityMetrics;
    use crate::risk::priority::module_detection::ModuleType;
    use crate::risk::priority::TestTarget;

    #[test]
    fn test_module_description_entry_point_with_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::EntryPoint, true);
        assert_eq!(result, "Critical entry point");
    }

    #[test]
    fn test_module_description_entry_point_without_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::EntryPoint, false);
        assert_eq!(result, "Critical entry point completely untested");
    }

    #[test]
    fn test_module_description_core_with_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Core, true);
        assert_eq!(result, "Core module");
    }

    #[test]
    fn test_module_description_core_without_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Core, false);
        assert_eq!(result, "Core module completely untested");
    }

    #[test]
    fn test_module_description_api_with_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Api, true);
        assert_eq!(result, "API module");
    }

    #[test]
    fn test_module_description_api_without_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Api, false);
        assert_eq!(result, "API module completely untested");
    }

    #[test]
    fn test_module_description_io_with_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::IO, true);
        assert_eq!(result, "I/O module");
    }

    #[test]
    fn test_module_description_io_without_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::IO, false);
        assert_eq!(result, "I/O module completely untested");
    }

    #[test]
    fn test_module_description_model_with_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Model, true);
        assert_eq!(result, "Data model");
    }

    #[test]
    fn test_module_description_model_without_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Model, false);
        assert_eq!(result, "Data model completely untested");
    }

    #[test]
    fn test_module_description_utility_with_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Utility, true);
        assert_eq!(result, "Utility module");
    }

    #[test]
    fn test_module_description_utility_without_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Utility, false);
        assert_eq!(result, "Utility module completely untested");
    }

    #[test]
    fn test_module_description_test_with_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Test, true);
        assert_eq!(result, "Test module");
    }

    #[test]
    fn test_module_description_test_without_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Test, false);
        assert_eq!(result, "Test module completely untested");
    }

    #[test]
    fn test_module_description_unknown_with_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Unknown, true);
        assert_eq!(result, "Module");
    }

    #[test]
    fn test_module_description_unknown_without_coverage() {
        let result = RationaleBuilder::module_description(&ModuleType::Unknown, false);
        assert_eq!(result, "Module completely untested");
    }

    #[test]
    fn test_describe_coverage_status_entry_point_zero_coverage() {
        let target = TestTarget {
            id: "test".to_string(),
            path: std::path::PathBuf::from("src/main.rs"),
            function: Some("main".to_string()),
            line: 1,
            module_type: ModuleType::EntryPoint,
            current_coverage: 0.0,
            current_risk: 5.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 50,
            priority_score: 10.0,
            debt_items: 0,
        };

        let result = describe_coverage_status(&target);
        assert_eq!(result, "Critical entry point with NO test coverage");
    }

    #[test]
    fn test_describe_coverage_status_core_zero_coverage() {
        let target = TestTarget {
            id: "test".to_string(),
            path: std::path::PathBuf::from("src/core.rs"),
            function: Some("process".to_string()),
            line: 1,
            module_type: ModuleType::Core,
            current_coverage: 0.0,
            current_risk: 5.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 50,
            priority_score: 10.0,
            debt_items: 0,
        };

        let result = describe_coverage_status(&target);
        assert_eq!(result, "Core module completely untested");
    }

    #[test]
    fn test_describe_coverage_status_api_zero_coverage() {
        let target = TestTarget {
            id: "test".to_string(),
            path: std::path::PathBuf::from("src/api.rs"),
            function: Some("handler".to_string()),
            line: 1,
            module_type: ModuleType::Api,
            current_coverage: 0.0,
            current_risk: 5.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 50,
            priority_score: 10.0,
            debt_items: 0,
        };

        let result = describe_coverage_status(&target);
        assert_eq!(result, "API handler with zero coverage");
    }

    #[test]
    fn test_describe_coverage_status_io_zero_coverage() {
        let target = TestTarget {
            id: "test".to_string(),
            path: std::path::PathBuf::from("src/io.rs"),
            function: Some("read_file".to_string()),
            line: 1,
            module_type: ModuleType::IO,
            current_coverage: 0.0,
            current_risk: 5.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 50,
            priority_score: 10.0,
            debt_items: 0,
        };

        let result = describe_coverage_status(&target);
        assert_eq!(result, "I/O module without any tests");
    }

    #[test]
    fn test_describe_coverage_status_other_zero_coverage() {
        let target = TestTarget {
            id: "test".to_string(),
            path: std::path::PathBuf::from("src/util.rs"),
            function: Some("helper".to_string()),
            line: 1,
            module_type: ModuleType::Utility,
            current_coverage: 0.0,
            current_risk: 5.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 50,
            priority_score: 10.0,
            debt_items: 0,
        };

        let result = describe_coverage_status(&target);
        assert_eq!(result, "Module has no test coverage");
    }

    #[test]
    fn test_describe_coverage_status_poorly_tested() {
        let target = TestTarget {
            id: "test".to_string(),
            path: std::path::PathBuf::from("src/core.rs"),
            function: Some("process".to_string()),
            line: 1,
            module_type: ModuleType::Core,
            current_coverage: 25.0,
            current_risk: 5.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 50,
            priority_score: 10.0,
            debt_items: 0,
        };

        let result = describe_coverage_status(&target);
        assert_eq!(result, "Poorly tested");
    }

    #[test]
    fn test_describe_coverage_status_moderately_tested() {
        let target = TestTarget {
            id: "test".to_string(),
            path: std::path::PathBuf::from("src/core.rs"),
            function: Some("process".to_string()),
            line: 1,
            module_type: ModuleType::Core,
            current_coverage: 45.0,
            current_risk: 5.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 50,
            priority_score: 10.0,
            debt_items: 0,
        };

        let result = describe_coverage_status(&target);
        assert_eq!(result, "Moderately tested");
    }

    #[test]
    fn test_describe_coverage_status_well_tested() {
        let target = TestTarget {
            id: "test".to_string(),
            path: std::path::PathBuf::from("src/core.rs"),
            function: Some("process".to_string()),
            line: 1,
            module_type: ModuleType::Core,
            current_coverage: 85.0,
            current_risk: 5.0,
            complexity: ComplexityMetrics::default(),
            dependencies: vec![],
            dependents: vec![],
            lines: 50,
            priority_score: 10.0,
            debt_items: 0,
        };

        let result = describe_coverage_status(&target);
        assert_eq!(result, "Well tested");
    }

    #[test]
    fn test_complexity_level_simple() {
        assert_eq!(RationaleBuilder::complexity_level(0), "Simple");
        assert_eq!(RationaleBuilder::complexity_level(3), "Simple");
        assert_eq!(RationaleBuilder::complexity_level(7), "Simple");
    }

    #[test]
    fn test_complexity_level_moderate() {
        assert_eq!(RationaleBuilder::complexity_level(8), "Moderate");
        assert_eq!(RationaleBuilder::complexity_level(12), "Moderate");
        assert_eq!(RationaleBuilder::complexity_level(15), "Moderate");
    }

    #[test]
    fn test_complexity_level_complex() {
        assert_eq!(RationaleBuilder::complexity_level(16), "Complex");
        assert_eq!(RationaleBuilder::complexity_level(23), "Complex");
        assert_eq!(RationaleBuilder::complexity_level(30), "Complex");
    }

    #[test]
    fn test_complexity_level_very_complex() {
        assert_eq!(RationaleBuilder::complexity_level(31), "Very complex");
        assert_eq!(RationaleBuilder::complexity_level(50), "Very complex");
        assert_eq!(RationaleBuilder::complexity_level(100), "Very complex");
    }

    #[test]
    fn test_effort_description_easy_win() {
        // Test case for (1..=3, 1..=7) => " - easy win"
        assert_eq!(RationaleBuilder::effort_description(1, 1), " - easy win");
        assert_eq!(RationaleBuilder::effort_description(2, 5), " - easy win");
        assert_eq!(RationaleBuilder::effort_description(3, 7), " - easy win");
    }

    #[test]
    fn test_effort_description_quick_test() {
        // Test case for (1..=5, 1..=10) => " - quick test"
        // This matches when cyclomatic is 4-5 with cognitive 1-10
        // or cyclomatic 1-3 with cognitive 8-10 (not already caught by easy win)
        assert_eq!(RationaleBuilder::effort_description(4, 8), " - quick test");
        assert_eq!(RationaleBuilder::effort_description(5, 10), " - quick test");
        assert_eq!(RationaleBuilder::effort_description(1, 9), " - quick test");
    }

    #[test]
    fn test_effort_description_moderate_effort() {
        // Test case for (6..=10, _) => " - moderate effort"
        assert_eq!(
            RationaleBuilder::effort_description(6, 1),
            " - moderate effort"
        );
        assert_eq!(
            RationaleBuilder::effort_description(8, 15),
            " - moderate effort"
        );
        assert_eq!(
            RationaleBuilder::effort_description(10, 100),
            " - moderate effort"
        );
    }

    #[test]
    fn test_effort_description_requires_effort() {
        // Test case for _ => " - requires effort"
        // This matches when cyclomatic > 10, or cyclomatic > 5 with cognitive > 10
        assert_eq!(
            RationaleBuilder::effort_description(11, 1),
            " - requires effort"
        );
        assert_eq!(
            RationaleBuilder::effort_description(15, 20),
            " - requires effort"
        );
        assert_eq!(
            RationaleBuilder::effort_description(5, 11),
            " - requires effort"
        );
        assert_eq!(
            RationaleBuilder::effort_description(0, 0),
            " - requires effort"
        );
    }

    #[test]
    fn test_describe_complexity_simple() {
        // Test max_complexity in range 0..=2 -> "simple"
        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
        };
        assert_eq!(describe_complexity(&metrics), "simple");

        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 4, // 4/2 = 2
            cognitive_complexity: 8,  // 8/4 = 2
        };
        assert_eq!(describe_complexity(&metrics), "simple");

        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 2, // 2/2 = 1
            cognitive_complexity: 4,  // 4/4 = 1
        };
        assert_eq!(describe_complexity(&metrics), "simple");
    }

    #[test]
    fn test_describe_complexity_moderately_complex() {
        // Test max_complexity in range 3..=5 -> "moderately complex"
        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 6, // 6/2 = 3
            cognitive_complexity: 12, // 12/4 = 3
        };
        assert_eq!(describe_complexity(&metrics), "moderately complex");

        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 10, // 10/2 = 5
            cognitive_complexity: 16,  // 16/4 = 4
        };
        assert_eq!(describe_complexity(&metrics), "moderately complex");

        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 8, // 8/2 = 4
            cognitive_complexity: 20, // 20/4 = 5
        };
        assert_eq!(describe_complexity(&metrics), "moderately complex");
    }

    #[test]
    fn test_describe_complexity_highly_complex() {
        // Test max_complexity in range 6..=10 -> "highly complex"
        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 12, // 12/2 = 6
            cognitive_complexity: 24,  // 24/4 = 6
        };
        assert_eq!(describe_complexity(&metrics), "highly complex");

        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 20, // 20/2 = 10
            cognitive_complexity: 32,  // 32/4 = 8
        };
        assert_eq!(describe_complexity(&metrics), "highly complex");

        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 16, // 16/2 = 8
            cognitive_complexity: 40,  // 40/4 = 10
        };
        assert_eq!(describe_complexity(&metrics), "highly complex");
    }

    #[test]
    fn test_describe_complexity_extremely_complex() {
        // Test max_complexity > 10 -> "extremely complex"
        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 22, // 22/2 = 11
            cognitive_complexity: 44,  // 44/4 = 11
        };
        assert_eq!(describe_complexity(&metrics), "extremely complex");

        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 50, // 50/2 = 25
            cognitive_complexity: 100, // 100/4 = 25
        };
        assert_eq!(describe_complexity(&metrics), "extremely complex");

        let metrics = ComplexityMetrics {
            functions: vec![],
            cyclomatic_complexity: 30, // 30/2 = 15
            cognitive_complexity: 80,  // 80/4 = 20
        };
        assert_eq!(describe_complexity(&metrics), "extremely complex");
    }
}
