//! Detail page enum for multi-page detail view.
//!
//! This module defines the different pages available in the detail view.
//! Each page shows different aspects of a selected debt item.
//! Following single responsibility principle, DetailPage is extracted
//! from app.rs to its own focused module.

/// Detail page selection for multi-page detail view.
///
/// Each variant represents a different page of information about
/// the currently selected debt item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailPage {
    /// Overview page with summary metrics.
    Overview,
    /// Dependencies page showing function relationships.
    Dependencies,
    /// Git context page with commit history.
    GitContext,
    /// Patterns page showing detected anti-patterns.
    Patterns,
    /// Data flow page with purity analysis.
    DataFlow,
    /// Responsibilities page showing detected responsibilities.
    Responsibilities,
}

impl DetailPage {
    /// Get next page with wrapping.
    ///
    /// Returns the next page in sequence, wrapping from Responsibilities
    /// back to Overview.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            DetailPage::Overview => DetailPage::Dependencies,
            DetailPage::Dependencies => DetailPage::GitContext,
            DetailPage::GitContext => DetailPage::Patterns,
            DetailPage::Patterns => DetailPage::DataFlow,
            DetailPage::DataFlow => DetailPage::Responsibilities,
            DetailPage::Responsibilities => DetailPage::Overview,
        }
    }

    /// Get previous page with wrapping.
    ///
    /// Returns the previous page in sequence, wrapping from Overview
    /// back to Responsibilities.
    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            DetailPage::Overview => DetailPage::Responsibilities,
            DetailPage::Dependencies => DetailPage::Overview,
            DetailPage::GitContext => DetailPage::Dependencies,
            DetailPage::Patterns => DetailPage::GitContext,
            DetailPage::DataFlow => DetailPage::Patterns,
            DetailPage::Responsibilities => DetailPage::DataFlow,
        }
    }

    /// Create from 0-based index.
    ///
    /// Returns `None` for invalid indices (>= 6).
    #[must_use]
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(DetailPage::Overview),
            1 => Some(DetailPage::Dependencies),
            2 => Some(DetailPage::GitContext),
            3 => Some(DetailPage::Patterns),
            4 => Some(DetailPage::DataFlow),
            5 => Some(DetailPage::Responsibilities),
            _ => None,
        }
    }

    /// Get 0-based index.
    ///
    /// Returns the index of this page in the sequence.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            DetailPage::Overview => 0,
            DetailPage::Dependencies => 1,
            DetailPage::GitContext => 2,
            DetailPage::Patterns => 3,
            DetailPage::DataFlow => 4,
            DetailPage::Responsibilities => 5,
        }
    }

    /// Get display name for page.
    ///
    /// Returns a human-readable name for use in the UI.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            DetailPage::Overview => "Overview",
            DetailPage::Dependencies => "Dependencies",
            DetailPage::GitContext => "Git Context",
            DetailPage::Patterns => "Patterns",
            DetailPage::DataFlow => "Data Flow",
            DetailPage::Responsibilities => "Responsibilities",
        }
    }

    /// Total number of detail pages.
    pub const COUNT: usize = 6;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detail_page_next_wraps_forward() {
        assert_eq!(DetailPage::Overview.next(), DetailPage::Dependencies);
        assert_eq!(DetailPage::Dependencies.next(), DetailPage::GitContext);
        assert_eq!(DetailPage::GitContext.next(), DetailPage::Patterns);
        assert_eq!(DetailPage::Patterns.next(), DetailPage::DataFlow);
        assert_eq!(DetailPage::DataFlow.next(), DetailPage::Responsibilities);
        assert_eq!(DetailPage::Responsibilities.next(), DetailPage::Overview);
    }

    #[test]
    fn test_detail_page_prev_wraps_backward() {
        assert_eq!(DetailPage::Overview.prev(), DetailPage::Responsibilities);
        assert_eq!(DetailPage::Dependencies.prev(), DetailPage::Overview);
        assert_eq!(DetailPage::GitContext.prev(), DetailPage::Dependencies);
        assert_eq!(DetailPage::Patterns.prev(), DetailPage::GitContext);
        assert_eq!(DetailPage::DataFlow.prev(), DetailPage::Patterns);
        assert_eq!(DetailPage::Responsibilities.prev(), DetailPage::DataFlow);
    }

    #[test]
    fn test_detail_page_from_index() {
        assert_eq!(DetailPage::from_index(0), Some(DetailPage::Overview));
        assert_eq!(DetailPage::from_index(1), Some(DetailPage::Dependencies));
        assert_eq!(DetailPage::from_index(2), Some(DetailPage::GitContext));
        assert_eq!(DetailPage::from_index(3), Some(DetailPage::Patterns));
        assert_eq!(DetailPage::from_index(4), Some(DetailPage::DataFlow));
        assert_eq!(
            DetailPage::from_index(5),
            Some(DetailPage::Responsibilities)
        );
        assert_eq!(DetailPage::from_index(6), None);
    }

    #[test]
    fn test_detail_page_index() {
        assert_eq!(DetailPage::Overview.index(), 0);
        assert_eq!(DetailPage::Dependencies.index(), 1);
        assert_eq!(DetailPage::GitContext.index(), 2);
        assert_eq!(DetailPage::Patterns.index(), 3);
        assert_eq!(DetailPage::DataFlow.index(), 4);
        assert_eq!(DetailPage::Responsibilities.index(), 5);
    }

    #[test]
    fn test_detail_page_name() {
        assert_eq!(DetailPage::Overview.name(), "Overview");
        assert_eq!(DetailPage::Dependencies.name(), "Dependencies");
        assert_eq!(DetailPage::GitContext.name(), "Git Context");
        assert_eq!(DetailPage::Patterns.name(), "Patterns");
        assert_eq!(DetailPage::DataFlow.name(), "Data Flow");
        assert_eq!(DetailPage::Responsibilities.name(), "Responsibilities");
    }

    #[test]
    fn test_next_prev_inverse() {
        // next().prev() should return original
        for page in [
            DetailPage::Overview,
            DetailPage::Dependencies,
            DetailPage::GitContext,
            DetailPage::Patterns,
            DetailPage::DataFlow,
            DetailPage::Responsibilities,
        ] {
            assert_eq!(page.next().prev(), page);
            assert_eq!(page.prev().next(), page);
        }
    }

    #[test]
    fn test_index_from_index_roundtrip() {
        // from_index(index()) should return original
        for page in [
            DetailPage::Overview,
            DetailPage::Dependencies,
            DetailPage::GitContext,
            DetailPage::Patterns,
            DetailPage::DataFlow,
            DetailPage::Responsibilities,
        ] {
            assert_eq!(DetailPage::from_index(page.index()), Some(page));
        }
    }
}
