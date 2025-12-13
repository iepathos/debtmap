//! Known Pure Standard Library Functions for Purity Propagation (Spec 261)
//!
//! This module provides utilities to identify standard library functions
//! that are known to be pure (no side effects), enabling more accurate
//! purity propagation through the call graph.

use crate::analyzers::purity_detector::{is_known_pure_call, is_known_pure_method};

/// Classification of callee purity for propagation
#[derive(Debug, Clone, PartialEq)]
pub enum CalleePurity {
    /// Standard library function, known to be pure
    KnownPure,

    /// Analyzed in this crate, determined to be pure with given confidence
    AnalyzedPure(f64),

    /// Analyzed in this crate, determined to be impure
    AnalyzedImpure,

    /// External function, not in whitelist, unknown purity
    Unknown,
}

/// Evidence of a callee's purity for reporting
#[derive(Debug, Clone)]
pub struct CalleeEvidence {
    pub callee_name: String,
    pub callee_purity: CalleePurity,
}

/// Resolve the purity of a function call based on available information
pub fn resolve_callee_purity(
    callee_name: &str,
    receiver_type: Option<&str>,
    cached_purity: Option<(bool, f64)>, // (is_pure, confidence)
) -> CalleePurity {
    // 1. Check if it's a known pure std function (by full name)
    if is_known_pure_call(callee_name, receiver_type) {
        return CalleePurity::KnownPure;
    }

    // 2. Check if it's a known pure method (by method name alone)
    if is_known_pure_method(callee_name) {
        return CalleePurity::KnownPure;
    }

    // 3. Check cache for already-analyzed functions
    if let Some((is_pure, confidence)) = cached_purity {
        return if is_pure {
            CalleePurity::AnalyzedPure(confidence)
        } else {
            CalleePurity::AnalyzedImpure
        };
    }

    // 4. Unknown external function
    CalleePurity::Unknown
}

/// Compute aggregated purity from multiple callees
pub fn aggregate_callee_purity(callees: &[CalleeEvidence]) -> (bool, f64, Vec<String>) {
    let mut is_pure = true;
    let mut confidence = 1.0_f64;
    let mut impure_reasons = Vec::new();

    for callee in callees {
        match &callee.callee_purity {
            CalleePurity::KnownPure => {
                // Known pure - slight confidence boost
                confidence *= 1.02;
            }
            CalleePurity::AnalyzedPure(callee_conf) => {
                // Propagate confidence from callee
                confidence *= callee_conf;
            }
            CalleePurity::AnalyzedImpure => {
                // Callee is impure - caller is impure
                is_pure = false;
                confidence = 0.95;
                impure_reasons.push(format!("Calls impure function: {}", callee.callee_name));
            }
            CalleePurity::Unknown => {
                // Unknown callee - reduce confidence but don't mark impure
                confidence *= 0.9;
                if confidence < 0.6 {
                    impure_reasons.push(format!("Calls unknown function: {}", callee.callee_name));
                }
            }
        }
    }

    // Clamp confidence
    confidence = confidence.clamp(0.3, 1.0);

    (is_pure, confidence, impure_reasons)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_pure_std_method() {
        assert_eq!(
            resolve_callee_purity("map", Some("Option"), None),
            CalleePurity::KnownPure
        );
        assert_eq!(
            resolve_callee_purity("and_then", Some("Result"), None),
            CalleePurity::KnownPure
        );
        assert_eq!(
            resolve_callee_purity("filter", Some("Iterator"), None),
            CalleePurity::KnownPure
        );
    }

    #[test]
    fn test_known_pure_method_name_only() {
        assert_eq!(
            resolve_callee_purity("map", None, None),
            CalleePurity::KnownPure
        );
        assert_eq!(
            resolve_callee_purity("collect", None, None),
            CalleePurity::KnownPure
        );
        assert_eq!(
            resolve_callee_purity("len", None, None),
            CalleePurity::KnownPure
        );
    }

    #[test]
    fn test_cached_pure_function() {
        assert_eq!(
            resolve_callee_purity("custom_func", None, Some((true, 0.95))),
            CalleePurity::AnalyzedPure(0.95)
        );
    }

    #[test]
    fn test_cached_impure_function() {
        assert_eq!(
            resolve_callee_purity("io_func", None, Some((false, 0.8))),
            CalleePurity::AnalyzedImpure
        );
    }

    #[test]
    fn test_unknown_function() {
        assert_eq!(
            resolve_callee_purity("external_crate::do_something", None, None),
            CalleePurity::Unknown
        );
    }

    #[test]
    fn test_aggregate_all_pure() {
        let callees = vec![
            CalleeEvidence {
                callee_name: "map".to_string(),
                callee_purity: CalleePurity::KnownPure,
            },
            CalleeEvidence {
                callee_name: "filter".to_string(),
                callee_purity: CalleePurity::KnownPure,
            },
            CalleeEvidence {
                callee_name: "collect".to_string(),
                callee_purity: CalleePurity::KnownPure,
            },
        ];

        let (is_pure, confidence, reasons) = aggregate_callee_purity(&callees);
        assert!(is_pure);
        // Confidence clamped to 1.0 max, but boosted for known pure callees
        assert!((confidence - 1.0).abs() < 0.01);
        assert!(reasons.is_empty());
    }

    #[test]
    fn test_aggregate_with_impure() {
        let callees = vec![
            CalleeEvidence {
                callee_name: "map".to_string(),
                callee_purity: CalleePurity::KnownPure,
            },
            CalleeEvidence {
                callee_name: "println".to_string(),
                callee_purity: CalleePurity::AnalyzedImpure,
            },
        ];

        let (is_pure, _confidence, reasons) = aggregate_callee_purity(&callees);
        assert!(!is_pure);
        assert!(reasons.iter().any(|r| r.contains("println")));
    }

    #[test]
    fn test_aggregate_with_unknown() {
        let callees = vec![CalleeEvidence {
            callee_name: "external_func".to_string(),
            callee_purity: CalleePurity::Unknown,
        }];

        let (is_pure, confidence, _reasons) = aggregate_callee_purity(&callees);
        // Unknown doesn't make it impure, just reduces confidence
        assert!(is_pure);
        assert!(confidence < 1.0);
    }
}
