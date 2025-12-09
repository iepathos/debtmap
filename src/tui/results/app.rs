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
    detail_view, dsm_view, filter::Filter, layout, list_view, navigation, search::SearchState,
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
    /// Whether to group items by location
    show_grouped: bool,
    /// Status message to display temporarily (cleared on next key press)
    status_message: Option<String>,
    /// DSM view horizontal scroll offset (Spec 205)
    dsm_scroll_x: usize,
    /// DSM view vertical scroll offset (Spec 205)
    dsm_scroll_y: usize,
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
            show_grouped: true, // Default: grouping enabled
            status_message: None,
            dsm_scroll_x: 0,
            dsm_scroll_y: 0,
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
            ViewMode::Dsm => dsm_view::render(frame, self),
        }
    }

    /// Get the currently selected item
    pub fn selected_item(&self) -> Option<&UnifiedDebtItem> {
        if self.show_grouped {
            // When grouped, selected_index refers to group index, not item index
            let groups = super::grouping::group_by_location(self.filtered_items(), self.sort_by);
            groups.get(self.selected_index).and_then(|group| {
                // Return the first item from the group (groups always have at least 1 item)
                group.items.first().copied()
            })
        } else {
            // When not grouped, selected_index refers to filtered_indices
            self.filtered_indices
                .get(self.selected_index)
                .and_then(|&idx| self.analysis.items.get(idx))
        }
    }

    /// Get all filtered items
    pub fn filtered_items(&self) -> impl Iterator<Item = &UnifiedDebtItem> {
        self.filtered_indices
            .iter()
            .filter_map(|&idx| self.analysis.items.get(idx))
    }

    /// Get total item count (filtered)
    /// Returns group count when grouped, item count otherwise
    pub fn item_count(&self) -> usize {
        if self.show_grouped {
            let groups = super::grouping::group_by_location(self.filtered_items(), self.sort_by);
            groups.len()
        } else {
            self.filtered_indices.len()
        }
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
                    || self
                        .analysis
                        .data_flow_graph
                        .get_purity_info(&func_id)
                        .is_some()
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
            })
            .unwrap_or(false)
    }

    /// Check if responsibilities data is available for current item
    fn has_responsibilities_data(&self) -> bool {
        self.selected_item()
            .map(|item| {
                // Has god object responsibilities
                item.god_object_indicators
                    .as_ref()
                    .map(|ind| ind.is_god_object && !ind.responsibilities.is_empty())
                    .unwrap_or(false)
                    // Or has single responsibility category
                    || item.responsibility_category.is_some()
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

        if self.has_responsibilities_data() {
            pages.push(DetailPage::Responsibilities);
        }

        pages
    }

    /// Get total page count for current item
    pub fn page_count(&self) -> usize {
        self.available_pages().len()
    }

    /// Get the index of current page within available pages (for display)
    /// Returns 0 if current page is not in available pages
    pub fn current_page_index(&self) -> usize {
        let available = self.available_pages();
        available
            .iter()
            .position(|&p| p == self.detail_page)
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
            .position(|&p| p == self.detail_page)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % available.len();
        self.detail_page = available[next_idx];
    }

    /// Navigate to previous available page (wrapping)
    pub fn prev_available_page(&mut self) {
        let available = self.available_pages();
        if available.is_empty() {
            return;
        }
        let current_idx = available
            .iter()
            .position(|&p| p == self.detail_page)
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            available.len() - 1
        } else {
            current_idx - 1
        };
        self.detail_page = available[prev_idx];
    }

    /// Check if a page is available for the current item
    pub fn is_page_available(&self, page: DetailPage) -> bool {
        self.available_pages().contains(&page)
    }

    /// Toggle grouping on/off
    pub fn toggle_grouping(&mut self) {
        self.show_grouped = !self.show_grouped;
    }

    /// Get grouping state
    pub fn is_grouped(&self) -> bool {
        self.show_grouped
    }

    /// Get count display for header (location count vs item count)
    pub fn count_display(&self) -> String {
        if self.show_grouped {
            let groups = super::grouping::group_by_location(self.filtered_items(), self.sort_by());
            let issue_count = self.filtered_indices.len();
            format!("{} locations ({} issues)", groups.len(), issue_count)
        } else {
            format!("{} items", self.filtered_indices.len())
        }
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

    /// Get DSM horizontal scroll offset (Spec 205)
    pub fn dsm_scroll_x(&self) -> usize {
        self.dsm_scroll_x
    }

    /// Set DSM horizontal scroll offset (Spec 205)
    pub fn set_dsm_scroll_x(&mut self, offset: usize) {
        self.dsm_scroll_x = offset;
    }

    /// Get DSM vertical scroll offset (Spec 205)
    pub fn dsm_scroll_y(&self) -> usize {
        self.dsm_scroll_y
    }

    /// Set DSM vertical scroll offset (Spec 205)
    pub fn set_dsm_scroll_y(&mut self, offset: usize) {
        self.dsm_scroll_y = offset;
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
