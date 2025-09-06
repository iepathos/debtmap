// Simplified implementation of Python organization pattern detection
// This implementation provides the core functionality while working with the current rustpython_parser API

use crate::common::{LocationConfidence, SourceLocation};
use crate::organization::{
    MagicValueType, OrganizationAntiPattern, Parameter, ParameterGroup,
    ParameterRefactoring, PrimitiveUsageContext, ResponsibilityGroup, ValueContext,
};
use rustpython_parser::ast;
use std::collections::HashMap;
use std::path::Path;

pub struct SimplifiedPythonOrganizationDetector {
    // Configurable thresholds
    god_object_method_threshold: usize,
    god_object_field_threshold: usize,
    long_parameter_threshold: usize,
    magic_value_min_occurrences: usize,
    feature_envy_threshold: f64,
    primitive_obsession_min_occurrences: usize,
}

impl SimplifiedPythonOrganizationDetector {
    pub fn new() -> Self {
        Self {
            god_object_method_threshold: 15,
            god_object_field_threshold: 10,
            long_parameter_threshold: 5,
            magic_value_min_occurrences: 2,
            feature_envy_threshold: 0.33, // If > 33% of calls are external
            primitive_obsession_min_occurrences: 3,
        }
    }

    pub fn with_thresholds(
        god_object_method_threshold: usize,
        god_object_field_threshold: usize,
        long_parameter_threshold: usize,
        magic_value_min_occurrences: usize,
        feature_envy_threshold: f64,
        primitive_obsession_min_occurrences: usize,
    ) -> Self {
        Self {
            god_object_method_threshold,
            god_object_field_threshold,
            long_parameter_threshold,
            magic_value_min_occurrences,
            feature_envy_threshold,
            primitive_obsession_min_occurrences,
        }
    }

