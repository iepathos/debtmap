use crate::analyzers::python_ast_extraction::PythonAstExtractor;
use crate::analyzers::python_asyncio_patterns::AsyncioPatternDetector;
use crate::analyzers::python_detectors::SimplifiedPythonDetector;
use crate::analyzers::python_exception_flow::ExceptionFlowAnalyzer;
use crate::analyzers::python_purity::PythonPurityDetector;
use crate::analyzers::Analyzer;
use crate::complexity::entropy_core::{EntropyConfig, UniversalEntropyCalculator};
use crate::complexity::languages::python::PythonEntropyAnalyzer;
use crate::complexity::python_pattern_adjustments::{
    apply_adjustments, detect_patterns, detect_patterns_async,
};
use crate::complexity::python_patterns::analyze_python_patterns;
use crate::complexity::python_specific_patterns::PythonSpecificPatternDetector;
use crate::core::{
    ast::{Ast, PythonAst},
    ComplexityMetrics, DebtItem, DebtType, Dependency, DependencyKind, FileMetrics,
    FunctionMetrics, Language, Priority,
};
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::{parse_suppression_comments, SuppressionContext};
use crate::organization::python::PythonOrganizationAnalyzer;
use crate::resource::python::PythonResourceAnalyzer;
use anyhow::Result;
use rustpython_parser::ast;
use std::path::{Path, PathBuf};

pub struct PythonAnalyzer {
    complexity_threshold: u32,
}

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self {
            complexity_threshold: 10,
        }
    }
}

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for PythonAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let module = rustpython_parser::parse(content, rustpython_parser::Mode::Module, "<module>")
            .map_err(|e| anyhow::anyhow!("Python parse error: {:?}", e))?;
        Ok(Ast::Python(PythonAst { module, path }))
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::Python(python_ast) => {
                let mut metrics = analyze_python_file(python_ast, self.complexity_threshold);

                // Add simplified detector analysis
                let mut detector = SimplifiedPythonDetector::new(python_ast.path.clone());
                detector.analyze_module(&python_ast.module);

                // Add detected patterns to debt items
                metrics.debt_items.extend(detector.get_debt_items());

                // Add organization anti-pattern detection
                let source_content = std::fs::read_to_string(&python_ast.path).unwrap_or_default();
                let org_analyzer = PythonOrganizationAnalyzer::new();
                let org_patterns =
                    org_analyzer.analyze(&python_ast.module, &python_ast.path, &source_content);

                // Convert organization patterns to debt items
                for pattern in org_patterns {
                    metrics
                        .debt_items
                        .push(convert_org_pattern_to_debt_item(pattern, &python_ast.path));
                }

                // Add resource management pattern detection
                let resource_analyzer = PythonResourceAnalyzer::new();
                let resource_debt_items =
                    resource_analyzer.analyze(&python_ast.module, &python_ast.path);
                metrics.debt_items.extend(resource_debt_items);

                // Add test quality analysis
                let mut test_analyzer = crate::testing::python::analyzer::PythonTestAnalyzer::new();
                let test_issues =
                    test_analyzer.analyze_module(&python_ast.module, &python_ast.path);
                for issue in test_issues {
                    metrics.debt_items.push(
                        crate::testing::python::convert_test_issue_to_debt_item(
                            issue,
                            &python_ast.path,
                        ),
                    );
                }

                // Add Python-specific pattern complexity detection
                let mut specific_detector = PythonSpecificPatternDetector::new();
                specific_detector.detect_patterns(&python_ast.module);
                let pattern_complexity = specific_detector.calculate_pattern_complexity();

                // Add pattern complexity to overall complexity metrics
                metrics.complexity.cognitive_complexity += pattern_complexity as u32;

                // Add asyncio pattern detection
                let mut asyncio_detector = AsyncioPatternDetector::new(python_ast.path.clone());
                let asyncio_debt_items = asyncio_detector.analyze_module(&python_ast.module);
                metrics.debt_items.extend(asyncio_debt_items);

                // Add exception flow analysis
                let mut exc_flow_analyzer = ExceptionFlowAnalyzer::new(python_ast.path.clone());
                let exception_patterns = exc_flow_analyzer.analyze_module(&python_ast.module);
                let exception_debt_items =
                    exc_flow_analyzer.patterns_to_debt_items(exception_patterns);
                metrics.debt_items.extend(exception_debt_items);

                // Add static error detection (undefined variables, missing imports)
                use crate::analysis::python_imports::EnhancedImportResolver;
                use crate::analysis::python_static_errors::{analyze_static_errors, errors_to_debt_items};

                let import_resolver = EnhancedImportResolver::new();
                let static_analysis_result = analyze_static_errors(&python_ast.module, &import_resolver);
                let static_error_debt_items = errors_to_debt_items(&static_analysis_result, &python_ast.path);
                metrics.debt_items.extend(static_error_debt_items);

                metrics
            }
            _ => FileMetrics {
                path: PathBuf::new(),
                language: Language::Python,
                complexity: ComplexityMetrics::default(),
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
                module_scope: None,
                classes: None,
            },
        }
    }

    fn language(&self) -> Language {
        Language::Python
    }
}

