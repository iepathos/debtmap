//! View mode enum for the results TUI.
//!
//! This module defines the different view modes available in the TUI.
//! Following single responsibility principle, ViewMode is extracted
//! from app.rs to its own focused module.

/// View mode for the TUI.
///
/// Represents the current view state of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Main list view showing all debt items.
    List,
    /// Detail view for the currently selected item.
    Detail,
    /// Search input mode for filtering items.
    Search,
    /// Sort menu for changing sort criteria.
    SortMenu,
    /// Filter menu for applying filters.
    FilterMenu,
    /// Help overlay showing keyboard shortcuts.
    Help,
}
