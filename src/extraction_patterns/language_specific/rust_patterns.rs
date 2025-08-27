use crate::extraction_patterns::{
    AccumulationOp, AnalysisContext, ComplexityImpact, ExtractablePattern, ExtractionSuggestion,
    GuardCheck, MatchedPattern, Parameter, PatternMatcher, ReturnType, TransformStage,
};
use syn::{visit::Visit, Expr, File, Pat, Stmt};

pub struct RustPatternMatcher {
    patterns: Vec<ExtractablePattern>,
}

impl Default for RustPatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl RustPatternMatcher {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    fn detect_accumulation_loops(&mut self, file: &File) {
        let mut visitor = AccumulationLoopVisitor::new();
        visitor.visit_file(file);
        self.patterns.extend(visitor.patterns);
    }

    fn detect_guard_chains(&mut self, file: &File) {
        let mut visitor = GuardChainVisitor::new();
        visitor.visit_file(file);
        self.patterns.extend(visitor.patterns);
    }

    fn detect_transformation_pipelines(&mut self, file: &File) {
        let mut visitor = PipelineVisitor::new();
        visitor.visit_file(file);
        self.patterns.extend(visitor.patterns);
    }
}

impl PatternMatcher for RustPatternMatcher {
    fn match_patterns(&self, ast: &File, _context: &AnalysisContext) -> Vec<MatchedPattern> {
        let mut matcher = Self::new();

        // Detect various patterns
        matcher.detect_accumulation_loops(ast);
        matcher.detect_guard_chains(ast);
        matcher.detect_transformation_pipelines(ast);

        // Convert to MatchedPattern
        matcher
            .patterns
            .into_iter()
            .map(|pattern| {
                MatchedPattern {
                    pattern,
                    confidence: 0.0, // Will be calculated by scorer
                    context: _context.clone(),
                }
            })
            .collect()
    }

    fn score_confidence(&self, pattern: &MatchedPattern, context: &AnalysisContext) -> f32 {
        use crate::extraction_patterns::confidence::ConfidenceScorer;
        ConfidenceScorer::score_pattern(pattern, context)
    }

    fn generate_extraction(&self, pattern: &MatchedPattern) -> ExtractionSuggestion {
        use crate::extraction_patterns::naming::FunctionNameInferrer;

        let suggested_name = FunctionNameInferrer::infer_name(&pattern.pattern, "rust");

        let (start_line, end_line) = match &pattern.pattern {
            ExtractablePattern::AccumulationLoop {
                start_line,
                end_line,
                ..
            }
            | ExtractablePattern::GuardChainSequence {
                start_line,
                end_line,
                ..
            }
            | ExtractablePattern::TransformationPipeline {
                start_line,
                end_line,
                ..
            }
            | ExtractablePattern::SimilarBranches {
                start_line,
                end_line,
                ..
            }
            | ExtractablePattern::NestedExtraction {
                start_line,
                end_line,
                ..
            } => (*start_line, *end_line),
        };

        let parameters = self.extract_parameters(&pattern.pattern);
        let return_type = self.infer_return_type(&pattern.pattern);
        let complexity_reduction =
            self.calculate_complexity_reduction(&pattern.pattern, &pattern.context);
        let example = self.generate_example(&pattern.pattern, &suggested_name);

        ExtractionSuggestion {
            pattern_type: pattern.pattern.clone(),
            start_line,
            end_line,
            suggested_name,
            confidence: pattern.confidence,
            parameters,
            return_type,
            complexity_reduction,
            example_transformation: example,
        }
    }
}

impl RustPatternMatcher {
    fn extract_parameters(&self, pattern: &ExtractablePattern) -> Vec<Parameter> {
        match pattern {
            ExtractablePattern::AccumulationLoop {
                iterator_binding, ..
            } => {
                vec![Parameter {
                    name: format!("{}_iter", iterator_binding),
                    type_hint: "impl Iterator<Item=T>".to_string(),
                    is_mutable: false,
                }]
            }
            ExtractablePattern::GuardChainSequence { .. } => {
                vec![Parameter {
                    name: "value".to_string(),
                    type_hint: "&T".to_string(),
                    is_mutable: false,
                }]
            }
            ExtractablePattern::TransformationPipeline { input_binding, .. } => {
                vec![Parameter {
                    name: input_binding.clone(),
                    type_hint: "T".to_string(),
                    is_mutable: false,
                }]
            }
            _ => Vec::new(),
        }
    }

    fn infer_return_type(&self, pattern: &ExtractablePattern) -> String {
        match pattern {
            ExtractablePattern::AccumulationLoop { operation, .. } => match operation {
                AccumulationOp::Sum | AccumulationOp::Product => "T".to_string(),
                AccumulationOp::Concatenation => "String".to_string(),
                AccumulationOp::Collection => "Vec<T>".to_string(),
                AccumulationOp::Custom(_) => "T".to_string(),
            },
            ExtractablePattern::GuardChainSequence { early_return, .. } => {
                early_return.type_name.clone()
            }
            ExtractablePattern::TransformationPipeline { output_type, .. } => output_type.clone(),
            _ => "()".to_string(),
        }
    }