fn convert_org_pattern_to_debt_item(
    pattern: crate::organization::OrganizationAntiPattern,
    path: &Path,
) -> DebtItem {
    use crate::organization::OrganizationAntiPattern;

    let (debt_type, message) = match &pattern {
        OrganizationAntiPattern::GodObject {
            type_name,
            method_count,
            field_count,
            ..
        } => (
            DebtType::Complexity,
            format!(
                "God object '{}' with {} methods and {} fields",
                type_name, method_count, field_count
            ),
        ),
        OrganizationAntiPattern::FeatureEnvy {
            method_name,
            envied_type,
            external_calls,
            internal_calls,
            ..
        } => (
            DebtType::CodeOrganization,
            format!(
                "Feature envy in '{}': {} calls to '{}' vs {} internal calls",
                method_name, external_calls, envied_type, internal_calls
            ),
        ),
        OrganizationAntiPattern::MagicValue {
            value,
            occurrence_count,
            suggested_constant_name,
            ..
        } => (
            DebtType::CodeSmell,
            format!(
                "Magic value '{}' appears {} times, suggest constant '{}'",
                value, occurrence_count, suggested_constant_name
            ),
        ),
        OrganizationAntiPattern::LongParameterList {
            function_name,
            parameter_count,
            ..
        } => (
            DebtType::Complexity,
            format!(
                "Long parameter list in '{}' with {} parameters",
                function_name, parameter_count
            ),
        ),
        OrganizationAntiPattern::PrimitiveObsession {
            primitive_type,
            occurrence_count,
            suggested_domain_type,
            ..
        } => (
            DebtType::CodeOrganization,
            format!(
                "Primitive obsession: '{}' used {} times, suggest {}",
                primitive_type, occurrence_count, suggested_domain_type
            ),
        ),
        OrganizationAntiPattern::DataClump {
            parameter_group,
            occurrence_count,
            suggested_struct_name,
            ..
        } => (
            DebtType::CodeOrganization,
            format!(
                "Data clump with {} parameters appears {} times, suggest struct '{}'",
                parameter_group.parameters.len(),
                occurrence_count,
                suggested_struct_name
            ),
        ),
    };

    let location = pattern.primary_location();

    DebtItem {
        id: format!("org-{}-{}", debt_type, location.line),
        file: path.to_path_buf(),
        line: location.line,
        column: location.column,
        debt_type,
        message,
        priority: Priority::Medium,
        context: Some("organization".to_string()),
    }
}

