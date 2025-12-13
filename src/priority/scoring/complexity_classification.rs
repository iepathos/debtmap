//! Complexity classification for functions
//!
//! Pure functions for classifying cyclomatic complexity into actionable levels.
//! Following Stillwater philosophy: small, focused, testable units.

/// Complexity level classification based on cyclomatic complexity thresholds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplexityLevel {
    /// Low complexity (1-4): Simple, easily testable
    Low,
    /// Low-moderate complexity (5-6): Minor refactoring beneficial
    LowModerate,
    /// Moderate complexity (7-10): Should be refactored
    Moderate,
    /// High complexity (11+): Needs decomposition
    High,
}

/// Classify cyclomatic complexity into actionable levels
///
/// # Thresholds
/// - 1-4: Low - simple functions, easily testable
/// - 5-6: Low-moderate - minor improvements beneficial
/// - 7-10: Moderate - should consider refactoring
/// - 11+: High - needs decomposition
///
/// # Example
/// ```
/// use debtmap::priority::scoring::complexity_classification::classify_complexity_level;
/// use debtmap::priority::scoring::complexity_classification::ComplexityLevel;
///
/// assert!(matches!(classify_complexity_level(3), ComplexityLevel::Low));
/// assert!(matches!(classify_complexity_level(8), ComplexityLevel::Moderate));
/// assert!(matches!(classify_complexity_level(15), ComplexityLevel::High));
/// ```
pub fn classify_complexity_level(cyclo: u32) -> ComplexityLevel {
    match cyclo {
        1..=4 => ComplexityLevel::Low,
        5..=6 => ComplexityLevel::LowModerate,
        7..=10 => ComplexityLevel::Moderate,
        _ => ComplexityLevel::High,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_low_complexity() {
        assert_eq!(classify_complexity_level(1), ComplexityLevel::Low);
        assert_eq!(classify_complexity_level(2), ComplexityLevel::Low);
        assert_eq!(classify_complexity_level(3), ComplexityLevel::Low);
        assert_eq!(classify_complexity_level(4), ComplexityLevel::Low);
    }

    #[test]
    fn test_classify_low_moderate_complexity() {
        assert_eq!(classify_complexity_level(5), ComplexityLevel::LowModerate);
        assert_eq!(classify_complexity_level(6), ComplexityLevel::LowModerate);
    }

    #[test]
    fn test_classify_moderate_complexity() {
        assert_eq!(classify_complexity_level(7), ComplexityLevel::Moderate);
        assert_eq!(classify_complexity_level(8), ComplexityLevel::Moderate);
        assert_eq!(classify_complexity_level(9), ComplexityLevel::Moderate);
        assert_eq!(classify_complexity_level(10), ComplexityLevel::Moderate);
    }

    #[test]
    fn test_classify_high_complexity() {
        assert_eq!(classify_complexity_level(11), ComplexityLevel::High);
        assert_eq!(classify_complexity_level(15), ComplexityLevel::High);
        assert_eq!(classify_complexity_level(20), ComplexityLevel::High);
        assert_eq!(classify_complexity_level(100), ComplexityLevel::High);
    }

    #[test]
    fn test_boundary_values() {
        // Boundary between Low and LowModerate
        assert_eq!(classify_complexity_level(4), ComplexityLevel::Low);
        assert_eq!(classify_complexity_level(5), ComplexityLevel::LowModerate);

        // Boundary between LowModerate and Moderate
        assert_eq!(classify_complexity_level(6), ComplexityLevel::LowModerate);
        assert_eq!(classify_complexity_level(7), ComplexityLevel::Moderate);

        // Boundary between Moderate and High
        assert_eq!(classify_complexity_level(10), ComplexityLevel::Moderate);
        assert_eq!(classify_complexity_level(11), ComplexityLevel::High);
    }
}
