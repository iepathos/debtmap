//! Main purity detector for TypeScript/JavaScript
//!
//! Analyzes function bodies to determine their purity level by visiting AST nodes.

use super::patterns::{
    is_browser_io_global, is_collection_mutation, is_dom_mutation, is_dom_mutation_property,
    is_dynamic_eval, is_known_pure_global, is_known_pure_method, is_node_io,
    is_non_deterministic_global, is_non_deterministic_method, is_object_mutation,
    is_time_dependent_constructor,
};
use super::scope::JsScopeTracker;
use super::types::{JsImpurityReason, JsPurityAnalysis};
use crate::core::PurityLevel;
use tree_sitter::Node;

// ============================================================================
// Confidence scoring constants
// ============================================================================

/// Base confidence level before adjustments (0.8 = 80%)
const BASE_CONFIDENCE: f32 = 0.8;

/// Boost applied when a known pure global function is called (e.g., Math.sqrt)
const KNOWN_PURE_GLOBAL_BOOST: f32 = 0.1;

/// Boost applied when a known pure method is called (e.g., .map, .filter)
const KNOWN_PURE_METHOD_BOOST: f32 = 0.1;

/// Boost applied for pure constructors (e.g., new Array())
const PURE_CONSTRUCTOR_BOOST: f32 = 0.05;

/// Penalty applied when an unknown direct function call is made
const UNKNOWN_DIRECT_CALL_PENALTY: f32 = 0.1;

/// Penalty applied for complex call expressions we can't fully analyze
const COMPLEX_CALL_PENALTY: f32 = 0.05;

/// Penalty applied for unknown method calls on non-local objects
const UNKNOWN_METHOD_PENALTY: f32 = 0.03;

/// Boost applied when clear impurity indicators are found (increases certainty)
const CLEAR_IMPURITY_BOOST: f32 = 0.1;

/// Weight for unknown call ratio in confidence calculation
const UNKNOWN_CALL_RATIO_WEIGHT: f32 = 0.2;

// ============================================================================
// Pure helper types and functions for assignment analysis
// ============================================================================

/// Result of classifying a mutation's effect
enum MutationEffect {
    /// External mutation with an impurity reason
    External(JsImpurityReason),
    /// Local mutation (no impurity reason needed)
    Local,
}

/// Classify member mutation (e.g., `obj.prop = value`)
///
/// Returns the mutation effect based on the object and property being assigned.
/// This is a pure function that doesn't modify any state.
fn classify_member_mutation(
    object: &str,
    property: &str,
    scope: &JsScopeTracker,
) -> Option<MutationEffect> {
    // this.x = y
    if object == "this" {
        return Some(MutationEffect::External(JsImpurityReason::ExternalMutation(
            format!("this.{}", property),
        )));
    }

    // window.x = y or globalThis.x = y
    if object == "window" || object == "globalThis" {
        return Some(MutationEffect::External(JsImpurityReason::GlobalAccess(
            format!("{}.{}", object, property),
        )));
    }

    // DOM property mutation
    if is_dom_mutation_property(property) {
        return Some(MutationEffect::External(JsImpurityReason::DomMutation(
            format!("{}.{}", object, property),
        )));
    }

    // Parameter property mutation - affects caller's object
    if scope.is_param(object) {
        return Some(MutationEffect::External(
            JsImpurityReason::ParameterMutation(format!("{}.{}", object, property)),
        ));
    }

    // Local variable property mutation
    if scope.is_local(object) {
        return Some(MutationEffect::Local);
    }

    // External object mutation
    Some(MutationEffect::External(JsImpurityReason::ExternalMutation(
        format!("{}.{}", object, property),
    )))
}

/// Classify subscript mutation (e.g., `arr[i] = value`)
///
/// Returns the mutation effect based on the object being indexed.
/// This is a pure function that doesn't modify any state.
fn classify_subscript_mutation(obj_name: &str, scope: &JsScopeTracker) -> Option<MutationEffect> {
    // Parameter array/object mutation affects caller
    if scope.is_param(obj_name) {
        return Some(MutationEffect::External(
            JsImpurityReason::ParameterMutation(format!("{}[...]", obj_name)),
        ));
    }

    // Local variable mutation
    if scope.is_local(obj_name) {
        return Some(MutationEffect::Local);
    }

    // External array/object mutation
    Some(MutationEffect::External(JsImpurityReason::ExternalMutation(
        format!("{}[...]", obj_name),
    )))
}

