/// Coverage level classification for test coverage percentages.
///
/// Classifies test coverage percentages into six levels:
/// - **Untested** (0.0%): No test coverage
/// - **Low** (<20%): Minimal coverage
/// - **Partial** (<50%): Some coverage
/// - **Moderate** (<80%): Decent coverage
/// - **Good** (<95%): Strong coverage
/// - **Excellent** (≥95%): Nearly complete coverage
///
/// # Examples
///
/// ```
/// use debtmap::priority::classification::CoverageLevel;
///
/// let level = CoverageLevel::from_percentage(85.0);
/// assert_eq!(level, CoverageLevel::Good);
/// assert_eq!(level.status_tag(), "[OK GOOD]");
/// assert_eq!(level.description(), "Good coverage");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoverageLevel {
    Untested,
    Low,
    Partial,
    Moderate,
    Good,
    Excellent,
}

impl CoverageLevel {
    /// Pure function: percentage → level
    ///
    /// Classifies a coverage percentage into a level based on these thresholds:
    /// - 0.0%: Untested
    /// - <20%: Low
    /// - <50%: Partial
    /// - <80%: Moderate
    /// - <95%: Good
    /// - ≥95%: Excellent
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::priority::classification::CoverageLevel;
    ///
    /// assert_eq!(CoverageLevel::from_percentage(0.0), CoverageLevel::Untested);
    /// assert_eq!(CoverageLevel::from_percentage(10.0), CoverageLevel::Low);
    /// assert_eq!(CoverageLevel::from_percentage(35.0), CoverageLevel::Partial);
    /// assert_eq!(CoverageLevel::from_percentage(65.0), CoverageLevel::Moderate);
    /// assert_eq!(CoverageLevel::from_percentage(88.0), CoverageLevel::Good);
    /// assert_eq!(CoverageLevel::from_percentage(100.0), CoverageLevel::Excellent);
    /// ```
    #[inline]
    pub fn from_percentage(pct: f64) -> Self {
        if pct == 0.0 {
            Self::Untested
        } else if pct < 20.0 {
            Self::Low
        } else if pct < 50.0 {
            Self::Partial
        } else if pct < 80.0 {
            Self::Moderate
        } else if pct < 95.0 {
            Self::Good
        } else {
            Self::Excellent
        }
    }

