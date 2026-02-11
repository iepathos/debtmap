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
use std::time::{Duration, Instant};

use super::{
    detail_view, layout, list_state::ListState, list_view, nav_state, navigation,
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
    status_message: Option<(String, Instant)>,
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
            nav: nav_state::NavigationState::new(), // DSM enabled by default
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
        // Uses ViewItem's as_function/as_file accessors for idiomatic extraction
        let prepared_items: im::Vector<UnifiedDebtItem> = view
            .items
            .iter()
            .filter_map(|item| item.as_function().cloned())
            .collect();

        let file_items: im::Vector<crate::priority::FileDebtItem> = view
            .items
            .iter()
            .filter_map(|item| item.as_file().cloned())
            .collect();

        // Update analysis with prepared items
        analysis.items = prepared_items;
        analysis.file_items = file_items;

        let item_count = analysis.items.len();

        Self {
            analysis,
            list: ListState::default(),
            query: QueryState::new(item_count),
            nav: nav_state::NavigationState::new(),
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

    /// Get detail view scroll offset for rendering.
    ///
    /// Returns (vertical_offset, horizontal_offset) tuple for use with
    /// `Paragraph::scroll()`.
    pub fn detail_scroll_offset(&self) -> (u16, u16) {
        let offset = self.nav.detail_scroll.offset();
        (offset.y, offset.x)
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
    ///
    /// Items are always grouped by location. When multiple debt types exist at
    /// the same location, returns the first item in the group. The overview page
    /// shows all debt types at the location.
    pub fn selected_item(&self) -> Option<&UnifiedDebtItem> {
        let groups =
            super::grouping::group_by_location(self.filtered_items(), self.query.sort_by());
        groups
            .get(self.list.selected_index())
            .and_then(|group| group.items.first().copied())
    }

    /// Get all filtered items
    pub fn filtered_items(&self) -> impl Iterator<Item = &UnifiedDebtItem> {
        self.query
            .filtered_indices()
            .iter()
            .filter_map(|&idx| self.analysis.items.get(idx))
    }

    /// Get total item count (filtered, grouped by location)
    pub fn item_count(&self) -> usize {
        let groups =
            super::grouping::group_by_location(self.filtered_items(), self.query.sort_by());
        groups.len()
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

    /// How long status messages remain visible before auto-dismissal.
    const STATUS_MESSAGE_DURATION: Duration = Duration::from_secs(3);

    /// Set status message to display temporarily (auto-dismisses after 3 seconds).
    pub fn set_status_message(&mut self, message: String) {
        self.status_message = Some((message, Instant::now()));
    }

    /// Get current status message if it hasn't expired.
    pub fn status_message(&self) -> Option<&str> {
        self.status_message
            .as_ref()
            .filter(|(_, created)| created.elapsed() < Self::STATUS_MESSAGE_DURATION)
            .map(|(msg, _)| msg.as_str())
    }

    /// Clear status message.
    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    /// Expire status message if its display duration has elapsed.
    pub fn expire_status_message(&mut self) {
        if self
            .status_message
            .as_ref()
            .is_some_and(|(_, created)| created.elapsed() >= Self::STATUS_MESSAGE_DURATION)
        {
            self.status_message = None;
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

    /// Get count display for header
    pub fn count_display(&self) -> String {
        let groups =
            super::grouping::group_by_location(self.filtered_items(), self.query.sort_by());
        let issue_count = self.query.filtered_indices().len();
        format!("{} locations ({} issues)", groups.len(), issue_count)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::CallGraph;

    fn create_test_analysis() -> UnifiedAnalysis {
        let call_graph = CallGraph::new();
        UnifiedAnalysis::new(call_graph)
    }

    #[test]
    fn test_results_app_new() {
        let analysis = create_test_analysis();
        let app = ResultsApp::new(analysis);

        assert_eq!(app.terminal_size(), (80, 24));
        assert!(!app.has_items());
        assert!(!app.has_selection());
    }

    #[test]
    fn test_status_message_lifecycle() {
        let analysis = create_test_analysis();
        let mut app = ResultsApp::new(analysis);

        // Initially no message
        assert!(app.status_message().is_none());

        // Set a message
        app.set_status_message("Test message".to_string());
        assert_eq!(app.status_message(), Some("Test message"));

        // Clear the message
        app.clear_status_message();
        assert!(app.status_message().is_none());
    }

    #[test]
    fn test_redraw_flag() {
        let analysis = create_test_analysis();
        let mut app = ResultsApp::new(analysis);

        // Initially no redraw needed
        assert!(!app.take_needs_redraw());

        // Request redraw
        app.request_redraw();
        assert!(app.take_needs_redraw());

        // Flag should be cleared after taking
        assert!(!app.take_needs_redraw());
    }

    #[test]
    fn test_state_accessors() {
        let analysis = create_test_analysis();
        let mut app = ResultsApp::new(analysis);

        // Test list accessor
        assert_eq!(app.list().selected_index(), 0);

        // Test query accessor
        assert!(app.query().filtered_indices().is_empty());

        // Test nav accessor
        assert_eq!(app.nav().view_mode, ViewMode::List);

        // Test mutable accessors work
        let _ = app.list_mut();
        let _ = app.query_mut();
        let _ = app.nav_mut();
    }

    #[test]
    fn test_count_display_empty() {
        let analysis = create_test_analysis();
        let app = ResultsApp::new(analysis);

        let display = app.count_display();
        assert!(display.contains("0 locations"));
        assert!(display.contains("0 issues"));
    }

    #[test]
    fn test_detail_scroll_offset() {
        let analysis = create_test_analysis();
        let app = ResultsApp::new(analysis);

        let (y, x) = app.detail_scroll_offset();
        assert_eq!(y, 0);
        assert_eq!(x, 0);
    }

    #[test]
    fn test_get_coverage_returns_none_without_coverage() {
        // Create a minimal item using the testkit pattern
        use crate::priority::{
            ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location, UnifiedScore,
        };
        use std::path::PathBuf;

        let item = UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                line: 10,
                function: "test_fn".to_string(),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
            unified_score: UnifiedScore {
                complexity_factor: 50.0,
                coverage_factor: 80.0,
                dependency_factor: 50.0,
                role_multiplier: 2.0,
                final_score: 50.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: false,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Fix".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 100.0,
                risk_reduction: 10.0,
                coverage_improvement: 100.0,
                lines_reduction: 500,
            },
            transitive_coverage: None,
            file_context: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 5,
            cognitive_complexity: 5,
            is_pure: Some(true),
            purity_confidence: Some(1.0),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        };

        assert_eq!(get_coverage(&item), None);
    }

    #[test]
    fn test_get_coverage_returns_direct_coverage() {
        use crate::priority::{
            ActionableRecommendation, DebtType, FunctionRole, ImpactMetrics, Location,
            TransitiveCoverage, UnifiedScore,
        };
        use std::path::PathBuf;

        let item = UnifiedDebtItem {
            location: Location {
                file: PathBuf::from("test.rs"),
                line: 10,
                function: "test_fn".to_string(),
            },
            debt_type: DebtType::ComplexityHotspot {
                cyclomatic: 15,
                cognitive: 25,
            },
            unified_score: UnifiedScore {
                complexity_factor: 50.0,
                coverage_factor: 80.0,
                dependency_factor: 50.0,
                role_multiplier: 2.0,
                final_score: 50.0,
                base_score: None,
                exponential_factor: None,
                risk_boost: None,
                pre_adjustment_score: None,
                adjustment_applied: None,
                purity_factor: None,
                refactorability_factor: None,
                pattern_factor: None,
                debt_adjustment: None,
                pre_normalization_score: None,
                structural_multiplier: Some(1.0),
                has_coverage_data: true,
                contextual_risk_multiplier: None,
                pre_contextual_score: None,
            },
            function_role: FunctionRole::PureLogic,
            recommendation: ActionableRecommendation {
                primary_action: "Fix".to_string(),
                rationale: "Test".to_string(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                complexity_reduction: 100.0,
                risk_reduction: 10.0,
                coverage_improvement: 100.0,
                lines_reduction: 500,
            },
            transitive_coverage: Some(TransitiveCoverage {
                direct: 75.5,
                transitive: 80.0,
                propagated_from: vec![],
                uncovered_lines: vec![],
            }),
            file_context: None,
            upstream_dependencies: 0,
            downstream_dependencies: 0,
            upstream_callers: vec![],
            downstream_callees: vec![],
            upstream_production_callers: vec![],
            upstream_test_callers: vec![],
            production_blast_radius: 0,
            nesting_depth: 1,
            function_length: 10,
            cyclomatic_complexity: 5,
            cognitive_complexity: 5,
            is_pure: Some(true),
            purity_confidence: Some(1.0),
            purity_level: None,
            god_object_indicators: None,
            tier: None,
            function_context: None,
            context_confidence: None,
            contextual_recommendation: None,
            pattern_analysis: None,
            context_multiplier: None,
            context_type: None,
            language_specific: None,
            detected_pattern: None,
            contextual_risk: None,
            file_line_count: None,
            responsibility_category: None,
            error_swallowing_count: None,
            error_swallowing_patterns: None,
            entropy_analysis: None,
            context_suggestion: None,
        };

        assert_eq!(get_coverage(&item), Some(75.5));
    }
}
