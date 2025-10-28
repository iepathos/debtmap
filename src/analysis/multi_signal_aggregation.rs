//! Multi-Signal Responsibility Aggregation
//!
//! This module implements weighted multi-signal aggregation for responsibility
//! classification, combining:
//! - I/O Detection (Spec 141)
//! - Call Graph Analysis (Spec 142)
//! - Purity Analysis (Spec 143)
//! - Framework Patterns (Spec 144)
//! - Type Signatures (Spec 147)
//! - Name Heuristics (fallback)
//!
//! Target accuracy: ~88% (vs ~50% name-based alone)

use crate::analysis::call_graph::RustCallGraph;
use crate::analysis::framework_patterns_multi::detector::{
    FileContext, FrameworkDetector, FunctionAst,
};
use crate::analysis::io_detection::{IoDetector, IoProfile, Language, Responsibility};
use crate::analysis::purity_analysis::{PurityAnalyzer, PurityLevel};
use crate::analysis::type_flow_tracker::TypeFlowTracker;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified responsibility category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResponsibilityCategory {
    FileIO,
    NetworkIO,
    DatabaseIO,
    ConfigurationIO,
    MixedIO,
    HttpRequestHandler,
    WebSocketHandler,
    CliHandler,
    DatabaseHandler,
    TestFunction,
    PureComputation,
    Validation,
    Transformation,
    Parsing,
    Formatting,
    Orchestration,
    Coordination,
    ErrorHandling,
    SideEffects,
    Unknown,
}

impl ResponsibilityCategory {
    /// Convert from I/O detection responsibility
    pub fn from_io_responsibility(resp: Responsibility) -> Self {
        match resp {
            Responsibility::FileIO => Self::FileIO,
            Responsibility::NetworkIO => Self::NetworkIO,
            Responsibility::DatabaseIO => Self::DatabaseIO,
            Responsibility::ConsoleIO => Self::Formatting, // Console I/O is typically formatting
            Responsibility::MixedIO => Self::MixedIO,
            Responsibility::SideEffects => Self::SideEffects,
            Responsibility::PureComputation => Self::PureComputation,
        }
    }

    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileIO => "File I/O",
            Self::NetworkIO => "Network I/O",
            Self::DatabaseIO => "Database I/O",
            Self::ConfigurationIO => "Configuration I/O",
            Self::MixedIO => "Mixed I/O",
            Self::HttpRequestHandler => "HTTP Request Handler",
            Self::WebSocketHandler => "WebSocket Handler",
            Self::CliHandler => "CLI Handler",
            Self::DatabaseHandler => "Database Handler",
            Self::TestFunction => "Test Function",
            Self::PureComputation => "Pure Computation",
            Self::Validation => "Validation",
            Self::Transformation => "Transformation",
            Self::Parsing => "Parsing",
            Self::Formatting => "Formatting",
            Self::Orchestration => "Orchestration",
            Self::Coordination => "Coordination",
            Self::ErrorHandling => "Error Handling",
            Self::SideEffects => "Side Effects",
            Self::Unknown => "Unknown",
        }
    }
}

/// Signal type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    IoDetection,
    CallGraph,
    Purity,
    Framework,
    TypeSignatures,
    Name,
}

/// Classification from I/O detection
#[derive(Debug, Clone)]
pub struct IoClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
}

/// Classification from call graph analysis
#[derive(Debug, Clone)]
pub struct CallGraphClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
}

/// Classification from purity analysis
#[derive(Debug, Clone)]
pub struct PurityClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
}

/// Classification from framework pattern detection
#[derive(Debug, Clone)]
pub struct FrameworkClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
    pub framework: String,
}

/// Classification from type signatures
#[derive(Debug, Clone)]
pub struct TypeSignatureClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
}

/// Classification from name heuristics
#[derive(Debug, Clone)]
pub struct NameBasedClassification {
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: String,
}

/// Collection of all signals for a function
#[derive(Debug, Clone, Default)]
pub struct SignalSet {
    pub io_signal: Option<IoClassification>,
    pub call_graph_signal: Option<CallGraphClassification>,
    pub purity_signal: Option<PurityClassification>,
    pub framework_signal: Option<FrameworkClassification>,
    pub type_signal: Option<TypeSignatureClassification>,
    pub name_signal: Option<NameBasedClassification>,
}

