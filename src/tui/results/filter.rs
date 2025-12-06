//! Filter functionality for results.

use crate::priority::UnifiedDebtItem;

/// Filter for debt items
#[derive(Debug, Clone)]
pub enum Filter {
    /// Filter by severity level
    Severity(SeverityFilter),
    /// Filter by coverage
    Coverage(CoverageFilter),
    /// Filter by complexity threshold
    Complexity(u32),
}

impl Filter {
    /// Check if an item matches this filter
    pub fn matches(&self, item: &UnifiedDebtItem) -> bool {
        match self {
            Filter::Severity(sev) => sev.matches(item),
            Filter::Coverage(cov) => cov.matches(item),
            Filter::Complexity(threshold) => item.cyclomatic_complexity >= *threshold,
        }
    }

    /// Get display name for filter
    pub fn display_name(&self) -> String {
        match self {
            Filter::Severity(sev) => format!("Severity: {}", sev.display_name()),
            Filter::Coverage(cov) => format!("Coverage: {}", cov.display_name()),
            Filter::Complexity(threshold) => format!("Complexity >= {}", threshold),
        }
    }
}

/// Severity filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeverityFilter {
    Critical,
    High,
    Medium,
    Low,
}

impl SeverityFilter {
    /// Check if item matches severity filter
    pub fn matches(&self, item: &UnifiedDebtItem) -> bool {
        let item_severity = calculate_severity(item.unified_score.final_score);
        match self {
            SeverityFilter::Critical => item_severity == "critical",
            SeverityFilter::High => item_severity == "high",
            SeverityFilter::Medium => item_severity == "medium",
            SeverityFilter::Low => item_severity == "low",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            SeverityFilter::Critical => "Critical",
            SeverityFilter::High => "High",
            SeverityFilter::Medium => "Medium",
            SeverityFilter::Low => "Low",
        }
    }

    /// Get all severity filters
    pub fn all() -> &'static [SeverityFilter] {
        &[
            SeverityFilter::Critical,
            SeverityFilter::High,
            SeverityFilter::Medium,
            SeverityFilter::Low,
        ]
    }
}

/// Coverage filter
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoverageFilter {
    /// No coverage data
    None,
    /// Low coverage (0-30%)
    Low,
    /// Medium coverage (30-70%)
    Medium,
    /// High coverage (70-100%)
    High,
}

impl CoverageFilter {
    /// Check if item matches coverage filter
    pub fn matches(&self, item: &UnifiedDebtItem) -> bool {
        let coverage = item.transitive_coverage.as_ref().map(|c| c.direct);
        match (self, coverage) {
            (CoverageFilter::None, None) => true,
            (CoverageFilter::None, Some(_)) => false,
            (CoverageFilter::Low, Some(cov)) => cov < 30.0,
            (CoverageFilter::Medium, Some(cov)) => (30.0..70.0).contains(&cov),
            (CoverageFilter::High, Some(cov)) => cov >= 70.0,
            _ => false,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            CoverageFilter::None => "No Coverage",
            CoverageFilter::Low => "Low (0-30%)",
            CoverageFilter::Medium => "Medium (30-70%)",
            CoverageFilter::High => "High (70-100%)",
        }
    }

    /// Get all coverage filters
    pub fn all() -> &'static [CoverageFilter] {
        &[
            CoverageFilter::None,
            CoverageFilter::Low,
            CoverageFilter::Medium,
            CoverageFilter::High,
        ]
    }
}

/// Calculate severity level from score
fn calculate_severity(score: f64) -> &'static str {
    if score >= 100.0 {
        "critical"
    } else if score >= 50.0 {
        "high"
    } else if score >= 10.0 {
        "medium"
    } else {
        "low"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_display() {
        assert_eq!(SeverityFilter::Critical.display_name(), "Critical");
        assert_eq!(SeverityFilter::High.display_name(), "High");
        assert_eq!(SeverityFilter::Medium.display_name(), "Medium");
        assert_eq!(SeverityFilter::Low.display_name(), "Low");
    }

    #[test]
    fn test_coverage_display() {
        assert_eq!(CoverageFilter::None.display_name(), "No Coverage");
        assert_eq!(CoverageFilter::Low.display_name(), "Low (0-30%)");
        assert_eq!(CoverageFilter::Medium.display_name(), "Medium (30-70%)");
        assert_eq!(CoverageFilter::High.display_name(), "High (70-100%)");
    }

    #[test]
    fn test_calculate_severity() {
        assert_eq!(calculate_severity(150.0), "critical");
        assert_eq!(calculate_severity(75.0), "high");
        assert_eq!(calculate_severity(25.0), "medium");
        assert_eq!(calculate_severity(5.0), "low");
    }

    #[test]
    fn test_all_filters() {
        assert_eq!(SeverityFilter::all().len(), 4);
        assert_eq!(CoverageFilter::all().len(), 4);
    }
}
