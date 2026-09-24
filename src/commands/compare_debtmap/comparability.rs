//! Shared evidence requirements for score comparisons.

use super::types::{Comparability, ComparabilityStatus, DebtmapJsonInput};
use crate::output::unified::{AnalysisReceipt, ScopeStatus};

/// Known differences override uncertainty, but neither permits improvement claims.
pub(crate) fn assess_comparability(
    before: &DebtmapJsonInput,
    after: &DebtmapJsonInput,
) -> Comparability {
    let mut incompatible = version_difference(before, after);
    let mut unknown = missing_version_reasons(before, after);
    match (&before.receipt, &after.receipt) {
        (Some(before), Some(after)) => {
            incompatible.extend(incompatible_receipt_reasons(before, after));
            unknown.extend(incomplete_scope_reasons(before, after));
        }
        _ => unknown.push("One or both reports do not contain an analysis receipt".to_string()),
    }
    let status = comparability_status(&incompatible, &unknown);
    incompatible.extend(unknown);
    Comparability {
        status,
        reasons: incompatible,
    }
}

fn comparability_status(incompatible: &[String], unknown: &[String]) -> ComparabilityStatus {
    match (incompatible.is_empty(), unknown.is_empty()) {
        (false, _) => ComparabilityStatus::Incompatible,
        (true, false) => ComparabilityStatus::Unknown,
        (true, true) => ComparabilityStatus::Comparable,
    }
}

fn known_version(input: &DebtmapJsonInput) -> Option<&str> {
    input
        .analyzer_version
        .as_deref()
        .filter(|version| !version.trim().is_empty())
}

fn version_difference(before: &DebtmapJsonInput, after: &DebtmapJsonInput) -> Vec<String> {
    match (known_version(before), known_version(after)) {
        (Some(before), Some(after)) if before != after => {
            vec![format!(
                "Analyzer versions differ ({before} versus {after})"
            )]
        }
        _ => Vec::new(),
    }
}

fn missing_version_reasons(before: &DebtmapJsonInput, after: &DebtmapJsonInput) -> Vec<String> {
    (known_version(before).is_none() || known_version(after).is_none())
        .then(|| "One or both reports do not identify the analyzer version".to_string())
        .into_iter()
        .collect()
}

fn incompatible_receipt_reasons(before: &AnalysisReceipt, after: &AnalysisReceipt) -> Vec<String> {
    [
        (
            before.policy != after.policy || before.policy_fingerprint != after.policy_fingerprint,
            "Analysis policies differ",
        ),
        (
            before.evidence != after.evidence,
            "Loaded or requested evidence differs",
        ),
        (
            before.selection != after.selection,
            "Output selection policies differ",
        ),
        (
            before.analysis_target != after.analysis_target,
            "Analysis targets differ",
        ),
        (
            before.execution.multi_pass != after.execution.multi_pass,
            "Multi-pass analysis settings differ",
        ),
    ]
    .into_iter()
    .filter(|(differs, _)| *differs)
    .map(|(_, reason)| reason.to_string())
    .collect()
}

fn incomplete_scope_reasons(before: &AnalysisReceipt, after: &AnalysisReceipt) -> Vec<String> {
    (before.scope.status != ScopeStatus::Complete || after.scope.status != ScopeStatus::Complete)
        .then(|| "One or both reports have incomplete or unknown scope".to_string())
        .into_iter()
        .collect()
}
