//! Application state for the results TUI.
//!
//! This module provides the main `ResultsApp` coordinator that composes
//! smaller state modules for list, query, and navigation state management.
//! Following the single responsibility principle, ResultsApp serves as
//! a slim coordinator rather than a god object.
//!
//! Call sites access component state directly via accessors:
//! - `app.list()` / `app.list_mut()` for list state (selection, scrolling, grouping)
//! - `app.query()` / `app.query_mut()` for query state (search, filters, sort)
//! - `app.nav()` / `app.nav_mut()` for navigation state (view mode, detail page, history)

use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

use super::{
    detail_view, dsm_view, layout, list_state::ListState, list_view, nav_state, navigation,
    query_state::QueryState,
};

// Re-export for backwards compatibility
pub use super::detail_page::DetailPage;
pub use super::view_mode::ViewMode;

/// Helper to get coverage percentage from UnifiedDebtItem
pub fn get_coverage(item: &UnifiedDebtItem) -> Option<f64> {
    item.transitive_coverage.as_ref().map(|c| c.direct)
}

/// Main application state - slim coordinator.
///
/// Composes ListState, QueryState, and NavigationState modules
/// following the single responsibility principle.
pub struct ResultsApp {
    // Core data (owned)
    analysis: UnifiedAnalysis,

    // Composed state modules
    list: ListState,
    query: QueryState,
    nav: nav_state::NavigationState,

    // UI state (minimal, stays here)
    terminal_size: (u16, u16),
    needs_redraw: bool,
    status_message: Option<String>,
}

impl ResultsApp {
    /// Create new application state from analysis results.
    ///
    /// This is the original API that uses all items from the analysis.
    pub fn new(analysis: UnifiedAnalysis) -> Self {
        let item_count = analysis.items.len();
        Self {
            analysis,
            list: ListState::default(),
            query: QueryState::new(item_count),
            nav: nav_state::NavigationState::new(true), // DSM enabled by default
            terminal_size: (80, 24),
            needs_redraw: false,
            status_message: None,
        }
    }

    /// Create application state from a PreparedDebtView (Spec 252).
    pub fn from_prepared_view(
        view: crate::priority::view::PreparedDebtView,
        mut analysis: UnifiedAnalysis,
    ) -> Self {
        // Extract items from PreparedDebtView into UnifiedAnalysis
        let prepared_items: im::Vector<UnifiedDebtItem> = view
            .items
            .iter()
            .filter_map(|item| {
                if let crate::priority::view::ViewItem::Function(func) = item {
                    Some((**func).clone())
                } else {
                    None
                }
            })
            .collect();

        let file_items: im::Vector<crate::priority::FileDebtItem> = view
            .items
            .iter()
            .filter_map(|item| {
                if let crate::priority::view::ViewItem::File(file) = item {
                    Some((**file).clone())
                } else {
                    None
                }
            })
            .collect();

        // Update analysis with prepared items
        analysis.items = prepared_items;
        analysis.file_items = file_items;

        let item_count = analysis.items.len();
        let show_grouped = view.config.compute_groups;

        let mut list = ListState::default();
        list.set_grouped(show_grouped);

        Self {
            analysis,
            list,
            query: QueryState::new(item_count),
            nav: nav_state::NavigationState::new(true),
            terminal_size: (80, 24),
            needs_redraw: false,
            status_message: None,
        }
    }

    // ========================================================================
    // STATE ACCESSORS
    // ========================================================================

    /// Get reference to list state.
    pub fn list(&self) -> &ListState {
        &self.list
    }

    /// Get mutable reference to list state.
    pub fn list_mut(&mut self) -> &mut ListState {
        &mut self.list
    }

    /// Get reference to query state.
    pub fn query(&self) -> &QueryState {
        &self.query
    }

    /// Get mutable reference to query state.
    pub fn query_mut(&mut self) -> &mut QueryState {
        &mut self.query
    }

    /// Get reference to navigation state.
    pub fn nav(&self) -> &nav_state::NavigationState {
        &self.nav
    }

    /// Get mutable reference to navigation state.
    pub fn nav_mut(&mut self) -> &mut nav_state::NavigationState {
        &mut self.nav
    }

