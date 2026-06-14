use crate::analyzers::go::types::GoFunction;
use crate::core::Dependency;
use crate::core::{ComplexityMetrics, DebtItem, FileMetrics, FunctionMetrics, Language};
use std::path::PathBuf;

pub fn build_file_metrics(
    path: PathBuf,
    functions: Vec<FunctionMetrics>,
    debt_items: Vec<DebtItem>,
    dependencies: Vec<Dependency>,
    total_lines: usize,
) -> FileMetrics {
    let complexity = complexity_totals(functions);

    FileMetrics {
        path,
        language: Language::Go,
        complexity,
        debt_items,
        dependencies,
        duplications: vec![],
        total_lines,
        module_scope: None,
        classes: None,
    }
}

fn complexity_totals(functions: Vec<FunctionMetrics>) -> ComplexityMetrics {
    let cyclomatic_complexity = functions.iter().map(|f| f.cyclomatic).sum();
    let cognitive_complexity = functions.iter().map(|f| f.cognitive).sum();

    ComplexityMetrics {
        functions,
        cyclomatic_complexity,
        cognitive_complexity,
    }
}

pub fn to_function_metrics(function: &GoFunction) -> FunctionMetrics {
    let detected_patterns = detected_patterns(function);

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
        is_pure: Some(is_pure(function.purity_level)),
        purity_confidence: Some(function.purity_confidence),
        purity_reason: purity_reason(function),
        call_dependencies: (!function.calls.is_empty()).then_some(function.calls.clone()),
        detected_patterns: (!detected_patterns.is_empty()).then_some(detected_patterns),
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: None,
        composition_metrics: None,
        language_specific: None,
        purity_level: Some(function.purity_level),
        error_swallowing_count: (function.error_swallowing_count > 0)
            .then_some(function.error_swallowing_count),
        error_swallowing_patterns: (!function.error_swallowing_patterns.is_empty())
            .then_some(function.error_swallowing_patterns.clone()),
        entropy_analysis: None,
    }
}

fn detected_patterns(function: &GoFunction) -> Vec<String> {
    let mut patterns = function.purity_patterns.clone();
    patterns.extend(function.advisory_patterns.clone());
    patterns.sort();
    patterns.dedup();
    patterns
}

fn purity_reason(function: &GoFunction) -> Option<String> {
    (!function.purity_patterns.is_empty()).then(|| function.purity_patterns.join(", "))
}

fn is_pure(level: crate::core::PurityLevel) -> bool {
    matches!(
        level,
        crate::core::PurityLevel::StrictlyPure | crate::core::PurityLevel::LocallyPure
    )
}
