//! Application state for the results TUI.

use crate::priority::{UnifiedAnalysis, UnifiedDebtItem};
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;

/// Helper to get coverage percentage from UnifiedDebtItem
pub fn get_coverage(item: &UnifiedDebtItem) -> Option<f64> {
    item.transitive_coverage.as_ref().map(|c| c.direct)
}

use super::{
    detail_view, filter::Filter, layout, list_view, navigation, search::SearchState,
    sort::SortCriteria,
};

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
}

/// Detail page selection for multi-page detail view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailPage {
    Overview,
    Dependencies,
    GitContext,
    Patterns,
    DataFlow,
}

impl DetailPage {
    /// Get next page with wrapping
    pub fn next(self) -> Self {
        match self {
            DetailPage::Overview => DetailPage::Dependencies,
            DetailPage::Dependencies => DetailPage::GitContext,
            DetailPage::GitContext => DetailPage::Patterns,
            DetailPage::Patterns => DetailPage::DataFlow,
            DetailPage::DataFlow => DetailPage::Overview,
        }
    }

    /// Get previous page with wrapping
    pub fn prev(self) -> Self {
        match self {
            DetailPage::Overview => DetailPage::DataFlow,
            DetailPage::Dependencies => DetailPage::Overview,
            DetailPage::GitContext => DetailPage::Dependencies,
            DetailPage::Patterns => DetailPage::GitContext,
            DetailPage::DataFlow => DetailPage::Patterns,
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
        }
    }
}

/// Main application state
pub struct ResultsApp {
    /// Full analysis results
    analysis: UnifiedAnalysis,
    /// Filtered item indices (indices into analysis.items)
    filtered_indices: Vec<usize>,
    /// Current view mode
    view_mode: ViewMode,
    /// Selected index in filtered list
    selected_index: usize,
    /// Scroll offset for list view
    scroll_offset: usize,
    /// Search state
    search: SearchState,
    /// Active filters
    filters: Vec<Filter>,
    /// Sort criteria
    sort_by: SortCriteria,
    /// Terminal size
    terminal_size: (u16, u16),
    /// Force full redraw on next render (set after external editor)
    needs_redraw: bool,
    /// Current detail page (for multi-page detail view)
    detail_page: DetailPage,
}

impl ResultsApp {
    /// Create new application state from analysis results
    pub fn new(analysis: UnifiedAnalysis) -> Self {
        let item_count = analysis.items.len();
        let filtered_indices: Vec<usize> = (0..item_count).collect();

        Self {
            analysis,
            filtered_indices,
            view_mode: ViewMode::List,
            selected_index: 0,
            scroll_offset: 0,
            search: SearchState::new(),
            filters: Vec::new(),
            sort_by: SortCriteria::Score,
            terminal_size: (80, 24),
            needs_redraw: false,
            detail_page: DetailPage::Overview,
        }
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        navigation::handle_key(self, key)
    }

    /// Render the current view
    pub fn render(&mut self, frame: &mut Frame) {
        self.terminal_size = (frame.size().width, frame.size().height);

        match self.view_mode {
            ViewMode::List => list_view::render(frame, self),
            ViewMode::Detail => detail_view::render(frame, self),
            ViewMode::Search => list_view::render_with_search(frame, self),
            ViewMode::SortMenu => list_view::render_with_sort_menu(frame, self),
            ViewMode::FilterMenu => list_view::render_with_filter_menu(frame, self),
            ViewMode::Help => layout::render_help_overlay(frame, self),
        }
    }