/// Evidence from a single signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalEvidence {
    pub signal_type: SignalType,
    pub category: ResponsibilityCategory,
    pub confidence: f64,
    pub weight: f64,
    pub contribution: f64,
    pub description: String,
}

/// Final aggregated classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedClassification {
    pub primary: ResponsibilityCategory,
    pub confidence: f64,
    pub evidence: Vec<SignalEvidence>,
    pub alternatives: Vec<(ResponsibilityCategory, f64)>,
}

/// Signal weights configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalWeights {
    pub io_detection: f64,
    pub call_graph: f64,
    pub type_signatures: f64,
    pub purity_side_effects: f64,
    pub framework_patterns: f64,
    pub name_heuristics: f64,
}

impl Default for SignalWeights {
    fn default() -> Self {
        SignalWeights {
            io_detection: 0.35,
            call_graph: 0.25,
            type_signatures: 0.15,
            purity_side_effects: 0.05,
            framework_patterns: 0.05,
            name_heuristics: 0.15,
        }
    }
}

impl SignalWeights {
    /// Validate that weights sum to 1.0
    pub fn validate(&self) -> anyhow::Result<()> {
        let sum = self.io_detection
            + self.call_graph
            + self.type_signatures
            + self.purity_side_effects
            + self.framework_patterns
            + self.name_heuristics;

        if (sum - 1.0).abs() > 0.01 {
            return Err(anyhow::anyhow!("Weights must sum to 1.0, got {}", sum));
        }

        Ok(())
    }
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    WeightedVoting,
    FrameworkFirst,
    IoFirst,
    HighestConfidence,
}

/// Aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    pub weights: SignalWeights,
    pub conflict_resolution: ConflictResolutionStrategy,
    pub minimum_confidence: f64,
    pub framework_override_threshold: f64,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        AggregationConfig {
            weights: SignalWeights::default(),
            conflict_resolution: ConflictResolutionStrategy::WeightedVoting,
            minimum_confidence: 0.30,
            framework_override_threshold: 0.70,
        }
    }
}

/// Multi-signal responsibility aggregator
pub struct ResponsibilityAggregator {
    config: AggregationConfig,
    io_detector: IoDetector,
    purity_analyzer: PurityAnalyzer,
    framework_detector: Option<FrameworkDetector>,
    call_graph: Option<RustCallGraph>,
    type_tracker: Option<TypeFlowTracker>,
}

impl ResponsibilityAggregator {
    /// Create new aggregator with default configuration
    pub fn new() -> Self {
        Self::with_config(AggregationConfig::default())
    }

    /// Create aggregator with custom configuration
    pub fn with_config(config: AggregationConfig) -> Self {
        ResponsibilityAggregator {
            config,
            io_detector: IoDetector::new(),
            purity_analyzer: PurityAnalyzer::new(),
            framework_detector: Some(FrameworkDetector::with_defaults()),
            call_graph: None,
            type_tracker: Some(TypeFlowTracker::new()),
        }
    }

    /// Set framework detector
    pub fn with_framework_detector(mut self, detector: FrameworkDetector) -> Self {
        self.framework_detector = Some(detector);
        self
    }

    /// Set call graph for analysis
    pub fn with_call_graph(mut self, call_graph: RustCallGraph) -> Self {
        self.call_graph = Some(call_graph);
        self
    }

    /// Set type tracker
    pub fn with_type_tracker(mut self, tracker: TypeFlowTracker) -> Self {
        self.type_tracker = Some(tracker);
        self
    }

    /// Collect I/O signal from function body
    pub fn collect_io_signal(&self, body: &str, language: Language) -> Option<IoClassification> {
        let profile = self.io_detector.detect_io(body, language);

        // Check if there's any I/O or impurity
        if profile.is_pure && profile.side_effects.is_empty() {
            return None;
        }

        let responsibility = profile.primary_responsibility();
        let category = ResponsibilityCategory::from_io_responsibility(responsibility);
        let confidence = calculate_io_confidence(&profile);
        let evidence = format_io_evidence(&profile);

        Some(IoClassification {
            category,
            confidence,
            evidence,
        })
    }

