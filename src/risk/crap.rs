//! CRAP (Change Risk Anti-Patterns) metric.
//!
//! `CRAP(m) = cc² · (1 − cov)³ + cc`, where `cc` is cyclomatic complexity and
//! `cov` is test coverage in `[0, 1]`. A fully covered function scores `cc`;
//! an untested one scores `cc² + cc`. Scores above 30 are conventionally
//! considered risky to change.
//!
//! # Examples
//!
//! ```
//! use debtmap::risk::crap::crap_score;
//!
//! assert_eq!(crap_score(5, 1.0), 5.0);
//! assert_eq!(crap_score(5, 0.0), 30.0);
//! ```

use crate::priority::UnifiedDebtItem;
use serde::{Deserialize, Serialize};

/// Which coverage measurement feeds coverage-based metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageBasis {
    /// Lines of this function executed by tests (LCOV).
    #[default]
    Direct,
    /// Direct coverage, falling back to the share of well-covered callees.
    Transitive,
}

/// `[crap]` configuration section.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrapConfig {
    #[serde(default)]
    pub coverage_source: CoverageBasis,
}

/// CRAP score for a function with the given complexity and coverage ratio.
pub fn crap_score(cyclomatic: u32, coverage: f64) -> f64 {
    let cc = cyclomatic as f64;
    let uncovered = 1.0 - coverage.clamp(0.0, 1.0);
    cc * cc * uncovered.powi(3) + cc
}

/// Coverage ratio of an item, or `None` when no coverage data was provided.
pub fn item_coverage(item: &UnifiedDebtItem, basis: CoverageBasis) -> Option<f64> {
    item.transitive_coverage.as_ref().map(|c| match basis {
        CoverageBasis::Direct => c.direct,
        CoverageBasis::Transitive => c.transitive,
    })
}

/// CRAP score of an item, or `None` when no coverage data was provided.
pub fn crap_for_item(item: &UnifiedDebtItem, basis: CoverageBasis) -> Option<f64> {
    item_coverage(item, basis).map(|cov| crap_score(item.cyclomatic_complexity, cov))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn untested_function_scores_cc_squared_plus_cc() {
        assert_eq!(crap_score(10, 0.0), 110.0);
    }

    #[test]
    fn fully_covered_function_scores_cc() {
        assert_eq!(crap_score(10, 1.0), 10.0);
    }

    #[test]
    fn half_covered_function() {
        assert_eq!(crap_score(4, 0.5), 16.0 * 0.125 + 4.0);
    }

    #[test]
    fn out_of_range_coverage_is_clamped() {
        assert_eq!(crap_score(3, 1.5), 3.0);
        assert_eq!(crap_score(3, -0.5), 12.0);
    }

    #[test]
    fn coverage_basis_deserializes_snake_case() {
        let config: CrapConfig = toml::from_str("coverage_source = \"transitive\"").unwrap();
        assert_eq!(config.coverage_source, CoverageBasis::Transitive);
        assert_eq!(CrapConfig::default().coverage_source, CoverageBasis::Direct);
    }

    proptest! {
        #[test]
        fn crap_is_bounded_by_cc_and_cc_squared_plus_cc(cc in 0u32..200, cov in 0.0f64..=1.0) {
            let score = crap_score(cc, cov);
            let c = cc as f64;
            prop_assert!(score >= c - 1e-9);
            prop_assert!(score <= c * c + c + 1e-9);
        }

        #[test]
        fn more_coverage_never_raises_crap(cc in 0u32..200, a in 0.0f64..=1.0, b in 0.0f64..=1.0) {
            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
            prop_assert!(crap_score(cc, hi) <= crap_score(cc, lo) + 1e-9);
        }
    }
}
