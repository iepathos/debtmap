//! Application state for the results TUI.
//!
//! This module provides the main `ResultsApp` coordinator that composes
//! smaller state modules for list, query, and navigation state management.
//! Following the single responsibility principle, ResultsApp serves as
//! a slim coordinator rather than a god object.

use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

use super::{
    detail_view, dsm_view, filter::Filter, layout, list_state::ListState, list_view, nav_state,
    navigation, query_state::QueryState, search::SearchState, sort::SortCriteria,
};

/// Helper to get coverage percentage from UnifiedDebtItem
pub fn get_coverage(item: &UnifiedDebtItem) -> Option<f64> {
    item.transitive_coverage.as_ref().map(|c| c.direct)
}

/// View mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Main list view
    List,
    /// Detail view for selected item
    Detail,
    /// Search input mode
    Search,
    /// Sort menu
    SortMenu,
    /// Filter menu
    FilterMenu,
    /// Help overlay
    Help,
    /// Dependency Structure Matrix view (Spec 205)
    Dsm,
}

/// Detail page selection for multi-page detail view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailPage {
    Overview,
    Dependencies,
    GitContext,
    Patterns,
    DataFlow,
    Responsibilities,
}

impl DetailPage {
    /// Get next page with wrapping
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

    /// Get previous page with wrapping
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

    /// Create from 0-based index
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

    /// Get 0-based index
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

    /// Get display name for page
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
    // DELEGATION TO LIST STATE
    // ========================================================================

    /// Get selected index
    pub fn selected_index(&self) -> usize {
        self.list.selected_index()
    }

    /// Set selected index (with bounds checking)
    pub fn set_selected_index(&mut self, index: usize) {
        let count = self.item_count();
        self.list.set_selected_index(index, count);
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.list.scroll_offset()
    }