fn analyze_python_file(ast: &PythonAst, threshold: u32) -> FileMetrics {
    let source_content = std::fs::read_to_string(&ast.path).unwrap_or_default();
    let mut entropy_calculator = UniversalEntropyCalculator::new(EntropyConfig::default());

    // Use TwoPassExtractor for two-pass analysis
    use crate::analysis::python_type_tracker::TwoPassExtractor;
    let mut extractor = TwoPassExtractor::new_with_source(ast.path.clone(), &source_content);

    // Phase 1: Register all functions and extract call relationships
    extractor.extract(&ast.module);

    // Get the extracted call graph
    let call_graph = extractor.get_call_graph();

    // Extract basic function metrics as before
    let mut functions = extract_function_metrics(
        &ast.module,
        &ast.path,
        &source_content,
        &mut entropy_calculator,
    );

    // Populate call graph data into function metrics
    use crate::analyzers::call_graph_integration::populate_call_graph_data;
    functions = populate_call_graph_data(functions, &call_graph);

    let debt_items = create_python_debt_items(
        &ast.module,
        &ast.path,
        threshold,
        &functions,
        &source_content,
    );
    let dependencies = extract_dependencies(&ast.module);

    let (cyclomatic, cognitive) = functions.iter().fold((0, 0), |(cyc, cog), f| {
        (cyc + f.cyclomatic, cog + f.cognitive)
    });

    // Extract AST pattern information using PythonAstExtractor
    let ast_extractor = PythonAstExtractor::new();
    let module_scope = Some(ast_extractor.extract_module_scope(&ast.module));
    let classes = Some(ast_extractor.extract_classes(&ast.module));

    FileMetrics {
        path: ast.path.clone(),
        language: Language::Python,
        complexity: ComplexityMetrics {
            functions,
            cyclomatic_complexity: cyclomatic,
            cognitive_complexity: cognitive,
        },
        debt_items,
        dependencies,
        duplications: vec![],
        module_scope,
        classes,
    }
}

fn create_python_debt_items(
    module: &ast::Mod,
    path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
) -> Vec<DebtItem> {
    let suppression_context = parse_suppression_comments(source_content, Language::Python, path);

    report_unclosed_blocks(&suppression_context);

    collect_all_debt_items(
        module,
        path,
        threshold,
        functions,
        source_content,
        &suppression_context,
    )
}

