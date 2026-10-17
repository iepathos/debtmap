use crate::analyzers::nix::types::NixFunction;
use crate::core::{
    ComplexityMetrics, DebtItem, Dependency, FileMetrics, FunctionMetrics, Language,
};
use std::path::PathBuf;

pub fn build_file_metrics(
    path: PathBuf,
    functions: Vec<FunctionMetrics>,
    debt_items: Vec<DebtItem>,
    dependencies: Vec<Dependency>,
    total_lines: usize,
) -> FileMetrics {
    let test_lines = crate::analysis::role_policy::estimate_test_lines(
        &path,
        Language::Nix,
        &functions,
        total_lines,
    );

    FileMetrics {
        path,
        language: Language::Nix,
        complexity: complexity_totals(functions),
        debt_items,
        dependencies,
        duplications: vec![],
        total_lines,
        test_lines,
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

pub fn to_function_metrics(function: &NixFunction) -> FunctionMetrics {
    FunctionMetrics {
        column: None,
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
        call_dependencies: None,
        detected_patterns: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn function(name: &str, cyclomatic: u32, cognitive: u32) -> NixFunction {
        NixFunction {
            name: name.to_string(),
            file: PathBuf::from("a.nix"),
            line: 1,
            length: 3,
            cyclomatic,
            cognitive,
            nesting: 0,
            is_test: false,
            visibility: Some("public".to_string()),
        }
    }

    #[test]
    fn test_function_metrics_conversion() {
        let metrics = to_function_metrics(&function("mkFoo", 3, 2));

        assert_eq!(metrics.name, "mkFoo");
        assert_eq!(metrics.cyclomatic, 3);
        assert_eq!(metrics.cognitive, 2);
        assert_eq!(metrics.visibility.as_deref(), Some("public"));
    }

    #[test]
    fn test_file_metrics_totals() {
        let functions = vec![
            to_function_metrics(&function("a", 2, 1)),
            to_function_metrics(&function("b", 3, 4)),
        ];
        let metrics = build_file_metrics(PathBuf::from("a.nix"), functions, vec![], vec![], 10);

        assert_eq!(metrics.language, Language::Nix);
        assert_eq!(metrics.complexity.cyclomatic_complexity, 5);
        assert_eq!(metrics.complexity.cognitive_complexity, 5);
        assert_eq!(metrics.total_lines, 10);
    }
}
