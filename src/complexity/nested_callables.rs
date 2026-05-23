//! Nested callable complexity summaries and scoring rollups.
//!
//! Parent function metrics exclude nested function/lambda bodies to avoid double
//! counting. Summaries are stored as `detected_patterns` and rolled into effective
//! complexity for hotspot detection.

use serde::{Deserialize, Serialize};

/// Aggregated complexity of nested functions/callbacks inside a parent body.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestedCallableSummary {
    pub count: u32,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub max_nesting: u32,
}

impl NestedCallableSummary {
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Encode a nested summary as `detected_patterns` entries.
pub fn nested_callable_patterns(summary: &NestedCallableSummary) -> Vec<String> {
    if summary.is_empty() {
        return Vec::new();
    }

    vec![
        format!("nested-functions:count={}", summary.count),
        format!("nested-functions:cyclomatic={}", summary.cyclomatic),
        format!("nested-functions:cognitive={}", summary.cognitive),
        format!("nested-functions:max-nesting={}", summary.max_nesting),
    ]
}

/// Parse nested summary from `detected_patterns` (returns default if absent).
pub fn parse_nested_callable_patterns(patterns: Option<&[String]>) -> NestedCallableSummary {
    let Some(patterns) = patterns else {
        return NestedCallableSummary::default();
    };

    let mut summary = NestedCallableSummary::default();
    for pattern in patterns {
        if let Some(value) = pattern.strip_prefix("nested-functions:count=") {
            summary.count = value.parse().unwrap_or(0);
        } else if let Some(value) = pattern.strip_prefix("nested-functions:cyclomatic=") {
            summary.cyclomatic = value.parse().unwrap_or(0);
        } else if let Some(value) = pattern.strip_prefix("nested-functions:cognitive=") {
            summary.cognitive = value.parse().unwrap_or(0);
        } else if let Some(value) = pattern.strip_prefix("nested-functions:max-nesting=") {
            summary.max_nesting = value.parse().unwrap_or(0);
        }
    }
    summary
}

/// Parent metrics plus nested bodies for threshold and hotspot decisions.
pub fn scoring_complexity(
    cyclomatic: u32,
    cognitive: u32,
    patterns: Option<&[String]>,
) -> (u32, u32) {
    let nested = parse_nested_callable_patterns(patterns);
    (
        cyclomatic.saturating_add(nested.cyclomatic),
        cognitive.saturating_add(nested.cognitive),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_nested_patterns() {
        let summary = NestedCallableSummary {
            count: 2,
            cyclomatic: 4,
            cognitive: 2,
            max_nesting: 1,
        };
        let patterns = nested_callable_patterns(&summary);
        let parsed = parse_nested_callable_patterns(Some(&patterns));
        assert_eq!(parsed, summary);
    }

    #[test]
    fn scoring_complexity_includes_nested_totals() {
        let patterns = nested_callable_patterns(&NestedCallableSummary {
            count: 1,
            cyclomatic: 3,
            cognitive: 5,
            max_nesting: 2,
        });
        let (cyc, cog) = scoring_complexity(2, 1, Some(&patterns));
        assert_eq!(cyc, 5);
        assert_eq!(cog, 6);
    }
}
