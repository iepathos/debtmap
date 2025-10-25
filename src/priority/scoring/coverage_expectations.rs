/// Coverage expectations and gap severity analysis for role-based testing priorities.
///
/// This module defines expected test coverage ranges for different function roles
/// and provides functionality to calculate coverage gaps and their severity.

use serde::{Deserialize, Serialize};

/// Represents a range of acceptable coverage percentages.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CoverageRange {
    /// Minimum acceptable coverage percentage (0-100)
    pub min: f64,
    /// Target/ideal coverage percentage (0-100)
    pub target: f64,
    /// Maximum meaningful coverage percentage (0-100)
    pub max: f64,
}

impl CoverageRange {
    /// Creates a new coverage range with validation.
    pub fn new(min: f64, target: f64, max: f64) -> Self {
        assert!(
            (0.0..=100.0).contains(&min),
            "min must be between 0 and 100"
        );
        assert!(
            (0.0..=100.0).contains(&target),
            "target must be between 0 and 100"
        );
        assert!(
            (0.0..=100.0).contains(&max),
            "max must be between 0 and 100"
        );
        assert!(min <= target, "min must be <= target");
        assert!(target <= max, "target must be <= max");

        Self { min, target, max }
    }
}

/// Severity of a coverage gap relative to expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GapSeverity {
    /// Coverage meets or exceeds target (🟢)
    None,
    /// Coverage is between min and target (🟡)
    Minor,
    /// Coverage is below min but above 50% of min (🟠)
    Moderate,
    /// Coverage is critically low, below 50% of min (🔴)
    Critical,
}

impl GapSeverity {
    /// Returns an emoji representation of the severity.
    pub fn emoji(&self) -> &'static str {
        match self {
            GapSeverity::None => "🟢",
            GapSeverity::Minor => "🟡",
            GapSeverity::Moderate => "🟠",
            GapSeverity::Critical => "🔴",
        }
    }
}

/// Represents the gap between actual and expected coverage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CoverageGap {
    /// Actual coverage percentage
    pub actual: f64,
    /// Expected/target coverage percentage
    pub expected: f64,
    /// Absolute gap (expected - actual)
    pub gap: f64,
    /// Severity of the gap
    pub severity: GapSeverity,
}

impl CoverageGap {
    /// Calculates a coverage gap from actual coverage and expected range.
    pub fn calculate(actual: f64, range: &CoverageRange) -> Self {
        let expected = range.target;
        let gap = expected - actual;

        let severity = if actual >= range.target {
            GapSeverity::None
        } else if actual >= range.min {
            GapSeverity::Minor
        } else if actual >= range.min * 0.5 {
            GapSeverity::Moderate
        } else {
            GapSeverity::Critical
        };

        Self {
            actual,
            expected,
            gap,
            severity,
        }
    }
}

/// Role-based coverage expectations following spec 119.
///
/// Defines expected test coverage ranges for each function role based on
/// their characteristics and testing priorities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverageExpectations {
    /// Pure functions: high expectations (90-100%)
    pub pure: CoverageRange,
    /// Business logic: very high expectations (80-95%)
    pub business_logic: CoverageRange,
    /// State management: high expectations (75-90%)
    pub state_management: CoverageRange,
    /// I/O operations: moderate expectations (60-80%)
    pub io_operations: CoverageRange,
    /// Validation: very high expectations (85-98%)
    pub validation: CoverageRange,
    /// Error handling: high expectations (70-90%)
    pub error_handling: CoverageRange,
    /// Configuration: moderate expectations (60-80%)
    pub configuration: CoverageRange,
    /// Initialization: moderate expectations (50-75%)
    pub initialization: CoverageRange,
    /// Orchestration: moderate-high expectations (65-85%)
    pub orchestration: CoverageRange,
    /// Utilities: high expectations (75-95%)
    pub utilities: CoverageRange,
    /// Debug/Development: low expectations (20-40%)
    pub debug: CoverageRange,
    /// Performance optimization: low-moderate expectations (40-60%)
    pub performance: CoverageRange,
}

