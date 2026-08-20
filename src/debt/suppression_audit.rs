//! Lossless audit records for suppression decisions applied to unified findings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuppressionKind {
    SameLine,
    NextLine,
    Function,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuppressionDecision {
    pub kind: SuppressionKind,
    pub directive_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedSuppression {
    pub file: PathBuf,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    pub debt_type: String,
    pub decision: SuppressionDecision,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuppressionAudit {
    pub applied: Vec<AppliedSuppression>,
}

impl SuppressionAudit {
    pub fn normalized(mut self) -> Self {
        self.applied
            .sort_by(|left, right| suppression_key(left).cmp(&suppression_key(right)));
        self
    }

    pub fn is_empty(&self) -> bool {
        self.applied.is_empty()
    }

    pub fn merged(self, other: Self) -> Self {
        Self {
            applied: self.applied.into_iter().chain(other.applied).collect(),
        }
        .normalized()
    }
}

fn suppression_key(
    record: &AppliedSuppression,
) -> (
    &PathBuf,
    usize,
    &Option<String>,
    &str,
    usize,
    SuppressionKind,
) {
    (
        &record.file,
        record.line,
        &record.function,
        &record.debt_type,
        record.decision.directive_line,
        record.decision.kind,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuppressionOutcome<T> {
    pub emitted: Vec<T>,
    pub audit: SuppressionAudit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_normalization_is_independent_of_input_order() {
        let record = |file: &str, line| AppliedSuppression {
            file: file.into(),
            line,
            function: None,
            debt_type: "Complexity".into(),
            decision: SuppressionDecision {
                kind: SuppressionKind::SameLine,
                directive_line: line,
                reason: None,
            },
        };
        let forward = SuppressionAudit {
            applied: vec![record("b.rs", 2), record("a.rs", 1)],
        }
        .normalized();
        let reverse = SuppressionAudit {
            applied: vec![record("a.rs", 1), record("b.rs", 2)],
        }
        .normalized();

        assert_eq!(forward, reverse);
    }
}