    fn calculate_complexity_reduction(
        &self,
        pattern: &ExtractablePattern,
        context: &AnalysisContext,
    ) -> ComplexityImpact {
        let extracted_complexity = match pattern {
            ExtractablePattern::AccumulationLoop {
                filter, transform, ..
            } => {
                let base = 2;
                let filter_complexity = if filter.is_some() { 1 } else { 0 };
                let transform_complexity = if transform.is_some() { 1 } else { 0 };
                base + filter_complexity + transform_complexity
            }
            ExtractablePattern::GuardChainSequence { checks, .. } => checks.len() as u32,
            ExtractablePattern::TransformationPipeline { stages, .. } => stages.len() as u32 * 2,
            _ => 3,
        };

        ComplexityImpact {
            current_cyclomatic: context.complexity_before,
            predicted_cyclomatic: context
                .complexity_before
                .saturating_sub(extracted_complexity),
            current_cognitive: context.complexity_before * 2, // Estimate
            predicted_cognitive: (context.complexity_before * 2)
                .saturating_sub(extracted_complexity * 2),
            extracted_function_complexity: extracted_complexity,
        }
    }

    fn generate_example(&self, pattern: &ExtractablePattern, function_name: &str) -> String {
        match pattern {
            ExtractablePattern::AccumulationLoop {
                iterator_binding,
                operation,
                ..
            } => {
                let op_symbol = match operation {
                    AccumulationOp::Sum => "+",
                    AccumulationOp::Product => "*",
                    _ => "+",
                };
                format!(
                    "// Before:\n\
                     let mut result = 0;\n\
                     for {} in iter {{\n\
                     result {} {};\n\
                     }}\n\n\
                     // After:\n\
                     let result = {}(iter);",
                    iterator_binding, op_symbol, iterator_binding, function_name
                )
            }
            ExtractablePattern::GuardChainSequence { .. } => {
                format!(
                    "// Before:\n\
                     if !condition1 {{ return Err(Error::Invalid); }}\n\
                     if !condition2 {{ return Err(Error::Invalid); }}\n\n\
                     // After:\n\
                     {}(&value)?;",
                    function_name
                )
            }
            _ => "// Example transformation".to_string(),
        }
    }
}

struct AccumulationLoopVisitor {
    patterns: Vec<ExtractablePattern>,
    current_line: usize,
}

impl AccumulationLoopVisitor {
    fn new() -> Self {
        Self {
            patterns: Vec::new(),
            current_line: 0,
        }
    }
}

impl<'ast> Visit<'ast> for AccumulationLoopVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Expr::ForLoop(for_loop) = expr {
            // Check if this is an accumulation pattern
            if let Pat::Ident(pat_ident) = &*for_loop.pat {
                let iterator_binding = pat_ident.ident.to_string();

                // Look for accumulation in the loop body
                let mut has_accumulation = false;
                for stmt in &for_loop.body.stmts {
                    if let Stmt::Expr(Expr::Binary(_), _) = stmt {
                        has_accumulation = true;
                        break;
                    }
                }

                if has_accumulation {
                    self.patterns.push(ExtractablePattern::AccumulationLoop {
                        iterator_binding,
                        accumulator: "acc".to_string(),
                        operation: AccumulationOp::Sum,
                        filter: None,
                        transform: None,
                        start_line: self.current_line,
                        end_line: self.current_line + 5, // Estimate
                    });
                }
            }
        }

        syn::visit::visit_expr(self, expr);
    }
}

struct GuardChainVisitor {
    patterns: Vec<ExtractablePattern>,
    current_guards: Vec<GuardCheck>,
    in_function: bool,
}

impl GuardChainVisitor {
    fn new() -> Self {
        Self {
            patterns: Vec::new(),
            current_guards: Vec::new(),
            in_function: false,
        }
    }
}

impl<'ast> Visit<'ast> for GuardChainVisitor {
    fn visit_item_fn(&mut self, func: &'ast syn::ItemFn) {
        self.in_function = true;
        self.current_guards.clear();

        // Visit function body
        syn::visit::visit_item_fn(self, func);

        // Check if we found guard chains
        if self.current_guards.len() >= 2 {
            self.patterns.push(ExtractablePattern::GuardChainSequence {
                checks: self.current_guards.clone(),
                early_return: ReturnType {
                    type_name: "Result<()>".to_string(),
                    is_early_return: true,
                },
                start_line: 0,
                end_line: 10,
            });
        }

        self.in_function = false;
        self.current_guards.clear();
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if self.in_function {
            if let Stmt::Expr(Expr::If(if_expr), _) = stmt {
                // Check if this is a guard clause (early return)
                let has_early_return = if_expr
                    .then_branch
                    .stmts
                    .iter()
                    .any(|s| matches!(s, Stmt::Expr(Expr::Return(_), _)));

                if has_early_return {
                    self.current_guards.push(GuardCheck {
                        condition: "condition".to_string(), // Would extract actual condition
                        return_value: Some("Error".to_string()),
                        line: 0,
                    });
                }
            }
        }

        syn::visit::visit_stmt(self, stmt);
    }
}

struct PipelineVisitor {
    patterns: Vec<ExtractablePattern>,
}

impl PipelineVisitor {
    fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for PipelineVisitor {
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if let Expr::MethodCall(_method) = expr {
            // Check for iterator chain patterns like .map().filter().collect()
            let mut stages = Vec::new();
            let mut current = expr;

            while let Expr::MethodCall(m) = current {
                let method_name = m.method.to_string();
                if ["map", "filter", "fold", "collect", "flat_map"].contains(&method_name.as_str())
                {
                    stages.push(TransformStage {
                        operation: method_name,
                        input: "item".to_string(),
                        output: "transformed".to_string(),
                    });
                }
                current = &m.receiver;
            }

            if stages.len() >= 2 {
                self.patterns
                    .push(ExtractablePattern::TransformationPipeline {
                        stages,
                        input_binding: "input".to_string(),
                        output_type: "Vec<T>".to_string(),
                        start_line: 0,
                        end_line: 5,
                    });
            }
        }

        syn::visit::visit_expr(self, expr);
    }
}
