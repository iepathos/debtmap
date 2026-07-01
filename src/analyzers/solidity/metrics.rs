use crate::analyzers::solidity::types::SolidityFunction;
use crate::core::Dependency;
use crate::core::{
    ComplexityMetrics, DebtItem, FileMetrics, FunctionMetrics, Language, LanguageSpecificData,
    PurityLevel, SolidityPatternResult,
};
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
    let entropy_analysis = function.entropy_analysis.clone();

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
        entropy_score: entropy_analysis.as_ref().map(|analysis| {
            crate::complexity::entropy_core::EntropyScore {
                token_entropy: analysis.entropy_score,
                pattern_repetition: analysis.pattern_repetition,
                branch_similarity: analysis.branch_similarity,
                effective_complexity: analysis.dampening_factor,
                unique_variables: 0,
                max_nesting: function.nesting,
                dampening_applied: analysis.dampening_factor,
            }
        }),
        is_pure: None,
        purity_confidence: None,
        purity_reason: None,
        call_dependencies: (!function.calls.is_empty()).then_some(function.calls.clone()),
        detected_patterns,
        upstream_callers: None,
        downstream_callees: None,
        mapping_pattern_result: None,
        adjusted_complexity: entropy_analysis
            .as_ref()
            .map(|analysis| analysis.adjusted_complexity as f64),
        composition_metrics: None,
        language_specific: Some(LanguageSpecificData::Solidity(solidity_pattern_result(
            function,
        ))),
        purity_level: solidity_purity_level(function),
        error_swallowing_count: None,
        error_swallowing_patterns: None,
        entropy_analysis,
    }
}

fn solidity_pattern_result(function: &SolidityFunction) -> SolidityPatternResult {
    SolidityPatternResult {
        state_mutability: function.state_mutability.clone(),
        is_payable: function.state_mutability.as_deref() == Some("payable"),
        advisory_pattern_count: function.advisory_patterns.len(),
        uses_delegatecall: function
            .advisory_patterns
            .iter()
            .any(|pattern| pattern.contains("delegatecall")),
    }
}

fn solidity_purity_level(function: &SolidityFunction) -> Option<PurityLevel> {
    match function.state_mutability.as_deref() {
        Some("pure") => Some(PurityLevel::StrictlyPure),
        Some("view") => Some(PurityLevel::ReadOnly),
        Some("payable") => Some(PurityLevel::Impure),
        _ if function
            .advisory_patterns
            .iter()
            .any(|pattern| pattern == "external-call-before-state-update") =>
        {
            Some(PurityLevel::Impure)
        }
        _ => None,
    }
}
