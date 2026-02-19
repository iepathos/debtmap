//! Purity analysis types for TypeScript/JavaScript
//!
//! Types specific to JS/TS purity detection, mapping to core PurityLevel.

use crate::core::PurityLevel;
use std::fmt;

/// Result of purity analysis for a JavaScript/TypeScript function
#[derive(Debug, Clone)]
pub struct JsPurityAnalysis {
    /// The determined purity level
    pub level: PurityLevel,
    /// Confidence in the analysis (0.0 to 1.0)
    pub confidence: f32,
    /// Reasons for impurity classification
    pub reasons: Vec<JsImpurityReason>,
}

impl JsPurityAnalysis {
    /// Create a new pure analysis result
    pub fn pure() -> Self {
        Self {
            level: PurityLevel::StrictlyPure,
            confidence: 1.0,
            reasons: Vec::new(),
        }
    }

    /// Create an impure analysis result with reasons
    pub fn impure(reasons: Vec<JsImpurityReason>, confidence: f32) -> Self {
        Self {
            level: PurityLevel::Impure,
            confidence,
            reasons,
        }
    }

    /// Create a locally pure analysis result
    pub fn locally_pure(confidence: f32) -> Self {
        Self {
            level: PurityLevel::LocallyPure,
            confidence,
            reasons: Vec::new(),
        }
    }

    /// Create a read-only analysis result
    pub fn read_only(reasons: Vec<JsImpurityReason>, confidence: f32) -> Self {
        Self {
            level: PurityLevel::ReadOnly,
            confidence,
            reasons,
        }
    }
}

/// Reasons why a function is classified as impure
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsImpurityReason {
    /// Browser I/O operation (console, fetch, DOM, etc.)
    BrowserIO(String),
    /// Node.js I/O operation (fs, http, process, etc.)
    NodeIO(String),
    /// DOM mutation (appendChild, innerHTML, etc.)
    DomMutation(String),
    /// External state mutation (this.x = y, window.foo = bar)
    ExternalMutation(String),
    /// External state read (window.foo, document.body)
    ExternalRead(String),
    /// Async operation (await expression)
    AsyncOperation,
    /// Dynamic evaluation (eval, Function constructor)
    DynamicEval,
    /// Global variable access (window, document, process)
    GlobalAccess(String),
    /// Collection mutation method (push, pop, splice, etc.)
    CollectionMutation(String),
    /// Parameter mutation (modifying function argument)
    ParameterMutation(String),
    /// Unknown function call (can't determine purity)
    UnknownCall(String),
    /// Non-deterministic operation (Math.random, Date.now, etc.)
    NonDeterministic(String),
}

impl fmt::Display for JsImpurityReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsImpurityReason::BrowserIO(detail) => write!(f, "Browser I/O: {}", detail),
            JsImpurityReason::NodeIO(detail) => write!(f, "Node.js I/O: {}", detail),
            JsImpurityReason::DomMutation(detail) => write!(f, "DOM mutation: {}", detail),
            JsImpurityReason::ExternalMutation(detail) => {
                write!(f, "External mutation: {}", detail)
            }
            JsImpurityReason::ExternalRead(detail) => write!(f, "External read: {}", detail),
            JsImpurityReason::AsyncOperation => write!(f, "Async operation"),
            JsImpurityReason::DynamicEval => write!(f, "Dynamic eval"),
            JsImpurityReason::GlobalAccess(detail) => write!(f, "Global access: {}", detail),
            JsImpurityReason::CollectionMutation(detail) => {
                write!(f, "Collection mutation: {}", detail)
            }
            JsImpurityReason::ParameterMutation(detail) => {
                write!(f, "Parameter mutation: {}", detail)
            }
            JsImpurityReason::UnknownCall(detail) => write!(f, "Unknown call: {}", detail),
            JsImpurityReason::NonDeterministic(detail) => {
                write!(f, "Non-deterministic: {}", detail)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_analysis() {
        let analysis = JsPurityAnalysis::pure();
        assert_eq!(analysis.level, PurityLevel::StrictlyPure);
        assert_eq!(analysis.confidence, 1.0);
        assert!(analysis.reasons.is_empty());
    }

    #[test]
    fn test_impure_analysis() {
        let reasons = vec![JsImpurityReason::BrowserIO("console.log".to_string())];
        let analysis = JsPurityAnalysis::impure(reasons.clone(), 0.9);
        assert_eq!(analysis.level, PurityLevel::Impure);
        assert_eq!(analysis.confidence, 0.9);
        assert_eq!(analysis.reasons.len(), 1);
    }

    #[test]
    fn test_impurity_reason_display() {
        let reason = JsImpurityReason::BrowserIO("console.log".to_string());
        assert_eq!(reason.to_string(), "Browser I/O: console.log");

        let reason = JsImpurityReason::CollectionMutation("push".to_string());
        assert_eq!(reason.to_string(), "Collection mutation: push");
    }
}
