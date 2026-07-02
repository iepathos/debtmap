use crate::analyzers::solidity::types::SolidityFunction;
use crate::core::Dependency;
use crate::core::{
    ComplexityMetrics, DebtItem, FileMetrics, FunctionMetrics, Language, LanguageSpecificData,
    SolidityPatternResult,
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
        is_pure: Some(function.effects.is_strictly_pure()),
        purity_confidence: None,
        purity_reason: purity_reason(function),
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
        purity_level: Some(function.effects.purity_level()),
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
        reads_state: function.effects.reads_state,
        writes_state: function.effects.writes_state,
    }
}

fn purity_reason(function: &SolidityFunction) -> Option<String> {
    if function
        .advisory_patterns
        .iter()
        .any(|pattern| pattern == "mutability-mismatch")
    {
        return Some(
            "Declared mutability conflicts with detected state or call effects".to_string(),
        );
    }

    match function.effects.purity_level() {
        crate::core::PurityLevel::StrictlyPure => {
            Some("No state access or external effects detected".to_string())
        }
        crate::core::PurityLevel::ReadOnly => {
            Some("Reads contract state without writes".to_string())
        }
        crate::core::PurityLevel::Impure => {
            Some("Writes state or performs external/value effects".to_string())
        }
        crate::core::PurityLevel::LocallyPure => {
            Some("Mostly pure with limited local effects".to_string())
        }
    }
}
