use rustpython_parser::ast;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PythonComplexityWeights {
    pub generator_weight: f64,
    pub yield_weight: f64,
    pub comprehension_depth_multiplier: f64,
    pub decorator_stack_weight: f64,
    pub context_manager_weight: f64,
    pub nested_context_weight: f64,
    pub metaclass_weight: f64,
    pub multiple_inheritance_weight: f64,
    pub dynamic_access_weight: f64,
    pub event_handler_weight: f64,
    pub exec_eval_weight: f64,
    pub property_decorator_weight: f64,
    pub class_decorator_weight: f64,
    pub decorator_factory_weight: f64,
    pub async_generator_weight: f64,
    pub mixin_weight: f64,
    pub monkey_patch_weight: f64,
}

impl Default for PythonComplexityWeights {
    fn default() -> Self {
        Self {
            generator_weight: 2.0,
            yield_weight: 2.0,
            comprehension_depth_multiplier: 2.0,
            decorator_stack_weight: 1.0,
            context_manager_weight: 1.0,
            nested_context_weight: 2.0,
            metaclass_weight: 5.0,
            multiple_inheritance_weight: 3.0,
            dynamic_access_weight: 2.0,
            event_handler_weight: 1.5,
            exec_eval_weight: 5.0,
            property_decorator_weight: 1.0,
            class_decorator_weight: 2.0,
            decorator_factory_weight: 3.0,
            async_generator_weight: 3.0,
            mixin_weight: 2.0,
            monkey_patch_weight: 4.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GeneratorPattern {
    pub location: (usize, usize),
    pub yield_count: u32,
    pub is_async: bool,
    pub function_name: String,
}

#[derive(Debug, Clone)]
pub struct DecoratorPattern {
    pub location: (usize, usize),
    pub stack_depth: u32,
    pub decorator_names: Vec<String>,
    pub is_property: bool,
    pub is_class_decorator: bool,
    pub is_factory: bool,
}

#[derive(Debug, Clone)]
pub struct EventHandlerPattern {
    pub location: (usize, usize),
    pub handler_name: String,
    pub framework: FrameworkType,
    pub event_type: String,
}

#[derive(Debug, Clone)]
pub enum FrameworkType {
    WxPython,
    Django,
    Flask,
    FastAPI,
    Tornado,
    Generic,
}

#[derive(Debug, Clone)]
pub struct MetaclassPattern {
    pub location: (usize, usize),
    pub class_name: String,
    pub metaclass_name: String,
    pub has_custom_new: bool,
    pub has_custom_init: bool,
}

#[derive(Debug, Clone)]
pub struct DynamicAccessPattern {
    pub location: (usize, usize),
    pub access_type: DynamicAccessType,
    pub context: String,
}

#[derive(Debug, Clone)]
pub enum DynamicAccessType {
    GetAttr,
    SetAttr,
    HasAttr,
    GetAttribute,
    Exec,
    Eval,
    Compile,
}

#[derive(Debug, Clone)]
pub struct ContextManagerPattern {
    pub location: (usize, usize),
    pub nesting_depth: u32,
    pub is_async: bool,
    pub has_exit_handler: bool,
}

#[derive(Debug, Clone)]
pub struct ComprehensionPattern {
    pub location: (usize, usize),
    pub depth: u32,
    pub comprehension_type: ComprehensionType,
    pub has_conditions: bool,
}

#[derive(Debug, Clone)]
pub enum ComprehensionType {
    List,
    Set,
    Dict,
    Generator,
}

#[derive(Debug, Clone)]
pub struct InheritancePattern {
    pub location: (usize, usize),
    pub class_name: String,
    pub base_classes: Vec<String>,
    pub is_diamond: bool,
    pub mixin_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct PythonPatterns {
    pub generators: Vec<GeneratorPattern>,
    pub decorators: Vec<DecoratorPattern>,
    pub event_handlers: Vec<EventHandlerPattern>,
    pub metaclasses: Vec<MetaclassPattern>,
    pub dynamic_accesses: Vec<DynamicAccessPattern>,
    pub context_managers: Vec<ContextManagerPattern>,
    pub comprehensions: Vec<ComprehensionPattern>,
    pub inheritance: Vec<InheritancePattern>,
}

pub struct PythonSpecificPatternDetector {
    patterns: PythonPatterns,
    weights: PythonComplexityWeights,
    current_function: Option<String>,
    current_class: Option<String>,
    context_depth: u32,
    comprehension_depth: u32,
    known_mixins: HashSet<String>,
    framework_indicators: HashMap<String, FrameworkType>,
}

// Pure helper functions for decorator analysis

/// Extracts decorator name from an expression
fn extract_decorator_name<F>(expr: &ast::Expr, expr_to_string: F) -> Option<String>
where
    F: Fn(&ast::Expr) -> String,
{
    match expr {
        ast::Expr::Name(name) => Some(name.id.to_string()),
        ast::Expr::Attribute(attr) => {
            Some(format!("{}.{}", expr_to_string(&attr.value), attr.attr))
        }
        ast::Expr::Call(call) => extract_decorator_name_from_call(call, expr_to_string),
        _ => None,
    }
}

/// Extracts decorator name from a call expression
fn extract_decorator_name_from_call<F>(call: &ast::ExprCall, expr_to_string: F) -> Option<String>
where
    F: Fn(&ast::Expr) -> String,
{
    match &*call.func {
        ast::Expr::Name(name) => Some(name.id.to_string()),
        ast::Expr::Attribute(attr) => {
            Some(format!("{}.{}", expr_to_string(&attr.value), attr.attr))
        }
        _ => None,
    }
}

/// Extracts all decorator names from a list of decorator expressions
fn extract_decorator_names<F>(decorators: &[ast::Expr], expr_to_string: F) -> Vec<String>
where
    F: Fn(&ast::Expr) -> String + Copy,
{
    decorators
        .iter()
        .filter_map(|dec| extract_decorator_name(dec, expr_to_string))
        .collect()
}

/// Checks if a decorator is a property decorator
fn is_property_decorator(name: &str) -> bool {
    name == "property" || name.ends_with(".property")
}

/// Checks if any decorator in the list is a property decorator
fn has_property_decorator(decorator_names: &[String]) -> bool {
    decorator_names
        .iter()
        .any(|name| is_property_decorator(name))
}

/// Checks if a decorator name indicates a factory pattern
fn is_factory_decorator(name: &str) -> bool {
    name.contains('(') || name.contains('.')
}

/// Checks if any decorator in the list is a factory decorator
fn has_factory_decorator(decorator_names: &[String]) -> bool {
    decorator_names
        .iter()
        .any(|name| is_factory_decorator(name))
}

/// Creates a decorator pattern from analyzed decorator information
fn create_decorator_pattern(
    decorators: &[ast::Expr],
    decorator_names: Vec<String>,
    is_class: bool,
) -> DecoratorPattern {
    DecoratorPattern {
        location: (0, 0),
        stack_depth: decorators.len() as u32,
        is_property: has_property_decorator(&decorator_names),
        is_class_decorator: is_class,
        is_factory: has_factory_decorator(&decorator_names),
        decorator_names,
    }
}

// Pure helper functions for class analysis

/// Extracts metaclass name from a keyword argument
fn extract_metaclass_name(keyword: &ast::Keyword) -> Option<String> {
    if let Some(ref arg) = keyword.arg {
        if arg == "metaclass" {
            if let ast::Expr::Name(name) = &keyword.value {
                return Some(name.id.to_string());
            }
        }
    }
    None
}

/// Checks if a class has a specific method
fn has_method(body: &[ast::Stmt], method_name: &str) -> bool {
    body.iter().any(|stmt| {
        if let ast::Stmt::FunctionDef(func) = stmt {
            func.name.as_str() == method_name
        } else {
            false
        }
    })
}

/// Extracts metaclass pattern from a class definition
fn extract_metaclass_pattern(
    class: &ast::StmtClassDef,
    _detector: &PythonSpecificPatternDetector,
) -> Option<MetaclassPattern> {
    class.keywords.iter().find_map(|keyword| {
        extract_metaclass_name(keyword).map(|metaclass_name| MetaclassPattern {
            location: (0, 0),
            class_name: class.name.to_string(),
            metaclass_name,
            has_custom_new: has_method(&class.body, "__new__"),
            has_custom_init: has_method(&class.body, "__init__"),
        })
    })
}

/// Extracts base class names from class bases
fn extract_base_class_names(bases: &[ast::Expr]) -> Vec<String> {
    bases
        .iter()
        .filter_map(|base| {
            if let ast::Expr::Name(name) = base {
                Some(name.id.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Counts mixins in base classes
fn count_mixins(base_names: &[String], known_mixins: &HashSet<String>) -> u32 {
    base_names
        .iter()
        .filter(|name| known_mixins.iter().any(|mixin| name.contains(mixin)))
        .count() as u32
}

/// Extracts inheritance pattern from a class definition
fn extract_inheritance_pattern(
    class: &ast::StmtClassDef,
    detector: &PythonSpecificPatternDetector,
) -> Option<InheritancePattern> {
    if class.bases.is_empty() {
        return None;
    }

    let base_names = extract_base_class_names(&class.bases);
    let mixin_count = count_mixins(&base_names, &detector.known_mixins);

    Some(InheritancePattern {
        location: (0, 0),
        class_name: class.name.to_string(),
        base_classes: base_names.clone(),
        is_diamond: detector.detect_diamond_inheritance(&base_names),
        mixin_count,
    })
}

impl PythonSpecificPatternDetector {
    pub fn new() -> Self {
        let mut framework_indicators = HashMap::new();

        // WxPython indicators
        framework_indicators.insert("EVT_".to_string(), FrameworkType::WxPython);
        framework_indicators.insert("wx.".to_string(), FrameworkType::WxPython);

        // Django indicators
        framework_indicators.insert("django.".to_string(), FrameworkType::Django);
        framework_indicators.insert("signals.".to_string(), FrameworkType::Django);

        // Flask indicators
        framework_indicators.insert("flask.".to_string(), FrameworkType::Flask);
        framework_indicators.insert("@app.".to_string(), FrameworkType::Flask);

        // FastAPI indicators
        framework_indicators.insert("fastapi.".to_string(), FrameworkType::FastAPI);
        framework_indicators.insert("@router.".to_string(), FrameworkType::FastAPI);

        let mut known_mixins = HashSet::new();
        known_mixins.insert("Mixin".to_string());
        known_mixins.insert("Mix".to_string());

        Self {
            patterns: PythonPatterns::default(),
            weights: PythonComplexityWeights::default(),
            current_function: None,
            current_class: None,
            context_depth: 0,
            comprehension_depth: 0,
            known_mixins,
            framework_indicators,
        }
    }

    pub fn with_weights(mut self, weights: PythonComplexityWeights) -> Self {
        self.weights = weights;
        self
    }

    pub fn detect_patterns(&mut self, module: &ast::Mod) -> &PythonPatterns {
        if let ast::Mod::Module(m) = module {
            self.analyze_body(&m.body);
        }
        &self.patterns
    }

    pub fn calculate_pattern_complexity(&self) -> f64 {
        let mut complexity = 0.0;

        // Generator complexity
        for gen in &self.patterns.generators {
            complexity += if gen.is_async {
                self.weights.async_generator_weight
            } else {
                self.weights.generator_weight
            };
            complexity += gen.yield_count as f64 * self.weights.yield_weight;
        }

        // Decorator complexity
        for dec in &self.patterns.decorators {
            let base_weight = if dec.is_property {
                self.weights.property_decorator_weight
            } else if dec.is_class_decorator {
                self.weights.class_decorator_weight
            } else if dec.is_factory {
                self.weights.decorator_factory_weight
            } else {
                self.weights.decorator_stack_weight
            };

            // Additional weight for decorator stacks
            if dec.stack_depth > 1 {
                complexity += base_weight * (dec.stack_depth - 1) as f64;
            }
        }

        // Context manager complexity
        for ctx in &self.patterns.context_managers {
            complexity += self.weights.context_manager_weight;
            if ctx.nesting_depth > 1 {
                complexity += self.weights.nested_context_weight * (ctx.nesting_depth - 1) as f64;
            }
        }

        // Event handler complexity
        for _handler in &self.patterns.event_handlers {
            complexity += self.weights.event_handler_weight;
        }

        // Metaclass complexity
        for meta in &self.patterns.metaclasses {
            complexity += self.weights.metaclass_weight;
            if meta.has_custom_new {
                complexity += 2.0;
            }
            if meta.has_custom_init {
                complexity += 1.0;
            }
        }

        // Inheritance complexity
        for inheritance in &self.patterns.inheritance {
            if inheritance.base_classes.len() > 1 {
                complexity += self.weights.multiple_inheritance_weight
                    * (inheritance.base_classes.len() - 1) as f64;
            }
            if inheritance.is_diamond {
                complexity += 3.0; // Additional penalty for diamond inheritance
            }
            complexity += inheritance.mixin_count as f64 * self.weights.mixin_weight;
        }

        // Dynamic access complexity
        for access in &self.patterns.dynamic_accesses {
            complexity += match access.access_type {
                DynamicAccessType::Exec | DynamicAccessType::Eval => self.weights.exec_eval_weight,
                DynamicAccessType::Compile => self.weights.exec_eval_weight * 0.8,
                _ => self.weights.dynamic_access_weight,
            };
        }

        // Comprehension complexity (exponential for deep nesting)
        for comp in &self.patterns.comprehensions {
            if comp.depth > 1 {
                complexity += self
                    .weights
                    .comprehension_depth_multiplier
                    .powf(comp.depth as f64);
            }
        }

        complexity
    }

    fn analyze_body(&mut self, body: &[ast::Stmt]) {
        for stmt in body {
            self.analyze_stmt(stmt);
        }
    }

    fn analyze_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::FunctionDef(func) => self.analyze_function_def(func),
            ast::Stmt::AsyncFunctionDef(func) => self.analyze_async_function_def(func),
            ast::Stmt::ClassDef(class) => self.analyze_class_def(class),
            ast::Stmt::With(with_stmt) => self.analyze_with(with_stmt),
            ast::Stmt::AsyncWith(with_stmt) => self.analyze_async_with(with_stmt),
            ast::Stmt::For(for_stmt) => self.analyze_for(for_stmt),
            ast::Stmt::If(if_stmt) => self.analyze_if(if_stmt),
            ast::Stmt::While(while_stmt) => self.analyze_while(while_stmt),
            ast::Stmt::Try(try_stmt) => self.analyze_try(try_stmt),
            ast::Stmt::Expr(expr) => self.analyze_expr_stmt(&expr.value),
            ast::Stmt::Assign(assign) => self.analyze_assign(assign),
            _ => {}
        }
    }

    fn analyze_function_def(&mut self, func: &ast::StmtFunctionDef) {
        let old_function = self.current_function.clone();
        self.current_function = Some(func.name.to_string());

        // Check if it's a generator
        if self.contains_yield(&func.body) {
            self.patterns.generators.push(GeneratorPattern {
                location: (0, 0), // Would need proper location tracking
                yield_count: self.count_yields(&func.body),
                is_async: false,
                function_name: func.name.to_string(),
            });
        }

        // Check decorators
        if !func.decorator_list.is_empty() {
            self.analyze_decorators(&func.decorator_list, false);
        }

        // Check if it's an event handler
        if self.is_event_handler(&func.name) {
            let framework = self.detect_framework(&func.name);
            self.patterns.event_handlers.push(EventHandlerPattern {
                location: (0, 0),
                handler_name: func.name.to_string(),
                framework,
                event_type: self.extract_event_type(&func.name),
            });
        }

        self.analyze_body(&func.body);
        self.current_function = old_function;
    }

    fn analyze_async_function_def(&mut self, func: &ast::StmtAsyncFunctionDef) {
        let old_function = self.current_function.clone();
        self.current_function = Some(func.name.to_string());

        // Check if it's an async generator
        if self.contains_yield(&func.body) {
            self.patterns.generators.push(GeneratorPattern {
                location: (0, 0),
                yield_count: self.count_yields(&func.body),
                is_async: true,
                function_name: func.name.to_string(),
            });
        }

        // Check decorators
        if !func.decorator_list.is_empty() {
            self.analyze_decorators(&func.decorator_list, false);
        }

        self.analyze_body(&func.body);
        self.current_function = old_function;
    }

    fn analyze_class_def(&mut self, class: &ast::StmtClassDef) {
        let old_class = self.current_class.clone();
        self.current_class = Some(class.name.to_string());

        // Analyze decorators
        if !class.decorator_list.is_empty() {
            self.analyze_decorators(&class.decorator_list, true);
        }

        // Analyze metaclass if present
        if let Some(metaclass_pattern) = extract_metaclass_pattern(class, self) {
            self.patterns.metaclasses.push(metaclass_pattern);
        }

        // Analyze inheritance if present
        if let Some(inheritance_pattern) = extract_inheritance_pattern(class, self) {
            self.patterns.inheritance.push(inheritance_pattern);
        }

        self.analyze_body(&class.body);
        self.current_class = old_class;
    }

    fn analyze_with(&mut self, with_stmt: &ast::StmtWith) {
        self.context_depth += 1;

        self.patterns.context_managers.push(ContextManagerPattern {
            location: (0, 0),
            nesting_depth: self.context_depth,
            is_async: false,
            has_exit_handler: true, // Assume true for with statements
        });

        self.analyze_body(&with_stmt.body);
        self.context_depth -= 1;
    }

    fn analyze_async_with(&mut self, with_stmt: &ast::StmtAsyncWith) {
        self.context_depth += 1;

        self.patterns.context_managers.push(ContextManagerPattern {
            location: (0, 0),
            nesting_depth: self.context_depth,
            is_async: true,
            has_exit_handler: true,
        });

        self.analyze_body(&with_stmt.body);
        self.context_depth -= 1;
    }

    fn analyze_for(&mut self, for_stmt: &ast::StmtFor) {
        self.analyze_expr(&for_stmt.iter);
        self.analyze_body(&for_stmt.body);
    }

    fn analyze_if(&mut self, if_stmt: &ast::StmtIf) {
        self.analyze_expr(&if_stmt.test);
        self.analyze_body(&if_stmt.body);
        self.analyze_body(&if_stmt.orelse);
    }

    fn analyze_while(&mut self, while_stmt: &ast::StmtWhile) {
        self.analyze_expr(&while_stmt.test);
        self.analyze_body(&while_stmt.body);
    }

    fn analyze_try(&mut self, try_stmt: &ast::StmtTry) {
        self.analyze_body(&try_stmt.body);
        for handler in &try_stmt.handlers {
            let ast::ExceptHandler::ExceptHandler(h) = handler;
            self.analyze_body(&h.body);
        }
        self.analyze_body(&try_stmt.orelse);
        self.analyze_body(&try_stmt.finalbody);
    }

    fn analyze_assign(&mut self, assign: &ast::StmtAssign) {
        self.analyze_expr(&assign.value);
    }

    fn analyze_expr(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::ListComp(comp) => self.analyze_comprehension(ComprehensionType::List, comp),
            ast::Expr::SetComp(comp) => {
                self.analyze_comprehension_set(ComprehensionType::Set, comp)
            }
            ast::Expr::DictComp(comp) => {
                self.analyze_comprehension_dict(ComprehensionType::Dict, comp)
            }
            ast::Expr::GeneratorExp(comp) => {
                self.analyze_comprehension_gen(ComprehensionType::Generator, comp)
            }
            ast::Expr::Call(call) => self.analyze_call(call),
            ast::Expr::Attribute(attr) => self.analyze_attribute(attr),
            _ => {}
        }
    }

    fn analyze_expr_stmt(&mut self, expr: &ast::Expr) {
        self.analyze_expr(expr);
    }

    fn analyze_comprehension(&mut self, comp_type: ComprehensionType, comp: &ast::ExprListComp) {
        self.comprehension_depth += 1;

        let has_conditions = comp.generators.iter().any(|gen| !gen.ifs.is_empty());

        self.patterns.comprehensions.push(ComprehensionPattern {
            location: (0, 0),
            depth: self.comprehension_depth,
            comprehension_type: comp_type,
            has_conditions,
        });

        // Recursively check for nested comprehensions
        self.analyze_expr(&comp.elt);
        for generator in &comp.generators {
            self.analyze_expr(&generator.iter);
            for condition in &generator.ifs {
                self.analyze_expr(condition);
            }
        }

        self.comprehension_depth -= 1;
    }

    fn analyze_comprehension_set(&mut self, comp_type: ComprehensionType, comp: &ast::ExprSetComp) {
        self.comprehension_depth += 1;

        let has_conditions = comp.generators.iter().any(|gen| !gen.ifs.is_empty());

        self.patterns.comprehensions.push(ComprehensionPattern {
            location: (0, 0),
            depth: self.comprehension_depth,
            comprehension_type: comp_type,
            has_conditions,
        });

        self.analyze_expr(&comp.elt);
        for generator in &comp.generators {
            self.analyze_expr(&generator.iter);
            for condition in &generator.ifs {
                self.analyze_expr(condition);
            }
        }

        self.comprehension_depth -= 1;
    }

    fn analyze_comprehension_dict(
        &mut self,
        comp_type: ComprehensionType,
        comp: &ast::ExprDictComp,
    ) {
        self.comprehension_depth += 1;

        let has_conditions = comp.generators.iter().any(|gen| !gen.ifs.is_empty());

        self.patterns.comprehensions.push(ComprehensionPattern {
            location: (0, 0),
            depth: self.comprehension_depth,
            comprehension_type: comp_type,
            has_conditions,
        });

        self.analyze_expr(&comp.key);
        self.analyze_expr(&comp.value);
        for generator in &comp.generators {
            self.analyze_expr(&generator.iter);
            for condition in &generator.ifs {
                self.analyze_expr(condition);
            }
        }

        self.comprehension_depth -= 1;
    }

    fn analyze_comprehension_gen(
        &mut self,
        comp_type: ComprehensionType,
        comp: &ast::ExprGeneratorExp,
    ) {
        self.comprehension_depth += 1;

        let has_conditions = comp.generators.iter().any(|gen| !gen.ifs.is_empty());

        self.patterns.comprehensions.push(ComprehensionPattern {
            location: (0, 0),
            depth: self.comprehension_depth,
            comprehension_type: comp_type,
            has_conditions,
        });

        self.analyze_expr(&comp.elt);
        for generator in &comp.generators {
            self.analyze_expr(&generator.iter);
            for condition in &generator.ifs {
                self.analyze_expr(condition);
            }
        }

        self.comprehension_depth -= 1;
    }

    fn analyze_call(&mut self, call: &ast::ExprCall) {
        // Check for dynamic access functions
        if let ast::Expr::Name(name) = &*call.func {
            let access_type = match name.id.as_str() {
                "getattr" => Some(DynamicAccessType::GetAttr),
                "setattr" => Some(DynamicAccessType::SetAttr),
                "hasattr" => Some(DynamicAccessType::HasAttr),
                "exec" => Some(DynamicAccessType::Exec),
                "eval" => Some(DynamicAccessType::Eval),
                "compile" => Some(DynamicAccessType::Compile),
                _ => None,
            };

            if let Some(access_type) = access_type {
                self.patterns.dynamic_accesses.push(DynamicAccessPattern {
                    location: (0, 0),
                    access_type,
                    context: self
                        .current_function
                        .clone()
                        .or_else(|| self.current_class.clone())
                        .unwrap_or_else(|| "module".to_string()),
                });
            }
        }

        // Analyze arguments
        for arg in &call.args {
            self.analyze_expr(arg);
        }
    }

    fn analyze_attribute(&mut self, attr: &ast::ExprAttribute) {
        // Check for __getattribute__ usage
        if attr.attr.as_str() == "__getattribute__" {
            self.patterns.dynamic_accesses.push(DynamicAccessPattern {
                location: (0, 0),
                access_type: DynamicAccessType::GetAttribute,
                context: self
                    .current_function
                    .clone()
                    .or_else(|| self.current_class.clone())
                    .unwrap_or_else(|| "module".to_string()),
            });
        }

        self.analyze_expr(&attr.value);
    }

    fn analyze_decorators(&mut self, decorators: &[ast::Expr], is_class: bool) {
        let decorator_names = extract_decorator_names(decorators, |expr| self.expr_to_string(expr));
        let pattern = create_decorator_pattern(decorators, decorator_names, is_class);
        self.patterns.decorators.push(pattern);
    }

    fn contains_yield(&self, body: &[ast::Stmt]) -> bool {
        for stmt in body {
            if self.stmt_contains_yield(stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_contains_yield(&self, stmt: &ast::Stmt) -> bool {
        match stmt {
            ast::Stmt::Expr(expr) => self.expr_contains_yield(&expr.value),
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.expr_contains_yield(value)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn expr_contains_yield(&self, expr: &ast::Expr) -> bool {
        matches!(expr, ast::Expr::Yield(_) | ast::Expr::YieldFrom(_))
    }

    fn count_yields(&self, body: &[ast::Stmt]) -> u32 {
        body.iter()
            .map(|stmt| self.count_yields_in_stmt(stmt))
            .sum()
    }

    fn count_yields_in_stmt(&self, stmt: &ast::Stmt) -> u32 {
        match stmt {
            ast::Stmt::Expr(expr) => self.count_yields_in_expr(&expr.value),
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.count_yields_in_expr(value)
                } else {
                    0
                }
            }
            ast::Stmt::For(for_stmt) => {
                self.count_yields(&for_stmt.body) + self.count_yields(&for_stmt.orelse)
            }
            ast::Stmt::While(while_stmt) => {
                self.count_yields(&while_stmt.body) + self.count_yields(&while_stmt.orelse)
            }
            ast::Stmt::If(if_stmt) => {
                self.count_yields(&if_stmt.body) + self.count_yields(&if_stmt.orelse)
            }
            _ => 0,
        }
    }

    fn count_yields_in_expr(&self, expr: &ast::Expr) -> u32 {
        match expr {
            ast::Expr::Yield(_) | ast::Expr::YieldFrom(_) => 1,
            _ => 0,
        }
    }

    fn is_event_handler(&self, name: &str) -> bool {
        name.starts_with("on_")
            || name.starts_with("handle_")
            || name.starts_with("process_")
            || name.contains("_handler")
            || name.contains("_callback")
            || name.contains("_listener")
    }

    fn detect_framework(&self, name: &str) -> FrameworkType {
        for (indicator, framework) in &self.framework_indicators {
            if name.contains(indicator) {
                return framework.clone();
            }
        }
        FrameworkType::Generic
    }

    fn extract_event_type(&self, name: &str) -> String {
        if let Some(suffix) = name.strip_prefix("on_") {
            suffix.to_string()
        } else if let Some(suffix) = name.strip_prefix("handle_") {
            suffix.to_string()
        } else if let Some(suffix) = name.strip_prefix("process_") {
            suffix.to_string()
        } else {
            name.to_string()
        }
    }

    fn detect_diamond_inheritance(&self, _base_classes: &[String]) -> bool {
        // Simplified detection - would need more complex MRO analysis
        false
    }

    fn expr_to_string(&self, expr: &ast::Expr) -> String {
        match expr {
            ast::Expr::Name(name) => name.id.to_string(),
            _ => "unknown".to_string(),
        }
    }
}

impl Default for PythonSpecificPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser::{parse, Mode};

    #[test]
    fn test_generator_detection() {
        let code = r#"
def my_generator():
    yield 1
    yield 2
    yield 3
"#;
        let module = parse(code, Mode::Module, "<test>").unwrap();
        let mut detector = PythonSpecificPatternDetector::new();
        let patterns = detector.detect_patterns(&module);

        assert_eq!(patterns.generators.len(), 1);
        assert_eq!(patterns.generators[0].yield_count, 3);
        assert!(!patterns.generators[0].is_async);
    }

    #[test]
    fn test_decorator_detection() {
        let code = r#"
@property
@cached
def my_property(self):
    return self._value

@dataclass
class MyClass:
    pass
"#;
        let module = parse(code, Mode::Module, "<test>").unwrap();
        let mut detector = PythonSpecificPatternDetector::new();
        let patterns = detector.detect_patterns(&module);

        assert_eq!(patterns.decorators.len(), 2);
        assert_eq!(patterns.decorators[0].stack_depth, 2);
        assert!(patterns.decorators[0].is_property);
        assert!(patterns.decorators[1].is_class_decorator);
    }

    #[test]
    fn test_context_manager_detection() {
        let code = r#"
with open('file.txt') as f:
    with lock:
        data = f.read()
"#;
        let module = parse(code, Mode::Module, "<test>").unwrap();
        let mut detector = PythonSpecificPatternDetector::new();
        let patterns = detector.detect_patterns(&module);

        assert_eq!(patterns.context_managers.len(), 2);
        assert_eq!(patterns.context_managers[0].nesting_depth, 1);
        assert_eq!(patterns.context_managers[1].nesting_depth, 2);
    }

    #[test]
    fn test_complexity_calculation() {
        let weights = PythonComplexityWeights::default();
        let mut patterns = PythonPatterns::default();

        patterns.generators.push(GeneratorPattern {
            location: (0, 0),
            yield_count: 3,
            is_async: false,
            function_name: "test".to_string(),
        });

        patterns.decorators.push(DecoratorPattern {
            location: (0, 0),
            stack_depth: 3,
            decorator_names: vec![],
            is_property: false,
            is_class_decorator: false,
            is_factory: false,
        });

        let detector = PythonSpecificPatternDetector {
            patterns,
            weights,
            ..PythonSpecificPatternDetector::new()
        };

        let complexity = detector.calculate_pattern_complexity();
        assert!(complexity > 0.0);
    }
}
