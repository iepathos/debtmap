//! Unified extraction for Python source files.
//!
//! This module implements the single-pass extraction for Python files using tree-sitter.

use crate::complexity::entropy_core::EntropyConfig;
use crate::core::ast::PythonAst;
use crate::extraction::types::{
    CallSite, CallType, DetectedPattern, ExtractedFileData, ExtractedFunctionData,
    ExtractedImplData, ExtractedStructData, FieldInfo, ImportInfo, IoOperation, IoType, MethodInfo,
};
use anyhow::Result;
use std::path::Path;
use tree_sitter::Node;

/// Extractor for Python source files.
pub struct PythonExtractor<'a> {
    source: &'a str,
    path: &'a Path,
    test_lines: usize,
    entropy_config: EntropyConfig,
}

impl<'a> PythonExtractor<'a> {
    pub fn new(source: &'a str, path: &'a Path) -> Self {
        Self {
            source,
            path,
            test_lines: 0,
            entropy_config: EntropyConfig::default(),
        }
    }

    pub fn with_entropy_config(mut self, config: EntropyConfig) -> Self {
        self.entropy_config = config;
        self
    }

    pub fn extract(ast: &'a PythonAst) -> Result<ExtractedFileData> {
        let mut extractor = Self::new(&ast.source, &ast.path);
        extractor.extract_from_tree(&ast.tree)
    }

    fn extract_from_tree(&mut self, tree: &tree_sitter::Tree) -> Result<ExtractedFileData> {
        let mut data = ExtractedFileData::empty(self.path.to_path_buf());
        data.total_lines = self.source.lines().count();

        let root_node = tree.root_node();
        self.traverse(root_node, &mut data, None)?;

        data.test_lines = self.test_lines;

        // Detect patterns from extracted data (Spec 204)
        data.detected_patterns = self.detect_patterns_from_extracted(&data);

        Ok(data)
    }