/// Purity analyzer for TypeScript/JavaScript functions
pub struct TypeScriptPurityAnalyzer<'a> {
    /// Source code for extracting text
    source: &'a str,
    /// Scope tracker for variable locality
    scope: JsScopeTracker,
    /// Whether I/O operations were detected
    has_io: bool,
    /// Whether external state mutation was detected
    has_external_mutation: bool,
    /// Whether external state was read
    has_external_read: bool,
    /// Whether local mutations were detected (arrays, objects)
    has_local_mutation: bool,
    /// Impurity reasons collected
    reasons: Vec<JsImpurityReason>,
    /// Confidence adjustments
    confidence_boost: f32,
    confidence_penalty: f32,
    /// Number of analyzed calls (for confidence calculation)
    analyzed_calls: u32,
    /// Number of unknown calls
    unknown_calls: u32,
}

impl<'a> TypeScriptPurityAnalyzer<'a> {
    /// Create a new analyzer
    fn new(source: &'a str) -> Self {
        Self {
            source,
            scope: JsScopeTracker::new(),
            has_io: false,
            has_external_mutation: false,
            has_external_read: false,
            has_local_mutation: false,
            reasons: Vec::new(),
            confidence_boost: 0.0,
            confidence_penalty: 0.0,
            analyzed_calls: 0,
            unknown_calls: 0,
        }
    }

    /// Analyze a function body and return purity analysis
    ///
    /// # Arguments
    /// * `body` - The function body node to analyze
    /// * `source` - The source code for text extraction
    /// * `params` - Optional list of parameter names for scope tracking
    pub fn analyze(body: &Node, source: &'a str, params: Option<Vec<String>>) -> JsPurityAnalysis {
        let mut analyzer = Self::new(source);

        // Set up initial scope with parameters
        if let Some(param_names) = params {
            analyzer.scope.enter_function(param_names);
        } else {
            analyzer.scope.enter_function(vec![]);
        }

        // Visit all nodes in the body
        analyzer.visit_node(body);

        analyzer.into_result()
    }

    /// Analyze a function body with parameter extraction from a function node
    pub fn analyze_function(func_node: &Node, source: &'a str) -> JsPurityAnalysis {
        // Extract parameters
        let params = extract_function_params(func_node, source);

        // Find the body
        if let Some(body) = func_node.child_by_field_name("body") {
            Self::analyze(&body, source, Some(params))
        } else {
            // No body found - likely an expression body arrow function
            // The whole node is the body
            Self::analyze(func_node, source, Some(params))
        }
    }

    /// Visit a node and analyze its purity implications
    fn visit_node(&mut self, node: &Node) {
        match node.kind() {
            // Variable declarations - add to scope
            "variable_declarator" => {
                self.handle_variable_declarator(node);
            }
            "lexical_declaration" | "variable_declaration" => {
                self.visit_children(node);
                return; // Children handled recursively
            }

            // Function boundaries - don't recurse into nested functions
            // (they have their own purity analysis)
            "function_declaration" | "function_expression" | "generator_function_declaration" => {
                // Don't analyze nested function bodies
                return;
            }
            "arrow_function" => {
                // Don't analyze nested arrow function bodies
                return;
            }

            // Call expressions - main source of impurity
            "call_expression" => {
                self.analyze_call(node);
            }

            // Member expressions - may indicate external access
            "member_expression" => {
                self.analyze_member_access(node);
            }

            // Assignment expressions - may mutate external state
            "assignment_expression" | "augmented_assignment_expression" => {
                self.analyze_assignment(node);
            }

            // Update expressions (++, --)
            "update_expression" => {
                self.analyze_update(node);
            }

            // Await expressions - async I/O
            "await_expression" => {
                self.has_io = true;
                self.reasons.push(JsImpurityReason::AsyncOperation);
            }

            // Identifiers - check for global access
            "identifier" => {
                self.check_global_access(node);
            }

            // Block scopes
            "statement_block" | "block" => {
                self.scope.enter_block();
                self.visit_children(node);
                self.scope.exit_scope();
                return;
            }

            // For loops with variable declaration
            "for_statement" | "for_in_statement" | "for_of_statement" => {
                self.scope.enter_block();
                self.visit_children(node);
                self.scope.exit_scope();
                return;
            }

            // Delete expressions
            "delete_expression" => {
                // delete obj.prop is a mutation
                self.has_external_mutation = true;
                self.reasons
                    .push(JsImpurityReason::ExternalMutation("delete".to_string()));
            }

            // New expressions - usually pure, but some are non-deterministic
            "new_expression" => {
                self.analyzed_calls += 1;
                // Check for time-dependent constructors like `new Date()`
                if let Some(constructor) = node.child_by_field_name("constructor") {
                    let name = self.node_text(&constructor);
                    if is_time_dependent_constructor(&name) {
                        // new Date() without arguments is non-deterministic
                        let args = node.child_by_field_name("arguments");
                        let has_args = args.map(|a| a.named_child_count() > 0).unwrap_or(false);
                        if !has_args {
                            self.has_io = true;
                            self.reasons
                                .push(JsImpurityReason::NonDeterministic(format!(
                                    "new {}()",
                                    name
                                )));
                            return;
                        }
                    }
                }
                // Most other constructors are pure (create new object)
                self.confidence_boost += PURE_CONSTRUCTOR_BOOST;
            }

            _ => {}
        }

        // Recurse into children
        self.visit_children(node);
    }