    /// Collect purity signal from function body
    pub fn collect_purity_signal(
        &self,
        body: &str,
        language: Language,
    ) -> Option<PurityClassification> {
        let analysis = self.purity_analyzer.analyze_code(body, language);

        // Only signal pure computation if STRICTLY pure with high confidence
        // This prevents overriding more specific classifications
        let (category, confidence) = match analysis.purity {
            PurityLevel::StrictlyPure if analysis.is_deterministic => {
                (ResponsibilityCategory::PureComputation, 0.70)
            }
            PurityLevel::StrictlyPure => (ResponsibilityCategory::PureComputation, 0.50),
            _ => return None, // Don't provide signal for non-strictly-pure functions
        };

        let determinism = if analysis.is_deterministic {
            "deterministic"
        } else {
            "non-deterministic"
        };
        let evidence = format!("{}, {}", analysis.purity.as_str(), determinism);

        Some(PurityClassification {
            category,
            confidence,
            evidence,
        })
    }

    /// Collect name-based signal (fallback)
    pub fn collect_name_signal(&self, name: &str) -> NameBasedClassification {
        let category = classify_from_name(name);

        // Higher confidence for strong naming patterns
        let confidence = if category != ResponsibilityCategory::Unknown {
            0.70 // Good confidence for matched patterns
        } else {
            0.30 // Low confidence for unknown
        };

        let evidence = format!("Name pattern: {}", name);

        NameBasedClassification {
            category,
            confidence,
            evidence,
        }
    }

    /// Collect call graph signal from function analysis
    pub fn collect_call_graph_signal(&self, func_name: &str) -> Option<CallGraphClassification> {
        let call_graph = self.call_graph.as_ref()?;

        // Analyze function based on call patterns
        let all_functions = call_graph.base_graph.find_all_functions();
        let matching_function = all_functions
            .into_iter()
            .find(|func_id| func_id.name.contains(func_name))?;

        // Get callees to understand responsibilities
        let callees = call_graph.base_graph.get_callees(&matching_function);
        let callers = call_graph.base_graph.get_callers(&matching_function);

        // High fan-out suggests orchestration
        let is_orchestration = callees.len() > 5;

        // Check for handler patterns (many callers, entry point)
        let is_handler =
            callers.is_empty() && call_graph.base_graph.is_entry_point(&matching_function);

        let category = if is_handler {
            ResponsibilityCategory::HttpRequestHandler
        } else if is_orchestration {
            ResponsibilityCategory::Orchestration
        } else if !callees.is_empty() {
            ResponsibilityCategory::Coordination
        } else {
            return None; // No clear signal from call graph
        };

        let confidence = if is_handler {
            0.85
        } else if is_orchestration {
            0.75
        } else {
            0.60
        };

        let evidence = format!("{} callers, {} callees", callers.len(), callees.len());

        Some(CallGraphClassification {
            category,
            confidence,
            evidence,
        })
    }

    /// Collect framework pattern signal
    pub fn collect_framework_signal(
        &self,
        function: &FunctionAst,
        file_context: &FileContext,
    ) -> Option<FrameworkClassification> {
        let detector = self.framework_detector.as_ref()?;

        let matches = detector.detect_framework_patterns(function, file_context);

        // Get the highest confidence match
        let best_match = matches.into_iter().next()?;

        let category = match best_match.category.as_str() {
            "HTTP Request Handler" => ResponsibilityCategory::HttpRequestHandler,
            "WebSocket Handler" => ResponsibilityCategory::WebSocketHandler,
            "CLI Handler" => ResponsibilityCategory::CliHandler,
            "Database Handler" => ResponsibilityCategory::DatabaseHandler,
            "Test Function" => ResponsibilityCategory::TestFunction,
            _ => ResponsibilityCategory::Unknown,
        };

        Some(FrameworkClassification {
            category,
            confidence: best_match.confidence,
            evidence: best_match.evidence.join(", "),
            framework: best_match.framework,
        })
    }