impl Default for CoverageExpectations {
    fn default() -> Self {
        Self {
            pure: CoverageRange::new(90.0, 95.0, 100.0),
            business_logic: CoverageRange::new(80.0, 90.0, 95.0),
            state_management: CoverageRange::new(75.0, 85.0, 90.0),
            io_operations: CoverageRange::new(60.0, 70.0, 80.0),
            validation: CoverageRange::new(85.0, 92.0, 98.0),
            error_handling: CoverageRange::new(70.0, 80.0, 90.0),
            configuration: CoverageRange::new(60.0, 70.0, 80.0),
            initialization: CoverageRange::new(50.0, 65.0, 75.0),
            orchestration: CoverageRange::new(65.0, 75.0, 85.0),
            utilities: CoverageRange::new(75.0, 85.0, 95.0),
            debug: CoverageRange::new(20.0, 30.0, 40.0),
            performance: CoverageRange::new(40.0, 50.0, 60.0),
        }
    }
}

impl CoverageExpectations {
    /// Gets the coverage range for a specific role.
    pub fn for_role(&self, role: &str) -> &CoverageRange {
        match role {
            "Pure" => &self.pure,
            "BusinessLogic" => &self.business_logic,
            "StateManagement" => &self.state_management,
            "IoOperations" => &self.io_operations,
            "Validation" => &self.validation,
            "ErrorHandling" => &self.error_handling,
            "Configuration" => &self.configuration,
            "Initialization" => &self.initialization,
            "Orchestration" => &self.orchestration,
            "Utilities" => &self.utilities,
            "Debug" => &self.debug,
            "Performance" => &self.performance,
            _ => &self.business_logic, // Default fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_range_validation() {
        let range = CoverageRange::new(80.0, 90.0, 100.0);
        assert_eq!(range.min, 80.0);
        assert_eq!(range.target, 90.0);
        assert_eq!(range.max, 100.0);
    }

    #[test]
    #[should_panic(expected = "min must be <= target")]
    fn test_coverage_range_invalid_order() {
        CoverageRange::new(90.0, 80.0, 100.0);
    }

    #[test]
    fn test_gap_severity_none() {
        let range = CoverageRange::new(80.0, 90.0, 100.0);
        let gap = CoverageGap::calculate(95.0, &range);
        assert_eq!(gap.severity, GapSeverity::None);
        assert_eq!(gap.severity.emoji(), "🟢");
    }

    #[test]
    fn test_gap_severity_minor() {
        let range = CoverageRange::new(80.0, 90.0, 100.0);
        let gap = CoverageGap::calculate(85.0, &range);
        assert_eq!(gap.severity, GapSeverity::Minor);
        assert_eq!(gap.gap, 5.0);
        assert_eq!(gap.severity.emoji(), "🟡");
    }

    #[test]
    fn test_gap_severity_moderate() {
        let range = CoverageRange::new(80.0, 90.0, 100.0);
        let gap = CoverageGap::calculate(50.0, &range); // Between 40 (50% of min) and 80 (min)
        assert_eq!(gap.severity, GapSeverity::Moderate);
        assert_eq!(gap.severity.emoji(), "🟠");
    }

    #[test]
    fn test_gap_severity_critical() {
        let range = CoverageRange::new(80.0, 90.0, 100.0);
        let gap = CoverageGap::calculate(30.0, &range); // Below 40 (50% of min)
        assert_eq!(gap.severity, GapSeverity::Critical);
        assert_eq!(gap.severity.emoji(), "🔴");
    }

    #[test]
    fn test_default_expectations() {
        let expectations = CoverageExpectations::default();

        // Pure functions should have highest expectations
        assert_eq!(expectations.pure.target, 95.0);

        // Debug should have lowest expectations
        assert_eq!(expectations.debug.target, 30.0);

        // Business logic should be high
        assert_eq!(expectations.business_logic.target, 90.0);
    }

    #[test]
    fn test_for_role() {
        let expectations = CoverageExpectations::default();

        assert_eq!(
            expectations.for_role("Pure").target,
            expectations.pure.target
        );
        assert_eq!(
            expectations.for_role("Debug").target,
            expectations.debug.target
        );
        assert_eq!(
            expectations.for_role("Validation").target,
            expectations.validation.target
        );

        // Unknown role should fall back to business logic
        assert_eq!(
            expectations.for_role("Unknown").target,
            expectations.business_logic.target
        );
    }
}
