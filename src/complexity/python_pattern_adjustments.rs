use rustpython_parser::ast;
use std::collections::HashMap;

/// Python pattern detection and complexity adjustment
#[derive(Debug, Clone)]
pub struct PythonPatternDetector {
    patterns: Vec<Box<dyn PythonPattern>>,
    #[allow(dead_code)]
    adjustment_config: AdjustmentConfig,
}

/// Configuration for pattern-based adjustments
#[derive(Debug, Clone)]
pub struct AdjustmentConfig {
    pub enable_dictionary_dispatch: bool,
    pub enable_strategy_pattern: bool,
    pub enable_chain_of_responsibility: bool,
    pub enable_visitor_pattern: bool,
    pub enable_decorator_pattern: bool,
    pub enable_context_manager: bool,
    pub adjustment_factors: HashMap<PatternType, f32>,
}

impl Default for AdjustmentConfig {
    fn default() -> Self {
        let mut factors = HashMap::new();
        factors.insert(PatternType::DictionaryDispatch, 0.5);
        factors.insert(PatternType::StrategyPattern, 0.6);
        factors.insert(PatternType::ChainOfResponsibility, 0.6);
        factors.insert(PatternType::VisitorPattern, 0.5);
        factors.insert(PatternType::DecoratorPattern, 0.7);
        factors.insert(PatternType::ContextManager, 0.8);

        Self {
            enable_dictionary_dispatch: true,
            enable_strategy_pattern: true,
            enable_chain_of_responsibility: true,
            enable_visitor_pattern: true,
            enable_decorator_pattern: true,
            enable_context_manager: true,
            adjustment_factors: factors,
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum PatternType {
    DictionaryDispatch,
    StrategyPattern,
    ChainOfResponsibility,
    VisitorPattern,
    DecoratorPattern,
    ContextManager,
}

/// Represents a detected pattern match
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub pattern_type: PatternType,
    pub confidence: f32,
    pub adjustment: f32,
    pub description: String,
}

/// Trait for Python pattern recognition
pub trait PythonPattern: std::fmt::Debug {
    fn detect(&self, func_def: &ast::StmtFunctionDef) -> Option<PatternMatch>;
    fn detect_async(&self, func_def: &ast::StmtAsyncFunctionDef) -> Option<PatternMatch>;
    fn adjustment_factor(&self) -> f32;
    fn clone_box(&self) -> Box<dyn PythonPattern>;
}

impl Clone for Box<dyn PythonPattern> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Dictionary dispatch pattern detector
#[derive(Debug, Clone)]
pub struct DictionaryDispatchPattern {
    min_branches: usize,
}

impl Default for DictionaryDispatchPattern {
    fn default() -> Self {
        Self { min_branches: 3 }
    }
}

impl DictionaryDispatchPattern {
    fn detect_in_body(&self, body: &[ast::Stmt]) -> Option<PatternMatch> {
        let mut has_dispatch_dict = false;
        let mut dispatch_keys = 0;
        let mut has_call_after_get = false;

        for stmt in body {
            if let Some(key_count) = self.count_dispatch_keys(stmt) {
                has_dispatch_dict = true;
                dispatch_keys += key_count;
            }
            if self.has_dictionary_get_and_call(stmt) {
                has_call_after_get = true;
            }
        }

        if (has_dispatch_dict || has_call_after_get) && dispatch_keys >= self.min_branches {
            Some(PatternMatch {
                pattern_type: PatternType::DictionaryDispatch,
                confidence: 0.85,
                adjustment: 0.5,
                description: "Dictionary dispatch pattern detected".to_string(),
            })
        } else {
            None
        }
    }
}

impl PythonPattern for DictionaryDispatchPattern {
    fn detect(&self, func_def: &ast::StmtFunctionDef) -> Option<PatternMatch> {
        self.detect_in_body(&func_def.body)
    }

    fn detect_async(&self, func_def: &ast::StmtAsyncFunctionDef) -> Option<PatternMatch> {
        self.detect_in_body(&func_def.body)
    }

    fn adjustment_factor(&self) -> f32 {
        0.5
    }

    fn clone_box(&self) -> Box<dyn PythonPattern> {
        Box::new(self.clone())
    }
}

impl DictionaryDispatchPattern {
    fn count_dispatch_keys(&self, stmt: &ast::Stmt) -> Option<usize> {
        match stmt {
            ast::Stmt::Assign(assign) => {
                if let ast::Expr::Dict(dict) = &*assign.value {
                    // Check if values are functions/lambdas
                    let is_dispatch = dict.values
                        .iter()
                        .any(|v| matches!(v, ast::Expr::Lambda(_) | ast::Expr::Name(_)));
                    if is_dispatch {
                        Some(dict.keys.len())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn has_dictionary_get_and_call(&self, stmt: &ast::Stmt) -> bool {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => self.is_dict_get_call(&expr_stmt.value),
            ast::Stmt::Return(ret) => ret
                .value
                .as_ref()
                .is_some_and(|v| self.is_dict_get_call(v)),
            _ => false,
        }
    }

    fn is_dict_get_call(&self, expr: &ast::Expr) -> bool {
        if let ast::Expr::Call(call) = expr {
            if let ast::Expr::Attribute(attr) = &*call.func {
                if let ast::Expr::Call(inner_call) = &*attr.value {
                    if let ast::Expr::Attribute(inner_attr) = &*inner_call.func {
                        return inner_attr.attr.as_str() == "get";
                    }
                }
            }
        }
        false
    }
}

/// Strategy pattern detector
#[derive(Debug, Clone)]
pub struct StrategyPattern {
    method_patterns: Vec<String>,
}

impl Default for StrategyPattern {
    fn default() -> Self {
        Self {
            method_patterns: vec![
                "execute".to_string(),
                "process".to_string(),
                "handle".to_string(),
                "apply".to_string(),
                "run".to_string(),
            ],
        }
    }
}

impl StrategyPattern {
    fn detect_in_body(&self, body: &[ast::Stmt]) -> Option<PatternMatch> {
        let mut has_strategy_call = false;
        let mut has_strategy_selection = false;

        for stmt in body {
            if self.has_strategy_invocation(stmt) {
                has_strategy_call = true;
            }
            if self.has_conditional_strategy_selection(stmt) {
                has_strategy_selection = true;
            }
        }

        if has_strategy_call && has_strategy_selection {
            Some(PatternMatch {
                pattern_type: PatternType::StrategyPattern,
                confidence: 0.75,
                adjustment: 0.6,
                description: "Strategy pattern detected".to_string(),
            })
        } else {
            None
        }
    }
}

impl PythonPattern for StrategyPattern {
    fn detect(&self, func_def: &ast::StmtFunctionDef) -> Option<PatternMatch> {
        self.detect_in_body(&func_def.body)
    }

    fn detect_async(&self, func_def: &ast::StmtAsyncFunctionDef) -> Option<PatternMatch> {
        self.detect_in_body(&func_def.body)
    }

    fn adjustment_factor(&self) -> f32 {
        0.6
    }

    fn clone_box(&self) -> Box<dyn PythonPattern> {
        Box::new(self.clone())
    }
}

impl StrategyPattern {
    fn has_strategy_invocation(&self, stmt: &ast::Stmt) -> bool {
        match stmt {
            ast::Stmt::Expr(expr_stmt) => self.is_strategy_method_call(&expr_stmt.value),
            ast::Stmt::Return(ret) => ret
                .value
                .as_ref()
                .is_some_and(|v| self.is_strategy_method_call(v)),
            _ => false,
        }
    }

    fn is_strategy_method_call(&self, expr: &ast::Expr) -> bool {
        if let ast::Expr::Call(call) = expr {
            if let ast::Expr::Attribute(attr) = &*call.func {
                return self.method_patterns.contains(attr.attr.as_ref());
            }
        }
        false
    }

    fn has_conditional_strategy_selection(&self, stmt: &ast::Stmt) -> bool {
        matches!(stmt, ast::Stmt::If(_) | ast::Stmt::Match(_))
    }
}

/// Chain of Responsibility pattern detector
#[derive(Debug, Clone, Default)]
pub struct ChainOfResponsibilityPattern;

impl ChainOfResponsibilityPattern {
    fn detect_in_body(&self, body: &[ast::Stmt]) -> Option<PatternMatch> {
        let mut has_handler_check = false;
        let mut has_next_call = false;
        let mut early_returns = 0;

        for stmt in body {
            if self.has_handler_condition(stmt) {
                has_handler_check = true;
            }
            if self.has_next_handler_call(stmt) {
                has_next_call = true;
            }
            if self.is_early_return(stmt) {
                early_returns += 1;
            }
        }

        if has_handler_check && (has_next_call || early_returns > 1) {
            Some(PatternMatch {
                pattern_type: PatternType::ChainOfResponsibility,
                confidence: 0.70,
                adjustment: 0.6,
                description: "Chain of Responsibility pattern detected".to_string(),
            })
        } else {
            None
        }
    }
}

impl PythonPattern for ChainOfResponsibilityPattern {
    fn detect(&self, func_def: &ast::StmtFunctionDef) -> Option<PatternMatch> {
        self.detect_in_body(&func_def.body)
    }

    fn detect_async(&self, func_def: &ast::StmtAsyncFunctionDef) -> Option<PatternMatch> {
        self.detect_in_body(&func_def.body)
    }

    fn adjustment_factor(&self) -> f32 {
        0.6
    }

    fn clone_box(&self) -> Box<dyn PythonPattern> {
        Box::new(self.clone())
    }
}

impl ChainOfResponsibilityPattern {
    fn has_handler_condition(&self, stmt: &ast::Stmt) -> bool {
        if let ast::Stmt::If(if_stmt) = stmt {
            // Check if condition involves can_handle or similar
            self.is_handler_check_expr(&if_stmt.test)
        } else {
            false
        }
    }

    fn is_handler_check_expr(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Call(call) => {
                if let ast::Expr::Attribute(attr) = &*call.func {
                    let method_name = attr.attr.to_string();
                    method_name.contains("handle")
                        || method_name.contains("can_")
                        || method_name.contains("supports")
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn has_next_handler_call(&self, stmt: &ast::Stmt) -> bool {
        match stmt {
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.is_next_handler_call(value)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn is_next_handler_call(&self, expr: &ast::Expr) -> bool {
        if let ast::Expr::Call(call) = expr {
            if let ast::Expr::Attribute(attr) = &*call.func {
                let attr_name = attr.attr.to_string();
                attr_name.contains("next") || attr_name.contains("successor")
            } else {
                false
            }
        } else {
            false
        }
    }

    fn is_early_return(&self, stmt: &ast::Stmt) -> bool {
        matches!(stmt, ast::Stmt::Return(_))
    }
}

/// Visitor pattern detector
#[derive(Debug, Clone)]
pub struct VisitorPatternDetector;

impl VisitorPatternDetector {
    fn detect_common(&self, func_name: &str, body: &[ast::Stmt]) -> Option<PatternMatch> {
        let has_visit_prefix = func_name.starts_with("visit_") || func_name.starts_with("accept");
        let has_dispatch = self.has_type_based_dispatch(body);

        if has_visit_prefix || has_dispatch {
            Some(PatternMatch {
                pattern_type: PatternType::VisitorPattern,
                confidence: if has_visit_prefix { 0.90 } else { 0.65 },
                adjustment: 0.5,
                description: "Visitor pattern detected".to_string(),
            })
        } else {
            None
        }
    }
}

impl PythonPattern for VisitorPatternDetector {
    fn detect(&self, func_def: &ast::StmtFunctionDef) -> Option<PatternMatch> {
        self.detect_common(func_def.name.as_ref(), &func_def.body)
    }

    fn detect_async(&self, func_def: &ast::StmtAsyncFunctionDef) -> Option<PatternMatch> {
        self.detect_common(func_def.name.as_ref(), &func_def.body)
    }

    fn adjustment_factor(&self) -> f32 {
        0.5
    }

    fn clone_box(&self) -> Box<dyn PythonPattern> {
        Box::new(self.clone())
    }
}

impl VisitorPatternDetector {
    fn has_type_based_dispatch(&self, body: &[ast::Stmt]) -> bool {
        body.iter().any(|stmt| {
            if let ast::Stmt::If(if_stmt) = stmt {
                self.is_isinstance_check(&if_stmt.test)
            } else if let ast::Stmt::Match(match_stmt) = stmt {
                // Pattern matching on type
                !match_stmt.cases.is_empty()
            } else {
                false
            }
        })
    }

    fn is_isinstance_check(&self, expr: &ast::Expr) -> bool {
        if let ast::Expr::Call(call) = expr {
            if let ast::Expr::Name(name) = &*call.func {
                return name.id.as_str() == "isinstance" || name.id.as_str() == "type";
            }
        }
        false
    }
}

/// Decorator pattern detector
#[derive(Debug, Clone)]
pub struct DecoratorPatternDetector;

impl DecoratorPatternDetector {
    fn detect_common(
        &self,
        decorator_list: &[ast::Expr],
        body: &[ast::Stmt],
    ) -> Option<PatternMatch> {
        // Check if function has decorators
        if !decorator_list.is_empty() {
            return Some(PatternMatch {
                pattern_type: PatternType::DecoratorPattern,
                confidence: 0.95,
                adjustment: 0.7,
                description: format!("Function uses {} decorator(s)", decorator_list.len()),
            });
        }

        // Check if function returns a wrapper function
        if self.returns_wrapper_function(body) {
            return Some(PatternMatch {
                pattern_type: PatternType::DecoratorPattern,
                confidence: 0.85,
                adjustment: 0.7,
                description: "Function implements decorator pattern".to_string(),
            });
        }

        None
    }
}

impl PythonPattern for DecoratorPatternDetector {
    fn detect(&self, func_def: &ast::StmtFunctionDef) -> Option<PatternMatch> {
        self.detect_common(&func_def.decorator_list, &func_def.body)
    }

    fn detect_async(&self, func_def: &ast::StmtAsyncFunctionDef) -> Option<PatternMatch> {
        self.detect_common(&func_def.decorator_list, &func_def.body)
    }

    fn adjustment_factor(&self) -> f32 {
        0.7
    }

    fn clone_box(&self) -> Box<dyn PythonPattern> {
        Box::new(self.clone())
    }
}

impl DecoratorPatternDetector {
    fn returns_wrapper_function(&self, body: &[ast::Stmt]) -> bool {
        body.iter().any(|stmt| {
            // Check if there's a nested function definition that gets returned
            if let ast::Stmt::FunctionDef(_) = stmt {
                // Look for a return statement that returns this function
                body.iter().any(|s| matches!(s, ast::Stmt::Return(_)))
            } else {
                false
            }
        })
    }
}

/// Context manager pattern detector
#[derive(Debug, Clone)]
pub struct ContextManagerPatternDetector;

impl ContextManagerPatternDetector {
    fn detect_common(&self, func_name: &str, body: &[ast::Stmt]) -> Option<PatternMatch> {
        // Check for __enter__ and __exit__ methods
        if func_name == "__enter__" || func_name == "__exit__" {
            return Some(PatternMatch {
                pattern_type: PatternType::ContextManager,
                confidence: 1.0,
                adjustment: 0.8,
                description: "Context manager method".to_string(),
            });
        }

        // Check for with statement usage
        let with_count = self.count_with_statements(body);
        if with_count >= 2 {
            return Some(PatternMatch {
                pattern_type: PatternType::ContextManager,
                confidence: 0.7,
                adjustment: 0.8,
                description: format!("Uses {} context managers", with_count),
            });
        }

        None
    }
}

impl PythonPattern for ContextManagerPatternDetector {
    fn detect(&self, func_def: &ast::StmtFunctionDef) -> Option<PatternMatch> {
        self.detect_common(func_def.name.as_ref(), &func_def.body)
    }

    fn detect_async(&self, func_def: &ast::StmtAsyncFunctionDef) -> Option<PatternMatch> {
        self.detect_common(func_def.name.as_ref(), &func_def.body)
    }

    fn adjustment_factor(&self) -> f32 {
        0.8
    }

    fn clone_box(&self) -> Box<dyn PythonPattern> {
        Box::new(self.clone())
    }
}

impl ContextManagerPatternDetector {
    fn count_with_statements(&self, body: &[ast::Stmt]) -> usize {
        body.iter()
            .filter(|stmt| matches!(stmt, ast::Stmt::With(_) | ast::Stmt::AsyncWith(_)))
            .count()
    }
}

impl Default for PythonPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonPatternDetector {
    pub fn new() -> Self {
        let config = AdjustmentConfig::default();
        let mut patterns: Vec<Box<dyn PythonPattern>> = Vec::new();

        if config.enable_dictionary_dispatch {
            patterns.push(Box::new(DictionaryDispatchPattern::default()));
        }
        if config.enable_strategy_pattern {
            patterns.push(Box::new(StrategyPattern::default()));
        }
        if config.enable_chain_of_responsibility {
            patterns.push(Box::new(ChainOfResponsibilityPattern));
        }
        if config.enable_visitor_pattern {
            patterns.push(Box::new(VisitorPatternDetector));
        }
        if config.enable_decorator_pattern {
            patterns.push(Box::new(DecoratorPatternDetector));
        }
        if config.enable_context_manager {
            patterns.push(Box::new(ContextManagerPatternDetector));
        }

        Self {
            patterns,
            adjustment_config: config,
        }
    }

    pub fn with_config(config: AdjustmentConfig) -> Self {
        let mut patterns: Vec<Box<dyn PythonPattern>> = Vec::new();

        if config.enable_dictionary_dispatch {
            patterns.push(Box::new(DictionaryDispatchPattern::default()));
        }
        if config.enable_strategy_pattern {
            patterns.push(Box::new(StrategyPattern::default()));
        }
        if config.enable_chain_of_responsibility {
            patterns.push(Box::new(ChainOfResponsibilityPattern));
        }
        if config.enable_visitor_pattern {
            patterns.push(Box::new(VisitorPatternDetector));
        }
        if config.enable_decorator_pattern {
            patterns.push(Box::new(DecoratorPatternDetector));
        }
        if config.enable_context_manager {
            patterns.push(Box::new(ContextManagerPatternDetector));
        }

        Self {
            patterns,
            adjustment_config: config,
        }
    }
}

/// Detect patterns in a Python function
pub fn detect_patterns(func_def: &ast::StmtFunctionDef) -> Vec<PatternMatch> {
    let detector = PythonPatternDetector::new();
    detector
        .patterns
        .iter()
        .filter_map(|pattern| pattern.detect(func_def))
        .collect()
}

/// Detect patterns in an async Python function
pub fn detect_patterns_async(func_def: &ast::StmtAsyncFunctionDef) -> Vec<PatternMatch> {
    let detector = PythonPatternDetector::new();
    detector
        .patterns
        .iter()
        .filter_map(|pattern| pattern.detect_async(func_def))
        .collect()
}

/// Apply pattern-based adjustments to complexity
pub fn apply_adjustments(base_complexity: u32, patterns: &[PatternMatch]) -> u32 {
    if patterns.is_empty() {
        return base_complexity;
    }

    // Find the pattern with the strongest adjustment
    let max_adjustment = patterns
        .iter()
        .map(|p| p.adjustment)
        .fold(1.0_f32, |a, b| a.min(b));

    // Apply the adjustment
    let adjusted = (base_complexity as f32 * max_adjustment) as u32;

    // Ensure we don't reduce below 1
    adjusted.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustpython_parser;

    #[test]
    fn test_dictionary_dispatch_detection() {
        let code = r#"
def dispatch(action):
    actions = {
        'start': lambda: print('Starting'),
        'stop': lambda: print('Stopping'),
        'pause': lambda: print('Pausing'),
    }
    return actions.get(action, lambda: None)()
"#;

        let module =
            rustpython_parser::parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
        if let ast::Mod::Module(module) = module {
            if let ast::Stmt::FunctionDef(func_def) = &module.body[0] {
                let patterns = detect_patterns(func_def);
                assert!(!patterns.is_empty());
                assert_eq!(patterns[0].pattern_type, PatternType::DictionaryDispatch);
            }
        }
    }

    #[test]
    fn test_decorator_pattern_detection() {
        let code = r#"
@property
@cached
def expensive_property(self):
    return self._calculate_value()
"#;

        let module =
            rustpython_parser::parse(code, rustpython_parser::Mode::Module, "<test>").unwrap();
        if let ast::Mod::Module(module) = module {
            if let ast::Stmt::FunctionDef(func_def) = &module.body[0] {
                let patterns = detect_patterns(func_def);
                assert!(!patterns.is_empty());
                assert_eq!(patterns[0].pattern_type, PatternType::DecoratorPattern);
            }
        }
    }

    #[test]
    fn test_complexity_adjustment() {
        let patterns = vec![PatternMatch {
            pattern_type: PatternType::DictionaryDispatch,
            confidence: 0.85,
            adjustment: 0.5,
            description: "test".to_string(),
        }];

        let adjusted = apply_adjustments(10, &patterns);
        assert_eq!(adjusted, 5);

        let adjusted_zero = apply_adjustments(1, &patterns);
        assert_eq!(adjusted_zero, 1); // Should not go below 1
    }
}
