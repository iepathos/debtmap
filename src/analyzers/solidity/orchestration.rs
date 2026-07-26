use crate::analyzers::solidity::debt::detect_debt_with_extraction;
use crate::analyzers::solidity::extraction::extract_solidity;
use crate::analyzers::solidity::generated::is_vendor_or_generated_solidity;
use crate::analyzers::solidity::metrics::{build_file_metrics, to_function_metrics};
use crate::analyzers::solidity::visitor::analyze_extracted;
use crate::config::{GeneratedCodeMode, SolidityLanguageConfig};
use crate::core::FileMetrics;
use crate::core::ast::SolidityAst;

pub fn analyze_solidity_file(
    ast: &SolidityAst,
    threshold: u32,
    config: &SolidityLanguageConfig,
) -> FileMetrics {
    let extraction = extract_solidity(ast);
    let analysis = analyze_extracted(ast, config, &extraction);
    let functions: Vec<_> = analysis.functions.iter().map(to_function_metrics).collect();
    let skip_debt = analysis.is_test_file || suppress_vendor_debt(ast, config);
    let debt_items = detect_debt_with_extraction(
        &ast.path,
        threshold,
        &functions,
        ast,
        &extraction,
        skip_debt,
        config,
    );
    let classes = (!analysis.contracts.is_empty()).then_some(
        analysis
            .contracts
            .iter()
            .map(|contract| contract.to_class_def())
            .collect(),
    );

    build_file_metrics(
        ast.path.clone(),
        functions,
        debt_items,
        extraction.dependencies,
        ast.source.lines().count(),
        classes,
    )
}

fn suppress_vendor_debt(ast: &SolidityAst, config: &SolidityLanguageConfig) -> bool {
    config.vendor_code == GeneratedCodeMode::SuppressDebt
        && is_vendor_or_generated_solidity(&ast.path, &ast.source)
}