fn collect_all_debt_items(
    module: &ast::Mod,
    path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
    source_content: &str,
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    [
        extract_debt_items(module, path, threshold, functions),
        find_todos_and_fixmes_with_suppression(source_content, path, Some(suppression_context)),
        find_code_smells_with_suppression(source_content, path, Some(suppression_context)),
        extract_module_smell_items(path, source_content, suppression_context),
        extract_function_smell_items(module, functions, suppression_context),
        crate::debt::python_error_handling::detect_error_swallowing(
            module,
            path,
            Some(suppression_context),
        ),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn extract_module_smell_items(
    path: &Path,
    source_content: &str,
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    analyze_module_smells(path, source_content.lines().count())
        .into_iter()
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect()
}

fn extract_function_smell_items(
    module: &ast::Mod,
    functions: &[FunctionMetrics],
    suppression_context: &SuppressionContext,
) -> Vec<DebtItem> {
    functions
        .iter()
        .flat_map(|func| {
            let param_count = count_python_params(module, &func.name);
            analyze_function_smells(func, param_count)
        })
        .map(|smell| smell.to_debt_item())
        .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
        .collect()
}

fn report_unclosed_blocks(suppression_context: &SuppressionContext) {
    suppression_context
        .unclosed_blocks
        .iter()
        .for_each(|unclosed| {
            eprintln!(
                "Warning: Unclosed suppression block in {} at line {}",
                unclosed.file.display(),
                unclosed.start_line
            );
        });
}

fn extract_function_metrics(
    module: &ast::Mod,
    path: &Path,
    source_content: &str,
    entropy_calculator: &mut UniversalEntropyCalculator,
) -> Vec<FunctionMetrics> {
    let ast::Mod::Module(module) = module else {
        return Vec::new();
    };

    let lines: Vec<&str> = source_content.lines().collect();
    let mut functions = Vec::new();

    // Recursively extract functions from the module with no class context
    extract_functions_from_stmts(
        &module.body,
        path,
        &lines,
        &mut functions,
        0,
        None,
        source_content,
        entropy_calculator,
    );

    functions
}

#[allow(clippy::only_used_in_recursion, clippy::too_many_arguments)]
fn extract_functions_from_stmts(
    stmts: &[ast::Stmt],
    path: &Path,
    lines: &[&str],
    functions: &mut Vec<FunctionMetrics>,
    stmt_offset: usize,
    class_context: Option<&str>,
    source: &str,
    entropy_calculator: &mut UniversalEntropyCalculator,
) {
    use crate::testing::python::test_detector::{PythonTestDetector, TestContext};

    let test_detector = PythonTestDetector::new();
    let is_test_file = test_detector.is_test_file(path);
    let is_test_class = class_context
        .map(|c| c.starts_with("Test") || c.ends_with("Test") || c.ends_with("Tests"))
        .unwrap_or(false);

    let mut test_context = TestContext::new().with_test_file(is_test_file);
    if let Some(class_name) = class_context {
        test_context = test_context.with_class(class_name.to_string(), is_test_class);
    }
    for (idx, stmt) in stmts.iter().enumerate() {
        match stmt {
            ast::Stmt::FunctionDef(func_def) => {
                let line_number =
                    estimate_line_number(lines, func_def.name.as_ref(), stmt_offset + idx);

                // Include class context in function name if inside a class
                let function_name = if let Some(class_name) = class_context {
                    format!("{}.{}", class_name, func_def.name)
                } else {
                    func_def.name.to_string()
                };

                let entropy_score =
                    calculate_function_entropy(&func_def.body, source, entropy_calculator);

                // Analyze function purity
                let mut purity_detector = PythonPurityDetector::new();
                let purity_analysis = purity_detector.analyze_function(func_def);

                // Calculate base complexity metrics
                let base_cyclomatic = calculate_cyclomatic_python(&func_def.body);
                let base_cognitive = calculate_cognitive_python(&func_def.body);

                // Detect patterns and apply adjustments
                let patterns = detect_patterns(func_def);
                let adjusted_cyclomatic = apply_adjustments(base_cyclomatic, &patterns);
                let adjusted_cognitive = apply_adjustments(base_cognitive, &patterns);

                // Extract pattern descriptions for reporting
                let detected_patterns = if !patterns.is_empty() {
                    Some(patterns.iter().map(|p| p.description.clone()).collect())
                } else {
                    None
                };

                // Detect if this is a test function using comprehensive patterns
                let test_result = test_detector.detect_test(func_def, &test_context);

                functions.push(FunctionMetrics {
                    name: function_name,
                    file: path.to_path_buf(),
                    line: line_number,
                    cyclomatic: adjusted_cyclomatic,
                    cognitive: adjusted_cognitive,
                    nesting: calculate_nesting_python(&func_def.body),
                    length: func_def.body.len(),
                    is_test: test_result.is_test,
                    visibility: None, // Python doesn't have explicit visibility modifiers
                    is_trait_method: false, // Python doesn't have traits like Rust
                    in_test_module: is_test_file,
                    entropy_score,
                    is_pure: Some(purity_analysis.is_pure),
                    purity_confidence: Some(purity_analysis.confidence),
                    detected_patterns,
                    upstream_callers: None,
                    downstream_callees: None,
                });

                // Recursively look for nested functions
                extract_functions_from_stmts(
                    &func_def.body,
                    path,
                    lines,
                    functions,
                    stmt_offset + idx,
                    class_context,
                    source,
                    entropy_calculator,
                );
            }
            ast::Stmt::AsyncFunctionDef(func_def) => {
                let line_number =
                    estimate_line_number(lines, func_def.name.as_ref(), stmt_offset + idx);

                // Include class context in function name if inside a class
                let function_name = if let Some(class_name) = class_context {
                    format!("{}.async {}", class_name, func_def.name)
                } else {
                    format!("async {}", func_def.name)
                };

                let entropy_score =
                    calculate_function_entropy(&func_def.body, source, entropy_calculator);

                // Analyze async function purity
                let mut purity_detector = PythonPurityDetector::new();
                let purity_analysis = purity_detector.analyze_async_function(func_def);

                // Calculate base complexity metrics
                let base_cyclomatic = calculate_cyclomatic_python(&func_def.body);
                let base_cognitive = calculate_cognitive_python(&func_def.body);

                // Detect patterns and apply adjustments
                let patterns = detect_patterns_async(func_def);
                let adjusted_cyclomatic = apply_adjustments(base_cyclomatic, &patterns);
                let adjusted_cognitive = apply_adjustments(base_cognitive, &patterns);

                // Extract pattern descriptions for reporting
                let detected_patterns = if !patterns.is_empty() {
                    Some(patterns.iter().map(|p| p.description.clone()).collect())
                } else {
                    None
                };

                // Detect if this is an async test function using comprehensive patterns
                let test_result = test_detector.detect_async_test(func_def, &test_context);

                functions.push(FunctionMetrics {
                    name: function_name,
                    file: path.to_path_buf(),
                    line: line_number,
                    cyclomatic: adjusted_cyclomatic,
                    cognitive: adjusted_cognitive,
                    nesting: calculate_nesting_python(&func_def.body),
                    length: func_def.body.len(),
                    is_test: test_result.is_test,
                    visibility: None, // Python doesn't have explicit visibility modifiers
                    is_trait_method: false, // Python doesn't have traits like Rust
                    in_test_module: is_test_file,
                    entropy_score,
                    is_pure: Some(purity_analysis.is_pure),
                    purity_confidence: Some(purity_analysis.confidence),
                    detected_patterns,
                    upstream_callers: None,
                    downstream_callees: None,
                });

                // Recursively look for nested functions
                extract_functions_from_stmts(
                    &func_def.body,
                    path,
                    lines,
                    functions,
                    stmt_offset + idx,
                    class_context,
                    source,
                    entropy_calculator,
                );
            }
            ast::Stmt::ClassDef(class_def) => {
                // Look for methods in classes - pass the class name as context
                extract_functions_from_stmts(
                    &class_def.body,
                    path,
                    lines,
                    functions,
                    stmt_offset + idx,
                    Some(class_def.name.as_ref()),
                    source,
                    entropy_calculator,
                );
            }
            _ => {}
        }
    }
}

fn calculate_function_entropy(
    body: &[ast::Stmt],
    source: &str,
    calculator: &mut UniversalEntropyCalculator,
) -> Option<crate::complexity::entropy_core::EntropyScore> {
    let analyzer = PythonEntropyAnalyzer::new(source);
    let score = calculator.calculate(&analyzer, &body.to_vec());
    Some(score)
}

fn estimate_line_number(lines: &[&str], func_name: &str, _stmt_idx: usize) -> usize {
    let def_pattern = format!("def {func_name}");
    lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.trim_start().starts_with(&def_pattern))
        .map(|(idx, _)| idx + 1) // Line numbers are 1-based
        .unwrap_or(1) // Default to line 1 if not found
}

fn count_python_params(module: &ast::Mod, func_name: &str) -> usize {
    let ast::Mod::Module(module) = module else {
        return 0;
    };

    module
        .body
        .iter()
        .find_map(|stmt| match stmt {
            ast::Stmt::FunctionDef(func_def) if func_def.name.to_string() == func_name => {
                Some(func_def.args.args.len())
            }
            _ => None,
        })
        .unwrap_or(0)
}

fn calculate_cyclomatic_python(body: &[ast::Stmt]) -> u32 {
    1 + body.iter().map(count_branches_stmt).sum::<u32>()
}

fn count_branches_stmt(stmt: &ast::Stmt) -> u32 {
    use ast::Stmt::*;

    match stmt {
        If(if_stmt) => count_if_branches(if_stmt),
        While(while_stmt) => count_loop_branches(&while_stmt.body),
        For(for_stmt) => count_loop_branches(&for_stmt.body),
        Try(try_stmt) => count_try_branches(try_stmt),
        With(with_stmt) => count_body_branches(&with_stmt.body),
        Match(match_stmt) => count_match_branches(match_stmt),
        _ => 0,
    }
}

fn count_if_branches(if_stmt: &ast::StmtIf) -> u32 {
    let base_count = 1;
    let body_count = count_body_branches(&if_stmt.body);
    let else_count = count_else_branches(&if_stmt.orelse);

    base_count + body_count + else_count
}

fn count_else_branches(orelse: &[ast::Stmt]) -> u32 {
    if orelse.is_empty() {
        return 0;
    }

    let is_elif = matches!(orelse.first(), Some(ast::Stmt::If(_)));
    let else_branch_count = if is_elif { 0 } else { 1 };
    let nested_count = count_body_branches(orelse);

    else_branch_count + nested_count
}

fn count_loop_branches(body: &[ast::Stmt]) -> u32 {
    1 + count_body_branches(body)
}

fn count_try_branches(try_stmt: &ast::StmtTry) -> u32 {
    let handler_count = try_stmt.handlers.len() as u32;
    let body_count = count_body_branches(&try_stmt.body);
    handler_count + body_count
}

fn count_body_branches(body: &[ast::Stmt]) -> u32 {
    body.iter().map(count_branches_stmt).sum()
}

fn count_match_branches(match_stmt: &ast::StmtMatch) -> u32 {
    match_stmt.cases.len().saturating_sub(1) as u32
}

fn calculate_cognitive_python(body: &[ast::Stmt]) -> u32 {
    let mut nesting = 0;
    let base_cognitive: u32 = body
        .iter()
        .map(|stmt| calculate_cognitive_stmt(stmt, &mut nesting))
        .sum();

    // Add pattern-based complexity
    let patterns = analyze_python_patterns(body);
    base_cognitive + patterns.total_complexity()
}

fn calculate_cognitive_stmt(stmt: &ast::Stmt, nesting: &mut u32) -> u32 {
    let bodies = extract_stmt_bodies(stmt);
    if bodies.is_empty() {
        return 0;
    }

    let base_cognitive = 1 + *nesting;
    *nesting += 1;
    let body_cognitive = bodies
        .into_iter()
        .flatten()
        .map(|s| calculate_cognitive_stmt(s, nesting))
        .sum::<u32>();
    *nesting -= 1;
    base_cognitive + body_cognitive
}

fn calculate_nesting_python(body: &[ast::Stmt]) -> u32 {
    body.iter()
        .map(|stmt| calculate_nesting_stmt(stmt, 0))
        .max()
        .unwrap_or(0)
}

fn calculate_nesting_stmt(stmt: &ast::Stmt, current_depth: u32) -> u32 {
    let bodies = extract_stmt_bodies(stmt);
    if bodies.is_empty() {
        return current_depth;
    }

    let next_depth = current_depth + 1;
    bodies
        .into_iter()
        .flatten()
        .map(|s| calculate_nesting_stmt(s, next_depth))
        .max()
        .unwrap_or(next_depth)
}

fn extract_stmt_bodies(stmt: &ast::Stmt) -> Vec<&[ast::Stmt]> {
    match stmt {
        ast::Stmt::If(if_stmt) => vec![&if_stmt.body[..], &if_stmt.orelse[..]],
        ast::Stmt::While(while_stmt) => vec![&while_stmt.body[..]],
        ast::Stmt::For(for_stmt) => vec![&for_stmt.body[..]],
        _ => vec![],
    }
}

fn extract_debt_items(
    _module: &ast::Mod,
    _path: &Path,
    threshold: u32,
    functions: &[FunctionMetrics],
) -> Vec<DebtItem> {
    functions
        .iter()
        .filter(|func| func.is_complex(threshold))
        .map(|func| create_python_complexity_debt_item(func, threshold))
        .collect()
}

fn create_python_complexity_debt_item(func: &FunctionMetrics, threshold: u32) -> DebtItem {
    DebtItem {
        id: format!("complexity-{}-{}", func.file.display(), func.line),
        debt_type: DebtType::Complexity,
        priority: if func.cyclomatic > threshold * 2 {
            Priority::High
        } else {
            Priority::Medium
        },
        file: func.file.clone(),
        line: func.line,
        column: None,
        message: format!(
            "Function '{}' has high complexity (cyclomatic: {}, cognitive: {})",
            func.name, func.cyclomatic, func.cognitive
        ),
        context: None,
    }
}

fn extract_dependencies(module: &ast::Mod) -> Vec<Dependency> {
    let ast::Mod::Module(module) = module else {
        return Vec::new();
    };

    module
        .body
        .iter()
        .flat_map(extract_stmt_dependencies)
        .collect()
}

fn extract_stmt_dependencies(stmt: &ast::Stmt) -> Vec<Dependency> {
    match stmt {
        ast::Stmt::Import(import) => extract_import_dependencies(import),
        ast::Stmt::ImportFrom(import_from) => extract_import_from_dependencies(import_from),
        _ => Vec::new(),
    }
}

fn extract_import_dependencies(import: &ast::StmtImport) -> Vec<Dependency> {
    import.names.iter().map(create_import_dependency).collect()
}

fn extract_import_from_dependencies(import_from: &ast::StmtImportFrom) -> Vec<Dependency> {
    import_from
        .module
        .as_ref()
        .map(create_module_dependency)
        .into_iter()
        .collect()
}

fn create_import_dependency(alias: &ast::Alias) -> Dependency {
    Dependency {
        name: alias.name.to_string(),
        kind: DependencyKind::Import,
    }
}

fn create_module_dependency(module: &ast::Identifier) -> Dependency {
    Dependency {
        name: module.to_string(),
        kind: DependencyKind::Module,
    }
}
