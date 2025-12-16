//! # God Object Classification (Pure Transformations)
//!
//! Pure functions for classifying and grouping god objects.
//!
//! All functions in this module are:
//! - Pure: No side effects, deterministic outputs
//! - Composable: Can be combined and chained
//! - Testable: No mocks or I/O needed

use std::collections::HashMap;

use super::classification_types::{
    ClassificationResult, MethodBodyAnalysis, MethodComplexityClass, ReturnExprType, SignalType,
};
use super::thresholds::GodObjectThresholds;
use super::types::GodObjectConfidence;
use crate::organization::confidence::MINIMUM_CONFIDENCE;

// ============================================================================
// Spec 209: Accessor and Boilerplate Method Detection
// ============================================================================

/// Classify a method's complexity for weighted god object scoring.
///
/// This is a pure function that analyzes method characteristics to determine
/// how much the method should contribute to god object detection.
///
/// # Arguments
///
/// * `method_name` - The name of the method
/// * `body_analysis` - Analysis of the method body (line count, control flow, etc.)
///
/// # Returns
///
/// A `MethodComplexityClass` indicating the method's complexity level.
///
/// # Classification Logic
///
/// 1. **Boilerplate detection by name**: Methods named `new`, `default`, `clone`,
///    `from`, `into` are classified as boilerplate (weight 0.0).
///
/// 2. **Accessor patterns**: Methods with accessor-style names (`get_*`, `set_*`,
///    `*_mut`, `is_*`, `has_*`) that are short and simple are classified as:
///    - `TrivialAccessor` (0.1): Single line, no control flow, returns field reference
///    - `SimpleAccessor` (0.3): Short (≤3 lines), no control flow
///
/// 3. **Delegating methods**: Short methods (≤2 lines) with exactly one call and
///    no control flow are classified as delegating (weight 0.5).
///
/// 4. **Default**: Methods that don't match any pattern are classified as
///    `Substantive` (weight 1.0).
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{
///     classify_method_complexity, MethodBodyAnalysis, MethodComplexityClass, ReturnExprType,
/// };
///
/// // Trivial accessor: get_name returning a field reference
/// let analysis = MethodBodyAnalysis {
///     line_count: 1,
///     has_control_flow: false,
///     call_count: 0,
///     return_expr_type: Some(ReturnExprType::FieldAccess),
///     has_self_param: true,
///     is_mutating: false,
/// };
/// assert_eq!(
///     classify_method_complexity("get_name", &analysis),
///     MethodComplexityClass::TrivialAccessor
/// );
///
/// // Boilerplate: constructor
/// let analysis = MethodBodyAnalysis {
///     line_count: 5,
///     has_control_flow: false,
///     call_count: 1,
///     return_expr_type: None,
///     has_self_param: false,
///     is_mutating: false,
/// };
/// assert_eq!(
///     classify_method_complexity("new", &analysis),
///     MethodComplexityClass::Boilerplate
/// );
///
/// // Substantive: complex business logic
/// let analysis = MethodBodyAnalysis {
///     line_count: 25,
///     has_control_flow: true,
///     call_count: 5,
///     return_expr_type: Some(ReturnExprType::Complex),
///     has_self_param: true,
///     is_mutating: true,
/// };
/// assert_eq!(
///     classify_method_complexity("analyze_workspace", &analysis),
///     MethodComplexityClass::Substantive
/// );
/// ```
pub fn classify_method_complexity(
    method_name: &str,
    body_analysis: &MethodBodyAnalysis,
) -> MethodComplexityClass {
    let name_lower = method_name.to_lowercase();

    // 1. Boilerplate detection by name
    // These methods are structural and don't contribute to god object sprawl
    if matches!(
        name_lower.as_str(),
        "new" | "default" | "clone" | "from" | "into" | "try_from" | "try_into"
    ) {
        return MethodComplexityClass::Boilerplate;
    }

    // Also detect trait implementation boilerplate
    if matches!(
        name_lower.as_str(),
        "fmt" | "eq" | "ne" | "hash" | "cmp" | "partial_cmp" | "drop"
    ) {
        return MethodComplexityClass::Boilerplate;
    }

    // 2. Accessor patterns
    let is_accessor_name = name_lower.starts_with("get_")
        || name_lower.starts_with("set_")
        || name_lower.ends_with("_mut")
        || name_lower.starts_with("is_")
        || name_lower.starts_with("has_")
        || name_lower.starts_with("with_"); // Builder-style setters

    // Trivial accessor: single line, no control flow, returns field reference
    if is_accessor_name && body_analysis.line_count <= 1 && !body_analysis.has_control_flow {
        if let Some(ReturnExprType::FieldAccess) = body_analysis.return_expr_type {
            return MethodComplexityClass::TrivialAccessor;
        }
        // Also trivial if it returns a literal (is_enabled returning a bool)
        if let Some(ReturnExprType::Literal) = body_analysis.return_expr_type {
            return MethodComplexityClass::TrivialAccessor;
        }
    }

    // Simple accessor: short accessor with minor work (e.g., .clone())
    if is_accessor_name && body_analysis.line_count <= 3 && !body_analysis.has_control_flow {
        return MethodComplexityClass::SimpleAccessor;
    }

    // 3. Delegating methods: short method that calls one other method
    if body_analysis.line_count <= 2
        && body_analysis.call_count == 1
        && !body_analysis.has_control_flow
    {
        return MethodComplexityClass::Delegating;
    }

    // 4. Default to substantive
    MethodComplexityClass::Substantive
}

/// Calculate the weighted method count for a collection of methods.
///
/// This applies the complexity weights from `MethodComplexityClass` to provide
/// a more accurate method count for god object scoring.
///
/// # Arguments
///
/// * `classes` - Iterator of complexity classes for each method
///
/// # Returns
///
/// Sum of weights for all methods.
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{
///     calculate_weighted_method_count, MethodComplexityClass,
/// };
///
/// let classes = vec![
///     MethodComplexityClass::TrivialAccessor,  // 0.1
///     MethodComplexityClass::TrivialAccessor,  // 0.1
///     MethodComplexityClass::Boilerplate,      // 0.0
///     MethodComplexityClass::Substantive,      // 1.0
/// ];
///
/// let weighted = calculate_weighted_method_count(classes.iter());
/// assert!((weighted - 1.2).abs() < 0.01);
/// ```
pub fn calculate_weighted_method_count<'a>(
    classes: impl Iterator<Item = &'a MethodComplexityClass>,
) -> f64 {
    classes.map(|c| c.weight()).sum()
}

/// Classify a method's complexity using only its name.
///
/// This is a simplified heuristic-based classification for when AST body analysis
/// is not available. It provides reasonable estimates based on naming conventions.
///
/// # Arguments
///
/// * `method_name` - The name of the method
///
/// # Returns
///
/// A `MethodComplexityClass` based on naming patterns.
///
/// # Classification Rules
///
/// 1. **Boilerplate** (0.0 weight): `new`, `default`, `clone`, `from`, `into`, `fmt`, `eq`, etc.
/// 2. **TrivialAccessor** (0.1 weight): `get_*`, `is_*`, `has_*` (assumed simple)
/// 3. **SimpleAccessor** (0.3 weight): `set_*`, `*_mut`, `with_*` (may do assignment)
/// 4. **Substantive** (1.0 weight): All other methods
///
/// Note: Without body analysis, we cannot detect delegating methods.
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{
///     classify_method_by_name, MethodComplexityClass,
/// };
///
/// assert_eq!(classify_method_by_name("new"), MethodComplexityClass::Boilerplate);
/// assert_eq!(classify_method_by_name("get_name"), MethodComplexityClass::TrivialAccessor);
/// assert_eq!(classify_method_by_name("set_value"), MethodComplexityClass::SimpleAccessor);
/// assert_eq!(classify_method_by_name("process_data"), MethodComplexityClass::Substantive);
/// ```
pub fn classify_method_by_name(method_name: &str) -> MethodComplexityClass {
    let name_lower = method_name.to_lowercase();

    // 1. Boilerplate detection by name
    if matches!(
        name_lower.as_str(),
        "new"
            | "default"
            | "clone"
            | "from"
            | "into"
            | "try_from"
            | "try_into"
            | "fmt"
            | "eq"
            | "ne"
            | "hash"
            | "cmp"
            | "partial_cmp"
            | "drop"
            | "deref"
            | "deref_mut"
            | "borrow"
            | "borrow_mut"
            | "as_ref"
            | "as_mut"
            | "index"
            | "index_mut"
    ) {
        return MethodComplexityClass::Boilerplate;
    }

    // 2. Trivial accessor patterns (read-only, assumed single-line)
    if name_lower.starts_with("get_")
        || name_lower.starts_with("is_")
        || name_lower.starts_with("has_")
        || name_lower == "len"
        || name_lower == "is_empty"
    {
        return MethodComplexityClass::TrivialAccessor;
    }

    // 3. Simple accessor patterns (may do assignment, but still simple)
    if name_lower.starts_with("set_")
        || name_lower.ends_with("_mut")
        || name_lower.starts_with("with_")
    {
        return MethodComplexityClass::SimpleAccessor;
    }

    // 4. Default to substantive
    MethodComplexityClass::Substantive
}

/// Calculate weighted method count from method names only.
///
/// This is a convenience function that applies `classify_method_by_name` to each
/// method name and sums their weights. Use this when AST body analysis is not available.
///
/// # Arguments
///
/// * `method_names` - Iterator of method names
///
/// # Returns
///
/// Sum of weights for all methods.
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::calculate_weighted_count_from_names;
///
/// let methods = vec![
///     "new",           // 0.0 (boilerplate)
///     "get_name",      // 0.1 (trivial)
///     "set_value",     // 0.3 (simple)
///     "process_data",  // 1.0 (substantive)
/// ];
///
/// let weighted = calculate_weighted_count_from_names(methods.iter().map(|s| *s));
/// assert!((weighted - 1.4).abs() < 0.01);
/// ```
pub fn calculate_weighted_count_from_names<'a>(method_names: impl Iterator<Item = &'a str>) -> f64 {
    method_names
        .map(|name| classify_method_by_name(name).weight())
        .sum()
}

// ============================================================================
// Spec 213: Pure Function Method Weighting
// ============================================================================

use super::classification_types::MethodSelfUsage;
use syn::visit::Visit;

/// Classify a method's self-usage from its signature and body (Spec 213).
///
/// This function analyzes whether a method uses `self` (instance state) or is a
/// pure associated function. Pure functions get reduced weight in god object
/// detection because they indicate functional decomposition rather than bloat.
///
/// # Arguments
///
/// * `method` - The method to analyze
///
/// # Returns
///
/// A `MethodSelfUsage` classification:
/// - `PureAssociated` (0.2 weight): No self parameter
/// - `UnusedSelf` (0.3 weight): Has self but doesn't use it
/// - `InstanceMethod` (1.0 weight): Has self and uses it
///
/// # Examples
///
/// ```ignore
/// // Pure associated function - no self parameter
/// fn helper(x: &str) -> bool { x.is_empty() }
/// // -> MethodSelfUsage::PureAssociated
///
/// // Instance method - uses self
/// fn get_data(&self) -> &Data { &self.data }
/// // -> MethodSelfUsage::InstanceMethod
///
/// // Unused self - has self but doesn't use it
/// fn debug(&self) { println!("debug"); }
/// // -> MethodSelfUsage::UnusedSelf
/// ```
pub fn classify_self_usage(method: &syn::ImplItemFn) -> MethodSelfUsage {
    // Check if method has self parameter
    let has_self_param = method
        .sig
        .inputs
        .iter()
        .any(|arg| matches!(arg, syn::FnArg::Receiver(_)));

    if !has_self_param {
        return MethodSelfUsage::PureAssociated;
    }

    // Check if self is actually used in the body
    let self_used = SelfUsageVisitor::check_body(&method.block);

    if self_used {
        MethodSelfUsage::InstanceMethod
    } else {
        MethodSelfUsage::UnusedSelf
    }
}

/// Classify self-usage from a standalone function (always PureAssociated).
///
/// Standalone functions (not in impl blocks) never have access to `self`,
/// so they are always classified as pure associated functions.
pub fn classify_self_usage_standalone(_func: &syn::ItemFn) -> MethodSelfUsage {
    MethodSelfUsage::PureAssociated
}

/// Visitor for detecting self-usage in method bodies.
///
/// This visitor traverses the AST of a method body to detect references to `self`:
/// - `self.field` (field access)
/// - `self.method()` (method calls)
/// - `&self` or `&mut self` (self references)
///
/// It does NOT count:
/// - `Self::method()` (associated function calls on Self type)
/// - Self in nested closures/functions (different binding context)
struct SelfUsageVisitor {
    uses_self: bool,
    /// Depth of nested closures/functions (self in nested context doesn't count)
    nested_depth: usize,
}

impl SelfUsageVisitor {
    /// Create a new visitor
    fn new() -> Self {
        Self {
            uses_self: false,
            nested_depth: 0,
        }
    }

    /// Check if a method body uses self
    pub fn check_body(block: &syn::Block) -> bool {
        let mut visitor = Self::new();
        visitor.visit_block(block);
        visitor.uses_self
    }

    /// Check if an expression is a reference to `self`
    fn is_self_expr(expr: &syn::Expr) -> bool {
        if let syn::Expr::Path(path) = expr {
            path.path.is_ident("self")
        } else {
            false
        }
    }
}

impl<'ast> Visit<'ast> for SelfUsageVisitor {
    fn visit_expr_field(&mut self, field: &'ast syn::ExprField) {
        // Check for self.field access
        if self.nested_depth == 0 && Self::is_self_expr(&field.base) {
            self.uses_self = true;
        }
        syn::visit::visit_expr_field(self, field);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        // Check for self.method() calls
        if self.nested_depth == 0 && Self::is_self_expr(&call.receiver) {
            self.uses_self = true;
        }
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_reference(&mut self, reference: &'ast syn::ExprReference) {
        // Check for &self or &mut self
        if self.nested_depth == 0 && Self::is_self_expr(&reference.expr) {
            self.uses_self = true;
        }
        syn::visit::visit_expr_reference(self, reference);
    }

    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        // Check for direct self usage (e.g., passing self as argument)
        if self.nested_depth == 0 && path.path.is_ident("self") {
            self.uses_self = true;
        }
        syn::visit::visit_expr_path(self, path);
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        // Don't count self usage in nested closures - they capture self
        // from the outer scope but represent different semantic context
        self.nested_depth += 1;
        syn::visit::visit_expr_closure(self, closure);
        self.nested_depth -= 1;
    }

    fn visit_item_fn(&mut self, _: &'ast syn::ItemFn) {
        // Don't traverse into nested functions
        // They have their own self context
    }
}

/// Calculate combined weight for a method (Spec 213).
///
/// Combines weights from:
/// - Spec 209: Accessor/boilerplate classification
/// - Spec 213: Self-usage classification (this spec)
///
/// Uses the minimum of both weights (a pure accessor should have very low weight).
///
/// # Arguments
///
/// * `accessor_class` - Accessor/boilerplate classification from Spec 209
/// * `self_usage` - Self-usage classification from Spec 213
///
/// # Returns
///
/// Combined weight in range [0.0, 1.0]
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{
///     MethodComplexityClass, MethodSelfUsage, calculate_combined_method_weight,
/// };
///
/// // Pure accessor: min(0.1, 0.2) = 0.1
/// let weight = calculate_combined_method_weight(
///     MethodComplexityClass::TrivialAccessor,
///     MethodSelfUsage::PureAssociated,
/// );
/// assert!((weight - 0.1).abs() < f64::EPSILON);
///
/// // Substantive instance method: min(1.0, 1.0) = 1.0
/// let weight = calculate_combined_method_weight(
///     MethodComplexityClass::Substantive,
///     MethodSelfUsage::InstanceMethod,
/// );
/// assert!((weight - 1.0).abs() < f64::EPSILON);
///
/// // Pure helper (substantive but no self): min(1.0, 0.2) = 0.2
/// let weight = calculate_combined_method_weight(
///     MethodComplexityClass::Substantive,
///     MethodSelfUsage::PureAssociated,
/// );
/// assert!((weight - 0.2).abs() < f64::EPSILON);
/// ```
pub fn calculate_combined_method_weight(
    accessor_class: MethodComplexityClass,
    self_usage: MethodSelfUsage,
) -> f64 {
    // Use minimum of both weights
    accessor_class.weight().min(self_usage.weight())
}

/// Calculate weighted method count combining Spec 209 and Spec 213.
///
/// For each method, the combined weight is the minimum of:
/// - Accessor/boilerplate weight (Spec 209)
/// - Self-usage weight (Spec 213)
///
/// # Arguments
///
/// * `classifications` - Iterator of (accessor_class, self_usage) pairs
///
/// # Returns
///
/// Sum of combined weights for all methods.
pub fn calculate_combined_weighted_count<'a>(
    classifications: impl Iterator<Item = (&'a MethodComplexityClass, &'a MethodSelfUsage)>,
) -> f64 {
    classifications
        .map(|(accessor, self_usage)| calculate_combined_method_weight(*accessor, *self_usage))
        .sum()
}

/// Determine confidence level from score and metrics.
///
/// Maps threshold violations to confidence levels:
/// - 5 violations → Definite
/// - 3-4 violations → Probable
/// - 1-2 violations → Possible
/// - 0 violations → NotGodObject
///
/// # Arguments
///
/// * `method_count` - Number of methods in the type
/// * `field_count` - Number of fields in the type
/// * `responsibility_count` - Number of distinct responsibilities
/// * `lines_of_code` - Total lines of code
/// * `complexity_sum` - Sum of cyclomatic complexity
/// * `thresholds` - Threshold configuration
///
/// # Returns
///
/// Confidence level based on number of threshold violations
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::{GodObjectThresholds, GodObjectConfidence};
/// use debtmap::organization::god_object::classifier::determine_confidence;
///
/// let thresholds = GodObjectThresholds::default();
/// let confidence = determine_confidence(30, 20, 8, 1500, 300, &thresholds);
/// assert_eq!(confidence, GodObjectConfidence::Definite);
/// ```
pub fn determine_confidence(
    method_count: usize,
    field_count: usize,
    responsibility_count: usize,
    lines_of_code: usize,
    complexity_sum: u32,
    thresholds: &GodObjectThresholds,
) -> GodObjectConfidence {
    let mut violations = 0;

    if method_count > thresholds.max_methods {
        violations += 1;
    }
    if field_count > thresholds.max_fields {
        violations += 1;
    }
    if responsibility_count > thresholds.max_traits {
        violations += 1;
    }
    if lines_of_code > thresholds.max_lines {
        violations += 1;
    }
    if complexity_sum > thresholds.max_complexity {
        violations += 1;
    }

    match violations {
        5 => GodObjectConfidence::Definite,
        3..=4 => GodObjectConfidence::Probable,
        1..=2 => GodObjectConfidence::Possible,
        _ => GodObjectConfidence::NotGodObject,
    }
}

/// Infer method responsibility domain from name and optional body.
///
/// This is a pure classification function that analyzes method names to determine
/// their likely responsibility category. Returns `None` category if confidence
/// is below the minimum threshold.
///
/// # Arguments
///
/// * `method_name` - The name of the method to classify
/// * `method_body` - Optional method body for deeper analysis (currently unused)
///
/// # Returns
///
/// A `ClassificationResult` with:
/// - `category`: `Some(name)` if confidence ≥ threshold, `None` otherwise
/// - `confidence`: Score from 0.0 to 1.0
/// - `signals_used`: Which signals contributed to the classification
///
/// # Confidence Thresholds
///
/// - Recognized categories: 0.85 (high confidence)
/// - Domain fallback: 0.45 (low confidence, rejected by MINIMUM_CONFIDENCE of 0.50)
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::infer_responsibility_with_confidence;
///
/// // High confidence classification
/// let result = infer_responsibility_with_confidence("parse_json", None);
/// assert!(result.category.is_some());
/// assert!(result.confidence >= 0.50);
///
/// // Low confidence - refused classification
/// let result = infer_responsibility_with_confidence("helper", None);
/// // May return None if confidence too low
/// ```
///
/// # Implementation
///
/// Currently uses name-based heuristics as the primary signal.
/// Future enhancements will integrate:
/// - I/O detection (weight: 0.40)
/// - Call graph analysis (weight: 0.30)
/// - Type signatures (weight: 0.15)
/// - Purity analysis (weight: 0.10)
pub fn infer_responsibility_with_confidence(
    method_name: &str,
    _method_body: Option<&str>,
) -> ClassificationResult {
    use crate::organization::BehavioralCategorizer;

    let category = BehavioralCategorizer::categorize_method(method_name);
    let category_name = category.display_name();

    // Assign confidence based on category type
    let confidence = match category {
        crate::organization::BehaviorCategory::Domain(_) => 0.45, // Below threshold
        crate::organization::BehaviorCategory::Utilities => 0.75, // Good confidence for utilities
        _ => 0.85, // High confidence for recognized patterns
    };

    // Apply confidence thresholds
    if confidence < MINIMUM_CONFIDENCE {
        log::debug!(
            "Low confidence classification for method '{}': confidence {:.2} below minimum {:.2}",
            method_name,
            confidence,
            MINIMUM_CONFIDENCE
        );
        return ClassificationResult {
            category: None,
            confidence,
            signals_used: vec![SignalType::NameHeuristic],
        };
    }

    ClassificationResult {
        category: Some(category_name),
        confidence,
        signals_used: vec![SignalType::NameHeuristic],
    }
}

/// Group methods by inferred responsibility domain.
///
/// This is a pure transformation that categorizes methods based on their names.
/// Methods with low confidence classification are grouped as "unclassified".
///
/// # Arguments
///
/// * `methods` - List of method names to classify
///
/// # Returns
///
/// HashMap mapping responsibility categories to lists of methods
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::group_methods_by_responsibility;
///
/// let methods = vec![
///     "parse_json".to_string(),
///     "format_output".to_string(),
///     "validate_input".to_string(),
/// ];
/// let groups = group_methods_by_responsibility(&methods);
/// assert!(groups.contains_key("Parsing"));
/// ```
pub fn group_methods_by_responsibility(methods: &[String]) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        let result = infer_responsibility_with_confidence(method, None);

        // If confidence is too low (None category), keep method in original location
        // by assigning it to "unclassified" group
        let responsibility = result
            .category
            .unwrap_or_else(|| "unclassified".to_string());

        groups
            .entry(responsibility)
            .or_default()
            .push(method.clone());
    }

    groups
}