    fn traverse(
        &mut self,
        node: Node,
        data: &mut ExtractedFileData,
        current_class: Option<&str>,
    ) -> Result<()> {
        let kind = node.kind();

        match kind {
            "function_definition" | "async_function_definition" => {
                let func_data = self.extract_function(node, current_class)?;
                if func_data.is_test || func_data.in_test_module {
                    self.test_lines += func_data.length;
                }
                data.functions.push(func_data);
            }
            "class_definition" => {
                let class_name = self.extract_class_name(node)?;
                let struct_data = self.extract_struct(node, &class_name)?;
                data.structs.push(struct_data);

                // For Python, we'll also create an "impl" for the class methods
                let (impl_data, methods) = self.extract_impl(node, &class_name)?;
                data.impls.push(impl_data);
                data.functions.extend(methods);

                // Note: We don't recurse into class body here because extract_impl handles it
            }
            "import_statement" | "import_from_statement" => {
                let imports = self.extract_imports(node)?;
                data.imports.extend(imports);
            }
            _ => {
                // Recurse into children
                let mut cursor = node.walk();
                if cursor.goto_first_child() {
                    loop {
                        self.traverse(cursor.node(), data, current_class)?;
                        if !cursor.goto_next_sibling() {
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn extract_function(
        &self,
        node: Node,
        class_name: Option<&str>,
    ) -> Result<ExtractedFunctionData> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| anyhow::anyhow!("Function missing name"))?;
        let name = self.node_text(name_node).to_string();
        let qualified_name = class_name
            .map(|c| format!("{}.{}", c, name))
            .unwrap_or_else(|| name.clone());

        let line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;
        let length = end_line.saturating_sub(line) + 1;

        // Simple complexity calculation (count branches)
        let (cyclomatic, cognitive, nesting) = self.calculate_complexity(node);

        // Parameter names for purity analysis
        let parameter_names = self.extract_parameters(node);

        // Entropy analysis (Spec 211)
        let entropy_score = node.child_by_field_name("body").map(|body| {
            crate::analyzers::python::calculate_entropy(&body, self.source, &self.entropy_config)
        });

        // Purity analysis (Spec 214)
        let purity_analysis = node
            .child_by_field_name("body")
            .map(|body| {
                crate::analyzers::python::purity::PythonPurityAnalyzer::analyze(
                    &body,
                    self.source,
                    parameter_names.clone(),
                )
            })
            .unwrap_or_default();

        Ok(ExtractedFunctionData {
            name: name.clone(),
            qualified_name,
            line,
            end_line,
            length,
            cyclomatic,
            cognitive,
            nesting,
            entropy_score,
            purity_analysis,
            io_operations: self.extract_io_operations(node),
            parameter_names,
            transformation_patterns: Vec::new(),
            calls: self.extract_calls(node),
            is_test: name.starts_with("test_"),
            is_async: node.kind() == "async_function_definition",
            visibility: if name.starts_with('_') {
                None
            } else {
                Some("pub".to_string())
            },
            is_trait_method: false,
            in_test_module: self.path.to_string_lossy().contains("test"),
        })
    }

    fn extract_class_name(&self, node: Node) -> Result<String> {
        let name_node = node
            .child_by_field_name("name")
            .ok_or_else(|| anyhow::anyhow!("Class missing name"))?;
        Ok(self.node_text(name_node).to_string())
    }

    fn extract_struct(&self, node: Node, name: &str) -> Result<ExtractedStructData> {
        let line = node.start_position().row + 1;

        // Extract fields (attributes assigned in __init__)
        let mut fields = Vec::new();
        // This is a bit complex for a single pass, but we can look for assignments to self.x
        self.find_fields(node, &mut fields);

        Ok(ExtractedStructData {
            name: name.to_string(),
            line,
            fields,
            is_public: !name.starts_with('_'),
        })
    }

    fn extract_impl(
        &mut self,
        node: Node,
        class_name: &str,
    ) -> Result<(ExtractedImplData, Vec<ExtractedFunctionData>)> {
        let line = node.start_position().row + 1;
        let mut methods = Vec::new();
        let mut method_infos = Vec::new();

        let body = node
            .child_by_field_name("body")
            .ok_or_else(|| anyhow::anyhow!("Class missing body"))?;

        let mut cursor = body.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "function_definition"
                    || child.kind() == "async_function_definition"
                {
                    let func_data = self.extract_function(child, Some(class_name))?;
                    if func_data.is_test || func_data.in_test_module {
                        self.test_lines += func_data.length;
                    }
                    method_infos.push(MethodInfo {
                        name: func_data.name.clone(),
                        line: func_data.line,
                        is_public: func_data.visibility.is_some(),
                    });
                    methods.push(func_data);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        let impl_data = ExtractedImplData {
            type_name: class_name.to_string(),
            trait_name: None, // Python doesn't have formal traits in the same way
            methods: method_infos,
            line,
        };

        Ok((impl_data, methods))
    }

    fn extract_imports(&self, node: Node) -> Result<Vec<ImportInfo>> {
        let mut imports = Vec::new();
        let kind = node.kind();

        if kind == "import_statement" {
            // import a, b as c
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "dotted_name" {
                    imports.push(ImportInfo {
                        path: self.node_text(child).to_string(),
                        alias: None,
                        is_glob: false,
                    });
                } else if child.kind() == "aliased_import" {
                    let name_node = child.child_by_field_name("name").unwrap();
                    let alias_node = child.child_by_field_name("alias").unwrap();
                    imports.push(ImportInfo {
                        path: self.node_text(name_node).to_string(),
                        alias: Some(self.node_text(alias_node).to_string()),
                        is_glob: false,
                    });
                }
            }
        } else if kind == "import_from_statement" {
            // from a.b import c, d as e, *
            let module_node = node.child_by_field_name("module_name");
            let module_path = module_node.map(|n| self.node_text(n)).unwrap_or("");

            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "dotted_name" | "aliased_import" => {
                        let (name, alias) = if child.kind() == "aliased_import" {
                            let name_node = child.child_by_field_name("name").unwrap();
                            let alias_node = child.child_by_field_name("alias").unwrap();
                            (
                                self.node_text(name_node),
                                Some(self.node_text(alias_node).to_string()),
                            )
                        } else {
                            (self.node_text(child), None)
                        };

                        imports.push(ImportInfo {
                            path: if module_path.is_empty() {
                                name.to_string()
                            } else {
                                format!("{}.{}", module_path, name)
                            },
                            alias,
                            is_glob: false,
                        });
                    }
                    "wildcard_import" => {
                        imports.push(ImportInfo {
                            path: format!("{}.*", module_path),
                            alias: None,
                            is_glob: true,
                        });
                    }
                    _ => {}
                }
            }
        }

        Ok(imports)
    }

    fn calculate_complexity(&self, node: Node) -> (u32, u32, u32) {
        let mut cyclomatic = 1;
        let mut cognitive = 0;
        let mut max_nesting = 0;

        traverse_complexity(node, 0, &mut cyclomatic, &mut cognitive, &mut max_nesting);

        (cyclomatic, cognitive, max_nesting)
    }

    fn extract_io_operations(&self, node: Node) -> Vec<IoOperation> {
        let mut ops = Vec::new();
        self.find_io(node, &mut ops);
        ops
    }

    fn find_io(&self, node: Node, ops: &mut Vec<IoOperation>) {
        let kind = node.kind();
        if kind == "call" {
            let function_node = node.child_by_field_name("function").unwrap();
            let function_name = self.node_text(function_node);

            let io_type = match function_name {
                "print" | "input" => Some(IoType::Console),
                "open" => Some(IoType::File),
                _ if function_name.contains("socket") || function_name.contains("requests") => {
                    Some(IoType::Network)
                }
                _ => None,
            };

            if let Some(io_type) = io_type {
                ops.push(IoOperation {
                    io_type,
                    description: function_name.to_string(),
                    line: node.start_position().row + 1,
                });
            }
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.find_io(cursor.node(), ops);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn extract_parameters(&self, node: Node) -> Vec<String> {
        let mut params = Vec::new();
        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if let Some(name) = self.extract_parameter_name(child) {
                    params.push(name);
                }
            }
        }
        params
    }

    fn extract_parameter_name(&self, node: Node) -> Option<String> {
        match node.kind() {
            "identifier" => Some(self.node_text(node).to_string()),
            "typed_parameter"
            | "default_parameter"
            | "typed_default_parameter"
            | "list_splat_pattern"
            | "dictionary_splat_pattern" => node
                .child_by_field_name("name")
                .and_then(|name| self.extract_parameter_name(name))
                .or_else(|| self.find_identifier_descendant(node).map(str::to_string)),
            _ => None,
        }
    }

    fn find_identifier_descendant(&self, node: Node) -> Option<&str> {
        if node.kind() == "identifier" {
            return Some(self.node_text(node));
        }

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if let Some(name) = self.find_identifier_descendant(child) {
                return Some(name);
            }
        }

        None
    }

    fn extract_calls(&self, node: Node) -> Vec<CallSite> {
        let mut calls = Vec::new();
        self.find_calls(node, &mut calls);
        calls
    }

    fn find_calls(&self, node: Node, calls: &mut Vec<CallSite>) {
        if node.kind() == "call" {
            let func_node = node.child_by_field_name("function").unwrap();
            let (name, call_type) = match func_node.kind() {
                "identifier" => (self.node_text(func_node).to_string(), CallType::Direct),
                "attribute" => {
                    let attr_node = func_node.child_by_field_name("attribute").unwrap();
                    (self.node_text(attr_node).to_string(), CallType::Method)
                }
                _ => (self.node_text(func_node).to_string(), CallType::Direct),
            };

            calls.push(CallSite {
                callee_name: name,
                call_type,
                line: node.start_position().row + 1,
            });
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.find_calls(cursor.node(), calls);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn find_fields(&self, node: Node, fields: &mut Vec<FieldInfo>) {
        // Look for self.x = ...
        if node.kind() == "assignment" {
            let left_node = node.child_by_field_name("left").unwrap();
            if left_node.kind() == "attribute" {
                let object_node = left_node.child_by_field_name("object").unwrap();
                if self.node_text(object_node) == "self" {
                    let attr_node = left_node.child_by_field_name("attribute").unwrap();
                    let field_name = self.node_text(attr_node).to_string();
                    if !fields.iter().any(|f| f.name == field_name) {
                        fields.push(FieldInfo {
                            name: field_name,
                            type_str: "Any".to_string(), // Type inference is hard
                            is_public: !self.node_text(attr_node).starts_with('_'),
                        });
                    }
                }
            }
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.find_fields(cursor.node(), fields);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn detect_patterns_from_extracted(&self, data: &ExtractedFileData) -> Vec<DetectedPattern> {
        let mut patterns = Vec::new();

        // God object detection: structs with >5 fields
        for s in &data.structs {
            if s.fields.len() > 5 {
                patterns.push(DetectedPattern::GodObject {
                    name: s.name.clone(),
                    field_count: s.fields.len(),
                });
            }
        }

        // Function patterns (excluding test functions)
        for func in &data.functions {
            if func.is_test || func.in_test_module {
                continue;
            }

            // Long function detection: >50 lines
            if func.length > 50 {
                patterns.push(DetectedPattern::LongFunction {
                    name: func.name.clone(),
                    lines: func.length,
                });
            }

            // Many parameters detection: >5 parameters
            if func.parameter_names.len() > 5 {
                patterns.push(DetectedPattern::ManyParameters {
                    name: func.name.clone(),
                    param_count: func.parameter_names.len(),
                });
            }

            // Deep nesting detection: >4 levels
            if func.nesting > 4 {
                patterns.push(DetectedPattern::DeepNesting {
                    function_name: func.name.clone(),
                    depth: func.nesting,
                });
            }
        }

        patterns
    }

    fn node_text(&self, node: Node) -> &str {
        &self.source[node.start_byte()..node.end_byte()]
    }
}

// ============================================================================
// Complexity Helpers
// ============================================================================

fn traverse_complexity(
    node: Node,
    depth: u32,
    cyclomatic: &mut u32,
    cognitive: &mut u32,
    max_nesting: &mut u32,
) {
    let kind = node.kind();
    let mut current_depth = depth;

    match kind {
        "if_statement" | "while_statement" | "for_statement" | "with_statement"
        | "try_statement" | "match_statement" => {
            *cyclomatic += 1;
            *cognitive += 1 + depth;
            current_depth += 1;
            if current_depth > *max_nesting {
                *max_nesting = current_depth;
            }
        }
        "elif_clause" | "case_clause" | "except_clause" => {
            // elif, except, and case each add a branch
            *cyclomatic += 1;
            *cognitive += 1 + depth;
        }
        "conditional_expression" => {
            *cyclomatic += 1;
            *cognitive += 1 + depth;
        }
        "boolean_operator" => {
            // and / or increase complexity
            *cyclomatic += 1;
            *cognitive += 1;
        }
        "list_comprehension"
        | "dictionary_comprehension"
        | "set_comprehension"
        | "generator_expression" => {
            *cyclomatic += 1;
            *cognitive += 1 + depth;
            // Note: We don't increment depth here as it's a single expression
        }
        _ => {}
    }

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            traverse_complexity(
                cursor.node(),
                current_depth,
                cyclomatic,
                cognitive,
                max_nesting,
            );
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}
