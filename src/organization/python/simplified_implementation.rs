// Simplified implementation of Python organization pattern detection
// This implementation provides the core functionality while working with the current rustpython_parser API

use crate::common::{LocationConfidence, SourceLocation};
use crate::organization::{
    MagicValueType, OrganizationAntiPattern, Parameter, ParameterGroup, ParameterRefactoring,
    PrimitiveUsageContext, ResponsibilityGroup, ValueContext,
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

impl Default for SimplifiedPythonOrganizationDetector {
    fn default() -> Self {
        Self::new()
    }
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
                self.process_statement(
                    stmt,
                    source,
                    &mut patterns,
                    &mut primitive_usage_tracker,
                    &mut parameter_groups_tracker,
                );
            }

            // Process collected primitive obsession patterns
            self.process_primitive_obsession_patterns(primitive_usage_tracker, &mut patterns);

            // Process collected data clumps
            self.process_data_clump_patterns(parameter_groups_tracker, &mut patterns);
        }

        patterns
    }

    /// Process a single statement for anti-patterns
    fn process_statement(
        &self,
        stmt: &ast::Stmt,
        source: &str,
        patterns: &mut Vec<OrganizationAntiPattern>,
        primitive_usage_tracker: &mut HashMap<
            (String, PrimitiveUsageContext),
            (usize, Vec<SourceLocation>),
        >,
        parameter_groups_tracker: &mut HashMap<
            String,
            (usize, Vec<Parameter>, Vec<SourceLocation>),
        >,
    ) {
        match stmt {
            ast::Stmt::ClassDef(class_def) => {
                self.process_class_definition(class_def, source, patterns);
            }
            ast::Stmt::FunctionDef(func_def) => {
                self.process_function_definition(
                    func_def,
                    source,
                    patterns,
                    primitive_usage_tracker,
                    parameter_groups_tracker,
                );
            }
            _ => {}
        }
    }

    /// Process a class definition for God Object pattern
    fn process_class_definition(
        &self,
        class_def: &ast::StmtClassDef,
        source: &str,
        patterns: &mut Vec<OrganizationAntiPattern>,
    ) {
        let (method_count, field_count, method_patterns) =
            self.analyze_class_members(&class_def.body, class_def.name.as_ref());

        patterns.extend(method_patterns);

        // Use configurable thresholds
        if method_count > self.god_object_method_threshold
            || field_count > self.god_object_field_threshold
        {
            patterns.push(OrganizationAntiPattern::GodObject {
                type_name: class_def.name.to_string(),
                method_count,
                field_count,
                responsibility_count: self.estimate_responsibilities(method_count, field_count),
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

    /// Analyze class members and count methods/fields
    fn analyze_class_members(
        &self,
        body: &[ast::Stmt],
        _class_name: &str,
    ) -> (usize, usize, Vec<OrganizationAntiPattern>) {
        let mut method_count = 0;
        let mut field_count = 0;
        let mut patterns = Vec::new();

        for class_stmt in body {
            match class_stmt {
                ast::Stmt::FunctionDef(func_def) => {
                    let (is_init, fields_in_init) = self.analyze_function_in_class(func_def);
                    if is_init {
                        field_count += fields_in_init;
                    } else {
                        method_count += 1;
                        // Check for feature envy in class methods
                        if let Some(envy) = self
                            .detect_feature_envy_in_function(&func_def.body, func_def.name.as_ref())
                        {
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

        (method_count, field_count, patterns)
    }

    /// Analyze a function within a class context
    fn analyze_function_in_class(&self, func_def: &ast::StmtFunctionDef) -> (bool, usize) {
        let func_name = format!("{:?}", func_def.name);
        if func_name.contains("__init__") {
            let field_count = self.count_init_field_assignments(&func_def.body);
            (true, field_count)
        } else {
            (false, 0)
        }
    }

    /// Count field assignments in __init__ method
    fn count_init_field_assignments(&self, body: &[ast::Stmt]) -> usize {
        let mut field_count = 0;
        for init_stmt in body {
            if let ast::Stmt::Assign(assign_stmt) = init_stmt {
                for target in &assign_stmt.targets {
                    if let ast::Expr::Attribute(_) = target {
                        field_count += 1;
                    }
                }
            }
        }
        field_count
    }

    /// Process a function definition for various anti-patterns
    fn process_function_definition(
        &self,
        func_def: &ast::StmtFunctionDef,
        source: &str,
        patterns: &mut Vec<OrganizationAntiPattern>,
        primitive_usage_tracker: &mut HashMap<
            (String, PrimitiveUsageContext),
            (usize, Vec<SourceLocation>),
        >,
        parameter_groups_tracker: &mut HashMap<
            String,
            (usize, Vec<Parameter>, Vec<SourceLocation>),
        >,
    ) {
        // Detect magic values
        self.process_magic_values(func_def, source, patterns);

        // Feature Envy detection
        if let Some(envy) = self.detect_feature_envy_in_function(&func_def.body, &func_def.name) {
            patterns.push(envy);
        }

        // Process parameters
        self.process_function_parameters(
            func_def,
            source,
            patterns,
            primitive_usage_tracker,
            parameter_groups_tracker,
        );
    }

    /// Process magic values in a function
    fn process_magic_values(
        &self,
        func_def: &ast::StmtFunctionDef,
        source: &str,
        patterns: &mut Vec<OrganizationAntiPattern>,
    ) {
        let magic_values = self.detect_magic_values_in_function(&func_def.body);
        for (value, count) in magic_values {
            if count >= self.magic_value_min_occurrences {
                patterns.push(OrganizationAntiPattern::MagicValue {
                    value_type: MagicValueType::NumericLiteral,
                    value: value.clone(),
                    occurrence_count: count,
                    suggested_constant_name: format!("CONSTANT_{}", value.to_uppercase()),
                    context: ValueContext::BusinessLogic,
                    locations: vec![self.get_location_for_node(source, func_def.name.as_str())],
                });
            }
        }
    }

    /// Process function parameters for long parameter list and data clumps
    fn process_function_parameters(
        &self,
        func_def: &ast::StmtFunctionDef,
        source: &str,
        patterns: &mut Vec<OrganizationAntiPattern>,
        primitive_usage_tracker: &mut HashMap<
            (String, PrimitiveUsageContext),
            (usize, Vec<SourceLocation>),
        >,
        parameter_groups_tracker: &mut HashMap<
            String,
            (usize, Vec<Parameter>, Vec<SourceLocation>),
        >,
    ) {
        let param_count = self.count_parameters(&func_def.args);
        let param_names = self.get_parameter_names(&func_def.args);

        // Track parameter groups for data clump detection
        if param_names.len() >= 3 {
            self.track_parameter_group(&param_names, parameter_groups_tracker);
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
        self.detect_primitive_obsession_in_function(
            &func_def.args,
            primitive_usage_tracker,
            source,
        );
    }

    /// Process collected primitive obsession patterns
    fn process_primitive_obsession_patterns(
        &self,
        primitive_usage_tracker: HashMap<
            (String, PrimitiveUsageContext),
            (usize, Vec<SourceLocation>),
        >,
        patterns: &mut Vec<OrganizationAntiPattern>,
    ) {
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
    }

    /// Process collected data clump patterns
    fn process_data_clump_patterns(
        &self,
        parameter_groups_tracker: HashMap<String, (usize, Vec<Parameter>, Vec<SourceLocation>)>,
        patterns: &mut Vec<OrganizationAntiPattern>,
    ) {
        for (group_key, (count, params, locations)) in parameter_groups_tracker {
            if count >= 2 {
                patterns.push(OrganizationAntiPattern::DataClump {
                    parameter_group: ParameterGroup {
                        parameters: params,
                        group_name: format!(
                            "ParameterGroup{}",
                            group_key.chars().take(10).collect::<String>()
                        ),
                        semantic_relationship: "Frequently occurring together".to_string(),
                    },
                    occurrence_count: count,
                    suggested_struct_name: format!(
                        "{}Config",
                        group_key.chars().take(15).collect::<String>()
                    ),
                    locations,
                });
            }
        }
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

    #[allow(clippy::only_used_in_recursion)]
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

    fn detect_feature_envy_in_function(
        &self,
        body: &[ast::Stmt],
        func_name: &str,
    ) -> Option<OrganizationAntiPattern> {
        let mut external_calls: HashMap<String, usize> = HashMap::new();
        let mut internal_calls = 0;

        for stmt in body {
            self.count_method_calls(stmt, &mut external_calls, &mut internal_calls);
        }

        // Find the most called external type
        if let Some((envied_type, external_count)) =
            external_calls.iter().max_by_key(|(_, count)| *count)
        {
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

    fn count_method_calls(
        &self,
        stmt: &ast::Stmt,
        external_calls: &mut HashMap<String, usize>,
        internal_calls: &mut usize,
    ) {
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

    #[allow(clippy::only_used_in_recursion)]
    fn count_calls_in_expr(
        &self,
        expr: &ast::Expr,
        external_calls: &mut HashMap<String, usize>,
        internal_calls: &mut usize,
    ) {
        if let ast::Expr::Call(call) = expr {
            if let ast::Expr::Attribute(attr) = &*call.func {
                if let ast::Expr::Name(name) = &*attr.value {
                    let name_str = format!("{:?}", name.id);
                    // Clean up the debug formatting to get just the identifier
                    let cleaned_name = name_str
                        .trim_matches('"')
                        .replace("Identifier(\"", "")
                        .replace("\")", "");
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
    }

    fn get_parameter_names(&self, args: &ast::Arguments) -> Vec<String> {
        let mut names = Vec::new();
        for arg in &args.args {
            // Extract the identifier from the arg
            let name_str = format!("{:?}", arg.def.arg);
            // Clean up the debug formatting to get just the identifier
            let cleaned_name = name_str
                .trim_matches('"')
                .replace("Identifier(\"", "")
                .replace("\")", "");
            if cleaned_name != "self" && cleaned_name != "cls" {
                names.push(cleaned_name);
            }
        }
        names
    }

    fn track_parameter_group(
        &self,
        params: &[String],
        tracker: &mut HashMap<String, (usize, Vec<Parameter>, Vec<SourceLocation>)>,
    ) {
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
            let cleaned_name = name_str
                .trim_matches('"')
                .replace("Identifier(\"", "")
                .replace("\")", "");
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
        } else if lower.contains("size")
            || lower.contains("length")
            || lower.contains("count")
            || lower.contains("width")
            || lower.contains("height")
            || lower.contains("weight")
        {
            Some(PrimitiveUsageContext::Measurement)
        } else if lower.contains("limit")
            || lower.contains("threshold")
            || lower.contains("max")
            || lower.contains("min")
        {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::ast;
    use std::path::Path;

    fn create_test_detector() -> SimplifiedPythonOrganizationDetector {
        SimplifiedPythonOrganizationDetector::new()
    }

    fn parse_python_code(code: &str) -> ast::Mod {
        rustpython_parser::parse(code, rustpython_parser::Mode::Module, "<test>")
            .expect("Failed to parse Python code")
    }

    #[test]
    fn test_process_statement_with_class() {
        let detector = create_test_detector();
        let code = r#"
class TestClass:
    def __init__(self):
        self.field1 = 1
        self.field2 = 2
    
    def method1(self):
        pass
    
    def method2(self):
        pass
"#;
        let module = parse_python_code(code);
        let mut patterns = Vec::new();
        let mut primitive_tracker = HashMap::new();
        let mut parameter_tracker = HashMap::new();

        if let ast::Mod::Module(m) = &module {
            for stmt in &m.body {
                detector.process_statement(
                    stmt,
                    code,
                    &mut patterns,
                    &mut primitive_tracker,
                    &mut parameter_tracker,
                );
            }
        }

        // Should not detect God Object with default thresholds
        assert!(patterns
            .iter()
            .all(|p| !matches!(p, OrganizationAntiPattern::GodObject { .. })));
    }

    #[test]
    fn test_process_statement_with_function() {
        let detector = create_test_detector();
        let code = r#"
def test_function(param1, param2, param3):
    value = 42
    return value * 42
"#;
        let module = parse_python_code(code);
        let mut patterns = Vec::new();
        let mut primitive_tracker = HashMap::new();
        let mut parameter_tracker = HashMap::new();

        if let ast::Mod::Module(m) = &module {
            for stmt in &m.body {
                detector.process_statement(
                    stmt,
                    code,
                    &mut patterns,
                    &mut primitive_tracker,
                    &mut parameter_tracker,
                );
            }
        }

        // Check if magic value was detected
        let has_magic_value = patterns.iter().any(
            |p| matches!(p, OrganizationAntiPattern::MagicValue { value, .. } if value == "42"),
        );
        assert!(has_magic_value, "Should detect magic value 42");
    }

    #[test]
    fn test_analyze_class_members() {
        let detector = create_test_detector();
        let code = r#"
class TestClass:
    def __init__(self):
        self.field1 = 1
        self.field2 = 2
        self.field3 = 3
    
    def method1(self):
        pass
    
    async def async_method(self):
        pass
    
    field4 = 4
"#;
        let module = parse_python_code(code);

        if let ast::Mod::Module(m) = &module {
            if let ast::Stmt::ClassDef(class_def) = &m.body[0] {
                let (method_count, field_count, _) =
                    detector.analyze_class_members(&class_def.body, "TestClass");

                assert_eq!(
                    method_count, 2,
                    "Should count 2 methods (excluding __init__)"
                );
                assert_eq!(
                    field_count, 4,
                    "Should count 4 fields (3 in __init__ + 1 class field)"
                );
            }
        }
    }

    #[test]
    fn test_analyze_function_in_class_init() {
        let detector = create_test_detector();
        let code = r#"
class C:
    def __init__(self):
        self.field1 = 1
        self.field2 = 2
"#;
        let module = parse_python_code(code);

        if let ast::Mod::Module(m) = &module {
            if let ast::Stmt::ClassDef(class_def) = &m.body[0] {
                if let ast::Stmt::FunctionDef(func_def) = &class_def.body[0] {
                    let (is_init, field_count) = detector.analyze_function_in_class(func_def);

                    assert!(is_init, "Should recognize __init__ method");
                    assert_eq!(field_count, 2, "Should count 2 field assignments");
                }
            }
        }
    }

    #[test]
    fn test_analyze_function_in_class_regular_method() {
        let detector = create_test_detector();
        let code = r#"
class C:
    def regular_method(self):
        pass
"#;
        let module = parse_python_code(code);

        if let ast::Mod::Module(m) = &module {
            if let ast::Stmt::ClassDef(class_def) = &m.body[0] {
                if let ast::Stmt::FunctionDef(func_def) = &class_def.body[0] {
                    let (is_init, field_count) = detector.analyze_function_in_class(func_def);

                    assert!(!is_init, "Should not recognize as __init__ method");
                    assert_eq!(field_count, 0, "Should have 0 field assignments");
                }
            }
        }
    }

    #[test]
    fn test_count_init_field_assignments() {
        let detector = create_test_detector();
        let code = r#"
class C:
    def __init__(self):
        self.field1 = 1
        self.field2 = 2
        local_var = 3
        self.field3 = local_var
"#;
        let module = parse_python_code(code);

        if let ast::Mod::Module(m) = &module {
            if let ast::Stmt::ClassDef(class_def) = &m.body[0] {
                if let ast::Stmt::FunctionDef(func_def) = &class_def.body[0] {
                    let count = detector.count_init_field_assignments(&func_def.body);
                    assert_eq!(count, 3, "Should count 3 self.field assignments");
                }
            }
        }
    }

    #[test]
    fn test_process_magic_values() {
        let detector = create_test_detector();
        let code = r#"
def calculate():
    return 100 * 100 * 100
"#;
        let module = parse_python_code(code);
        let mut patterns = Vec::new();

        if let ast::Mod::Module(m) = &module {
            if let ast::Stmt::FunctionDef(func_def) = &m.body[0] {
                detector.process_magic_values(func_def, code, &mut patterns);
            }
        }

        // Should detect repeated magic value 100
        let magic_count = patterns.iter()
            .filter(|p| matches!(p, OrganizationAntiPattern::MagicValue { value, .. } if value == "100"))
            .count();
        assert_eq!(
            magic_count, 1,
            "Should detect one magic value pattern for 100"
        );
    }

    #[test]
    fn test_process_function_parameters_long_list() {
        let detector = create_test_detector();
        let code = r#"
def complex_function(a, b, c, d, e, f, g, h):
    pass
"#;
        let module = parse_python_code(code);
        let mut patterns = Vec::new();
        let mut primitive_tracker = HashMap::new();
        let mut parameter_tracker = HashMap::new();

        if let ast::Mod::Module(m) = &module {
            if let ast::Stmt::FunctionDef(func_def) = &m.body[0] {
                detector.process_function_parameters(
                    func_def,
                    code,
                    &mut patterns,
                    &mut primitive_tracker,
                    &mut parameter_tracker,
                );
            }
        }

        // Should detect long parameter list (8 > default threshold of 4)
        let has_long_params = patterns.iter().any(|p|
            matches!(p, OrganizationAntiPattern::LongParameterList { parameter_count, .. } if *parameter_count == 8)
        );
        assert!(has_long_params, "Should detect long parameter list");
    }

    #[test]
    fn test_process_primitive_obsession_patterns() {
        let detector = create_test_detector();
        let mut patterns = Vec::new();
        let mut primitive_tracker = HashMap::new();

        // Simulate collected primitive usage
        primitive_tracker.insert(
            ("int".to_string(), PrimitiveUsageContext::Identifier),
            (5, vec![]), // 5 occurrences, exceeds default threshold of 3
        );

        detector.process_primitive_obsession_patterns(primitive_tracker, &mut patterns);

        assert_eq!(
            patterns.len(),
            1,
            "Should create one primitive obsession pattern"
        );
        assert!(matches!(
            &patterns[0],
            OrganizationAntiPattern::PrimitiveObsession { occurrence_count, .. } if *occurrence_count == 5
        ));
    }

    #[test]
    fn test_process_data_clump_patterns() {
        let detector = create_test_detector();
        let mut patterns = Vec::new();
        let mut parameter_tracker = HashMap::new();

        // Simulate collected parameter groups
        parameter_tracker.insert(
            "test_group".to_string(),
            (
                3,
                vec![
                    Parameter {
                        name: "param1".to_string(),
                        type_name: "Any".to_string(),
                        position: 0,
                    },
                    Parameter {
                        name: "param2".to_string(),
                        type_name: "Any".to_string(),
                        position: 1,
                    },
                    Parameter {
                        name: "param3".to_string(),
                        type_name: "Any".to_string(),
                        position: 2,
                    },
                ],
                vec![],
            ),
        );

        detector.process_data_clump_patterns(parameter_tracker, &mut patterns);

        assert_eq!(patterns.len(), 1, "Should create one data clump pattern");
        assert!(matches!(
            &patterns[0],
            OrganizationAntiPattern::DataClump { occurrence_count, .. } if *occurrence_count == 3
        ));
    }

    #[test]
    fn test_detect_patterns_integration() {
        // Create detector with lower threshold for testing
        let detector = SimplifiedPythonOrganizationDetector::with_thresholds(
            10,   // god_object_method_threshold
            10,   // god_object_field_threshold
            5,    // long_parameter_threshold
            2,    // magic_value_min_occurrences
            0.33, // feature_envy_threshold
            3,    // primitive_obsession_min_occurrences
        );
        let code = r#"
class LargeClass:
    def __init__(self):
        self.a = 1
        self.b = 2
    
    def method1(self): pass
    def method2(self): pass
    def method3(self): pass
    def method4(self): pass
    def method5(self): pass
    def method6(self): pass
    def method7(self): pass
    def method8(self): pass
    def method9(self): pass
    def method10(self): pass
    def method11(self): pass
    def method12(self): pass

def magic_function():
    value = 42
    result = value * 42 + 42
    return result / 42
"#;
        let module = parse_python_code(code);
        let path = Path::new("test.py");
        let patterns = detector.detect_patterns(&module, path, code);

        // Should detect God Object (12 methods > threshold of 10)
        let has_god_object = patterns.iter().any(|p|
            matches!(p, OrganizationAntiPattern::GodObject { method_count, .. } if *method_count == 12)
        );

        assert!(has_god_object, "Should detect God Object with 12 methods");

        // Should detect magic value 42
        let has_magic_value = patterns.iter().any(
            |p| matches!(p, OrganizationAntiPattern::MagicValue { value, .. } if value == "42"),
        );
        assert!(has_magic_value, "Should detect magic value 42");
    }
}