    /// Collect type signature signal
    pub fn collect_type_signal(
        &self,
        return_type: Option<&str>,
        parameters: &[(String, String)],
    ) -> Option<TypeSignatureClassification> {
        // Analyze return type for I/O hints
        let return_io = return_type.and_then(|rt| {
            if rt.contains("Result<File") || rt.contains("std::fs::File") {
                Some((ResponsibilityCategory::FileIO, 0.80))
            } else if rt.contains("Response") || rt.contains("HttpResponse") {
                Some((ResponsibilityCategory::HttpRequestHandler, 0.75))
            } else if rt.contains("Connection") || rt.contains("Stream") {
                Some((ResponsibilityCategory::NetworkIO, 0.80))
            } else {
                None
            }
        });

        // Analyze parameters for I/O hints
        let param_io = parameters.iter().find_map(|(name, type_ann)| {
            if type_ann.contains("Path") || type_ann.contains("&str") && name.contains("path") {
                Some((ResponsibilityCategory::FileIO, 0.70))
            } else if type_ann.contains("Request") || type_ann.contains("HttpRequest") {
                Some((ResponsibilityCategory::HttpRequestHandler, 0.85))
            } else if type_ann.contains("TcpStream") || type_ann.contains("Socket") {
                Some((ResponsibilityCategory::NetworkIO, 0.80))
            } else if type_ann.contains("Connection") || type_ann.contains("Client") {
                Some((ResponsibilityCategory::DatabaseIO, 0.75))
            } else {
                None
            }
        });

        // Prefer parameter evidence over return type
        let (category, confidence) = param_io.or(return_io)?;

        let evidence = format!(
            "Type signature indicates {} (from {})",
            category.as_str(),
            if param_io.is_some() {
                "parameters"
            } else {
                "return type"
            }
        );

        Some(TypeSignatureClassification {
            category,
            confidence,
            evidence,
        })
    }

    /// Aggregate all signals into final classification
    pub fn aggregate(&self, signals: &SignalSet) -> AggregatedClassification {
        // Check for high-confidence framework override
        if let Some(ref framework) = signals.framework_signal {
            if framework.confidence >= self.config.framework_override_threshold {
                return self.framework_override(framework, signals);
            }
        }

        // Collect weighted votes
        let mut category_scores: HashMap<ResponsibilityCategory, f64> = HashMap::new();
        let mut evidence: Vec<SignalEvidence> = Vec::new();

        // I/O Detection (40%)
        if let Some(ref io) = signals.io_signal {
            let contribution = io.confidence * self.config.weights.io_detection;
            *category_scores.entry(io.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::IoDetection,
                category: io.category,
                confidence: io.confidence,
                weight: self.config.weights.io_detection,
                contribution,
                description: io.evidence.clone(),
            });
        }

        // Call Graph (30%)
        if let Some(ref cg) = signals.call_graph_signal {
            let contribution = cg.confidence * self.config.weights.call_graph;
            *category_scores.entry(cg.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::CallGraph,
                category: cg.category,
                confidence: cg.confidence,
                weight: self.config.weights.call_graph,
                contribution,
                description: cg.evidence.clone(),
            });
        }

        // Type Signatures (15%)
        if let Some(ref ts) = signals.type_signal {
            let contribution = ts.confidence * self.config.weights.type_signatures;
            *category_scores.entry(ts.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::TypeSignatures,
                category: ts.category,
                confidence: ts.confidence,
                weight: self.config.weights.type_signatures,
                contribution,
                description: ts.evidence.clone(),
            });
        }

        // Purity (10%)
        if let Some(ref purity) = signals.purity_signal {
            let contribution = purity.confidence * self.config.weights.purity_side_effects;
            *category_scores.entry(purity.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::Purity,
                category: purity.category,
                confidence: purity.confidence,
                weight: self.config.weights.purity_side_effects,
                contribution,
                description: purity.evidence.clone(),
            });
        }

        // Framework Patterns (5%, low confidence)
        if let Some(ref framework) = signals.framework_signal {
            if framework.confidence < self.config.framework_override_threshold {
                let contribution = framework.confidence * self.config.weights.framework_patterns;
                *category_scores.entry(framework.category).or_insert(0.0) += contribution;

                evidence.push(SignalEvidence {
                    signal_type: SignalType::Framework,
                    category: framework.category,
                    confidence: framework.confidence,
                    weight: self.config.weights.framework_patterns,
                    contribution,
                    description: framework.evidence.clone(),
                });
            }
        }

        // Name Heuristics (5%, fallback)
        if let Some(ref name) = signals.name_signal {
            let contribution = name.confidence * self.config.weights.name_heuristics;
            *category_scores.entry(name.category).or_insert(0.0) += contribution;

            evidence.push(SignalEvidence {
                signal_type: SignalType::Name,
                category: name.category,
                confidence: name.confidence,
                weight: self.config.weights.name_heuristics,
                contribution,
                description: name.evidence.clone(),
            });
        }

        // Select primary and alternatives
        let mut sorted_categories: Vec<_> = category_scores.into_iter().collect();
        sorted_categories.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let (primary_category, primary_score) = sorted_categories
            .first()
            .copied()
            .unwrap_or((ResponsibilityCategory::Unknown, 0.0));

        let alternatives: Vec<_> = sorted_categories.into_iter().skip(1).take(2).collect();

        AggregatedClassification {
            primary: primary_category,
            confidence: primary_score,
            evidence,
            alternatives,
        }
    }

    /// Handle framework override case
    fn framework_override(
        &self,
        framework: &FrameworkClassification,
        signals: &SignalSet,
    ) -> AggregatedClassification {
        let mut evidence = vec![SignalEvidence {
            signal_type: SignalType::Framework,
            category: framework.category,
            confidence: framework.confidence,
            weight: 1.0,
            contribution: framework.confidence,
            description: framework.evidence.clone(),
        }];

        // Include other signals as supporting evidence
        if let Some(ref io) = signals.io_signal {
            evidence.push(SignalEvidence {
                signal_type: SignalType::IoDetection,
                category: io.category,
                confidence: io.confidence,
                weight: 0.0,
                contribution: 0.0,
                description: format!("Also detected: {}", io.evidence),
            });
        }

        AggregatedClassification {
            primary: framework.category,
            confidence: framework.confidence,
            evidence,
            alternatives: vec![],
        }
    }
}

