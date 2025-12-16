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

// ============================================================================
// Spec 213: Pure Function Method Weighting
// ============================================================================

/// Classification of method self-usage for weighted god object scoring.
///
/// Methods are classified by whether and how they use `self` to provide more
/// accurate god object detection. Pure associated functions that don't use
/// instance state contribute less to the "god object" pattern than methods
/// that actually manipulate instance state.
///
/// # Weights
///
/// - `PureAssociated`: 0.2 (no self parameter, stateless helper)
/// - `UnusedSelf`: 0.3 (has self but doesn't use it)
/// - `InstanceMethod`: 1.0 (actually uses self state)
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::MethodSelfUsage;
///
/// // Pure associated function: weight = 0.2
/// let usage = MethodSelfUsage::PureAssociated;
/// assert!((usage.weight() - 0.2).abs() < f64::EPSILON);
///
/// // Unused self: weight = 0.3
/// let usage = MethodSelfUsage::UnusedSelf;
/// assert!((usage.weight() - 0.3).abs() < f64::EPSILON);
///
/// // Instance method: weight = 1.0
/// let usage = MethodSelfUsage::InstanceMethod;
/// assert!((usage.weight() - 1.0).abs() < f64::EPSILON);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MethodSelfUsage {
    /// No self parameter, stateless function
    /// Example: `fn helper(x: &str) -> bool { x.is_empty() }`
    PureAssociated,
    /// Has self parameter but doesn't actually use it
    /// Example: `fn debug(&self) { println!("debug"); }`
    UnusedSelf,
    /// Has self parameter and uses instance state
    /// Example: `fn get_data(&self) -> &Data { &self.data }`
    InstanceMethod,
}

impl MethodSelfUsage {
    /// Returns the weight for this self-usage classification.
    ///
    /// Used in weighted method counting for god object scoring.
    /// Lower weights mean the method contributes less to the god object score.
    #[must_use]
    pub fn weight(&self) -> f64 {
        match self {
            Self::PureAssociated => 0.2, // Pure helpers barely count
            Self::UnusedSelf => 0.3,     // Slight reduction
            Self::InstanceMethod => 1.0, // Full weight
        }
    }

    /// Returns a human-readable description of this self-usage classification.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::PureAssociated => "pure associated (no self, stateless helper)",
            Self::UnusedSelf => "unused self (has self but doesn't use it)",
            Self::InstanceMethod => "instance method (uses self state)",
        }
    }

    /// Returns whether this method is a pure helper (not using instance state).
    #[must_use]
    pub fn is_pure(&self) -> bool {
        matches!(self, Self::PureAssociated | Self::UnusedSelf)
    }
}

impl Default for MethodSelfUsage {
    fn default() -> Self {
        // Default to instance method (conservative - counts fully)
        Self::InstanceMethod
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
    /// Self-usage classification (Spec 213)
    pub self_usage: MethodSelfUsage,
}

// ============================================================================
// Spec 213: Method Breakdown for Reporting
// ============================================================================

/// Breakdown of instance vs pure methods for reporting (Spec 213).
///
/// This provides visibility into the "instance vs pure" method breakdown
/// for god object analysis output.
///
/// # Examples
///
/// ```
/// use debtmap::organization::god_object::MethodSelfUsageBreakdown;
///
/// let breakdown = MethodSelfUsageBreakdown {
///     total_methods: 24,
///     instance_methods: 3,
///     pure_associated: 20,
///     unused_self: 1,
/// };
///
/// // Display shows breakdown
/// assert_eq!(breakdown.to_string(), "24 (3 instance, 21 pure helpers)");
///
/// // Check if mostly pure (>50%)
/// assert!(breakdown.is_mostly_pure());
///
/// // Calculate effective weighted count
/// let weighted = breakdown.weighted_count();
/// assert!((weighted - 7.3).abs() < 0.01); // 3*1.0 + 20*0.2 + 1*0.3 = 7.3
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MethodSelfUsageBreakdown {
    /// Total number of methods
    pub total_methods: usize,
    /// Methods that use instance state
    pub instance_methods: usize,
    /// Pure associated functions (no self)
    pub pure_associated: usize,
    /// Methods with unused self parameter
    pub unused_self: usize,
}

impl MethodSelfUsageBreakdown {
    /// Create a new breakdown from method classifications.
    pub fn from_classifications(classifications: &[MethodSelfUsage]) -> Self {
        let (instance_methods, pure_associated, unused_self) =
            classifications
                .iter()
                .fold((0, 0, 0), |(inst, pure, unused), class| match class {
                    MethodSelfUsage::InstanceMethod => (inst + 1, pure, unused),
                    MethodSelfUsage::PureAssociated => (inst, pure + 1, unused),
                    MethodSelfUsage::UnusedSelf => (inst, pure, unused + 1),
                });

        Self {
            total_methods: classifications.len(),
            instance_methods,
            pure_associated,
            unused_self,
        }
    }

    /// Calculate the weighted method count based on self-usage.
    ///
    /// Applies the weights from `MethodSelfUsage`:
    /// - Instance methods: 1.0
    /// - Pure associated: 0.2
    /// - Unused self: 0.3
    pub fn weighted_count(&self) -> f64 {
        (self.instance_methods as f64 * MethodSelfUsage::InstanceMethod.weight())
            + (self.pure_associated as f64 * MethodSelfUsage::PureAssociated.weight())
            + (self.unused_self as f64 * MethodSelfUsage::UnusedSelf.weight())
    }

    /// Returns the count of pure helper methods (pure_associated + unused_self).
    pub fn pure_helper_count(&self) -> usize {
        self.pure_associated + self.unused_self
    }

    /// Returns the ratio of pure helpers to total methods (0.0 to 1.0).
    pub fn pure_ratio(&self) -> f64 {
        if self.total_methods == 0 {
            return 0.0;
        }
        self.pure_helper_count() as f64 / self.total_methods as f64
    }

    /// Returns true if more than 50% of methods are pure helpers.
    ///
    /// This indicates intentional functional decomposition per Spec 213.
    pub fn is_mostly_pure(&self) -> bool {
        self.pure_ratio() > 0.5
    }

    /// Returns true if more than 70% of methods are pure helpers.
    ///
    /// This is a strong signal of cohesive functional design.
    pub fn is_highly_pure(&self) -> bool {
        self.pure_ratio() > 0.7
    }
}

impl std::fmt::Display for MethodSelfUsageBreakdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({} instance, {} pure helpers)",
            self.total_methods,
            self.instance_methods,
            self.pure_helper_count()
        )
    }
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