/// Pure function: analyzes function name and returns primary responsibility category.
///
/// Reuses existing behavioral categorization infrastructure to provide uniform
/// responsibility analysis across all debt items (not just god objects).
///
/// # Arguments
///
/// * `function_name` - Name of the function to analyze
///
/// # Returns
///
/// * `Some(category)` - If behavioral category can be inferred with high confidence (>= 0.7)
/// * `None` - If function name doesn't clearly indicate a behavioral pattern
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::analyze_function_responsibility;
///
/// assert_eq!(analyze_function_responsibility("validate_email"), Some("Validation".to_string()));
/// assert_eq!(analyze_function_responsibility("parse_json"), Some("Parsing".to_string()));
/// assert_eq!(analyze_function_responsibility("get_user"), Some("Data Access".to_string()));
/// assert_eq!(analyze_function_responsibility("do_stuff"), None); // Low confidence
/// ```
///
/// # Stillwater Principle: Pure Core
///
/// This function is pure - same input always gives same output, no side effects.
/// Responsibility inference happens once during analysis, not during rendering.
pub fn analyze_function_responsibility(function_name: &str) -> Option<String> {
    // Reuse existing inference with confidence threshold
    let result = infer_responsibility_with_confidence(function_name, None);

    // Only return category if confidence meets threshold (>= 0.7)
    // Threshold of 0.7 matches existing god object analysis patterns
    // The infer_responsibility_with_confidence function uses MINIMUM_CONFIDENCE (0.50)
    // but we want higher confidence for universal responsibility analysis
    if result.confidence >= 0.7 {
        result.category
    } else {
        None
    }
}

