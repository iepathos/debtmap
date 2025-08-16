mod complexity;
mod dependencies;

use crate::analyzers::Analyzer;
use crate::core::{
    ast::{Ast, JavaScriptAst, TypeScriptAst},
    ComplexityMetrics, DebtItem, DebtType, FileMetrics, FunctionMetrics, Language, Priority,
};
use crate::debt::patterns::{
    find_code_smells_with_suppression, find_todos_and_fixmes_with_suppression,
};
use crate::debt::smells::{analyze_function_smells, analyze_module_smells};
use crate::debt::suppression::{parse_suppression_comments, SuppressionContext};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tree_sitter::{Parser, Tree};

pub struct JavaScriptAnalyzer {
    parser: Mutex<Parser>,
    language: Language,
    complexity_threshold: u32,
}

impl JavaScriptAnalyzer {
    pub fn new_javascript() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_javascript::LANGUAGE.into())
            .context("Failed to set JavaScript language")?;
        Ok(Self {
            parser: Mutex::new(parser),
            language: Language::JavaScript,
            complexity_threshold: 10,
        })
    }

    pub fn new_typescript() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .context("Failed to set TypeScript language")?;
        Ok(Self {
            parser: Mutex::new(parser),
            language: Language::TypeScript,
            complexity_threshold: 10,
        })
    }

    fn parse_tree(&self, content: &str) -> Result<tree_sitter::Tree> {
        self.parser
            .lock()
            .unwrap()
            .parse(content, None)
            .context("Failed to parse JavaScript/TypeScript code")
    }

    fn create_ast(&self, tree: tree_sitter::Tree, source: String, path: PathBuf) -> Ast {
        match self.language {
            Language::JavaScript => Ast::JavaScript(JavaScriptAst { tree, source, path }),
            Language::TypeScript => Ast::TypeScript(TypeScriptAst { tree, source, path }),
            _ => unreachable!("JavaScriptAnalyzer should only handle JS/TS"),
        }
    }

    fn create_debt_items(
        &self,
        _tree: &tree_sitter::Tree,
        source: &str,
        path: &Path,
        functions: &[FunctionMetrics],
    ) -> Vec<DebtItem> {
        let suppression_context = parse_suppression_comments(source, self.language, path);

        // Report unclosed blocks
        report_unclosed_blocks(&suppression_context);

        // Collect all debt items using functional approach
        [
            self.collect_todos_and_fixmes(source, path, &suppression_context),
            self.collect_code_smells(source, path, &suppression_context),
            self.collect_function_smells(functions, &suppression_context),
            self.collect_module_smells(source, path, &suppression_context),
            self.collect_complexity_issues(functions, path, &suppression_context),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn collect_todos_and_fixmes(
        &self,
        source: &str,
        path: &Path,
        suppression_context: &SuppressionContext,
    ) -> Vec<DebtItem> {
        find_todos_and_fixmes_with_suppression(source, path, Some(suppression_context))
    }

    fn collect_code_smells(
        &self,
        source: &str,
        path: &Path,
        suppression_context: &SuppressionContext,
    ) -> Vec<DebtItem> {
        find_code_smells_with_suppression(source, path, Some(suppression_context))
    }

    fn collect_function_smells(
        &self,
        functions: &[FunctionMetrics],
        suppression_context: &SuppressionContext,
    ) -> Vec<DebtItem> {
        functions
            .iter()
            .flat_map(|func| analyze_function_smells(func, 0))
            .map(|smell| smell.to_debt_item())
            .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
            .collect()
    }

    fn collect_module_smells(
        &self,
        source: &str,
        path: &Path,
        suppression_context: &SuppressionContext,
    ) -> Vec<DebtItem> {
        let lines = source.lines().count();
        analyze_module_smells(path, lines)
            .into_iter()
            .map(|smell| smell.to_debt_item())
            .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
            .collect()
    }

    fn collect_complexity_issues(
        &self,
        functions: &[FunctionMetrics],
        path: &Path,
        suppression_context: &SuppressionContext,
    ) -> Vec<DebtItem> {
        functions
            .iter()
            .filter(|func| func.is_complex(self.complexity_threshold))
            .map(|func| self.create_complexity_debt_item(func, path))
            .filter(|item| !suppression_context.is_suppressed(item.line, &item.debt_type))
            .collect()
    }

    fn create_complexity_debt_item(&self, func: &FunctionMetrics, path: &Path) -> DebtItem {
        DebtItem {
            id: format!("complexity-{}-{}", path.display(), func.line),
            debt_type: DebtType::Complexity,
            priority: if func.cyclomatic > 20 || func.cognitive > 20 {
                Priority::High
            } else {
                Priority::Medium
            },
            file: path.to_path_buf(),
            line: func.line,
            message: format!(
                "Function '{}' has high complexity (cyclomatic: {}, cognitive: {})",
                func.name, func.cyclomatic, func.cognitive
            ),
            context: None,
        }
    }
}

impl JavaScriptAnalyzer {
    /// Analyzes a JavaScript or TypeScript AST to extract metrics
    fn analyze_js_ts_ast(
        &self,
        tree: &Tree,
        source: &str,
        path: &Path,
        language: Language,
    ) -> FileMetrics {
        let root_node = tree.root_node();
        let functions = complexity::extract_functions(root_node, source, path);
        let dependencies = dependencies::extract_dependencies(root_node, source);
        let debt_items = self.create_debt_items(tree, source, path, &functions);

        let (cyclomatic, cognitive) = functions.iter().fold((0, 0), |(cyc, cog), f| {
            (cyc + f.cyclomatic, cog + f.cognitive)
        });

        FileMetrics {
            path: path.to_path_buf(),
            language,
            complexity: ComplexityMetrics {
                functions,
                cyclomatic_complexity: cyclomatic,
                cognitive_complexity: cognitive,
            },
            debt_items,
            dependencies,
            duplications: vec![],
        }
    }
}

impl Analyzer for JavaScriptAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        // Parse the content directly using the already configured parser
        let tree = self.parse_tree(content)?;

        // Create the appropriate AST type based on language
        let ast = self.create_ast(tree, content.to_string(), path);
        Ok(ast)
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::JavaScript(js_ast) => self.analyze_js_ts_ast(
                &js_ast.tree,
                &js_ast.source,
                &js_ast.path,
                Language::JavaScript,
            ),
            Ast::TypeScript(ts_ast) => self.analyze_js_ts_ast(
                &ts_ast.tree,
                &ts_ast.source,
                &ts_ast.path,
                Language::TypeScript,
            ),
            _ => FileMetrics {
                path: PathBuf::new(),
                language: self.language,
                complexity: ComplexityMetrics::default(),
                debt_items: vec![],
                dependencies: vec![],
                duplications: vec![],
            },
        }
    }

    fn language(&self) -> Language {
        self.language
    }
}

fn report_unclosed_blocks(suppression_context: &SuppressionContext) {
    for unclosed in &suppression_context.unclosed_blocks {
        eprintln!(
            "Warning: Unclosed suppression block starting at line {} in {}",
            unclosed.start_line,
            unclosed.file.display()
        );
    }
}
