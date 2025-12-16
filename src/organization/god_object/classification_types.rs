//! # Classification Types
//!
//! Types for god object classification and enhanced analysis.
//!
//! ## Stillwater Architecture
//!
//! This module is part of the **Pure Core** - data structures with no behavior.

use serde::{Deserialize, Serialize};

use super::core_types::{GodObjectAnalysis, StructMetrics};
use super::split_types::ModuleSplit;

// ============================================================================
// Spec 209: Accessor and Boilerplate Method Detection
// ============================================================================

/// Classification of method complexity for weighted god object scoring.
///
/// Methods are classified by their complexity to provide more accurate god object
/// detection. Trivial accessor methods and boilerplate don't contribute as much
/// to the "god object" pattern as substantive business logic methods.
///
/// # Weights
///
/// - `TrivialAccessor`: 0.1 (single-line field return)
/// - `SimpleAccessor`: 0.3 (getter/setter with minor transformation)
/// - `Boilerplate`: 0.0 (constructor, Default, Clone, From/Into)
/// - `Delegating`: 0.5 (method that simply calls another method)
/// - `Substantive`: 1.0 (actual business logic)
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::MethodComplexityClass;
///
/// // Trivial accessor: weight = 0.1
/// let class = MethodComplexityClass::TrivialAccessor;
/// assert!((class.weight() - 0.1).abs() < f64::EPSILON);
///
/// // Boilerplate methods don't count
/// let class = MethodComplexityClass::Boilerplate;
/// assert!((class.weight() - 0.0).abs() < f64::EPSILON);
///
/// // Substantive methods count fully
/// let class = MethodComplexityClass::Substantive;
/// assert!((class.weight() - 1.0).abs() < f64::EPSILON);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MethodComplexityClass {
    /// Single-line field return: `fn get_x(&self) -> &X { &self.x }`
    TrivialAccessor,
    /// Minor transformation: `fn get_x(&self) -> X { self.x.clone() }`
    SimpleAccessor,
    /// Constructor, Default, Clone, From, Into methods
    Boilerplate,
    /// Method that simply calls another method: `fn foo(&self) { self.inner.foo() }`
    Delegating,
    /// Actual business logic with control flow, multiple operations, etc.
    Substantive,
}

impl MethodComplexityClass {
    /// Returns the weight for this complexity class.
    ///
    /// Used in weighted method counting for god object scoring.
    /// Lower weights mean the method contributes less to the god object score.
    #[must_use]
    pub fn weight(&self) -> f64 {
        match self {
            Self::TrivialAccessor => 0.1,
            Self::SimpleAccessor => 0.3,
            Self::Boilerplate => 0.0,
            Self::Delegating => 0.5,
            Self::Substantive => 1.0,
        }
    }

    /// Returns a human-readable description of this complexity class.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::TrivialAccessor => "trivial accessor (single-line field return)",
            Self::SimpleAccessor => "simple accessor (getter/setter with minor transformation)",
            Self::Boilerplate => "boilerplate (constructor/Default/Clone/From/Into)",
            Self::Delegating => "delegating (single method call)",
            Self::Substantive => "substantive (business logic)",
        }
    }
}

impl Default for MethodComplexityClass {
    fn default() -> Self {
        // Default to substantive (conservative - counts fully)
        Self::Substantive
    }
}

/// Type of expression in a method's return statement.
///
/// Used to help classify accessor methods. A method returning a direct field
/// reference is more likely to be a trivial accessor than one with complex logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReturnExprType {
    /// Direct field access: `&self.field` or `self.field`
    FieldAccess,
    /// Method call: `self.other_method()` or `self.field.method()`
    MethodCall,
    /// Literal value: `true`, `false`, `0`, `"string"`, etc.
    Literal,
    /// Any other expression (complex logic)
    Complex,
}

impl Default for ReturnExprType {
    fn default() -> Self {
        Self::Complex
    }
}

/// Analysis results for a single method body.
///
/// Captures the information needed to classify a method's complexity.
#[derive(Debug, Clone, Default)]
pub struct MethodBodyAnalysis {
    /// Number of substantive lines (excluding empty, comments)
    pub line_count: usize,
    /// Whether the method contains control flow (if, match, loop, while, for)
    pub has_control_flow: bool,
    /// Number of method/function calls in the body
    pub call_count: usize,
    /// Type of the return expression (if any)
    pub return_expr_type: Option<ReturnExprType>,
    /// Whether the method has a `self` parameter
    pub has_self_param: bool,
    /// Whether the method modifies self (`&mut self`)
    pub is_mutating: bool,
}

/// Complete method analysis including classification.
///
/// This is the result of analyzing a method for god object detection purposes.
#[derive(Debug, Clone)]
pub struct MethodAnalysis {
    /// Name of the method
    pub name: String,
    /// Analysis of the method body
    pub body_analysis: MethodBodyAnalysis,
    /// Determined complexity class
    pub complexity_class: MethodComplexityClass,
}

/// Classification of god object types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GodObjectType {
    /// Single struct with excessive methods and responsibilities
    GodClass {
        struct_name: String,
        method_count: usize,
        field_count: usize,
        responsibilities: usize,
    },
    /// Multiple structs in a file that collectively exceed thresholds
    GodModule {
        total_structs: usize,
        total_methods: usize,
        largest_struct: StructMetrics,
        suggested_splits: Vec<ModuleSplit>,
    },
    /// Registry/catalog pattern - intentional centralization of trait implementations
    Registry {
        pattern: crate::organization::registry_pattern::RegistryPattern,
        confidence: f64,
        original_score: f64,
        adjusted_score: f64,
    },
    /// Builder pattern - intentional fluent API with many setter methods
    Builder {
        pattern: crate::organization::builder_pattern::BuilderPattern,
        confidence: f64,
        original_score: f64,
        adjusted_score: f64,
    },
    /// Boilerplate pattern - repetitive low-complexity code that should be macro-ified
    BoilerplatePattern {
        pattern: crate::organization::boilerplate_detector::BoilerplatePattern,
        confidence: f64,
        recommendation: String,
    },
    /// No god object detected
    NotGodObject,
}

/// Enhanced god object analysis with struct-level detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedGodObjectAnalysis {
    pub file_metrics: GodObjectAnalysis,
    pub per_struct_metrics: Vec<StructMetrics>,
    pub classification: GodObjectType,
    pub recommendation: String,
}

/// Data structure for grouping structs with their methods
#[derive(Debug, Clone)]
pub struct StructWithMethods {
    pub name: String,
    pub methods: Vec<String>,
    pub line_span: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassificationResult {
    /// The classified responsibility category, or `None` if confidence is too low
    pub category: Option<String>,
    /// Confidence score from 0.0 to 1.0
    pub confidence: f64,
    /// Signal types that contributed to this classification
    pub signals_used: Vec<SignalType>,
}

/// Types of signals used for responsibility classification.
///
/// These represent different sources of evidence used to determine
/// a method's responsibility category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignalType {
    /// Method name pattern matching
    NameHeuristic,
    /// I/O operation detection in method body
    IoDetection,
    /// Call graph analysis
    CallGraph,
    /// Type signature analysis
    TypeSignature,
    /// Purity and side effect analysis
    PurityAnalysis,
    /// Framework-specific patterns
    FrameworkPattern,
}