/// Classify struct into a domain based on naming patterns.
///
/// Pure function that extracts semantic domain from struct names:
/// - `*Weight`, `*Multiplier`, `*Factor`, `*Scoring` → "scoring"
/// - `*Threshold`, `*Limit`, `*Bound` → "thresholds"
/// - `*Detection`, `*Detector`, `*Checker` → "detection"
/// - `*Config`, `*Settings`, `*Options` → "core_config"
/// - `*Data`, `*Info`, `*Metrics` → "data"
/// - Default: Extract first meaningful word from name
///
/// # Arguments
///
/// * `struct_name` - Name of the struct to classify
///
/// # Returns
///
/// Domain name as a string
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::classify_struct_domain;
///
/// assert_eq!(classify_struct_domain("ThresholdConfig"), "thresholds");
/// assert_eq!(classify_struct_domain("ScoringWeight"), "scoring");
/// assert_eq!(classify_struct_domain("DetectorSettings"), "detection");
/// ```
pub fn classify_struct_domain(struct_name: &str) -> String {
    let lower = struct_name.to_lowercase();

    if lower.contains("weight")
        || lower.contains("multiplier")
        || lower.contains("factor")
        || lower.contains("scoring")
    {
        "scoring".to_string()
    } else if lower.contains("threshold") || lower.contains("limit") || lower.contains("bound") {
        "thresholds".to_string()
    } else if lower.contains("detection") || lower.contains("detector") || lower.contains("checker")
    {
        "detection".to_string()
    } else if lower.contains("config") || lower.contains("settings") || lower.contains("options") {
        "core_config".to_string()
    } else if lower.contains("data") || lower.contains("info") || lower.contains("metrics") {
        "data".to_string()
    } else {
        // Extract first meaningful word from struct name as domain
        extract_domain_from_name(struct_name)
    }
}

/// Extract domain name from struct/type name by taking first meaningful word.
///
/// Handles both camelCase/PascalCase and snake_case naming conventions.
///
/// # Arguments
///
/// * `name` - Struct or type name
///
/// # Returns
///
/// Extracted domain name
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::extract_domain_from_name;
///
/// assert_eq!(extract_domain_from_name("UserProfile"), "User");
/// assert_eq!(extract_domain_from_name("user_data"), "user");
/// ```
pub fn extract_domain_from_name(name: &str) -> String {
    // First try snake_case extraction
    if name.contains('_') {
        return name
            .split('_')
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Core".to_string());
    }

    // Handle camelCase and PascalCase
    let mut domain = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            break;
        }
        domain.push(c);
    }

    if !domain.is_empty() {
        domain
    } else {
        "Core".to_string()
    }
}

/// Count distinct semantic domains in struct list.
///
/// Pure aggregation function that counts unique domain classifications.
///
/// # Arguments
///
/// * `structs` - List of struct metrics
///
/// # Returns
///
/// Number of distinct domains
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::count_distinct_domains;
/// use debtmap::organization::god_object::StructMetrics;
///
/// let structs = vec![
///     StructMetrics {
///         name: "ThresholdConfig".to_string(),
///         line_span: (0, 10),
///         method_count: 2,
///         field_count: 5,
///         responsibilities: vec!["configuration".to_string()],
///     },
///     StructMetrics {
///         name: "ScoringWeight".to_string(),
///         line_span: (11, 20),
///         method_count: 3,
///         field_count: 4,
///         responsibilities: vec!["calculation".to_string()],
///     },
/// ];
/// assert_eq!(count_distinct_domains(&structs), 2);
/// ```
pub fn count_distinct_domains(structs: &[super::types::StructMetrics]) -> usize {
    use std::collections::HashSet;
    let domains: HashSet<String> = structs
        .iter()
        .map(|s| classify_struct_domain(&s.name))
        .collect();
    domains.len()
}

/// Calculate struct-to-function ratio.
///
/// Pure computation that measures how struct-heavy a module is.
/// Returns 0.0 if total_functions is 0 to avoid division by zero.
///
/// # Arguments
///
/// * `struct_count` - Number of structs in the module
/// * `total_functions` - Total number of functions
///
/// # Returns
///
/// Ratio of structs to functions (0.0 if no functions)
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::calculate_struct_ratio;
///
/// assert_eq!(calculate_struct_ratio(8, 10), 0.8);
/// assert_eq!(calculate_struct_ratio(10, 0), 0.0);
/// ```
pub fn calculate_struct_ratio(struct_count: usize, total_functions: usize) -> f64 {
    if total_functions == 0 {
        return 0.0;
    }
    (struct_count as f64) / (total_functions as f64)
}

// ============================================================================
// Spec 208: Domain-Aware Responsibility Grouping
// ============================================================================

/// Domain context extracted from struct name, field names, and field types.
///
/// Used for domain-aware responsibility grouping (Spec 208).
/// Methods are grouped by their alignment to the struct's domain rather than
/// just behavioral prefixes like `get_*`, `parse_*`, etc.
///
/// # Examples
///
/// For `CrossModuleTracker`:
/// - `primary_keywords`: ["cross", "module", "tracker"]
/// - `secondary_keywords`: keywords from field names/types
/// - `domain_suffix`: Some("tracker")
#[derive(Debug, Clone)]
pub struct DomainContext {
    /// Primary domain keywords from struct name (e.g., "module", "tracker")
    pub primary_keywords: Vec<String>,
    /// Secondary keywords from field names and types
    pub secondary_keywords: Vec<String>,
    /// Domain suffix detected (Tracker, Manager, Builder, etc.)
    pub domain_suffix: Option<String>,
}

impl DomainContext {
    /// Create an empty domain context
    pub fn empty() -> Self {
        Self {
            primary_keywords: Vec::new(),
            secondary_keywords: Vec::new(),
            domain_suffix: None,
        }
    }

    /// Check if context has meaningful domain information
    pub fn has_domain(&self) -> bool {
        !self.primary_keywords.is_empty()
    }