    /// Visit all child nodes
    fn visit_children(&mut self, node: &Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(&child);
        }
    }

    /// Handle variable declarator - add to scope
    fn handle_variable_declarator(&mut self, node: &Node) {
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = self.node_text(&name_node);
            self.scope.declare_variable(&name);
        }
        // Visit the value if present
        if let Some(value) = node.child_by_field_name("value") {
            self.visit_node(&value);
        }
    }

    /// Analyze a call expression for impurity
    fn analyze_call(&mut self, node: &Node) {
        self.analyzed_calls += 1;

        // Get the function being called
        if let Some(function) = node.child_by_field_name("function") {
            let call_text = self.node_text(&function);

            // Check for dynamic eval
            if is_dynamic_eval(&call_text) {
                self.has_io = true;
                self.reasons.push(JsImpurityReason::DynamicEval);
                return;
            }

            // Check for non-deterministic global calls (Math.random, Date.now, etc.)
            if is_non_deterministic_global(&call_text) {
                self.has_io = true;
                self.reasons
                    .push(JsImpurityReason::NonDeterministic(call_text.clone()));
                return;
            }

            // Check for known pure globals
            if is_known_pure_global(&call_text) {
                self.confidence_boost += KNOWN_PURE_GLOBAL_BOOST;
                return;
            }

            match function.kind() {
                // Direct function call: foo()
                "identifier" => {
                    let name = self.node_text(&function);

                    // Check browser I/O
                    if is_browser_io_global(&name) {
                        self.has_io = true;
                        self.reasons.push(JsImpurityReason::BrowserIO(name));
                        return;
                    }

                    // Check Node.js I/O
                    if is_node_io(&name) {
                        self.has_io = true;
                        self.reasons.push(JsImpurityReason::NodeIO(name));
                        return;
                    }

                    // Check dynamic eval
                    if is_dynamic_eval(&name) {
                        self.has_io = true;
                        self.reasons.push(JsImpurityReason::DynamicEval);
                        return;
                    }

                    // Unknown function - might be impure
                    if !self.scope.is_local(&name) {
                        self.unknown_calls += 1;
                        self.confidence_penalty += UNKNOWN_DIRECT_CALL_PENALTY;
                    }
                }

                // Method call: obj.method()
                "member_expression" => {
                    self.analyze_method_call(&function);
                }

                _ => {
                    // Complex call expression - reduce confidence
                    self.unknown_calls += 1;
                    self.confidence_penalty += COMPLEX_CALL_PENALTY;
                }
            }
        }

        // Visit arguments (they might have side effects)
        if let Some(args) = node.child_by_field_name("arguments") {
            self.visit_children(&args);
        }
    }

    /// Analyze a method call: obj.method(args)
    fn analyze_method_call(&mut self, member_expr: &Node) {
        let (object, property) = self.extract_member_parts(member_expr);

        // Check for console.log, console.error, etc.
        if object == "console" {
            self.has_io = true;
            self.reasons
                .push(JsImpurityReason::BrowserIO(format!("console.{}", property)));
            return;
        }

        // Check for document/window methods
        if object == "document" || object == "window" {
            self.has_io = true;
            self.reasons.push(JsImpurityReason::BrowserIO(format!(
                "{}.{}",
                object, property
            )));
            return;
        }

        // Check for non-deterministic methods (Math.random, Date.now, etc.)
        if is_non_deterministic_method(&property) {
            self.has_io = true;
            self.reasons
                .push(JsImpurityReason::NonDeterministic(format!(
                    "{}.{}",
                    object, property
                )));
            return;
        }

        // Check for known pure methods
        if is_known_pure_method(&property) {
            self.confidence_boost += KNOWN_PURE_METHOD_BOOST;
            return;
        }

        // Check for collection mutations
        if is_collection_mutation(&property) {
            // Is it a local variable?
            if self.scope.is_local(&object) {
                self.has_local_mutation = true;
            } else {
                self.has_external_mutation = true;
                self.reasons
                    .push(JsImpurityReason::CollectionMutation(format!(
                        "{}.{}",
                        object, property
                    )));
            }
            return;
        }

        // Check for DOM mutations
        if is_dom_mutation(&property) {
            self.has_external_mutation = true;
            self.reasons.push(JsImpurityReason::DomMutation(format!(
                "{}.{}",
                object, property
            )));
            return;
        }

        // Check for Object mutations (Object.assign, etc.)
        if object == "Object" && is_object_mutation(&property) {
            // Object.assign modifies first argument
            self.has_external_mutation = true;
            self.reasons
                .push(JsImpurityReason::ExternalMutation(format!(
                    "Object.{}",
                    property
                )));
            return;
        }

        // Check for fs, http, etc.
        if is_node_io(&object) {
            self.has_io = true;
            self.reasons
                .push(JsImpurityReason::NodeIO(format!("{}.{}", object, property)));
            return;
        }

        // Check for process access
        if object == "process" {
            match property.as_str() {
                "env" | "cwd" | "exit" | "abort" | "kill" => {
                    self.has_io = true;
                    self.reasons
                        .push(JsImpurityReason::NodeIO(format!("process.{}", property)));
                }
                _ => {
                    self.has_external_read = true;
                    self.reasons.push(JsImpurityReason::ExternalRead(format!(
                        "process.{}",
                        property
                    )));
                }
            }
            return;
        }

        // Unknown method call - reduce confidence slightly
        if !self.scope.is_local(&object) {
            self.unknown_calls += 1;
            self.confidence_penalty += UNKNOWN_METHOD_PENALTY;
        }
    }

    /// Analyze member access for external state reads
    fn analyze_member_access(&mut self, node: &Node) {
        let (object, property) = self.extract_member_parts(node);

        // Check for global access in read context
        if is_browser_io_global(&object) && !self.is_in_call_context(node) {
            self.has_external_read = true;
            self.reasons.push(JsImpurityReason::ExternalRead(format!(
                "{}.{}",
                object, property
            )));
        }
    }

    /// Analyze an assignment expression
    fn analyze_assignment(&mut self, node: &Node) {
        if let Some(left) = node.child_by_field_name("left") {
            match left.kind() {
                "identifier" => self.analyze_identifier_assignment(&left),
                "member_expression" => self.analyze_member_assignment(&left),
                "subscript_expression" => self.analyze_subscript_assignment(&left),
                _ => {}
            }
        }

        // Visit the right side (might have side effects)
        if let Some(right) = node.child_by_field_name("right") {
            self.visit_node(&right);
        }
    }

    /// Analyze assignment to a simple identifier (e.g., `x = 5`)
    fn analyze_identifier_assignment(&mut self, left: &Node) {
        let name = self.node_text(left);
        if self.scope.is_param(&name) {
            self.has_external_mutation = true;
            self.reasons.push(JsImpurityReason::ParameterMutation(name));
        } else if !self.scope.is_local(&name) {
            self.has_external_mutation = true;
            self.reasons.push(JsImpurityReason::ExternalMutation(name));
        }
    }

    /// Analyze assignment to a member expression (e.g., `obj.prop = value`)
    fn analyze_member_assignment(&mut self, left: &Node) {
        let (object, property) = self.extract_member_parts(left);

        if let Some(effect) = classify_member_mutation(&object, &property, &self.scope) {
            match effect {
                MutationEffect::External(reason) => {
                    self.has_external_mutation = true;
                    self.reasons.push(reason);
                }
                MutationEffect::Local => {
                    self.has_local_mutation = true;
                }
            }
        }
    }

    /// Analyze assignment to a subscript expression (e.g., `arr[i] = value`)
    fn analyze_subscript_assignment(&mut self, left: &Node) {
        if let Some(obj) = left.child_by_field_name("object") {
            let obj_name = self.node_text(&obj);
            if let Some(effect) = classify_subscript_mutation(&obj_name, &self.scope) {
                match effect {
                    MutationEffect::External(reason) => {
                        self.has_external_mutation = true;
                        self.reasons.push(reason);
                    }
                    MutationEffect::Local => {
                        self.has_local_mutation = true;
                    }
                }
            }
        }
    }

    /// Analyze update expression (++, --)
    fn analyze_update(&mut self, node: &Node) {
        if let Some(argument) = node.child_by_field_name("argument") {
            match argument.kind() {
                "identifier" => {
                    let name = self.node_text(&argument);
                    if !self.scope.is_local(&name) {
                        self.has_external_mutation = true;
                        self.reasons
                            .push(JsImpurityReason::ExternalMutation(format!("{}++/--", name)));
                    } else {
                        self.has_local_mutation = true;
                    }
                }
                "member_expression" => {
                    let (object, property) = self.extract_member_parts(&argument);
                    if self.scope.is_local(&object) {
                        self.has_local_mutation = true;
                    } else {
                        self.has_external_mutation = true;
                        self.reasons
                            .push(JsImpurityReason::ExternalMutation(format!(
                                "{}.{}++/--",
                                object, property
                            )));
                    }
                }
                _ => {}
            }
        }
    }

    /// Check identifier for global access
    fn check_global_access(&mut self, node: &Node) {
        let name = self.node_text(node);

        // Skip if local variable
        if self.scope.is_local(&name) {
            return;
        }

        // Check for global I/O access in non-call context
        if is_browser_io_global(&name) && !self.is_in_call_context(node) {
            // Reading from a global like window or document
            self.has_external_read = true;
            self.reasons
                .push(JsImpurityReason::GlobalAccess(name.clone()));
        }

        if is_node_io(&name) && !self.is_in_call_context(node) {
            self.has_external_read = true;
            self.reasons
                .push(JsImpurityReason::GlobalAccess(name.clone()));
        }
    }

    /// Check if a node is in a call context (being called, not just accessed)
    fn is_in_call_context(&self, node: &Node) -> bool {
        if let Some(parent) = node.parent() {
            parent.kind() == "call_expression" || parent.kind() == "member_expression"
        } else {
            false
        }
    }

    /// Extract object and property from a member expression
    fn extract_member_parts(&self, node: &Node) -> (String, String) {
        let object = node
            .child_by_field_name("object")
            .map(|n| self.node_text(&n))
            .unwrap_or_default();
        let property = node
            .child_by_field_name("property")
            .map(|n| self.node_text(&n))
            .unwrap_or_default();
        (object, property)
    }

    /// Get text for a node
    fn node_text(&self, node: &Node) -> String {
        node.utf8_text(self.source.as_bytes())
            .unwrap_or("")
            .to_string()
    }

    /// Convert analysis state into result
    fn into_result(self) -> JsPurityAnalysis {
        let level = self.determine_level();
        let confidence = self.calculate_confidence();

        JsPurityAnalysis {
            level,
            confidence,
            reasons: self.reasons,
        }
    }

    /// Determine the purity level based on analysis
    fn determine_level(&self) -> PurityLevel {
        if self.has_io || self.has_external_mutation {
            PurityLevel::Impure
        } else if self.has_external_read {
            PurityLevel::ReadOnly
        } else if self.has_local_mutation {
            PurityLevel::LocallyPure
        } else {
            PurityLevel::StrictlyPure
        }
    }

    /// Calculate confidence score
    fn calculate_confidence(&self) -> f32 {
        let mut confidence = BASE_CONFIDENCE;

        // Adjust for known pure/impure calls
        confidence += self.confidence_boost;
        confidence -= self.confidence_penalty;

        // Adjust for unknown calls ratio
        if self.analyzed_calls > 0 {
            let unknown_ratio = self.unknown_calls as f32 / self.analyzed_calls as f32;
            confidence -= unknown_ratio * UNKNOWN_CALL_RATIO_WEIGHT;
        }

        // Boost confidence if we found clear impurity indicators
        if !self.reasons.is_empty() {
            confidence += CLEAR_IMPURITY_BOOST;
        }

        // Clamp to valid range
        confidence.clamp(0.0, 1.0)
    }
}

