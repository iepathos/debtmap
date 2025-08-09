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
use tree_sitter::Parser;

pub struct JavaScriptAnalyzer {
    parser: Parser,
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
            parser,
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
            parser,
            language: Language::TypeScript,
            complexity_threshold: 10,
        })
    }

    fn parse_tree(&mut self, content: &str) -> Result<tree_sitter::Tree> {
        self.parser
            .parse(content, None)
            .context("Failed to parse JavaScript/TypeScript code")
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

        let mut debt_items = Vec::new();

        // Find TODOs and FIXMEs
        debt_items.extend(find_todos_and_fixmes_with_suppression(
            source,
            path,
            Some(&suppression_context),
        ));

        // Find code smells
        debt_items.extend(find_code_smells_with_suppression(
            source,
            path,
            Some(&suppression_context),
        ));

        // Analyze function-level smells
        for func in functions {
            let smells = analyze_function_smells(func, 0); // 0 for param count (not available from tree-sitter easily)
            for smell in smells {
                let debt_item = smell.to_debt_item();
                if !suppression_context.is_suppressed(debt_item.line, &debt_item.debt_type) {
                    debt_items.push(debt_item);
                }
            }
        }

        // Analyze module-level smells
        let lines = source.lines().count();
        let smells = analyze_module_smells(path, lines);
        for smell in smells {
            let debt_item = smell.to_debt_item();
            if !suppression_context.is_suppressed(debt_item.line, &debt_item.debt_type) {
                debt_items.push(debt_item);
            }
        }

        // Check for high complexity
        for func in functions {
            if func.is_complex(self.complexity_threshold) {
                let debt_item = DebtItem {
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
                };

                // Check if this debt item is suppressed
                if !suppression_context.is_suppressed(func.line, &debt_item.debt_type) {
                    debt_items.push(debt_item);
                }
            }
        }

        debt_items
    }
}

impl Analyzer for JavaScriptAnalyzer {
    fn parse(&self, content: &str, path: PathBuf) -> Result<Ast> {
        let mut analyzer = Self {
            parser: Parser::new(),
            language: self.language,
            complexity_threshold: self.complexity_threshold,
        };

        // Set the appropriate language
        match self.language {
            Language::JavaScript => {
                analyzer
                    .parser
                    .set_language(&tree_sitter_javascript::LANGUAGE.into())
                    .context("Failed to set JavaScript language")?;
            }
            Language::TypeScript => {
                analyzer
                    .parser
                    .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                    .context("Failed to set TypeScript language")?;
            }
            _ => unreachable!("JavaScriptAnalyzer should only handle JS/TS"),
        }

        let tree = analyzer.parse_tree(content)?;
        match self.language {
            Language::JavaScript => Ok(Ast::JavaScript(JavaScriptAst {
                tree,
                source: content.to_string(),
                path,
            })),
            Language::TypeScript => Ok(Ast::TypeScript(TypeScriptAst {
                tree,
                source: content.to_string(),
                path,
            })),
            _ => unreachable!(),
        }
    }

    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::JavaScript(js_ast) => {
                let root_node = js_ast.tree.root_node();
                let functions =
                    complexity::extract_functions(root_node, &js_ast.source, &js_ast.path);
                let dependencies = dependencies::extract_dependencies(root_node, &js_ast.source);
                let debt_items =
                    self.create_debt_items(&js_ast.tree, &js_ast.source, &js_ast.path, &functions);

                FileMetrics {
                    path: js_ast.path.clone(),
                    language: Language::JavaScript,
                    complexity: ComplexityMetrics { functions },
                    debt_items,
                    dependencies,
                    duplications: vec![],
                }
            }
            Ast::TypeScript(ts_ast) => {
                let root_node = ts_ast.tree.root_node();
                let functions =
                    complexity::extract_functions(root_node, &ts_ast.source, &ts_ast.path);
                let dependencies = dependencies::extract_dependencies(root_node, &ts_ast.source);
                let debt_items =
                    self.create_debt_items(&ts_ast.tree, &ts_ast.source, &ts_ast.path, &functions);

                FileMetrics {
                    path: ts_ast.path.clone(),
                    language: Language::TypeScript,
                    complexity: ComplexityMetrics { functions },
                    debt_items,
                    dependencies,
                    duplications: vec![],
                }
            }
            _ => FileMetrics {
                path: PathBuf::new(),
                language: self.language,
                complexity: ComplexityMetrics { functions: vec![] },
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

