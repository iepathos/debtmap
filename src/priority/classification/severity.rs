use colored::Color;

/// Severity classification for technical debt items.
///
/// Classifies debt scores into four levels based on priority thresholds:
/// - **Critical** (≥8.0): Immediate action required
/// - **High** (≥6.0): High priority, address soon
/// - **Medium** (≥4.0): Moderate priority
/// - **Low** (<4.0): Low priority
///
/// # Examples
///
/// ```
/// use debtmap::priority::classification::Severity;
///
/// let sev = Severity::from_score(8.5);
/// assert_eq!(sev, Severity::Critical);
/// assert_eq!(sev.as_str(), "CRITICAL");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    /// Pure function: score → severity
    ///
    /// Classifies a debt score into a severity level based on these thresholds:
    /// - score >= 8.0: Critical
    /// - score >= 6.0: High
    /// - score >= 4.0: Medium
    /// - score < 4.0: Low
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::priority::classification::Severity;
    ///
    /// assert_eq!(Severity::from_score(10.0), Severity::Critical);
    /// assert_eq!(Severity::from_score(7.5), Severity::High);
    /// assert_eq!(Severity::from_score(5.0), Severity::Medium);
    /// assert_eq!(Severity::from_score(2.0), Severity::Low);
    /// ```
    #[inline]
    pub fn from_score(score: f64) -> Self {
        if score >= 8.0 {
            Self::Critical
        } else if score >= 6.0 {
            Self::High
        } else if score >= 4.0 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    /// Returns the static string label for this severity level.
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::priority::classification::Severity;
    ///
    /// assert_eq!(Severity::Critical.as_str(), "CRITICAL");
    /// assert_eq!(Severity::High.as_str(), "HIGH");
    /// assert_eq!(Severity::Medium.as_str(), "MEDIUM");
    /// assert_eq!(Severity::Low.as_str(), "LOW");
    /// ```
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }

    /// Returns the terminal color for this severity level.
    ///
    /// # Examples
    ///
    /// ```
    /// use debtmap::priority::classification::Severity;
    /// use colored::Color;
    ///
    /// assert_eq!(Severity::Critical.color(), Color::Red);
    /// assert_eq!(Severity::High.color(), Color::Yellow);
    /// assert_eq!(Severity::Medium.color(), Color::Blue);
    /// assert_eq!(Severity::Low.color(), Color::Green);
    /// ```
    #[inline]
    pub const fn color(self) -> Color {
        match self {
            Self::Critical => Color::Red,
            Self::High => Color::Yellow,
            Self::Medium => Color::Blue,
            Self::Low => Color::Green,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_thresholds() {
        assert_eq!(Severity::from_score(10.0), Severity::Critical);
        assert_eq!(Severity::from_score(8.0), Severity::Critical);
        assert_eq!(Severity::from_score(7.9), Severity::High);
        assert_eq!(Severity::from_score(6.0), Severity::High);
        assert_eq!(Severity::from_score(5.9), Severity::Medium);
        assert_eq!(Severity::from_score(4.0), Severity::Medium);
        assert_eq!(Severity::from_score(3.9), Severity::Low);
        assert_eq!(Severity::from_score(0.0), Severity::Low);
    }

    #[test]
    fn severity_labels() {
        assert_eq!(Severity::Critical.as_str(), "CRITICAL");
        assert_eq!(Severity::High.as_str(), "HIGH");
        assert_eq!(Severity::Medium.as_str(), "MEDIUM");
        assert_eq!(Severity::Low.as_str(), "LOW");
    }

    #[test]
    fn severity_colors() {
        assert_eq!(Severity::Critical.color(), Color::Red);
        assert_eq!(Severity::High.color(), Color::Yellow);
        assert_eq!(Severity::Medium.color(), Color::Blue);
        assert_eq!(Severity::Low.color(), Color::Green);
    }

    #[test]
    fn severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }

    #[test]
    fn severity_is_monotonic() {
        // Test that higher scores produce same or higher severity
        let test_cases = [(0.0, 3.9), (4.0, 5.9), (6.0, 7.9), (8.0, 10.0)];

        for (lower, higher) in test_cases {
            let sev_lower = Severity::from_score(lower);
            let sev_higher = Severity::from_score(higher);
            assert!(
                sev_higher >= sev_lower,
                "Higher score ({}) should have same or higher severity than lower score ({})",
                higher,
                lower
            );
        }
    }
}