    pub fn detect_patterns(
        &self,
        module: &ast::Mod,
        _path: &Path,
        source: &str,
    ) -> Vec<OrganizationAntiPattern> {
        let mut patterns = Vec::new();
        let mut primitive_usage_tracker = HashMap::new();
        let mut parameter_groups_tracker = HashMap::new();

        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                // Simple God Object detection for classes
                if let ast::Stmt::ClassDef(class_def) = stmt {
                    let mut method_count = 0;
                    let mut field_count = 0;

                    for class_stmt in &class_def.body {
                        match class_stmt {
                            ast::Stmt::FunctionDef(func_def) => {
                                let func_name = format!("{:?}", func_def.name);
                                if func_name.contains("__init__") {
                                    // Count field assignments in __init__
                                    for init_stmt in &func_def.body {
                                        if let ast::Stmt::Assign(assign_stmt) = init_stmt {
                                            for target in &assign_stmt.targets {
                                                if let ast::Expr::Attribute(_) = target {
                                                    field_count += 1;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    method_count += 1;
                                    // Also check for feature envy in class methods
                                    let feature_envy = self.detect_feature_envy_in_function(&func_def.body, &func_def.name.to_string());
                                    if let Some(envy) = feature_envy {
                                        patterns.push(envy);
                                    }
                                }
                            }
                            ast::Stmt::AsyncFunctionDef(_) => {
                                method_count += 1;
                            }
                            ast::Stmt::Assign(_) | ast::Stmt::AnnAssign(_) => {
                                field_count += 1;
                            }
                            _ => {}
                        }
                    }

                    // Use configurable thresholds
                    if method_count > self.god_object_method_threshold || field_count > self.god_object_field_threshold {
                        patterns.push(OrganizationAntiPattern::GodObject {
                            type_name: class_def.name.to_string(),
                            method_count,
                            field_count,
                            responsibility_count: self
                                .estimate_responsibilities(method_count, field_count),
                            suggested_split: vec![ResponsibilityGroup {
                                name: format!("{}Core", class_def.name),
                                methods: vec![],
                                fields: vec![],
                                responsibility: "Core functionality".to_string(),
                            }],
                            location: self.get_location_for_node(source, class_def.name.as_str()),
                        });
                    }
                }

                // Magic Value and Feature Envy detection for functions
                if let ast::Stmt::FunctionDef(func_def) = stmt {
                    // Detect magic values with improved traversal
                    let magic_values = self.detect_magic_values_in_function(&func_def.body);
                    for (value, count) in magic_values {
                        if count >= self.magic_value_min_occurrences {
                            patterns.push(OrganizationAntiPattern::MagicValue {
                                value_type: MagicValueType::NumericLiteral,
                                value: value.clone(),
                                occurrence_count: count,
                                suggested_constant_name: format!(
                                    "CONSTANT_{}",
                                    value.to_uppercase()
                                ),
                                context: ValueContext::BusinessLogic,
                                locations: vec![self.get_location_for_node(source, func_def.name.as_str())],
                            });
                        }
                    }

                    // Feature Envy detection
                    let feature_envy = self.detect_feature_envy_in_function(&func_def.body, &func_def.name);
                    if let Some(envy) = feature_envy {
                        patterns.push(envy);
                    }

                    // Long Parameter List and Data Clump detection
                    let param_count = self.count_parameters(&func_def.args);
                    let param_names = self.get_parameter_names(&func_def.args);
                    
                    // Track parameter groups for data clump detection
                    if param_names.len() >= 3 {
                        self.track_parameter_group(&param_names, &mut parameter_groups_tracker);
                    }
                    
                    if param_count > self.long_parameter_threshold {
                        patterns.push(OrganizationAntiPattern::LongParameterList {
                            function_name: func_def.name.to_string(),
                            parameter_count: param_count,
                            data_clumps: vec![],
                            suggested_refactoring: ParameterRefactoring::ExtractStruct,
                            location: self.get_location_for_node(source, func_def.name.as_str()),
                        });
                    }

                    // Primitive Obsession detection
                    self.detect_primitive_obsession_in_function(&func_def.args, &mut primitive_usage_tracker, source);
                }
            }
            
            // Process collected primitive obsession patterns
            for ((primitive_type, context), (count, locations)) in primitive_usage_tracker {
                if count >= self.primitive_obsession_min_occurrences {
                    patterns.push(OrganizationAntiPattern::PrimitiveObsession {
                        primitive_type: primitive_type.clone(),
                        usage_context: context.clone(),
                        occurrence_count: count,
                        suggested_domain_type: self.suggest_domain_type(&primitive_type, &context),
                        locations,
                    });
                }
            }
            
            // Process collected data clumps
            for (group_key, (count, params, locations)) in parameter_groups_tracker {
                if count >= 2 {
                    patterns.push(OrganizationAntiPattern::DataClump {
                        parameter_group: ParameterGroup {
                            parameters: params,
                            group_name: format!("ParameterGroup{}", group_key.chars().take(10).collect::<String>()),
                            semantic_relationship: "Frequently occurring together".to_string(),
                        },
                        occurrence_count: count,
                        suggested_struct_name: format!("{}Config", group_key.chars().take(15).collect::<String>()),
                        locations,
                    });
                }
            }
        }

        patterns
    }

    fn estimate_responsibilities(&self, method_count: usize, field_count: usize) -> usize {
        // Simple heuristic
        if method_count > 20 || field_count > 15 {
            4
        } else if method_count > 15 || field_count > 10 {
            3
        } else {
            2
        }
    }

    fn detect_magic_values_in_function(&self, body: &[ast::Stmt]) -> HashMap<String, usize> {
        let mut values = HashMap::new();

        for stmt in body {
            self.collect_constants_in_stmt(stmt, &mut values);
        }

        values
    }

    fn collect_constants_in_stmt(&self, stmt: &ast::Stmt, values: &mut HashMap<String, usize>) {
        match stmt {
            ast::Stmt::If(if_stmt) => {
                self.collect_constants_in_expr(&if_stmt.test, values);
                for s in &if_stmt.body {
                    self.collect_constants_in_stmt(s, values);
                }
                for s in &if_stmt.orelse {
                    self.collect_constants_in_stmt(s, values);
                }
            }
            ast::Stmt::While(while_stmt) => {
                self.collect_constants_in_expr(&while_stmt.test, values);
                for s in &while_stmt.body {
                    self.collect_constants_in_stmt(s, values);
                }
            }
            ast::Stmt::For(for_stmt) => {
                self.collect_constants_in_expr(&for_stmt.iter, values);
                for s in &for_stmt.body {
                    self.collect_constants_in_stmt(s, values);
                }
            }
            ast::Stmt::Assign(assign) => {
                self.collect_constants_in_expr(&assign.value, values);
            }
            ast::Stmt::AugAssign(aug) => {
                self.collect_constants_in_expr(&aug.value, values);
            }
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.collect_constants_in_expr(value, values);
                }
            }
            ast::Stmt::Expr(expr_stmt) => {
                self.collect_constants_in_expr(&expr_stmt.value, values);
            }
            _ => {}
        }
    }