/// Extract parameter names from a function node
fn extract_function_params(node: &Node, source: &str) -> Vec<String> {
    let mut params = Vec::new();

    // Try to get parameters field
    if let Some(params_node) = node.child_by_field_name("parameters") {
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            match child.kind() {
                "identifier" | "shorthand_property_identifier" => {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        params.push(text.to_string());
                    }
                }
                "required_parameter" | "optional_parameter" => {
                    // TypeScript parameter - get the pattern/name
                    if let Some(pattern) = child.child_by_field_name("pattern") {
                        if let Ok(text) = pattern.utf8_text(source.as_bytes()) {
                            params.push(text.to_string());
                        }
                    }
                }
                "assignment_pattern" => {
                    // Default parameter: x = defaultValue
                    if let Some(left) = child.child_by_field_name("left") {
                        if let Ok(text) = left.utf8_text(source.as_bytes()) {
                            params.push(text.to_string());
                        }
                    }
                }
                "rest_pattern" => {
                    // Rest parameter: ...args
                    let mut cursor2 = child.walk();
                    for rest_child in child.children(&mut cursor2) {
                        if rest_child.kind() == "identifier" {
                            if let Ok(text) = rest_child.utf8_text(source.as_bytes()) {
                                params.push(text.to_string());
                            }
                        }
                    }
                }
                "object_pattern" | "array_pattern" => {
                    // Destructuring - extract all identifiers
                    extract_pattern_identifiers(&child, source, &mut params);
                }
                _ => {}
            }
        }
    }

    // Also check for single parameter (arrow functions: x => x + 1)
    if let Some(param) = node.child_by_field_name("parameter") {
        if let Ok(text) = param.utf8_text(source.as_bytes()) {
            params.push(text.to_string());
        }
    }

    params
}