    /// Get the currently selected item
    pub fn selected_item(&self) -> Option<&UnifiedDebtItem> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&idx| self.analysis.items.get(idx))
    }

    /// Get all filtered items
    pub fn filtered_items(&self) -> impl Iterator<Item = &UnifiedDebtItem> {
        self.filtered_indices
            .iter()
            .filter_map(|&idx| self.analysis.items.get(idx))
    }

    /// Get total item count (filtered)
    pub fn item_count(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Get selected index
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Set selected index (with bounds checking)
    pub fn set_selected_index(&mut self, index: usize) {
        if self.filtered_indices.is_empty() {
            self.selected_index = 0;
        } else {
            self.selected_index = index.min(self.filtered_indices.len() - 1);
        }
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    /// Get current view mode
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// Set view mode
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
    }

    /// Get reference to analysis
    pub fn analysis(&self) -> &UnifiedAnalysis {
        &self.analysis
    }

    /// Get reference to search state
    pub fn search(&self) -> &SearchState {
        &self.search
    }

    /// Get mutable reference to search state
    pub fn search_mut(&mut self) -> &mut SearchState {
        &mut self.search
    }

    /// Get active filters
    pub fn filters(&self) -> &[Filter] {
        &self.filters
    }

    /// Get current sort criteria
    pub fn sort_by(&self) -> SortCriteria {
        self.sort_by
    }

    /// Set sort criteria and re-sort
    pub fn set_sort_by(&mut self, criteria: SortCriteria) {
        self.sort_by = criteria;
        self.apply_sort();
    }

    /// Apply current sorting
    fn apply_sort(&mut self) {
        super::sort::sort_indices(&mut self.filtered_indices, &self.analysis, self.sort_by);
    }

    /// Apply search filter
    pub fn apply_search(&mut self) {
        let query = self.search.query();
        if query.is_empty() {
            // No search - show all items (filtered by other filters)
            self.filtered_indices = (0..self.analysis.items.len()).collect();
        } else {
            // Apply search filter
            self.filtered_indices = super::search::filter_items(&self.analysis, query);
        }

        // Apply other filters
        self.apply_filters();

        // Re-apply sort
        self.apply_sort();

        // Reset selection to top
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Add a filter
    pub fn add_filter(&mut self, filter: Filter) {
        self.filters.push(filter);
        self.apply_filters();
        self.apply_sort();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Remove a filter
    pub fn remove_filter(&mut self, index: usize) {
        if index < self.filters.len() {
            self.filters.remove(index);
            self.apply_filters();
            self.apply_sort();
        }
    }

    /// Clear all filters
    pub fn clear_filters(&mut self) {
        self.filters.clear();
        self.apply_filters();
        self.apply_sort();
    }

    /// Apply all active filters
    fn apply_filters(&mut self) {
        if self.filters.is_empty() {
            return;
        }

        self.filtered_indices.retain(|&idx| {
            if let Some(item) = self.analysis.items.get(idx) {
                self.filters.iter().all(|filter| filter.matches(item))
            } else {
                false
            }
        });
    }

    /// Get terminal size
    pub fn terminal_size(&self) -> (u16, u16) {
        self.terminal_size
    }

    /// Request a full redraw on next render (used after external editor)
    pub fn request_redraw(&mut self) {
        self.needs_redraw = true;
    }

    /// Check if a full redraw is needed and clear the flag
    pub fn take_needs_redraw(&mut self) -> bool {
        let needs = self.needs_redraw;
        self.needs_redraw = false;
        needs
    }

    /// Get current detail page
    pub fn detail_page(&self) -> DetailPage {
        self.detail_page
    }

    /// Set detail page
    pub fn set_detail_page(&mut self, page: DetailPage) {
        self.detail_page = page;
    }

    /// Check if git context data is available for current item
    fn has_git_context(&self) -> bool {
        self.selected_item()
            .and_then(|item| item.contextual_risk.as_ref())
            .is_some()
    }

    /// Check if pattern data is available for current item
    fn has_pattern_data(&self) -> bool {
        self.selected_item()
            .map(|item| {
                item.pattern_analysis.is_some()
                    || item.detected_pattern.is_some()
                    || item.is_pure.is_some()
                    || item.language_specific.is_some()
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
                    .get_mutation_info(&func_id)
                    .is_some()
                    || self
                        .analysis
                        .data_flow_graph
                        .get_io_operations(&func_id)
                        .is_some()
                    || self
                        .analysis
                        .data_flow_graph
                        .get_cfg_analysis(&func_id)
                        .is_some()
                    || self
                        .analysis
                        .data_flow_graph
                        .get_purity_info(&func_id)
                        .is_some()
            })
            .unwrap_or(false)
    }

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

        pages
    }

    /// Get total page count for current item
    pub fn page_count(&self) -> usize {
        self.available_pages().len()
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
        assert_eq!(DetailPage::Patterns.next(), DetailPage::Overview);
    }

    #[test]
    fn test_detail_page_prev_wraps_backward() {
        assert_eq!(DetailPage::Overview.prev(), DetailPage::Patterns);
        assert_eq!(DetailPage::Dependencies.prev(), DetailPage::Overview);
        assert_eq!(DetailPage::GitContext.prev(), DetailPage::Dependencies);
        assert_eq!(DetailPage::Patterns.prev(), DetailPage::GitContext);
    }

    #[test]
    fn test_detail_page_from_index() {
        assert_eq!(DetailPage::from_index(0), Some(DetailPage::Overview));
        assert_eq!(DetailPage::from_index(1), Some(DetailPage::Dependencies));
        assert_eq!(DetailPage::from_index(2), Some(DetailPage::GitContext));
        assert_eq!(DetailPage::from_index(3), Some(DetailPage::Patterns));
        assert_eq!(DetailPage::from_index(4), None);
    }

    #[test]
    fn test_detail_page_index() {
        assert_eq!(DetailPage::Overview.index(), 0);
        assert_eq!(DetailPage::Dependencies.index(), 1);
        assert_eq!(DetailPage::GitContext.index(), 2);
        assert_eq!(DetailPage::Patterns.index(), 3);
    }

    #[test]
    fn test_detail_page_name() {
        assert_eq!(DetailPage::Overview.name(), "Overview");
        assert_eq!(DetailPage::Dependencies.name(), "Dependencies");
        assert_eq!(DetailPage::GitContext.name(), "Git Context");
        assert_eq!(DetailPage::Patterns.name(), "Patterns");
    }
}