    fn collect_constants_in_expr(&self, expr: &ast::Expr, values: &mut HashMap<String, usize>) {
        match expr {
            ast::Expr::Constant(constant) => {
                let value_str = match &constant.value {
                    ast::Constant::Int(i) => i.to_string(),
                    ast::Constant::Float(f) => f.to_string(),
                    ast::Constant::Str(s) => {
                        if s.len() > 1 && !s.starts_with("_") && !s.starts_with("test") {
                            s.clone()
                        } else {
                            return;
                        }
                    }
                    _ => return,
                };
                *values.entry(value_str).or_insert(0) += 1;
            }
            ast::Expr::BinOp(binop) => {
                self.collect_constants_in_expr(&binop.left, values);
                self.collect_constants_in_expr(&binop.right, values);
            }
            ast::Expr::UnaryOp(unaryop) => {
                self.collect_constants_in_expr(&unaryop.operand, values);
            }
            ast::Expr::Compare(compare) => {
                self.collect_constants_in_expr(&compare.left, values);
                for comparator in &compare.comparators {
                    self.collect_constants_in_expr(comparator, values);
                }
            }
            ast::Expr::Call(call) => {
                for arg in &call.args {
                    self.collect_constants_in_expr(arg, values);
                }
            }
            ast::Expr::List(list) => {
                for element in &list.elts {
                    self.collect_constants_in_expr(element, values);
                }
            }
            ast::Expr::Tuple(tuple) => {
                for element in &tuple.elts {
                    self.collect_constants_in_expr(element, values);
                }
            }
            ast::Expr::Dict(dict) => {
                for key in dict.keys.iter().flatten() {
                    self.collect_constants_in_expr(key, values);
                }
                for value in &dict.values {
                    self.collect_constants_in_expr(value, values);
                }
            }
            ast::Expr::IfExp(ifexp) => {
                self.collect_constants_in_expr(&ifexp.test, values);
                self.collect_constants_in_expr(&ifexp.body, values);
                self.collect_constants_in_expr(&ifexp.orelse, values);
            }
            _ => {}
        }
    }

    fn count_parameters(&self, args: &ast::Arguments) -> usize {
        // Count all parameters directly
        let mut count = args.args.len();

        // Add vararg and kwarg if present
        if args.vararg.is_some() {
            count += 1;
        }
        if args.kwarg.is_some() {
            count += 1;
        }

        count + args.kwonlyargs.len()
    }