    /// Returns a status tag suitable for terminal display.
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::priority::classification::CoverageLevel;
    ///
    /// assert_eq!(CoverageLevel::Untested.status_tag(), "[UNTESTED]");
    /// assert_eq!(CoverageLevel::Low.status_tag(), "[WARN LOW]");
    /// assert_eq!(CoverageLevel::Partial.status_tag(), "[WARN PARTIAL]");
    /// assert_eq!(CoverageLevel::Moderate.status_tag(), "[INFO MODERATE]");
    /// assert_eq!(CoverageLevel::Good.status_tag(), "[OK GOOD]");
    /// assert_eq!(CoverageLevel::Excellent.status_tag(), "[OK EXCELLENT]");
    /// ```
    #[inline]
    pub const fn status_tag(self) -> &'static str {
        match self {
            Self::Untested => "[UNTESTED]",
            Self::Low => "[WARN LOW]",
            Self::Partial => "[WARN PARTIAL]",
            Self::Moderate => "[INFO MODERATE]",
            Self::Good => "[OK GOOD]",
            Self::Excellent => "[OK EXCELLENT]",
        }
    }

    /// Returns a human-readable description of the coverage level.
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::priority::classification::CoverageLevel;
    ///
    /// assert_eq!(CoverageLevel::Untested.description(), "No test coverage");
    /// assert_eq!(CoverageLevel::Low.description(), "Low coverage");
    /// assert_eq!(CoverageLevel::Good.description(), "Good coverage");
    /// ```
    #[inline]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Untested => "No test coverage",
            Self::Low => "Low coverage",
            Self::Partial => "Partial coverage",
            Self::Moderate => "Moderate coverage",
            Self::Good => "Good coverage",
            Self::Excellent => "Excellent coverage",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_thresholds() {
        assert_eq!(CoverageLevel::from_percentage(0.0), CoverageLevel::Untested);
        assert_eq!(CoverageLevel::from_percentage(10.0), CoverageLevel::Low);
        assert_eq!(CoverageLevel::from_percentage(30.0), CoverageLevel::Partial);
        assert_eq!(
            CoverageLevel::from_percentage(60.0),
            CoverageLevel::Moderate
        );
        assert_eq!(CoverageLevel::from_percentage(85.0), CoverageLevel::Good);
        assert_eq!(
            CoverageLevel::from_percentage(100.0),
            CoverageLevel::Excellent
        );
    }

    #[test]
    fn coverage_boundary_cases() {
        assert_eq!(CoverageLevel::from_percentage(0.0), CoverageLevel::Untested);
        assert_eq!(
            CoverageLevel::from_percentage(0.1),
            CoverageLevel::Low,
            "0.1% should be Low, not Untested"
        );
        assert_eq!(
            CoverageLevel::from_percentage(19.9),
            CoverageLevel::Low,
            "19.9% should be Low"
        );
        assert_eq!(
            CoverageLevel::from_percentage(20.0),
            CoverageLevel::Partial,
            "20.0% should be Partial"
        );
        assert_eq!(
            CoverageLevel::from_percentage(49.9),
            CoverageLevel::Partial,
            "49.9% should be Partial"
        );
        assert_eq!(
            CoverageLevel::from_percentage(50.0),
            CoverageLevel::Moderate,
            "50.0% should be Moderate"
        );
        assert_eq!(
            CoverageLevel::from_percentage(79.9),
            CoverageLevel::Moderate,
            "79.9% should be Moderate"
        );
        assert_eq!(
            CoverageLevel::from_percentage(80.0),
            CoverageLevel::Good,
            "80.0% should be Good"
        );
        assert_eq!(
            CoverageLevel::from_percentage(94.9),
            CoverageLevel::Good,
            "94.9% should be Good"
        );
        assert_eq!(
            CoverageLevel::from_percentage(95.0),
            CoverageLevel::Excellent,
            "95.0% should be Excellent"
        );
    }

    #[test]
    fn coverage_status_tags() {
        assert_eq!(CoverageLevel::Untested.status_tag(), "[UNTESTED]");
        assert_eq!(CoverageLevel::Low.status_tag(), "[WARN LOW]");
        assert_eq!(CoverageLevel::Partial.status_tag(), "[WARN PARTIAL]");
        assert_eq!(CoverageLevel::Moderate.status_tag(), "[INFO MODERATE]");
        assert_eq!(CoverageLevel::Good.status_tag(), "[OK GOOD]");
        assert_eq!(CoverageLevel::Excellent.status_tag(), "[OK EXCELLENT]");
    }

    #[test]
    fn coverage_descriptions() {
        assert_eq!(CoverageLevel::Untested.description(), "No test coverage");
        assert_eq!(CoverageLevel::Low.description(), "Low coverage");
        assert_eq!(CoverageLevel::Partial.description(), "Partial coverage");
        assert_eq!(CoverageLevel::Moderate.description(), "Moderate coverage");
        assert_eq!(CoverageLevel::Good.description(), "Good coverage");
        assert_eq!(CoverageLevel::Excellent.description(), "Excellent coverage");
    }

    #[test]
    fn coverage_ordering() {
        assert!(CoverageLevel::Excellent > CoverageLevel::Good);
        assert!(CoverageLevel::Good > CoverageLevel::Moderate);
        assert!(CoverageLevel::Moderate > CoverageLevel::Partial);
        assert!(CoverageLevel::Partial > CoverageLevel::Low);
        assert!(CoverageLevel::Low > CoverageLevel::Untested);
    }

    #[test]
    fn coverage_is_monotonic() {
        // Test that higher percentages produce same or higher coverage levels
        let test_cases = [
            (0.0, 0.0),
            (0.1, 19.9),
            (20.0, 49.9),
            (50.0, 79.9),
            (80.0, 94.9),
            (95.0, 100.0),
        ];

        for (lower, higher) in test_cases {
            let level_lower = CoverageLevel::from_percentage(lower);
            let level_higher = CoverageLevel::from_percentage(higher);
            assert!(
                level_higher >= level_lower,
                "Higher percentage ({}%) should have same or higher level than lower percentage ({}%)",
                higher,
                lower
            );
        }
    }
}