    // ========================================================================
    // COORDINATION METHODS
    // ========================================================================

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        navigation::handle_key(self, key)
    }

    /// Render the current view
    pub fn render(&mut self, frame: &mut Frame) {
        self.terminal_size = (frame.area().width, frame.area().height);

        match self.nav.view_mode {
            ViewMode::List => list_view::render(frame, self),
            ViewMode::Detail => detail_view::render(frame, self),
            ViewMode::Search => list_view::render_with_search(frame, self),
            ViewMode::SortMenu => list_view::render_with_sort_menu(frame, self),
            ViewMode::FilterMenu => list_view::render_with_filter_menu(frame, self),
            ViewMode::Help => layout::render_help_overlay(frame, self),
            ViewMode::Dsm => dsm_view::render(frame, self),
        }
    }

    // ========================================================================
    // DATA ACCESS
    // ========================================================================

    /// Get reference to analysis
    pub fn analysis(&self) -> &UnifiedAnalysis {
        &self.analysis
    }

    /// Get the currently selected item
    pub fn selected_item(&self) -> Option<&UnifiedDebtItem> {
        if self.list.is_grouped() {
            let groups =
                super::grouping::group_by_location(self.filtered_items(), self.query.sort_by());
            groups
                .get(self.list.selected_index())
                .and_then(|group| group.items.first().copied())
        } else {
            self.query
                .filtered_indices()
                .get(self.list.selected_index())
                .and_then(|&idx| self.analysis.items.get(idx))
        }
    }

    /// Get all filtered items
    pub fn filtered_items(&self) -> impl Iterator<Item = &UnifiedDebtItem> {
        self.query
            .filtered_indices()
            .iter()
            .filter_map(|&idx| self.analysis.items.get(idx))
    }

    /// Get total item count (filtered)
    pub fn item_count(&self) -> usize {
        if self.list.is_grouped() {
            let groups =
                super::grouping::group_by_location(self.filtered_items(), self.query.sort_by());
            groups.len()
        } else {
            self.query.filtered_indices().len()
        }
    }

    // ========================================================================
    // UI STATE
    // ========================================================================

    /// Get terminal size
    pub fn terminal_size(&self) -> (u16, u16) {
        self.terminal_size
    }

    /// Request a full redraw on next render
    pub fn request_redraw(&mut self) {
        self.needs_redraw = true;
    }

    /// Check if a full redraw is needed and clear the flag
    pub fn take_needs_redraw(&mut self) -> bool {
        let needs = self.needs_redraw;
        self.needs_redraw = false;
        needs
    }

    /// Set status message to display temporarily
    pub fn set_status_message(&mut self, message: String) {
        self.status_message = Some(message);
    }

    /// Get current status message
    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    /// Clear status message
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    // ========================================================================
    // NAVIGATION HELPERS
    // ========================================================================

    /// Check if there's a selection (for navigation guards).
    pub fn has_selection(&self) -> bool {
        self.list.selected_index() < self.item_count()
    }

    /// Check if there are items (for navigation guards).
    pub fn has_items(&self) -> bool {
        self.item_count() > 0
    }

    /// Get count display for header
    pub fn count_display(&self) -> String {
        if self.list.is_grouped() {
            let groups =
                super::grouping::group_by_location(self.filtered_items(), self.query.sort_by());
            let issue_count = self.query.filtered_indices().len();
            format!("{} locations ({} issues)", groups.len(), issue_count)
        } else {
            format!("{} items", self.query.filtered_indices().len())
        }
    }

    // ========================================================================
    // QUERY COORDINATION (handles borrow issues)
    // ========================================================================

    /// Apply search and reset list (coordinates query with analysis).
    pub fn apply_search(&mut self) {
        self.query.apply_search(&self.analysis);
        self.list.reset();
    }

    /// Set sort criteria (coordinates query with analysis).
    pub fn set_sort_by(&mut self, criteria: super::sort::SortCriteria) {
        self.query.set_sort_by(criteria, &self.analysis);
    }

    /// Add filter and reset list (coordinates query with analysis).
    pub fn add_filter(&mut self, filter: super::filter::Filter) {
        self.query.add_filter(filter, &self.analysis);
        self.list.reset();
    }

    /// Clear filters (coordinates query with analysis).
    pub fn clear_filters(&mut self) {
        self.query.clear_filters(&self.analysis);
    }
}
