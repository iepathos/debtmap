//! Pure data structures for formatted priority output.
//!
//! This module defines immutable data structures that represent formatted
//! technical debt items. These structures separate formatting logic from I/O,
//! following the "Pure Core, Imperative Shell" pattern.
//!
//! # Architecture
//!
//! - **Pure Core**: `format_priority_item()` transforms data → structured output
//! - **Imperative Shell**: Writer layer renders structured output → terminal/file
//!
//! # Examples
//!
//! ```
//! use debtmap::priority::formatted_output::FormattedPriorityItem;
//! use debtmap::priority::classification::Severity;
//!
//! let formatted = FormattedPriorityItem {
//!     rank: 1,
//!     score: 8.5,
//!     severity: Severity::Critical,
//!     sections: vec![],
//! };
//!
//! assert_eq!(formatted.rank, 1);
//! assert_eq!(formatted.severity, Severity::Critical);
//! ```

use crate::priority::classification::{CoverageLevel, Severity};
use colored::Color;
use std::path::PathBuf;

/// Pure data structure representing a formatted priority item.
///
/// Contains all data needed to render a technical debt item to any output format.
/// This is a pure data structure with no I/O operations.
#[derive(Debug, Clone, PartialEq)]
pub struct FormattedPriorityItem {
    pub rank: usize,
    pub score: f64,
    pub severity: Severity,
    pub sections: Vec<FormattedSection>,
}

/// Represents a single section of formatted output.
///
/// Each variant contains all data needed to render that section type.
#[derive(Debug, Clone, PartialEq)]
pub enum FormattedSection {
    /// Header line with rank, score, coverage, and severity
    Header {
        rank: usize,
        score: f64,
        coverage_tag: Option<CoverageTag>,
        severity: SeverityInfo,
    },
    /// Location information (file, line, function)
    Location {
        file: PathBuf,
        line: u32,
        function: String,
    },
    /// Context dampening information (spec 191)
    ContextDampening {
        description: String,
        dampening_percentage: i32,
    },
    /// Recommended action
    Action { action: String },
    /// Expected impact metrics
    Impact {
        complexity_reduction: u32,
        risk_reduction: f64,
    },
    /// Evidence combining complexity and metrics
    Evidence { text: String },
    /// Complexity metrics
    Complexity {
        cyclomatic: u32,
        cognitive: u32,
        nesting: u32,
        entropy: Option<f64>,
    },
    /// Detected pattern information (spec 190)
    Pattern {
        pattern_type: String,
        icon: String,
        metrics: Vec<(String, String)>,
        confidence: f64,
    },
    /// Coverage information
    Coverage {
        percentage: f64,
        level: CoverageLevel,
        details: Option<String>,
    },
    /// Dependency information
    Dependencies {
        upstream: usize,
        downstream: usize,
        callers: Vec<String>,
        callees: Vec<String>,
    },
    /// Debt-specific information
    DebtSpecific { text: String },
    /// Contextual risk information from context providers (spec 202)
    ContextualRisk {
        base_risk: f64,
        contextual_risk: f64,
        multiplier: f64,
        providers: Vec<ContextProviderInfo>,
    },
    /// Rationale for recommendation
    Rationale { text: String },
}

/// Coverage tag information for display
#[derive(Debug, Clone, PartialEq)]
pub struct CoverageTag {
    pub text: String,
    pub color: Color,
}

/// Severity information for display
#[derive(Debug, Clone, PartialEq)]
pub struct SeverityInfo {
    pub label: String,
    pub color: Color,
}

/// Context provider contribution information for display (spec 202)
#[derive(Debug, Clone, PartialEq)]
pub struct ContextProviderInfo {
    pub name: String,
    pub contribution: f64,
    pub weight: f64,
    pub impact: f64,
    pub details: Option<String>,
}

impl FormattedPriorityItem {
    /// Creates a new formatted priority item with the given rank, score, and severity.
    pub fn new(rank: usize, score: f64, severity: Severity) -> Self {
        Self {
            rank,
            score,
            severity,
            sections: Vec::new(),
        }
    }

    /// Adds a section to this formatted item.
    pub fn with_section(mut self, section: FormattedSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Adds multiple sections to this formatted item.
    pub fn with_sections(mut self, sections: Vec<FormattedSection>) -> Self {
        self.sections.extend(sections);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatted_item_builder() {
        let item = FormattedPriorityItem::new(1, 8.5, Severity::Critical)
            .with_section(FormattedSection::Location {
                file: PathBuf::from("test.rs"),
                line: 10,
                function: "test_fn".to_string(),
            })
            .with_section(FormattedSection::Impact {
                complexity_reduction: 5,
                risk_reduction: 3.2,
            });

        assert_eq!(item.rank, 1);
        assert_eq!(item.score, 8.5);
        assert_eq!(item.severity, Severity::Critical);
        assert_eq!(item.sections.len(), 2);
    }

    #[test]
    fn coverage_tag_equality() {
        let tag1 = CoverageTag {
            text: "[ERROR UNTESTED]".to_string(),
            color: Color::Red,
        };
        let tag2 = CoverageTag {
            text: "[ERROR UNTESTED]".to_string(),
            color: Color::Red,
        };
        assert_eq!(tag1, tag2);
    }

    #[test]
    fn severity_info_equality() {
        let sev1 = SeverityInfo {
            label: "CRITICAL".to_string(),
            color: Color::Red,
        };
        let sev2 = SeverityInfo {
            label: "CRITICAL".to_string(),
            color: Color::Red,
        };
        assert_eq!(sev1, sev2);
    }
}
