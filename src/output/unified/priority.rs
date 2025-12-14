//! Priority classification for debt items (spec 108)
//!
//! Provides the `Priority` enum that classifies debt items based on their
//! score thresholds:
//! - Critical: score >= 100
//! - High: score >= 50
//! - Medium: score >= 20
//! - Low: score < 20

use serde::{Deserialize, Serialize};

/// Priority level based on score
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Critical, // >= 100
    High,     // >= 50
    Medium,   // >= 20
    Low,      // < 20
}

impl Priority {
    pub fn from_score(score: f64) -> Self {
        if score >= 100.0 {
            Priority::Critical
        } else if score >= 50.0 {
            Priority::High
        } else if score >= 20.0 {
            Priority::Medium
        } else {
            Priority::Low
        }
    }
}

/// Assert priority matches score thresholds
#[inline]
pub fn assert_priority_invariants(priority: &Priority, score: f64) {
    let expected = Priority::from_score(score);
    debug_assert!(
        std::mem::discriminant(priority) == std::mem::discriminant(&expected),
        "Priority {:?} doesn't match score {} (expected {:?})",
        priority,
        score,
        expected
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_score() {
        assert!(matches!(Priority::from_score(150.0), Priority::Critical));
        assert!(matches!(Priority::from_score(75.0), Priority::High));
        assert!(matches!(Priority::from_score(35.0), Priority::Medium));
        assert!(matches!(Priority::from_score(10.0), Priority::Low));
    }

    #[test]
    fn test_priority_from_score_thresholds() {
        // Verify exact threshold behavior
        assert!(matches!(Priority::from_score(100.0), Priority::Critical));
        assert!(matches!(Priority::from_score(99.99), Priority::High));
        assert!(matches!(Priority::from_score(50.0), Priority::High));
        assert!(matches!(Priority::from_score(49.99), Priority::Medium));
        assert!(matches!(Priority::from_score(20.0), Priority::Medium));
        assert!(matches!(Priority::from_score(19.99), Priority::Low));
        assert!(matches!(Priority::from_score(0.0), Priority::Low));
    }
}