impl Default for ResponsibilityAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate confidence score from I/O profile
fn calculate_io_confidence(profile: &IoProfile) -> f64 {
    let io_count = profile.file_operations.len()
        + profile.network_operations.len()
        + profile.database_operations.len();

    if io_count == 0 {
        return 0.3; // Low confidence if no I/O
    }

    // Higher confidence with more I/O operations
    (0.6 + (io_count as f64 * 0.1)).min(0.95)
}

/// Format I/O evidence string
fn format_io_evidence(profile: &IoProfile) -> String {
    let mut parts = Vec::new();

    if !profile.file_operations.is_empty() {
        parts.push(format!("{} file ops", profile.file_operations.len()));
    }
    if !profile.network_operations.is_empty() {
        parts.push(format!("{} network ops", profile.network_operations.len()));
    }
    if !profile.database_operations.is_empty() {
        parts.push(format!("{} DB ops", profile.database_operations.len()));
    }
    if !profile.side_effects.is_empty() {
        parts.push(format!("{} side effects", profile.side_effects.len()));
    }

    if parts.is_empty() {
        "No I/O detected".to_string()
    } else {
        parts.join(", ")
    }
}

/// Classify responsibility from function name (fallback)
fn classify_from_name(name: &str) -> ResponsibilityCategory {
    let lower = name.to_lowercase();

    // Test functions
    if lower.starts_with("test_") || lower.contains("_test") {
        return ResponsibilityCategory::TestFunction;
    }

    // Parsing (check before HTTP patterns that might contain "request")
    if lower.starts_with("parse")
        || lower.contains("_parse")
        || lower.ends_with("_body") && lower.contains("parse")
    {
        return ResponsibilityCategory::Parsing;
    }

    // Error handling (check before other handlers to avoid conflicts)
    if (lower.starts_with("handle") && lower.contains("error"))
        || (lower.contains("error") && lower.contains("handle"))
    {
        return ResponsibilityCategory::ErrorHandling;
    }

    // Framework handlers (check before I/O patterns)
    if lower.contains("handle")
        && (lower.contains("command") || lower.contains("cli") || lower.contains("init"))
    {
        return ResponsibilityCategory::CliHandler;
    }
    if lower.contains("handle")
        && (lower.contains("http")
            || lower.contains("request")
            || lower.contains("get_")
            || lower.contains("post_"))
    {
        return ResponsibilityCategory::HttpRequestHandler;
    }
    if lower.contains("handler") && !lower.contains("error") {
        return ResponsibilityCategory::HttpRequestHandler;
    }

    // I/O patterns (check config first before generic file operations)
    if (lower.contains("config") && lower.contains("load"))
        || (lower.contains("app") && lower.contains("config"))
    {
        return ResponsibilityCategory::ConfigurationIO;
    }
    if lower.contains("read") || lower.contains("write") || lower.contains("file") {
        return ResponsibilityCategory::FileIO;
    }
    if lower.contains("http") || lower.contains("fetch") {
        return ResponsibilityCategory::NetworkIO;
    }
    if lower.contains("database")
        || lower.contains("query")
        || lower.contains("sql")
        || (lower.contains("find") && lower.contains("by"))
    {
        return ResponsibilityCategory::DatabaseIO;
    }

    // Orchestration and coordination
    if lower.starts_with("process_") || lower.starts_with("execute_") || lower.contains("workflow")
    {
        return ResponsibilityCategory::Orchestration;
    }
    if lower.starts_with("delegate")
        || lower.starts_with("coordinate")
        || lower.contains("dispatch")
    {
        return ResponsibilityCategory::Coordination;
    }

    // Pure operations
    if lower.starts_with("calculate") || lower.starts_with("compute") || lower.starts_with("sum") {
        return ResponsibilityCategory::PureComputation;
    }

    if lower.starts_with("format") || lower.starts_with("render") || lower.contains("_format") {
        return ResponsibilityCategory::Formatting;
    }

    if lower.starts_with("validate") || lower.contains("_valid") {
        return ResponsibilityCategory::Validation;
    }

    if lower.starts_with("transform") || lower.starts_with("convert") || lower.contains("_to_") {
        return ResponsibilityCategory::Transformation;
    }

    ResponsibilityCategory::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_aggregation_io_wins() {
        let signals = SignalSet {
            io_signal: Some(IoClassification {
                category: ResponsibilityCategory::FileIO,
                confidence: 0.9,
                evidence: "Reads config file".to_string(),
            }),
            call_graph_signal: Some(CallGraphClassification {
                category: ResponsibilityCategory::Orchestration,
                confidence: 0.6,
                evidence: "Calls 5 functions".to_string(),
            }),
            purity_signal: Some(PurityClassification {
                category: ResponsibilityCategory::PureComputation,
                confidence: 0.3,
                evidence: "Impure".to_string(),
            }),
            ..Default::default()
        };

        let aggregator = ResponsibilityAggregator::new();
        let result = aggregator.aggregate(&signals);

        // I/O has highest weight (0.4) and confidence (0.9)
        assert_eq!(result.primary, ResponsibilityCategory::FileIO);
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn framework_override_high_confidence() {
        let signals = SignalSet {
            io_signal: Some(IoClassification {
                category: ResponsibilityCategory::NetworkIO,
                confidence: 0.8,
                evidence: "HTTP request".to_string(),
            }),
            framework_signal: Some(FrameworkClassification {
                category: ResponsibilityCategory::HttpRequestHandler,
                confidence: 0.95,
                evidence: "Axum handler pattern".to_string(),
                framework: "Axum".to_string(),
            }),
            ..Default::default()
        };

        let aggregator = ResponsibilityAggregator::new();
        let result = aggregator.aggregate(&signals);

        // Framework should override with high confidence
        assert_eq!(result.primary, ResponsibilityCategory::HttpRequestHandler);
        assert!(result.confidence >= 0.95);
    }

    #[test]
    fn multiple_weak_signals_combine() {
        let signals = SignalSet {
            purity_signal: Some(PurityClassification {
                category: ResponsibilityCategory::PureComputation,
                confidence: 0.7,
                evidence: "Strictly pure".to_string(),
            }),
            name_signal: Some(NameBasedClassification {
                category: ResponsibilityCategory::PureComputation,
                confidence: 0.5,
                evidence: "Name: calculate_total".to_string(),
            }),
            ..Default::default()
        };

        let aggregator = ResponsibilityAggregator::new();
        let result = aggregator.aggregate(&signals);

        // Purity and name signals should combine for pure computation
        assert_eq!(result.primary, ResponsibilityCategory::PureComputation);
    }

    #[test]
    fn weights_validation() {
        let weights = SignalWeights::default();
        assert!(weights.validate().is_ok());

        let invalid_weights = SignalWeights {
            io_detection: 0.5,
            call_graph: 0.5,
            type_signatures: 0.5, // Sum = 1.5, should fail
            purity_side_effects: 0.0,
            framework_patterns: 0.0,
            name_heuristics: 0.0,
        };
        assert!(
            invalid_weights.validate().is_err(),
            "Weights summing to 1.5 should be invalid"
        );
    }

    #[test]
    fn name_classification_fallback() {
        assert_eq!(
            classify_from_name("read_file"),
            ResponsibilityCategory::FileIO
        );
        assert_eq!(
            classify_from_name("http_request"),
            ResponsibilityCategory::NetworkIO
        );
        assert_eq!(
            classify_from_name("calculate_sum"),
            ResponsibilityCategory::PureComputation
        );
        assert_eq!(
            classify_from_name("parse_json"),
            ResponsibilityCategory::Parsing
        );
        assert_eq!(
            classify_from_name("validate_input"),
            ResponsibilityCategory::Validation
        );
    }
}