    /// Get the primary domain name for grouping
    ///
    /// Returns a concatenated string of primary keywords, or "Primary" if empty.
    pub fn primary_domain_name(&self) -> String {
        if self.primary_keywords.is_empty() {
            "Primary".to_string()
        } else {
            // Capitalize first letter of each keyword and join
            self.primary_keywords
                .iter()
                .map(|k| {
                    let mut chars = k.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join("")
        }
    }
}

/// Extract domain context from struct name, field names, and field types.
///
/// Spec 208: This builds the context needed for domain-aware responsibility grouping.
///
/// # Arguments
///
/// * `struct_name` - The name of the struct
/// * `field_names` - Names of the struct's fields
/// * `field_types` - Type names of the struct's fields
///
/// # Returns
///
/// A `DomainContext` with extracted domain information
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::extract_domain_context;
///
/// let context = extract_domain_context(
///     "ModuleTracker",
///     &["modules".to_string(), "boundaries".to_string()],
///     &["HashMap".to_string(), "Vec".to_string()],
/// );
/// assert!(context.primary_keywords.contains(&"module".to_string()));
/// assert_eq!(context.domain_suffix, Some("tracker".to_string()));
/// ```
pub fn extract_domain_context(
    struct_name: &str,
    field_names: &[String],
    field_types: &[String],
) -> DomainContext {
    // Extract keywords from struct name
    let primary_keywords = extract_domain_keywords(struct_name);

    // Extract keywords from field names and types (excluding common types)
    let common_types: std::collections::HashSet<&str> = [
        "vec", "hashmap", "btreemap", "hashset", "btreeset", "option", "result", "string", "str",
        "bool", "usize", "isize", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "f32",
        "f64", "box", "arc", "rc", "refcell", "cell", "mutex", "rwlock", "pathbuf", "path",
    ]
    .into_iter()
    .collect();

    let secondary_keywords: Vec<String> = field_names
        .iter()
        .chain(field_types.iter())
        .flat_map(|name| extract_domain_keywords(name))
        .filter(|kw| !common_types.contains(kw.as_str()))
        .collect();

    // Detect domain suffix
    let domain_suffix = detect_domain_suffix(struct_name);

    DomainContext {
        primary_keywords,
        secondary_keywords,
        domain_suffix,
    }
}

/// Detect domain suffix from struct name.
///
/// Common domain suffixes indicate a cohesive struct pattern.
///
/// # Arguments
///
/// * `struct_name` - The name of the struct
///
/// # Returns
///
/// The detected suffix (lowercase), or None if no common suffix found
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::detect_domain_suffix;
///
/// assert_eq!(detect_domain_suffix("ModuleTracker"), Some("tracker".to_string()));
/// assert_eq!(detect_domain_suffix("RequestHandler"), Some("handler".to_string()));
/// assert_eq!(detect_domain_suffix("SomeStruct"), None);
/// ```
pub fn detect_domain_suffix(struct_name: &str) -> Option<String> {
    let cohesive_suffixes = [
        "tracker",
        "analyzer",
        "builder",
        "visitor",
        "handler",
        "processor",
        "calculator",
        "resolver",
        "extractor",
        "detector",
        "validator",
        "formatter",
        "parser",
        "renderer",
        "serializer",
        "deserializer",
        "iterator",
        "generator",
        "factory",
        "provider",
        "repository",
        "service",
        "client",
        "server",
        "cache",
        "pool",
        "queue",
        "stack",
        "manager",
        "controller",
        "adapter",
        "wrapper",
        "proxy",
        "decorator",
        "observer",
        "listener",
        "emitter",
        "dispatcher",
        "scheduler",
        "executor",
        "runner",
        "loader",
        "writer",
        "reader",
        "mapper",
        "converter",
        "transformer",
    ];

    let name_lower = struct_name.to_lowercase();

    for suffix in &cohesive_suffixes {
        if name_lower.ends_with(suffix) {
            return Some(suffix.to_string());
        }
    }

    None
}

/// Group methods by domain alignment rather than behavioral prefix.
///
/// Spec 208: Domain-aware grouping considers:
/// 1. Primary domain keywords from struct name
/// 2. Secondary keywords from field names/types
/// 3. Falls back to behavioral classification for unrelated methods
///
/// This reduces false positives where cohesive structs like `ModuleTracker`
/// would otherwise be flagged as god objects due to diverse method prefixes.
///
/// # Arguments
///
/// * `methods` - List of method names to group
/// * `context` - Domain context extracted from the struct
///
/// # Returns
///
/// HashMap mapping domain/responsibility names to lists of methods
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::{DomainContext, group_methods_by_domain};
///
/// let context = DomainContext {
///     primary_keywords: vec!["module".to_string()],
///     secondary_keywords: vec!["boundary".to_string()],
///     domain_suffix: Some("tracker".to_string()),
/// };
///
/// let methods = vec![
///     "get_modules".to_string(),
///     "track_module".to_string(),
///     "resolve_boundary".to_string(),
///     "new".to_string(),
/// ];
///
/// let groups = group_methods_by_domain(&methods, &context);
/// // Methods matching "module" domain should be grouped together
/// assert!(groups.len() <= 2, "Cohesive methods should group together");
/// ```
pub fn group_methods_by_domain(
    methods: &[String],
    context: &DomainContext,
) -> HashMap<String, Vec<String>> {
    // If no domain context, fall back to responsibility-based grouping
    if !context.has_domain() {
        return group_methods_by_responsibility(methods);
    }

    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for method in methods {
        let domain = infer_method_domain(method, context);
        groups.entry(domain).or_default().push(method.clone());
    }

    groups
}

/// Infer which domain a method belongs to based on context.
///
/// Spec 208: Checks method alignment with:
/// 1. Primary domain keywords (struct name)
/// 2. Secondary keywords (field names/types)
/// 3. Falls back to behavioral classification
///
/// # Arguments
///
/// * `method` - The method name to classify
/// * `context` - Domain context from the struct
///
/// # Returns
///
/// Domain name for the method
fn infer_method_domain(method: &str, context: &DomainContext) -> String {
    let method_lower = method.to_lowercase();
    let method_keywords = extract_domain_keywords(method);

    // Common utility methods that don't count toward domain classification
    let utility_methods: std::collections::HashSet<&str> = [
        "new",
        "default",
        "clone",
        "fmt",
        "drop",
        "from",
        "into",
        "as_ref",
        "as_mut",
        "len",
        "is_empty",
        "iter",
        "iter_mut",
        "clear",
        "with_capacity",
    ]
    .into_iter()
    .collect();

    // Utility methods go to primary domain by default
    if utility_methods.contains(method_lower.as_str()) {
        return context.primary_domain_name();
    }

    // Check if method aligns with primary domain keywords
    let matches_primary = context.primary_keywords.iter().any(|pk| {
        method_lower.contains(pk.as_str())
            || method_keywords.iter().any(|mk| mk == pk || mk.contains(pk))
    });

    if matches_primary {
        return context.primary_domain_name();
    }

    // Check secondary domain alignment (field names/types)
    let matching_secondary: Vec<&String> = context
        .secondary_keywords
        .iter()
        .filter(|sk| {
            method_lower.contains(sk.as_str())
                || method_keywords
                    .iter()
                    .any(|mk| mk == *sk || mk.contains(*sk))
        })
        .collect();

    if !matching_secondary.is_empty() {
        // Group under primary domain if secondary keyword matches
        // This keeps cohesive structs together
        return context.primary_domain_name();
    }

    // Check for domain suffix alignment (e.g., "tracker" suffix + "track_*" methods)
    if let Some(ref suffix) = context.domain_suffix {
        // Check if method contains the suffix root (e.g., "track" from "tracker")
        let suffix_root = suffix.trim_end_matches("er").trim_end_matches("or");
        if method_lower.contains(suffix_root) {
            return context.primary_domain_name();
        }
    }

    // Fall back to behavioral classification for truly unrelated methods
    let result = infer_responsibility_with_confidence(method, None);
    result
        .category
        .unwrap_or_else(|| "unclassified".to_string())
}

/// Extract domain keywords from a struct name.
///
/// Splits camelCase/PascalCase and snake_case names into individual words.
/// Filters out common non-domain words like "new", "get", "set".
///
/// # Arguments
///
/// * `struct_name` - The name of the struct to analyze
///
/// # Returns
///
/// Vector of lowercase domain keywords
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::extract_domain_keywords;
///
/// let keywords = extract_domain_keywords("CrossModuleTracker");
/// assert!(keywords.contains(&"cross".to_string()));
/// assert!(keywords.contains(&"module".to_string()));
/// assert!(keywords.contains(&"tracker".to_string()));
/// ```
pub fn extract_domain_keywords(name: &str) -> Vec<String> {
    let mut keywords = Vec::new();

    // Handle snake_case
    if name.contains('_') {
        for part in name.split('_') {
            if !part.is_empty() && part.len() > 1 {
                keywords.push(part.to_lowercase());
            }
        }
    } else {
        // Handle PascalCase/camelCase
        let mut current_word = String::new();
        for c in name.chars() {
            if c.is_uppercase() && !current_word.is_empty() {
                if current_word.len() > 1 {
                    keywords.push(current_word.to_lowercase());
                }
                current_word = c.to_string();
            } else {
                current_word.push(c);
            }
        }
        if current_word.len() > 1 {
            keywords.push(current_word.to_lowercase());
        }
    }

    // Filter out common non-domain words
    let non_domain_words: std::collections::HashSet<&str> = [
        "new", "get", "set", "is", "has", "the", "a", "an", "impl", "default",
    ]
    .into_iter()
    .collect();

    keywords
        .into_iter()
        .filter(|w| !non_domain_words.contains(w.as_str()))
        .collect()
}

/// Calculate domain cohesion score for a struct based on method names.
///
/// Measures how many methods contain domain keywords from the struct name.
/// High cohesion indicates methods are related to the struct's core purpose.
///
/// # Arguments
///
/// * `struct_name` - The name of the struct
/// * `methods` - List of method names in the struct
///
/// # Returns
///
/// Cohesion score between 0.0 (no cohesion) and 1.0 (perfect cohesion)
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::calculate_domain_cohesion;
///
/// let cohesion = calculate_domain_cohesion(
///     "ModuleTracker",
///     &["get_module".to_string(), "track_module".to_string(), "new".to_string()]
/// );
/// assert!(cohesion > 0.5);
/// ```
pub fn calculate_domain_cohesion(struct_name: &str, methods: &[String]) -> f64 {
    if methods.is_empty() {
        return 1.0; // Empty struct is trivially cohesive
    }

    let domain_keywords = extract_domain_keywords(struct_name);
    if domain_keywords.is_empty() {
        return 0.5; // Can't determine cohesion without domain keywords
    }

    // Common utility methods that don't need to match domain
    let utility_methods: std::collections::HashSet<&str> = [
        "new", "default", "clone", "fmt", "drop", "from", "into", "as_ref", "as_mut",
    ]
    .into_iter()
    .collect();

    let mut domain_aligned = 0;
    let mut non_utility_count = 0;

    for method in methods {
        let method_lower = method.to_lowercase();

        // Skip utility methods in cohesion calculation
        if utility_methods.contains(method_lower.as_str()) {
            continue;
        }

        non_utility_count += 1;

        // Check if method name contains any domain keyword
        let method_keywords = extract_domain_keywords(method);
        let has_domain_keyword = domain_keywords
            .iter()
            .any(|dk| method_lower.contains(dk) || method_keywords.contains(dk));

        if has_domain_keyword {
            domain_aligned += 1;
        }
    }

    if non_utility_count == 0 {
        return 1.0; // All utility methods is cohesive
    }

    domain_aligned as f64 / non_utility_count as f64
}

/// Determine if a struct is cohesive based on domain keyword analysis.
///
/// A struct is considered cohesive if:
/// 1. More than 50% of non-utility methods contain domain keywords, OR
/// 2. The struct has a clear domain suffix (Tracker, Manager, Builder, etc.)
///    and methods align with that pattern
///
/// # Arguments
///
/// * `struct_name` - The name of the struct
/// * `methods` - List of method names in the struct
///
/// # Returns
///
/// `true` if the struct appears to be cohesive, `false` otherwise
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::classifier::is_cohesive_struct;
///
/// // Cohesive: methods align with "Module" domain
/// assert!(is_cohesive_struct(
///     "ModuleTracker",
///     &["get_module".to_string(), "track_module".to_string()]
/// ));
///
/// // Not cohesive: methods don't align with struct name
/// assert!(!is_cohesive_struct(
///     "Manager",
///     &["parse_json".to_string(), "render_html".to_string(), "send_email".to_string()]
/// ));
/// ```
pub fn is_cohesive_struct(struct_name: &str, methods: &[String]) -> bool {
    // Threshold for cohesion - structs with >50% domain-aligned methods are cohesive
    const COHESION_THRESHOLD: f64 = 0.5;

    let cohesion = calculate_domain_cohesion(struct_name, methods);

    // Also check for known cohesive patterns in struct name
    let name_lower = struct_name.to_lowercase();
    let cohesive_suffixes = [
        "tracker",
        "analyzer",
        "builder",
        "visitor",
        "handler",
        "processor",
        "calculator",
        "resolver",
        "extractor",
        "detector",
        "validator",
        "formatter",
        "parser",
        "renderer",
        "serializer",
        "deserializer",
        "iterator",
        "generator",
        "factory",
        "provider",
        "repository",
        "service",
        "client",
        "server",
        "cache",
        "pool",
        "queue",
        "stack",
    ];

    let has_cohesive_suffix = cohesive_suffixes
        .iter()
        .any(|suffix| name_lower.ends_with(suffix));

    // Cohesive if: high domain alignment OR (cohesive suffix AND moderate alignment)
    cohesion > COHESION_THRESHOLD || (has_cohesive_suffix && cohesion > 0.3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_mapping_definite() {
        let thresholds = GodObjectThresholds::default();
        // All 5 thresholds violated
        let confidence = determine_confidence(30, 20, 8, 1500, 300, &thresholds);
        assert_eq!(confidence, GodObjectConfidence::Definite);
    }

    #[test]
    fn test_confidence_mapping_probable() {
        let thresholds = GodObjectThresholds::default();
        // 3 thresholds violated (methods, fields, responsibilities)
        let confidence = determine_confidence(30, 20, 8, 500, 100, &thresholds);
        assert_eq!(confidence, GodObjectConfidence::Probable);
    }

    #[test]
    fn test_confidence_mapping_possible() {
        let thresholds = GodObjectThresholds::default();
        // 2 thresholds violated (methods, fields)
        let confidence = determine_confidence(30, 20, 3, 500, 100, &thresholds);
        assert_eq!(confidence, GodObjectConfidence::Possible);
    }

    #[test]
    fn test_confidence_mapping_not_god_object() {
        let thresholds = GodObjectThresholds::default();
        // No thresholds violated
        let confidence = determine_confidence(10, 8, 3, 500, 100, &thresholds);
        assert_eq!(confidence, GodObjectConfidence::NotGodObject);
    }

    #[test]
    fn test_group_methods_by_responsibility_basic() {
        let methods = vec![
            "parse_json".to_string(),
            "format_output".to_string(),
            "validate_input".to_string(),
        ];
        let groups = group_methods_by_responsibility(&methods);
        assert!(groups.contains_key("Parsing"));
        assert!(groups.contains_key("Rendering"));
        assert!(groups.contains_key("Validation"));
    }

    #[test]
    fn test_group_methods_by_responsibility_unclassified() {
        let methods = vec!["foo".to_string()]; // Single-word method with no pattern
        let groups = group_methods_by_responsibility(&methods);
        // Low confidence methods go to "unclassified"
        assert!(groups.contains_key("unclassified"));
    }

    #[test]
    fn test_classify_struct_domain_scoring() {
        assert_eq!(classify_struct_domain("ScoringWeight"), "scoring");
        assert_eq!(classify_struct_domain("MultiplicandFactor"), "scoring");
    }

    #[test]
    fn test_classify_struct_domain_thresholds() {
        assert_eq!(classify_struct_domain("ThresholdConfig"), "thresholds");
        assert_eq!(classify_struct_domain("LimitSettings"), "thresholds");
    }

    #[test]
    fn test_classify_struct_domain_detection() {
        assert_eq!(classify_struct_domain("DetectorModule"), "detection");
        assert_eq!(classify_struct_domain("CheckerSystem"), "detection");
    }

    #[test]
    fn test_classify_struct_domain_config() {
        assert_eq!(classify_struct_domain("ConfigOptions"), "core_config");
        assert_eq!(classify_struct_domain("SystemSettings"), "core_config");
    }

    #[test]
    fn test_classify_struct_domain_data() {
        assert_eq!(classify_struct_domain("DataStructure"), "data");
        assert_eq!(classify_struct_domain("MetricsInfo"), "data");
    }

    #[test]
    fn test_classify_struct_domain_fallback() {
        // Should extract first word
        assert_eq!(classify_struct_domain("UserProfile"), "User");
        assert_eq!(classify_struct_domain("OrderProcessor"), "Order");
    }

    #[test]
    fn test_extract_domain_from_name_camel_case() {
        assert_eq!(extract_domain_from_name("UserProfile"), "User");
        assert_eq!(extract_domain_from_name("OrderManager"), "Order");
    }

    #[test]
    fn test_extract_domain_from_name_snake_case() {
        assert_eq!(extract_domain_from_name("user_profile"), "user");
        assert_eq!(extract_domain_from_name("order_data"), "order");
    }

    #[test]
    fn test_extract_domain_from_name_empty() {
        assert_eq!(extract_domain_from_name(""), "Core");
    }

    #[test]
    fn test_count_distinct_domains() {
        use super::super::types::StructMetrics;
        let structs = vec![
            StructMetrics {
                name: "ThresholdConfig".to_string(),
                line_span: (0, 10),
                method_count: 2,
                field_count: 5,
                responsibilities: vec!["configuration".to_string()],
            },
            StructMetrics {
                name: "ThresholdValidator".to_string(),
                line_span: (11, 20),
                method_count: 3,
                field_count: 4,
                responsibilities: vec!["validation".to_string()],
            },
            StructMetrics {
                name: "ScoringWeight".to_string(),
                line_span: (21, 30),
                method_count: 4,
                field_count: 3,
                responsibilities: vec!["calculation".to_string()],
            },
        ];
        // Should identify 2 domains: "thresholds" and "scoring"
        assert_eq!(count_distinct_domains(&structs), 2);
    }

    #[test]
    fn test_calculate_struct_ratio_normal() {
        assert_eq!(calculate_struct_ratio(10, 20), 0.5);
        assert_eq!(calculate_struct_ratio(15, 10), 1.5);
    }

    #[test]
    fn test_calculate_struct_ratio_zero_functions() {
        assert_eq!(calculate_struct_ratio(10, 0), 0.0);
    }

    #[test]
    fn test_calculate_struct_ratio_zero_structs() {
        assert_eq!(calculate_struct_ratio(0, 10), 0.0);
    }

    // Property-based tests using proptest
    use proptest::prelude::*;

    proptest! {
        /// Verify classification is idempotent - same input always produces same output
        #[test]
        fn prop_classification_idempotent(method_name in "[a-z_]{1,20}") {
            let r1 = infer_responsibility_with_confidence(&method_name, None);
            let r2 = infer_responsibility_with_confidence(&method_name, None);
            prop_assert_eq!(r1.category, r2.category);
            prop_assert_eq!(r1.confidence, r2.confidence);
            prop_assert_eq!(r1.signals_used, r2.signals_used);
        }

        /// Verify struct domain classification is idempotent
        #[test]
        fn prop_struct_domain_classification_idempotent(struct_name in "[A-Z][a-zA-Z0-9_]{1,30}") {
            let d1 = classify_struct_domain(&struct_name);
            let d2 = classify_struct_domain(&struct_name);
            prop_assert_eq!(d1, d2);
        }

        /// Verify confidence calculation is idempotent
        #[test]
        fn prop_confidence_calculation_idempotent(
            method_count in 0usize..100,
            field_count in 0usize..50,
            responsibility_count in 0usize..20,
            lines_of_code in 0usize..5000,
            complexity_sum in 0u32..1000
        ) {
            let thresholds = GodObjectThresholds::default();
            let c1 = determine_confidence(method_count, field_count, responsibility_count, lines_of_code, complexity_sum, &thresholds);
            let c2 = determine_confidence(method_count, field_count, responsibility_count, lines_of_code, complexity_sum, &thresholds);
            prop_assert_eq!(c1, c2);
        }

        /// Verify confidence levels map correctly based on violation count
        #[test]
        fn prop_confidence_violations_mapping(
            method_count in 0usize..100,
            field_count in 0usize..50,
            responsibility_count in 0usize..20,
            lines_of_code in 0usize..5000,
            complexity_sum in 0u32..1000
        ) {
            let thresholds = GodObjectThresholds::default();
            let confidence = determine_confidence(method_count, field_count, responsibility_count, lines_of_code, complexity_sum, &thresholds);

            // Count violations
            let mut violations = 0;
            if method_count > thresholds.max_methods { violations += 1; }
            if field_count > thresholds.max_fields { violations += 1; }
            if responsibility_count > thresholds.max_traits { violations += 1; }
            if lines_of_code > thresholds.max_lines { violations += 1; }
            if complexity_sum > thresholds.max_complexity { violations += 1; }

            // Verify mapping matches spec
            match violations {
                5 => prop_assert_eq!(confidence, GodObjectConfidence::Definite),
                3..=4 => prop_assert_eq!(confidence, GodObjectConfidence::Probable),
                1..=2 => prop_assert_eq!(confidence, GodObjectConfidence::Possible),
                _ => prop_assert_eq!(confidence, GodObjectConfidence::NotGodObject),
            }
        }

        /// Verify struct ratio calculation is always non-negative
        #[test]
        fn prop_struct_ratio_non_negative(struct_count in 0usize..100, total_functions in 0usize..200) {
            let ratio = calculate_struct_ratio(struct_count, total_functions);
            prop_assert!(ratio >= 0.0);
        }

        /// Verify struct ratio calculation handles zero functions gracefully
        #[test]
        fn prop_struct_ratio_zero_functions(struct_count in 0usize..100) {
            let ratio = calculate_struct_ratio(struct_count, 0);
            prop_assert_eq!(ratio, 0.0);
        }
    }

    // Spec 254: Universal Responsibility Analysis Tests
    #[test]
    fn test_analyze_function_responsibility_validation() {
        assert_eq!(
            analyze_function_responsibility("validate_email"),
            Some("Validation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("check_bounds"),
            Some("Validation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("verify_signature"),
            Some("Validation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("is_valid"),
            Some("Validation".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_parsing() {
        assert_eq!(
            analyze_function_responsibility("parse_json"),
            Some("Parsing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("read_config"),
            Some("Parsing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("extract_data"),
            Some("Parsing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("decode_message"),
            Some("Parsing".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_data_access() {
        assert_eq!(
            analyze_function_responsibility("get_user"),
            Some("Data Access".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("set_property"),
            Some("Data Access".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("fetch_record"),
            Some("Data Access".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("retrieve_data"),
            Some("Data Access".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_rendering() {
        assert_eq!(
            analyze_function_responsibility("render_view"),
            Some("Rendering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("draw_chart"),
            Some("Rendering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("paint_canvas"),
            Some("Rendering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("format_output"),
            Some("Rendering".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_construction() {
        assert_eq!(
            analyze_function_responsibility("create_instance"),
            Some("Construction".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("build_object"),
            Some("Construction".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("make_widget"),
            Some("Construction".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_filtering() {
        assert_eq!(
            analyze_function_responsibility("filter_results"),
            Some("Filtering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("select_items"),
            Some("Filtering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("find_matches"),
            Some("Filtering".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("search_database"),
            Some("Filtering".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_transformation() {
        assert_eq!(
            analyze_function_responsibility("transform_data"),
            Some("Transformation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("convert_to_json"),
            Some("Transformation".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("map_values"),
            Some("Transformation".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_communication() {
        assert_eq!(
            analyze_function_responsibility("send_message"),
            Some("Communication".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("receive_data"),
            Some("Communication".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("transmit_packet"),
            Some("Communication".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("notify_observers"),
            Some("Communication".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_persistence() {
        assert_eq!(
            analyze_function_responsibility("save_state"),
            Some("Persistence".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("load_config"),
            Some("Persistence".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_event_handling() {
        assert_eq!(
            analyze_function_responsibility("handle_keypress"),
            Some("Event Handling".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("on_mouse_down"),
            Some("Event Handling".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("dispatch_event"),
            Some("Event Handling".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_processing() {
        assert_eq!(
            analyze_function_responsibility("process_request"),
            Some("Processing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("execute_task"),
            Some("Processing".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("run_pipeline"),
            Some("Processing".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_low_confidence() {
        // Generic names should return None due to low confidence
        // Note: "process" and "handle" are recognized patterns (Processing/Event Handling)
        // so we test truly generic names
        assert_eq!(analyze_function_responsibility("do_something"), None);
        assert_eq!(analyze_function_responsibility("helper"), None);
        assert_eq!(analyze_function_responsibility("utils"), None);
        assert_eq!(analyze_function_responsibility("foo"), None);
        assert_eq!(analyze_function_responsibility("bar"), None);
    }

    #[test]
    fn test_analyze_function_responsibility_purity() {
        // Pure function: same input = same output
        let result1 = analyze_function_responsibility("validate_input");
        let result2 = analyze_function_responsibility("validate_input");
        assert_eq!(result1, result2);
        assert_eq!(result1, Some("Validation".to_string()));

        // Test multiple times to ensure determinism
        for _ in 0..10 {
            assert_eq!(
                analyze_function_responsibility("parse_json"),
                Some("Parsing".to_string())
            );
        }
    }

    #[test]
    fn test_analyze_function_responsibility_empty_string() {
        // Edge case: empty string
        assert_eq!(analyze_function_responsibility(""), None);
    }

    #[test]
    fn test_analyze_function_responsibility_lifecycle() {
        assert_eq!(
            analyze_function_responsibility("initialize_system"),
            Some("Lifecycle".to_string())
        );
        assert_eq!(
            analyze_function_responsibility("cleanup"),
            Some("Lifecycle".to_string())
        );
    }

    #[test]
    fn test_analyze_function_responsibility_state_management() {
        assert_eq!(
            analyze_function_responsibility("update_state"),
            Some("State Management".to_string())
        );
    }

    // Domain cohesion tests
    #[test]
    fn test_extract_domain_keywords_camel_case() {
        let keywords = extract_domain_keywords("CrossModuleTracker");
        assert!(keywords.contains(&"cross".to_string()));
        assert!(keywords.contains(&"module".to_string()));
        assert!(keywords.contains(&"tracker".to_string()));
    }

    #[test]
    fn test_extract_domain_keywords_snake_case() {
        let keywords = extract_domain_keywords("call_graph_builder");
        assert!(keywords.contains(&"call".to_string()));
        assert!(keywords.contains(&"graph".to_string()));
        assert!(keywords.contains(&"builder".to_string()));
    }

    #[test]
    fn test_calculate_domain_cohesion_high() {
        let struct_name = "ModuleTracker";
        let methods = vec![
            "get_module".to_string(),
            "track_module".to_string(),
            "resolve_module_call".to_string(),
            "is_module_valid".to_string(),
            "new".to_string(),
        ];
        let cohesion = calculate_domain_cohesion(struct_name, &methods);
        assert!(cohesion > 0.6, "Expected high cohesion, got {}", cohesion);
    }

    #[test]
    fn test_calculate_domain_cohesion_low() {
        let struct_name = "GodObject";
        let methods = vec![
            "parse_json".to_string(),
            "render_html".to_string(),
            "validate_email".to_string(),
            "send_notification".to_string(),
            "save_to_database".to_string(),
        ];
        let cohesion = calculate_domain_cohesion(struct_name, &methods);
        assert!(cohesion < 0.3, "Expected low cohesion, got {}", cohesion);
    }

    #[test]
    fn test_is_cohesive_struct_tracker() {
        let struct_name = "CrossModuleTracker";
        let methods = vec![
            "new".to_string(),
            "analyze_workspace".to_string(),
            "get_cross_module_calls".to_string(),
            "get_public_apis".to_string(),
            "is_public_api".to_string(),
            "resolve_module_call".to_string(),
            "get_statistics".to_string(),
            "infer_module_path".to_string(),
        ];
        assert!(
            is_cohesive_struct(struct_name, &methods),
            "CrossModuleTracker should be detected as cohesive"
        );
    }

    #[test]
    fn test_is_cohesive_struct_god_object() {
        let struct_name = "ApplicationManager";
        let methods = vec![
            "parse_config".to_string(),
            "render_ui".to_string(),
            "validate_input".to_string(),
            "send_email".to_string(),
            "save_data".to_string(),
            "load_plugin".to_string(),
            "handle_request".to_string(),
        ];
        assert!(
            !is_cohesive_struct(struct_name, &methods),
            "ApplicationManager with unrelated methods should NOT be cohesive"
        );
    }

    // =========================================================================
    // Spec 208: Domain-Aware Responsibility Grouping Tests
    // =========================================================================

    #[test]
    fn test_domain_context_extraction() {
        let context = extract_domain_context(
            "ModuleTracker",
            &["modules".into(), "boundaries".into()],
            &["HashMap".into(), "Vec".into()],
        );
        assert!(context.primary_keywords.contains(&"module".to_string()));
        assert!(context.primary_keywords.contains(&"tracker".to_string()));
        assert_eq!(context.domain_suffix, Some("tracker".to_string()));
        // Secondary keywords should include field names
        assert!(context.secondary_keywords.contains(&"modules".to_string()));
        assert!(context
            .secondary_keywords
            .contains(&"boundaries".to_string()));
    }

    #[test]
    fn test_domain_context_filters_common_types() {
        let context = extract_domain_context(
            "DataStore",
            &["items".into()],
            &["HashMap".into(), "String".into(), "Vec".into()],
        );
        // Common types like HashMap, String, Vec should be filtered out
        assert!(!context.secondary_keywords.contains(&"hashmap".to_string()));
        assert!(!context.secondary_keywords.contains(&"string".to_string()));
        assert!(!context.secondary_keywords.contains(&"vec".to_string()));
    }

    #[test]
    fn test_detect_domain_suffix() {
        assert_eq!(
            detect_domain_suffix("ModuleTracker"),
            Some("tracker".to_string())
        );
        assert_eq!(
            detect_domain_suffix("RequestHandler"),
            Some("handler".to_string())
        );
        assert_eq!(
            detect_domain_suffix("JsonParser"),
            Some("parser".to_string())
        );
        assert_eq!(
            detect_domain_suffix("UserRepository"),
            Some("repository".to_string())
        );
        assert_eq!(detect_domain_suffix("SomeStruct"), None);
        assert_eq!(detect_domain_suffix("Config"), None);
    }

    #[test]
    fn test_domain_context_primary_domain_name() {
        let context = DomainContext {
            primary_keywords: vec!["module".into(), "tracker".into()],
            secondary_keywords: vec![],
            domain_suffix: Some("tracker".into()),
        };
        assert_eq!(context.primary_domain_name(), "ModuleTracker");

        let empty_context = DomainContext::empty();
        assert_eq!(empty_context.primary_domain_name(), "Primary");
    }

    #[test]
    fn test_group_methods_by_domain_cohesive_struct() {
        let context = DomainContext {
            primary_keywords: vec!["module".into()],
            secondary_keywords: vec!["boundary".into()],
            domain_suffix: Some("tracker".into()),
        };

        let methods = vec![
            "get_modules".into(),
            "track_module".into(),
            "resolve_boundary".into(),
            "new".into(),
        ];

        let groups = group_methods_by_domain(&methods, &context);

        // Should have 1-2 groups, not 4 (all methods align with "module" domain)
        assert!(
            groups.len() <= 2,
            "Cohesive methods should group together, got {} groups: {:?}",
            groups.len(),
            groups.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_group_methods_by_domain_god_object() {
        let context = DomainContext {
            primary_keywords: vec!["application".into()],
            secondary_keywords: vec![],
            domain_suffix: Some("manager".into()),
        };

        let methods = vec![
            "parse_json".into(),
            "render_html".into(),
            "validate_email".into(),
            "send_notification".into(),
        ];

        let groups = group_methods_by_domain(&methods, &context);

        // Should have 4 groups (none match "application")
        assert!(
            groups.len() >= 4,
            "Unrelated methods should stay separate, got {} groups: {:?}",
            groups.len(),
            groups.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_group_methods_by_domain_utility_methods() {
        let context = DomainContext {
            primary_keywords: vec!["module".into()],
            secondary_keywords: vec![],
            domain_suffix: Some("tracker".into()),
        };

        let methods = vec!["new".into(), "default".into(), "clone".into(), "len".into()];

        let groups = group_methods_by_domain(&methods, &context);

        // Utility methods should all be grouped under primary domain
        assert_eq!(
            groups.len(),
            1,
            "Utility methods should all group together under primary domain"
        );
    }

    #[test]
    fn test_group_methods_by_domain_suffix_alignment() {
        let context = DomainContext {
            primary_keywords: vec!["module".into()],
            secondary_keywords: vec![],
            domain_suffix: Some("tracker".into()),
        };

        // Methods containing "track" should align with "tracker" suffix
        let methods = vec!["track_item".into(), "get_tracked".into(), "untrack".into()];

        let groups = group_methods_by_domain(&methods, &context);

        // All should be grouped under primary domain due to "track" suffix alignment
        assert!(
            groups.len() <= 2,
            "Methods aligning with suffix should group together"
        );
    }

    #[test]
    fn test_group_methods_by_domain_empty_context() {
        // Empty context should fall back to responsibility-based grouping
        let context = DomainContext::empty();

        let methods = vec![
            "parse_json".into(),
            "validate_input".into(),
            "get_data".into(),
        ];

        let groups = group_methods_by_domain(&methods, &context);

        // Should fall back to behavioral classification
        assert!(
            !groups.is_empty(),
            "Should produce groups even with empty context"
        );
    }

    #[test]
    fn test_cross_module_tracker_domain_grouping() {
        // Real-world test case from spec 208
        let context = extract_domain_context(
            "CrossModuleTracker",
            &["modules".into(), "calls".into(), "boundaries".into()],
            &["HashMap".into(), "Vec".into()],
        );

        let methods = vec![
            "get_module_calls".into(),
            "is_public_api".into(),
            "resolve_module_call".into(),
            "infer_module_path".into(),
            "new".into(),
        ];

        let groups = group_methods_by_domain(&methods, &context);

        // With domain-aware grouping, should produce 1-2 groups, not 4+
        assert!(
            groups.len() <= 2,
            "CrossModuleTracker methods should have ≤2 domain groups, got {}: {:?}",
            groups.len(),
            groups.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_domain_context_has_domain() {
        let context = DomainContext {
            primary_keywords: vec!["module".into()],
            secondary_keywords: vec![],
            domain_suffix: None,
        };
        assert!(context.has_domain());

        let empty = DomainContext::empty();
        assert!(!empty.has_domain());
    }

    proptest! {
        /// Verify domain context extraction is idempotent
        #[test]
        fn prop_domain_context_extraction_idempotent(
            struct_name in "[A-Z][a-zA-Z0-9]{1,20}",
            field_name in "[a-z_][a-z_0-9]{0,15}"
        ) {
            let fields = vec![field_name.clone()];
            let types = vec!["String".to_string()];
            let c1 = extract_domain_context(&struct_name, &fields, &types);
            let c2 = extract_domain_context(&struct_name, &fields, &types);
            prop_assert_eq!(c1.primary_keywords, c2.primary_keywords);
            prop_assert_eq!(c1.secondary_keywords, c2.secondary_keywords);
            prop_assert_eq!(c1.domain_suffix, c2.domain_suffix);
        }

        /// Verify domain-aware grouping is idempotent
        #[test]
        fn prop_group_methods_by_domain_idempotent(
            struct_name in "[A-Z][a-zA-Z0-9]{1,15}Tracker",
            method_name in "[a-z_][a-z_0-9]{0,15}"
        ) {
            let context = extract_domain_context(&struct_name, &[], &[]);
            let methods = vec![method_name.clone()];
            let g1 = group_methods_by_domain(&methods, &context);
            let g2 = group_methods_by_domain(&methods, &context);
            prop_assert_eq!(g1.len(), g2.len());
            for key in g1.keys() {
                prop_assert!(g2.contains_key(key));
            }
        }
    }

    // =========================================================================
    // Spec 209: Accessor and Boilerplate Method Detection Tests
    // =========================================================================

    #[test]
    fn test_classify_trivial_accessor_field_access() {
        let analysis = MethodBodyAnalysis {
            line_count: 1,
            has_control_flow: false,
            call_count: 0,
            return_expr_type: Some(ReturnExprType::FieldAccess),
            has_self_param: true,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("get_name", &analysis),
            MethodComplexityClass::TrivialAccessor
        );
    }

    #[test]
    fn test_classify_trivial_accessor_literal() {
        let analysis = MethodBodyAnalysis {
            line_count: 1,
            has_control_flow: false,
            call_count: 0,
            return_expr_type: Some(ReturnExprType::Literal),
            has_self_param: true,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("is_enabled", &analysis),
            MethodComplexityClass::TrivialAccessor
        );
    }

    #[test]
    fn test_classify_trivial_accessor_mut() {
        let analysis = MethodBodyAnalysis {
            line_count: 1,
            has_control_flow: false,
            call_count: 0,
            return_expr_type: Some(ReturnExprType::FieldAccess),
            has_self_param: true,
            is_mutating: true,
        };
        assert_eq!(
            classify_method_complexity("data_mut", &analysis),
            MethodComplexityClass::TrivialAccessor
        );
    }

    #[test]
    fn test_classify_simple_accessor() {
        let analysis = MethodBodyAnalysis {
            line_count: 2,
            has_control_flow: false,
            call_count: 1, // .clone() call
            return_expr_type: Some(ReturnExprType::MethodCall),
            has_self_param: true,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("get_name", &analysis),
            MethodComplexityClass::SimpleAccessor
        );
    }

    #[test]
    fn test_classify_boilerplate_new() {
        let analysis = MethodBodyAnalysis {
            line_count: 5,
            has_control_flow: false,
            call_count: 1,
            return_expr_type: None,
            has_self_param: false,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("new", &analysis),
            MethodComplexityClass::Boilerplate
        );
    }

    #[test]
    fn test_classify_boilerplate_default() {
        let analysis = MethodBodyAnalysis {
            line_count: 10,
            has_control_flow: false,
            call_count: 0,
            return_expr_type: None,
            has_self_param: false,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("default", &analysis),
            MethodComplexityClass::Boilerplate
        );
    }

    #[test]
    fn test_classify_boilerplate_clone() {
        let analysis = MethodBodyAnalysis {
            line_count: 1,
            has_control_flow: false,
            call_count: 1,
            return_expr_type: None,
            has_self_param: true,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("clone", &analysis),
            MethodComplexityClass::Boilerplate
        );
    }

    #[test]
    fn test_classify_boilerplate_from() {
        let analysis = MethodBodyAnalysis {
            line_count: 3,
            has_control_flow: false,
            call_count: 2,
            return_expr_type: None,
            has_self_param: false,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("from", &analysis),
            MethodComplexityClass::Boilerplate
        );
    }

    #[test]
    fn test_classify_boilerplate_into() {
        let analysis = MethodBodyAnalysis::default();
        assert_eq!(
            classify_method_complexity("into", &analysis),
            MethodComplexityClass::Boilerplate
        );
    }

    #[test]
    fn test_classify_boilerplate_trait_impls() {
        let analysis = MethodBodyAnalysis::default();
        assert_eq!(
            classify_method_complexity("fmt", &analysis),
            MethodComplexityClass::Boilerplate
        );
        assert_eq!(
            classify_method_complexity("eq", &analysis),
            MethodComplexityClass::Boilerplate
        );
        assert_eq!(
            classify_method_complexity("hash", &analysis),
            MethodComplexityClass::Boilerplate
        );
        assert_eq!(
            classify_method_complexity("drop", &analysis),
            MethodComplexityClass::Boilerplate
        );
    }

    #[test]
    fn test_classify_delegating() {
        let analysis = MethodBodyAnalysis {
            line_count: 1,
            has_control_flow: false,
            call_count: 1,
            return_expr_type: Some(ReturnExprType::MethodCall),
            has_self_param: true,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("process", &analysis),
            MethodComplexityClass::Delegating
        );
    }

    #[test]
    fn test_classify_substantive_complex_logic() {
        let analysis = MethodBodyAnalysis {
            line_count: 25,
            has_control_flow: true,
            call_count: 5,
            return_expr_type: Some(ReturnExprType::Complex),
            has_self_param: true,
            is_mutating: true,
        };
        assert_eq!(
            classify_method_complexity("analyze_workspace", &analysis),
            MethodComplexityClass::Substantive
        );
    }

    #[test]
    fn test_classify_substantive_with_control_flow() {
        let analysis = MethodBodyAnalysis {
            line_count: 5,
            has_control_flow: true, // Control flow makes it substantive
            call_count: 2,
            return_expr_type: Some(ReturnExprType::Complex),
            has_self_param: true,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("get_name", &analysis), // Even with accessor name
            MethodComplexityClass::Substantive
        );
    }

    #[test]
    fn test_classify_has_prefix() {
        let analysis = MethodBodyAnalysis {
            line_count: 1,
            has_control_flow: false,
            call_count: 0,
            return_expr_type: Some(ReturnExprType::FieldAccess),
            has_self_param: true,
            is_mutating: false,
        };
        assert_eq!(
            classify_method_complexity("has_children", &analysis),
            MethodComplexityClass::TrivialAccessor
        );
    }

    #[test]
    fn test_classify_with_prefix_builder() {
        let analysis = MethodBodyAnalysis {
            line_count: 2,
            has_control_flow: false,
            call_count: 0,
            return_expr_type: Some(ReturnExprType::Complex),
            has_self_param: true,
            is_mutating: true,
        };
        assert_eq!(
            classify_method_complexity("with_timeout", &analysis),
            MethodComplexityClass::SimpleAccessor
        );
    }

    #[test]
    fn test_calculate_weighted_method_count() {
        let classes = [
            MethodComplexityClass::TrivialAccessor, // 0.1
            MethodComplexityClass::TrivialAccessor, // 0.1
            MethodComplexityClass::Boilerplate,     // 0.0
            MethodComplexityClass::Substantive,     // 1.0
        ];
        let weighted = calculate_weighted_method_count(classes.iter());
        assert!(
            (weighted - 1.2).abs() < 0.01,
            "Expected 1.2, got {}",
            weighted
        );
    }

    #[test]
    fn test_weighted_count_all_accessors() {
        let classes = [MethodComplexityClass::TrivialAccessor; 10];
        let weighted = calculate_weighted_method_count(classes.iter());
        // 10 trivial accessors = 1.0 weighted
        assert!(
            (weighted - 1.0).abs() < 0.01,
            "Expected 1.0, got {}",
            weighted
        );
    }

    #[test]
    fn test_weighted_count_mixed() {
        // Example from spec: 10 accessors + 5 business methods
        let mut classes = vec![];
        // 10 trivial accessors (0.1 each = 1.0)
        for _ in 0..10 {
            classes.push(MethodComplexityClass::TrivialAccessor);
        }
        // 1 boilerplate (0.0)
        classes.push(MethodComplexityClass::Boilerplate);
        // 5 substantive methods (1.0 each = 5.0)
        for _ in 0..5 {
            classes.push(MethodComplexityClass::Substantive);
        }

        let weighted = calculate_weighted_method_count(classes.iter());
        // 10 * 0.1 + 1 * 0.0 + 5 * 1.0 = 1.0 + 0.0 + 5.0 = 6.0
        assert!(
            (weighted - 6.0).abs() < 0.01,
            "Expected 6.0, got {}",
            weighted
        );
    }

    #[test]
    fn test_method_complexity_class_weights() {
        assert!((MethodComplexityClass::TrivialAccessor.weight() - 0.1).abs() < f64::EPSILON);
        assert!((MethodComplexityClass::SimpleAccessor.weight() - 0.3).abs() < f64::EPSILON);
        assert!((MethodComplexityClass::Boilerplate.weight() - 0.0).abs() < f64::EPSILON);
        assert!((MethodComplexityClass::Delegating.weight() - 0.5).abs() < f64::EPSILON);
        assert!((MethodComplexityClass::Substantive.weight() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_method_complexity_class_default() {
        assert_eq!(
            MethodComplexityClass::default(),
            MethodComplexityClass::Substantive
        );
    }

    proptest! {
        /// Verify method classification is idempotent
        #[test]
        fn prop_classify_method_idempotent(
            method_name in "[a-z_]{1,20}",
            line_count in 0usize..100,
            has_control_flow in proptest::bool::ANY,
            call_count in 0usize..20
        ) {
            let analysis = MethodBodyAnalysis {
                line_count,
                has_control_flow,
                call_count,
                return_expr_type: Some(ReturnExprType::Complex),
                has_self_param: true,
                is_mutating: false,
            };
            let c1 = classify_method_complexity(&method_name, &analysis);
            let c2 = classify_method_complexity(&method_name, &analysis);
            prop_assert_eq!(c1, c2);
        }

        /// Verify weighted count is always non-negative
        #[test]
        fn prop_weighted_count_non_negative(
            num_trivial in 0usize..50,
            num_simple in 0usize..50,
            num_boilerplate in 0usize..50,
            num_delegating in 0usize..50,
            num_substantive in 0usize..50
        ) {
            let mut classes = vec![];
            classes.extend(std::iter::repeat_n(MethodComplexityClass::TrivialAccessor, num_trivial));
            classes.extend(std::iter::repeat_n(MethodComplexityClass::SimpleAccessor, num_simple));
            classes.extend(std::iter::repeat_n(MethodComplexityClass::Boilerplate, num_boilerplate));
            classes.extend(std::iter::repeat_n(MethodComplexityClass::Delegating, num_delegating));
            classes.extend(std::iter::repeat_n(MethodComplexityClass::Substantive, num_substantive));

            let weighted = calculate_weighted_method_count(classes.iter());
            prop_assert!(weighted >= 0.0);
        }
    }

    // =========================================================================
    // Spec 213: Pure Function Method Weighting Tests
    // =========================================================================

    #[test]
    fn test_classify_self_usage_pure_associated() {
        // Parse a method without self parameter
        let code = r#"
            impl Foo {
                fn helper(x: &str) -> bool { x.is_empty() }
            }
        "#;
        let file: syn::File = syn::parse_str(code).unwrap();
        if let syn::Item::Impl(impl_block) = &file.items[0] {
            if let syn::ImplItem::Fn(method) = &impl_block.items[0] {
                assert_eq!(
                    classify_self_usage(method),
                    MethodSelfUsage::PureAssociated,
                    "Method without self should be PureAssociated"
                );
            }
        }
    }

    #[test]
    fn test_classify_self_usage_instance_method() {
        // Parse a method that uses self.field
        let code = r#"
            impl Foo {
                fn get_data(&self) -> &Data { &self.data }
            }
        "#;
        let file: syn::File = syn::parse_str(code).unwrap();
        if let syn::Item::Impl(impl_block) = &file.items[0] {
            if let syn::ImplItem::Fn(method) = &impl_block.items[0] {
                assert_eq!(
                    classify_self_usage(method),
                    MethodSelfUsage::InstanceMethod,
                    "Method using self.field should be InstanceMethod"
                );
            }
        }
    }

    #[test]
    fn test_classify_self_usage_instance_method_call() {
        // Parse a method that calls self.method()
        let code = r#"
            impl Foo {
                fn process(&self) { self.do_work(); }
            }
        "#;
        let file: syn::File = syn::parse_str(code).unwrap();
        if let syn::Item::Impl(impl_block) = &file.items[0] {
            if let syn::ImplItem::Fn(method) = &impl_block.items[0] {
                assert_eq!(
                    classify_self_usage(method),
                    MethodSelfUsage::InstanceMethod,
                    "Method calling self.method() should be InstanceMethod"
                );
            }
        }
    }

    #[test]
    fn test_classify_self_usage_unused_self() {
        // Parse a method with self that doesn't use it
        let code = r#"
            impl Foo {
                fn debug(&self) { println!("debug"); }
            }
        "#;
        let file: syn::File = syn::parse_str(code).unwrap();
        if let syn::Item::Impl(impl_block) = &file.items[0] {
            if let syn::ImplItem::Fn(method) = &impl_block.items[0] {
                assert_eq!(
                    classify_self_usage(method),
                    MethodSelfUsage::UnusedSelf,
                    "Method with unused self should be UnusedSelf"
                );
            }
        }
    }

    #[test]
    fn test_classify_self_usage_self_type_call() {
        // Parse a method that calls Self::method() - should NOT count as self usage
        let code = r#"
            impl Foo {
                fn helper(&self) { Self::other_helper(); }
            }
        "#;
        let file: syn::File = syn::parse_str(code).unwrap();
        if let syn::Item::Impl(impl_block) = &file.items[0] {
            if let syn::ImplItem::Fn(method) = &impl_block.items[0] {
                assert_eq!(
                    classify_self_usage(method),
                    MethodSelfUsage::UnusedSelf,
                    "Method calling Self::method() (not self) should be UnusedSelf"
                );
            }
        }
    }

    #[test]
    fn test_classify_self_usage_closure_capture() {
        // Parse a method where self is used only in a closure - should NOT count
        // (closures capture self from outer scope, different semantic context)
        let code = r#"
            impl Foo {
                fn with_closure(&self) {
                    let f = || { println!("no self usage"); };
                    f();
                }
            }
        "#;
        let file: syn::File = syn::parse_str(code).unwrap();
        if let syn::Item::Impl(impl_block) = &file.items[0] {
            if let syn::ImplItem::Fn(method) = &impl_block.items[0] {
                assert_eq!(
                    classify_self_usage(method),
                    MethodSelfUsage::UnusedSelf,
                    "Self in closure should not count as self usage"
                );
            }
        }
    }

    #[test]
    fn test_classify_self_usage_mut_self() {
        // Parse a method with &mut self that mutates self
        let code = r#"
            impl Foo {
                fn add_item(&mut self, item: Item) { self.items.push(item); }
            }
        "#;
        let file: syn::File = syn::parse_str(code).unwrap();
        if let syn::Item::Impl(impl_block) = &file.items[0] {
            if let syn::ImplItem::Fn(method) = &impl_block.items[0] {
                assert_eq!(
                    classify_self_usage(method),
                    MethodSelfUsage::InstanceMethod,
                    "Method with &mut self that mutates should be InstanceMethod"
                );
            }
        }
    }

    #[test]
    fn test_calculate_combined_method_weight_pure_accessor() {
        // Pure accessor: min(0.1, 0.2) = 0.1
        let weight = calculate_combined_method_weight(
            MethodComplexityClass::TrivialAccessor,
            MethodSelfUsage::PureAssociated,
        );
        assert!(
            (weight - 0.1).abs() < f64::EPSILON,
            "Pure accessor should have weight 0.1, got {}",
            weight
        );
    }

    #[test]
    fn test_calculate_combined_method_weight_instance_substantive() {
        // Substantive instance method: min(1.0, 1.0) = 1.0
        let weight = calculate_combined_method_weight(
            MethodComplexityClass::Substantive,
            MethodSelfUsage::InstanceMethod,
        );
        assert!(
            (weight - 1.0).abs() < f64::EPSILON,
            "Instance substantive should have weight 1.0, got {}",
            weight
        );
    }

    #[test]
    fn test_calculate_combined_method_weight_pure_substantive() {
        // Pure helper (substantive but no self): min(1.0, 0.2) = 0.2
        let weight = calculate_combined_method_weight(
            MethodComplexityClass::Substantive,
            MethodSelfUsage::PureAssociated,
        );
        assert!(
            (weight - 0.2).abs() < f64::EPSILON,
            "Pure substantive should have weight 0.2, got {}",
            weight
        );
    }

    #[test]
    fn test_calculate_combined_method_weight_boilerplate_instance() {
        // Boilerplate instance: min(0.0, 1.0) = 0.0
        let weight = calculate_combined_method_weight(
            MethodComplexityClass::Boilerplate,
            MethodSelfUsage::InstanceMethod,
        );
        assert!(
            weight.abs() < f64::EPSILON,
            "Boilerplate should have weight 0.0, got {}",
            weight
        );
    }

    #[test]
    fn test_calculate_combined_weighted_count() {
        // Example from spec: 3 instance + 21 pure = 3*1.0 + 21*0.2 = 7.2
        let accessor_classes = vec![
            MethodComplexityClass::Substantive,
            MethodComplexityClass::Substantive,
            MethodComplexityClass::Substantive,
        ]
        .into_iter()
        .chain(std::iter::repeat(MethodComplexityClass::Substantive).take(21))
        .collect::<Vec<_>>();

        let self_usages = vec![
            MethodSelfUsage::InstanceMethod,
            MethodSelfUsage::InstanceMethod,
            MethodSelfUsage::InstanceMethod,
        ]
        .into_iter()
        .chain(std::iter::repeat(MethodSelfUsage::PureAssociated).take(21))
        .collect::<Vec<_>>();

        let weighted =
            calculate_combined_weighted_count(accessor_classes.iter().zip(self_usages.iter()));

        // 3*1.0 + 21*0.2 = 3.0 + 4.2 = 7.2
        assert!(
            (weighted - 7.2).abs() < 0.01,
            "Expected ~7.2, got {}",
            weighted
        );
    }

    #[test]
    fn test_method_self_usage_weights() {
        assert!(
            (MethodSelfUsage::PureAssociated.weight() - 0.2).abs() < f64::EPSILON,
            "PureAssociated weight"
        );
        assert!(
            (MethodSelfUsage::UnusedSelf.weight() - 0.3).abs() < f64::EPSILON,
            "UnusedSelf weight"
        );
        assert!(
            (MethodSelfUsage::InstanceMethod.weight() - 1.0).abs() < f64::EPSILON,
            "InstanceMethod weight"
        );
    }

    #[test]
    fn test_method_self_usage_default() {
        assert_eq!(
            MethodSelfUsage::default(),
            MethodSelfUsage::InstanceMethod,
            "Default should be conservative (InstanceMethod)"
        );
    }

    #[test]
    fn test_method_self_usage_is_pure() {
        assert!(
            MethodSelfUsage::PureAssociated.is_pure(),
            "PureAssociated should be pure"
        );
        assert!(
            MethodSelfUsage::UnusedSelf.is_pure(),
            "UnusedSelf should be pure"
        );
        assert!(
            !MethodSelfUsage::InstanceMethod.is_pure(),
            "InstanceMethod should NOT be pure"
        );
    }

    proptest! {
        /// Verify self-usage classification is deterministic
        #[test]
        fn prop_classify_self_usage_deterministic(
            has_self in proptest::bool::ANY,
            uses_field in proptest::bool::ANY
        ) {
            let code = if !has_self {
                r#"impl Foo { fn helper() {} }"#.to_string()
            } else if uses_field {
                r#"impl Foo { fn get(&self) { self.x } }"#.to_string()
            } else {
                r#"impl Foo { fn debug(&self) { println!("x"); } }"#.to_string()
            };

            if let Ok(file) = syn::parse_str::<syn::File>(&code) {
                if let syn::Item::Impl(impl_block) = &file.items[0] {
                    if let syn::ImplItem::Fn(method) = &impl_block.items[0] {
                        let c1 = classify_self_usage(method);
                        let c2 = classify_self_usage(method);
                        prop_assert_eq!(c1, c2);
                    }
                }
            }
        }

        /// Verify combined weight is always in [0.0, 1.0]
        #[test]
        fn prop_combined_weight_bounded(
            accessor in prop::sample::select(vec![
                MethodComplexityClass::TrivialAccessor,
                MethodComplexityClass::SimpleAccessor,
                MethodComplexityClass::Boilerplate,
                MethodComplexityClass::Delegating,
                MethodComplexityClass::Substantive,
            ]),
            self_usage in prop::sample::select(vec![
                MethodSelfUsage::PureAssociated,
                MethodSelfUsage::UnusedSelf,
                MethodSelfUsage::InstanceMethod,
            ])
        ) {
            let weight = calculate_combined_method_weight(accessor, self_usage);
            prop_assert!(weight >= 0.0 && weight <= 1.0,
                "Combined weight {} out of bounds", weight);
        }

        /// Verify pure methods always have lower weight than instance
        #[test]
        fn prop_pure_always_lower_weight_than_instance(
            num_methods in 1..50usize
        ) {
            let pure_weight: f64 = (0..num_methods)
                .map(|_| MethodSelfUsage::PureAssociated.weight())
                .sum();
            let instance_weight: f64 = (0..num_methods)
                .map(|_| MethodSelfUsage::InstanceMethod.weight())
                .sum();

            prop_assert!(pure_weight < instance_weight);
        }
    }
}