    /// Set scroll offset
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.list.set_scroll_offset(offset);
    }

    /// Toggle grouping on/off
    pub fn toggle_grouping(&mut self) {
        self.list.toggle_grouping();
    }

    /// Get grouping state
    pub fn is_grouped(&self) -> bool {
        self.list.is_grouped()
    }

    // ========================================================================
    // DELEGATION TO QUERY STATE
    // ========================================================================

    /// Get reference to search state
    pub fn search(&self) -> &SearchState {
        self.query.search()
    }

    /// Get mutable reference to search state
    pub fn search_mut(&mut self) -> &mut SearchState {
        self.query.search_mut()
    }

    /// Get active filters
    pub fn filters(&self) -> &[Filter] {
        self.query.filters()
    }

    /// Get current sort criteria
    pub fn sort_by(&self) -> SortCriteria {
        self.query.sort_by()
    }

    /// Set sort criteria and re-sort
    pub fn set_sort_by(&mut self, criteria: SortCriteria) {
        self.query.set_sort_by(criteria, &self.analysis);
    }

    /// Apply search filter
    pub fn apply_search(&mut self) {
        self.query.apply_search(&self.analysis);
        self.list.reset();
    }

    /// Add a filter
    pub fn add_filter(&mut self, filter: Filter) {
        self.query.add_filter(filter, &self.analysis);
        self.list.reset();
    }

    /// Remove a filter
    pub fn remove_filter(&mut self, index: usize) {
        self.query.remove_filter(index, &self.analysis);
    }

    /// Clear all filters
    pub fn clear_filters(&mut self) {
        self.query.clear_filters(&self.analysis);
    }

    // ========================================================================
    // DELEGATION TO NAVIGATION STATE
    // ========================================================================

    /// Get current view mode
    pub fn view_mode(&self) -> ViewMode {
        self.nav.view_mode
    }

    /// Set view mode
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.nav.view_mode = mode;
    }

    /// Get current detail page
    pub fn detail_page(&self) -> DetailPage {
        self.nav.detail_page
    }

    /// Set detail page
    pub fn set_detail_page(&mut self, page: DetailPage) {
        self.nav.detail_page = page;
    }

    /// Get navigation history for back navigation.
    pub fn nav_history(&self) -> &[ViewMode] {
        &self.nav.history
    }

    /// Push view mode to navigation history.
    pub fn push_nav_history(&mut self, mode: ViewMode) {
        self.nav.history.push(mode);
    }

    /// Pop from navigation history.
    pub fn pop_nav_history(&mut self) -> Option<ViewMode> {
        self.nav.history.pop()
    }

    /// Clear navigation history.
    pub fn clear_nav_history(&mut self) {
        self.nav.clear_history();
    }

    /// Check if DSM feature is enabled.
    pub fn dsm_enabled(&self) -> bool {
        self.nav.dsm_enabled
    }

    /// Get DSM horizontal scroll offset
    pub fn dsm_scroll_x(&self) -> usize {
        self.nav.dsm_scroll_x
    }

    /// Set DSM horizontal scroll offset
    pub fn set_dsm_scroll_x(&mut self, offset: usize) {
        self.nav.dsm_scroll_x = offset;
    }

    /// Get DSM vertical scroll offset
    pub fn dsm_scroll_y(&self) -> usize {
        self.nav.dsm_scroll_y
    }

    /// Set DSM vertical scroll offset
    pub fn set_dsm_scroll_y(&mut self, offset: usize) {
        self.nav.dsm_scroll_y = offset;
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
    // DETAIL PAGE HELPERS
    // ========================================================================

    /// Get available pages for current item (skip pages with no data)
    pub fn available_pages(&self) -> Vec<DetailPage> {
        let mut pages = vec![DetailPage::Overview, DetailPage::Dependencies];

        if self.has_git_context() {
            pages.push(DetailPage::GitContext);
        }

        if self.has_pattern_data() {
            pages.push(DetailPage::Patterns);
        }

        if self.has_data_flow_data() {
            pages.push(DetailPage::DataFlow);
        }

        pages.push(DetailPage::Responsibilities);
        pages
    }

    /// Get total page count for current item
    pub fn page_count(&self) -> usize {
        self.available_pages().len()
    }

    /// Get the index of current page within available pages
    pub fn current_page_index(&self) -> usize {
        let available = self.available_pages();
        available
            .iter()
            .position(|&p| p == self.nav.detail_page)
            .unwrap_or(0)
    }

    /// Navigate to next available page (wrapping)
    pub fn next_available_page(&mut self) {
        let available = self.available_pages();
        if available.is_empty() {
            return;
        }
        let current_idx = available
            .iter()
            .position(|&p| p == self.nav.detail_page)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % available.len();
        self.nav.detail_page = available[next_idx];
    }

    /// Navigate to previous available page (wrapping)
    pub fn prev_available_page(&mut self) {
        let available = self.available_pages();
        if available.is_empty() {
            return;
        }
        let current_idx = available
            .iter()
            .position(|&p| p == self.nav.detail_page)
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            available.len() - 1
        } else {
            current_idx - 1
        };
        self.nav.detail_page = available[prev_idx];
    }

    /// Check if a page is available for the current item
    pub fn is_page_available(&self, page: DetailPage) -> bool {
        self.available_pages().contains(&page)
    }

    /// Ensure current page is valid for the selected item
    pub fn ensure_valid_page(&mut self) {
        if !self.is_page_available(self.nav.detail_page) {
            self.nav.detail_page = DetailPage::Overview;
        }
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

    /// Get available navigation actions for current state.
    pub fn available_nav_actions(&self) -> Vec<(&'static str, &'static str)> {
        nav_state::available_actions(
            &nav_state::NavigationState {
                view_mode: self.nav.view_mode,
                detail_page: self.nav.detail_page,
                history: self.nav.history.clone(),
                dsm_enabled: self.nav.dsm_enabled,
                dsm_scroll_x: self.nav.dsm_scroll_x,
                dsm_scroll_y: self.nav.dsm_scroll_y,
            },
            self.has_items(),
            self.has_selection(),
        )
    }

    /// Get count display for header
    pub fn count_display(&self) -> String {
        if self.list.is_grouped() {
            let groups = super::grouping::group_by_location(self.filtered_items(), self.sort_by());
            let issue_count = self.query.filtered_indices().len();
            format!("{} locations ({} issues)", groups.len(), issue_count)
        } else {
            format!("{} items", self.query.filtered_indices().len())
        }
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    fn has_git_context(&self) -> bool {
        self.selected_item()
            .and_then(|item| item.contextual_risk.as_ref())
            .is_some()
    }

    fn has_pattern_data(&self) -> bool {
        self.selected_item()
            .map(|item| {
                let func_id = crate::priority::call_graph::FunctionId::new(
                    item.location.file.clone(),
                    item.location.function.clone(),
                    item.location.line,
                );

                item.pattern_analysis.is_some()
                    || item.detected_pattern.is_some()
                    || item.is_pure.is_some()
                    || item.language_specific.is_some()
                    || item.entropy_details.is_some()
                    || item.error_swallowing_count.is_some()
                    || item.error_swallowing_patterns.is_some()
                    || self
                        .analysis
                        .data_flow_graph
                        .get_purity_info(&func_id)
                        .is_some()
                    || item
                        .god_object_indicators
                        .as_ref()
                        .map(|god| {
                            god.is_god_object
                                && (god.aggregated_entropy.is_some()
                                    || god.aggregated_error_swallowing_count.is_some()
                                    || god
                                        .aggregated_error_swallowing_patterns
                                        .as_ref()
                                        .map(|p| !p.is_empty())
                                        .unwrap_or(false))
                        })
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    fn has_data_flow_data(&self) -> bool {
        self.selected_item()
            .map(|item| {
                let func_id = crate::priority::call_graph::FunctionId::new(
                    item.location.file.clone(),
                    item.location.function.clone(),
                    item.location.line,
                );

                self.analysis
                    .data_flow_graph
                    .get_purity_info(&func_id)
                    .is_some()
                    || self
                        .analysis
                        .data_flow_graph
                        .get_mutation_info(&func_id)
                        .is_some()
                    || self
                        .analysis
                        .data_flow_graph
                        .get_io_operations(&func_id)
                        .is_some()
            })
            .unwrap_or(false)
    }
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
}
