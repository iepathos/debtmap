use crate::analyzers::solidity::types::SolidityFunction;
use crate::core::Dependency;
use crate::core::{ComplexityMetrics, DebtItem, FileMetrics, FunctionMetrics, Language};
use std::path::PathBuf;

pub fn build_file_metrics(
    path: PathBuf,
    functions: Vec<FunctionMetrics>,
    debt_items: Vec<DebtItem>,
    dependencies: Vec<Dependency>,
    total_lines: usize,
    classes: Option<Vec<crate::core::ast::ClassDef>>,
) -> FileMetrics {
    FileMetrics {
        path,
        language: Language::Solidity,
        complexity: complexity_totals(functions),
        debt_items,
        dependencies,
        duplications: vec![],
        total_lines,
        module_scope: None,
        classes,
    }
}

fn complexity_totals(functions: Vec<FunctionMetrics>) -> ComplexityMetrics {
    ComplexityMetrics {
        cyclomatic_complexity: functions.iter().map(|f| f.cyclomatic).sum(),
        cognitive_complexity: functions.iter().map(|f| f.cognitive).sum(),
        functions,
    }
}

pub fn to_function_metrics(function: &SolidityFunction) -> FunctionMetrics {
    let detected_patterns =
        (!function.advisory_patterns.is_empty()).then_some(function.advisory_patterns.clone());

    FunctionMetrics {
        name: function.name.clone(),
        file: function.file.clone(),
        line: function.line,
        cyclomatic: function.cyclomatic,
        cognitive: function.cognitive,
        nesting: function.nesting,
        length: function.length,
        is_test: function.is_test,
        visibility: function.visibility.clone(),
        is_trait_method: false,
        in_test_module: function.is_test,
        entropy_score: None,
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: (!function.calls.is_empty()).then_some(function.calls.clone()),
        detected_patterns,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: None,
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis: None,
    }
}