/// Extract identifiers from destructuring patterns
fn extract_pattern_identifiers(node: &Node, source: &str, params: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern" => {
                if let Ok(text) = child.utf8_text(source.as_bytes()) {
                    params.push(text.to_string());
                }
            }
            "object_pattern" | "array_pattern" | "pair_pattern" => {
                extract_pattern_identifiers(&child, source, params);
            }
            "assignment_pattern" => {
                // Get the left side of default assignment
                if let Some(left) = child.child_by_field_name("left") {
                    extract_pattern_identifiers(&left, source, params);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::typescript::parser::parse_source;
    use crate::core::ast::JsLanguageVariant;
    use std::path::PathBuf;

    fn analyze_code(source: &str) -> JsPurityAnalysis {
        let path = PathBuf::from("test.js");
        let ast = parse_source(source, &path, JsLanguageVariant::JavaScript).unwrap();
        let root = ast.tree.root_node();

        // Find the first function and analyze it
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                "function_declaration" => {
                    return TypeScriptPurityAnalyzer::analyze_function(&child, &ast.source);
                }
                "lexical_declaration" | "variable_declaration" => {
                    // Look for arrow function
                    let mut cursor2 = child.walk();
                    for decl_child in child.children(&mut cursor2) {
                        if decl_child.kind() == "variable_declarator" {
                            if let Some(value) = decl_child.child_by_field_name("value") {
                                if value.kind() == "arrow_function" {
                                    return TypeScriptPurityAnalyzer::analyze_function(
                                        &value,
                                        &ast.source,
                                    );
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Default to analyzing root
        TypeScriptPurityAnalyzer::analyze(&root, &ast.source, None)
    }

    #[test]
    fn test_pure_arithmetic() {
        let analysis = analyze_code("function add(a, b) { return a + b; }");
        assert_eq!(analysis.level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_pure_arrow_function() {
        let analysis = analyze_code("const add = (a, b) => a + b;");
        assert_eq!(analysis.level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_impure_console_log() {
        let analysis = analyze_code("function log(msg) { console.log(msg); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
        assert!(analysis
            .reasons
            .iter()
            .any(|r| matches!(r, JsImpurityReason::BrowserIO(_))));
    }

    #[test]
    fn test_impure_dom_mutation() {
        let analysis =
            analyze_code("function update(el) { el.appendChild(document.createElement('div')); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
    }

    #[test]
    fn test_impure_fetch() {
        let analysis = analyze_code("async function getData() { await fetch('/api'); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
        assert!(analysis
            .reasons
            .iter()
            .any(|r| matches!(r, JsImpurityReason::AsyncOperation)));
    }

    #[test]
    fn test_locally_pure_array_push() {
        let analysis =
            analyze_code("function build() { const arr = []; arr.push(1); return arr; }");
        // Local array mutation is LocallyPure
        assert_eq!(analysis.level, PurityLevel::LocallyPure);
    }

    #[test]
    fn test_impure_external_mutation() {
        let analysis = analyze_code("function mutate(obj) { obj.x = 5; }");
        // Mutating a parameter is external mutation
        assert_eq!(analysis.level, PurityLevel::Impure);
    }

    #[test]
    fn test_impure_this_mutation() {
        let analysis = analyze_code("function setName(name) { this.name = name; }");
        assert_eq!(analysis.level, PurityLevel::Impure);
        assert!(analysis
            .reasons
            .iter()
            .any(|r| matches!(r, JsImpurityReason::ExternalMutation(_))));
    }

    #[test]
    fn test_read_only_global_access() {
        let analysis = analyze_code("function getWidth() { return window.innerWidth; }");
        // Just reading from window is ReadOnly
        assert_eq!(analysis.level, PurityLevel::ReadOnly);
    }

    #[test]
    fn test_pure_with_known_pure_methods() {
        let analysis = analyze_code(
            "function transform(arr) { return arr.map(x => x * 2).filter(x => x > 5); }",
        );
        assert_eq!(analysis.level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_impure_eval() {
        let analysis = analyze_code("function dangerous(code) { eval(code); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
        assert!(analysis
            .reasons
            .iter()
            .any(|r| matches!(r, JsImpurityReason::DynamicEval)));
    }

    #[test]
    fn test_pure_math_operations() {
        let analysis =
            analyze_code("function calc(x) { return Math.sqrt(Math.abs(x)) + Math.PI; }");
        assert_eq!(analysis.level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_impure_settimeout() {
        let analysis = analyze_code("function delay() { setTimeout(() => {}, 100); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
    }

    #[test]
    fn test_locally_pure_with_local_object() {
        let analysis = analyze_code("function build() { const obj = {}; obj.x = 1; return obj; }");
        assert_eq!(analysis.level, PurityLevel::LocallyPure);
    }

    #[test]
    fn test_impure_node_fs() {
        // This would be analyzed differently in actual use with Node detection
        let analysis = analyze_code("function read() { fs.readFileSync('file.txt'); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
    }

    #[test]
    fn test_confidence_high_for_known_patterns() {
        let analysis = analyze_code("function log(msg) { console.log(msg); }");
        assert!(
            analysis.confidence >= 0.8,
            "Confidence should be high for known patterns"
        );
    }

    #[test]
    fn test_impure_math_random() {
        let analysis = analyze_code("function rand() { return Math.random(); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
        assert!(analysis
            .reasons
            .iter()
            .any(|r| matches!(r, JsImpurityReason::NonDeterministic(_))));
    }

    #[test]
    fn test_impure_date_now() {
        let analysis = analyze_code("function timestamp() { return Date.now(); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
        assert!(analysis
            .reasons
            .iter()
            .any(|r| matches!(r, JsImpurityReason::NonDeterministic(_))));
    }

    #[test]
    fn test_impure_new_date_no_args() {
        let analysis = analyze_code("function now() { return new Date(); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
        assert!(analysis
            .reasons
            .iter()
            .any(|r| matches!(r, JsImpurityReason::NonDeterministic(_))));
    }

    #[test]
    fn test_pure_new_date_with_args() {
        // new Date(timestamp) is deterministic - same input produces same output
        let analysis = analyze_code("function toDate(ts) { return new Date(ts); }");
        assert_eq!(analysis.level, PurityLevel::StrictlyPure);
    }

    #[test]
    fn test_impure_performance_now() {
        let analysis = analyze_code("function measure() { return performance.now(); }");
        assert_eq!(analysis.level, PurityLevel::Impure);
    }
}