    fn detect_feature_envy_in_function(&self, body: &[ast::Stmt], func_name: &str) -> Option<OrganizationAntiPattern> {
        let mut external_calls: HashMap<String, usize> = HashMap::new();
        let mut internal_calls = 0;

        for stmt in body {
            self.count_method_calls(stmt, &mut external_calls, &mut internal_calls);
        }

        // Find the most called external type
        if let Some((envied_type, external_count)) = external_calls.iter().max_by_key(|(_, count)| *count) {
            let total_calls = internal_calls + external_calls.values().sum::<usize>();
            if total_calls > 0 {
                let external_ratio = *external_count as f64 / total_calls as f64;
                if external_ratio > self.feature_envy_threshold && *external_count >= 3 {
                    return Some(OrganizationAntiPattern::FeatureEnvy {
                        method_name: func_name.to_string(),
                        envied_type: envied_type.clone(),
                        external_calls: *external_count,
                        internal_calls,
                        suggested_move: external_ratio > 0.6,
                        location: SourceLocation {
                            line: 1,
                            column: None,
                            end_line: None,
                            end_column: None,
                            confidence: LocationConfidence::Approximate,
                        },
                    });
                }
            }
        }

        None
    }

    fn count_method_calls(&self, stmt: &ast::Stmt, external_calls: &mut HashMap<String, usize>, internal_calls: &mut usize) {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => {
                self.count_calls_in_expr(&expr_stmt.value, external_calls, internal_calls);
            }
            ast::Stmt::Assign(assign) => {
                self.count_calls_in_expr(&assign.value, external_calls, internal_calls);
            }
            ast::Stmt::Return(ret_stmt) => {
                if let Some(value) = &ret_stmt.value {
                    self.count_calls_in_expr(value, external_calls, internal_calls);
                }
            }
            ast::Stmt::If(if_stmt) => {
                for s in &if_stmt.body {
                    self.count_method_calls(s, external_calls, internal_calls);
                }
                for s in &if_stmt.orelse {
                    self.count_method_calls(s, external_calls, internal_calls);
                }
            }
            ast::Stmt::While(while_stmt) => {
                for s in &while_stmt.body {
                    self.count_method_calls(s, external_calls, internal_calls);
                }
            }
            ast::Stmt::For(for_stmt) => {
                for s in &for_stmt.body {
                    self.count_method_calls(s, external_calls, internal_calls);
                }
            }
            _ => {}
        }
    }

    fn count_calls_in_expr(&self, expr: &ast::Expr, external_calls: &mut HashMap<String, usize>, internal_calls: &mut usize) {
        match expr {
            ast::Expr::Call(call) => {
                if let ast::Expr::Attribute(attr) = &*call.func {
                    if let ast::Expr::Name(name) = &*attr.value {
                        let name_str = format!("{:?}", name.id);
                        // Clean up the debug formatting to get just the identifier
                        let cleaned_name = name_str.trim_matches('"').replace("Identifier(\"", "").replace("\")", "");
                        if cleaned_name == "self" {
                            *internal_calls += 1;
                        } else {
                            *external_calls.entry(cleaned_name).or_insert(0) += 1;
                        }
                    }
                }
                for arg in &call.args {
                    self.count_calls_in_expr(arg, external_calls, internal_calls);
                }
            }
            _ => {}
        }
    }

    fn get_parameter_names(&self, args: &ast::Arguments) -> Vec<String> {
        let mut names = Vec::new();
        for arg in &args.args {
            // Extract the identifier from the arg
            let name_str = format!("{:?}", arg.def.arg);
            // Clean up the debug formatting to get just the identifier
            let cleaned_name = name_str.trim_matches('"').replace("Identifier(\"", "").replace("\")", "");
            if cleaned_name != "self" && cleaned_name != "cls" {
                names.push(cleaned_name);
            }
        }
        names
    }

    fn track_parameter_group(&self, params: &[String], tracker: &mut HashMap<String, (usize, Vec<Parameter>, Vec<SourceLocation>)>) {
        if params.len() >= 3 {
            // Create a key from sorted parameter names
            let mut sorted_params = params.to_vec();
            sorted_params.sort();
            let key = sorted_params.join(",");
            
            let entry = tracker.entry(key).or_insert((0, Vec::new(), Vec::new()));
            entry.0 += 1;
            
            if entry.1.is_empty() {
                for (i, param) in params.iter().enumerate() {
                    entry.1.push(Parameter {
                        name: param.clone(),
                        type_name: "Any".to_string(), // Simplified - would need type inference
                        position: i,
                    });
                }
            }
            
            entry.2.push(SourceLocation {
                line: 1,
                column: None,
                end_line: None,
                end_column: None,
                confidence: LocationConfidence::Approximate,
            });
        }
    }

    fn detect_primitive_obsession_in_function(
        &self,
        args: &ast::Arguments,
        tracker: &mut HashMap<(String, PrimitiveUsageContext), (usize, Vec<SourceLocation>)>,
        _source: &str,
    ) {
        for arg in &args.args {
            let name_str = format!("{:?}", arg.def.arg);
            // Clean up the debug formatting to get just the identifier
            let cleaned_name = name_str.trim_matches('"').replace("Identifier(\"", "").replace("\")", "");
            if cleaned_name == "self" || cleaned_name == "cls" {
                continue;
            }
            
            // Detect primitive obsession patterns based on parameter names
            let context = self.infer_primitive_context(&cleaned_name);
            if let Some(ctx) = context {
                let primitive_type = "str".to_string(); // Simplified - would need type inference
                let key = (primitive_type, ctx);
                let entry = tracker.entry(key).or_insert((0, Vec::new()));
                entry.0 += 1;
                entry.1.push(SourceLocation {
                    line: 1,
                    column: None,
                    end_line: None,
                    end_column: None,
                    confidence: LocationConfidence::Approximate,
                });
            }
        }
    }

    fn infer_primitive_context(&self, param_name: &str) -> Option<PrimitiveUsageContext> {
        let lower = param_name.to_lowercase();
        if lower.contains("id") || lower.contains("key") || lower.contains("uuid") {
            Some(PrimitiveUsageContext::Identifier)
        } else if lower.contains("status") || lower.contains("state") || lower.contains("flag") {
            Some(PrimitiveUsageContext::Status)
        } else if lower.contains("type") || lower.contains("category") || lower.contains("kind") {
            Some(PrimitiveUsageContext::Category)
        } else if lower.contains("size") || lower.contains("length") || lower.contains("count") ||
                  lower.contains("width") || lower.contains("height") || lower.contains("weight") {
            Some(PrimitiveUsageContext::Measurement)
        } else if lower.contains("limit") || lower.contains("threshold") || lower.contains("max") ||
                  lower.contains("min") {
            Some(PrimitiveUsageContext::BusinessRule)
        } else {
            None
        }
    }

    fn suggest_domain_type(&self, primitive_type: &str, context: &PrimitiveUsageContext) -> String {
        match context {
            PrimitiveUsageContext::Identifier => format!("{}Id", primitive_type.to_uppercase()),
            PrimitiveUsageContext::Measurement => "Measurement".to_string(),
            PrimitiveUsageContext::Status => "Status".to_string(),
            PrimitiveUsageContext::Category => "Category".to_string(),
            PrimitiveUsageContext::BusinessRule => "BusinessRule".to_string(),
        }
    }

    fn get_location_for_node(&self, source: &str, name: &str) -> SourceLocation {
        // Simple line search for the name
        let lines: Vec<&str> = source.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains(name) {
                return SourceLocation {
                    line: i + 1,
                    column: line.find(name).map(|c| c + 1),
                    end_line: Some(i + 1),
                    end_column: line.find(name).map(|c| c + name.len() + 1),
                    confidence: LocationConfidence::Exact,
                };
            }
        }
        
        // Fallback to approximate location
        SourceLocation {
            line: 1,
            column: None,
            end_line: None,
            end_column: None,
            confidence: LocationConfidence::Approximate,
        }
    }
}
