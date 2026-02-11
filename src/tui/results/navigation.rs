//! Keyboard navigation handling.
//!
//! This module handles keyboard input and delegates to the navigation state
//! machine (nav_state.rs) for state transitions. Guards are used to validate
//! transitions, and history is tracked for proper back navigation.
//!
//! Following Stillwater philosophy:
//! - Pure action determination in `list_actions` module
//! - Imperative shell (this module) executes the actions

use super::detail_actions::{classify_detail_key, DetailAction, DetailActionContext};
use super::filter::{CoverageFilter, Filter, SeverityFilter};
use super::list_actions::{determine_list_action, ListAction, ListActionContext};
use super::nav_state::can_enter_help;
use super::sort::SortCriteria;
use super::{app::ResultsApp, page_availability, view_mode::ViewMode};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keyboard input and return true if should quit
pub fn handle_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    // Clear status message on any key press (except on first press that sets it)
    app.clear_status_message();

    match app.nav().view_mode {
        ViewMode::List => handle_list_key(app, key),
        ViewMode::Detail => handle_detail_key(app, key),
        ViewMode::Search => handle_search_key(app, key),
        ViewMode::SortMenu => handle_sort_menu_key(app, key),
        ViewMode::FilterMenu => handle_filter_menu_key(app, key),
        ViewMode::Help => handle_help_key(app, key),
    }
}

/// Handle keys in list view.
///
/// Uses the Stillwater pattern: pure action determination + imperative execution.
/// The `determine_list_action` function is pure and testable; this function
/// is the thin imperative shell that executes the determined action.
fn handle_list_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    // Build context for pure action determination
    let ctx = ListActionContext {
        has_items: app.has_items(),
        has_selection: app.has_selection(),
    };

    // Pure function call - determine which action to take
    let Some(action) = determine_list_action(key, ctx) else {
        return Ok(false);
    };

    // Imperative shell - execute the action
    execute_list_action(app, action)
}

/// Execute a list action (imperative shell).
///
/// This function contains all the side effects for list view actions.
/// It's kept separate from the pure action determination to maintain
/// clear boundaries between pure logic and effects.
fn execute_list_action(app: &mut ResultsApp, action: ListAction) -> Result<bool> {
    match action {
        ListAction::Quit => return Ok(true),

        ListAction::MoveUp => move_selection(app, -1),
        ListAction::MoveDown => move_selection(app, 1),

        ListAction::JumpToTop => {
            let count = app.item_count();
            app.list_mut().set_selected_index(0, count);
            app.list_mut().set_scroll_offset(0);
        }

        ListAction::JumpToBottom => {
            let last = app.item_count().saturating_sub(1);
            let count = app.item_count();
            app.list_mut().set_selected_index(last, count);
            adjust_scroll(app);
        }

        ListAction::PageUp => {
            let page_size = 20;
            move_selection(app, -(page_size as isize));
        }

        ListAction::PageDown => {
            let page_size = 20;
            move_selection(app, page_size);
        }

        ListAction::EnterDetail => {
            app.nav_mut().push_and_set_view(ViewMode::Detail);
        }

        ListAction::EnterSearch => {
            app.query_mut().search_mut().clear();
            app.nav_mut().push_and_set_view(ViewMode::Search);
        }

        ListAction::OpenSortMenu => {
            app.nav_mut().push_and_set_view(ViewMode::SortMenu);
        }

        ListAction::OpenFilterMenu => {
            app.nav_mut().push_and_set_view(ViewMode::FilterMenu);
        }

        ListAction::ShowHelp => {
            app.nav_mut().push_and_set_view(ViewMode::Help);
        }

        ListAction::CopyPath => {
            if let Some(item) = app.selected_item() {
                let message = super::actions::copy_path_to_clipboard(&item.location.file)?;
                app.set_status_message(message);
            }
        }

        ListAction::CopyItemAsLlm => {
            if let Some(item) = app.selected_item() {
                let message = super::actions::copy_item_as_llm_to_clipboard(item)?;
                app.set_status_message(message);
            }
        }

        ListAction::OpenInEditor => {
            if let Some(item) = app.selected_item() {
                super::actions::open_in_editor(&item.location.file, Some(item.location.line))?;
                app.request_redraw();
            }
        }
    }

    Ok(false)
}

/// Handle keys in detail view.
///
/// Uses the Stillwater pattern: pure action determination + imperative execution.
/// The `classify_detail_key` function is pure and testable; this function
/// is the thin imperative shell that executes the determined action.
fn handle_detail_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    // Build context for pure action determination
    let ctx = DetailActionContext::new(app.nav().detail_page);

    // Pure function call - determine which action to take
    let Some(action) = classify_detail_key(key, ctx) else {
        return Ok(false);
    };

    // Imperative shell - execute the action
    execute_detail_action(app, action)
}

/// Execute a detail action (imperative shell).
///
/// This function contains all the side effects for detail view actions.
/// It's kept separate from the pure action determination to maintain
/// clear boundaries between pure logic and effects.
fn execute_detail_action(app: &mut ResultsApp, action: DetailAction) -> Result<bool> {
    match action {
        DetailAction::NavigateBack => {
            navigate_back(app);
        }

        DetailAction::NextPage => {
            let new_page = page_availability::next_available_page(
                app.nav().detail_page,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            );
            app.nav_mut().detail_page = new_page;
            // Reset scroll when changing pages
            app.nav_mut().reset_detail_scroll();
        }

        DetailAction::PrevPage => {
            let new_page = page_availability::prev_available_page(
                app.nav().detail_page,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            );
            app.nav_mut().detail_page = new_page;
            // Reset scroll when changing pages
            app.nav_mut().reset_detail_scroll();
        }

        DetailAction::JumpToPage(page) => {
            if page_availability::is_page_available(
                page,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            ) {
                app.nav_mut().detail_page = page;
                // Reset scroll when changing pages
                app.nav_mut().reset_detail_scroll();
            }
        }

        DetailAction::MoveSelection(delta) => {
            move_selection(app, delta as isize);
            ensure_valid_page(app);
            // Reset scroll when changing items
            app.nav_mut().reset_detail_scroll();
        }

        // Content scrolling actions
        DetailAction::ScrollUp => {
            app.nav_mut().detail_scroll.scroll_up();
        }

        DetailAction::ScrollDown => {
            app.nav_mut().detail_scroll.scroll_down();
        }

        DetailAction::ScrollHalfPageUp => {
            // Scroll up by half page (approximated by repeated scroll_up)
            for _ in 0..10 {
                app.nav_mut().detail_scroll.scroll_up();
            }
        }

        DetailAction::ScrollHalfPageDown => {
            // Scroll down by half page (approximated by repeated scroll_down)
            for _ in 0..10 {
                app.nav_mut().detail_scroll.scroll_down();
            }
        }

        DetailAction::ScrollPageUp => {
            app.nav_mut().detail_scroll.scroll_page_up();
        }

        DetailAction::ScrollPageDown => {
            app.nav_mut().detail_scroll.scroll_page_down();
        }

        DetailAction::ScrollToTop => {
            app.nav_mut().detail_scroll.scroll_to_top();
        }

        DetailAction::ScrollToBottom => {
            app.nav_mut().detail_scroll.scroll_to_bottom();
        }

        DetailAction::CopyPage => {
            if let Some(item) = app.selected_item() {
                let detail_page = app.nav().detail_page;
                let message = super::actions::copy_page_to_clipboard(item, detail_page, app)?;
                app.set_status_message(message);
            }
        }

        DetailAction::CopyItemAsLlm => {
            if let Some(item) = app.selected_item() {
                let message = super::actions::copy_item_as_llm_to_clipboard(item)?;
                app.set_status_message(message);
            }
        }

        DetailAction::OpenInEditor => {
            if let Some(item) = app.selected_item() {
                super::actions::open_in_editor(&item.location.file, Some(item.location.line))?;
                app.request_redraw();
            }
        }

        DetailAction::ShowHelp => {
            if can_enter_help(app.nav().view_mode) {
                app.nav_mut().push_and_set_view(ViewMode::Help);
            }
        }
    }

    Ok(false)
}

/// Handle keys in search mode
fn handle_search_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match key.code {
        // Exit search - use history-based navigation
        KeyCode::Esc => {
            app.query_mut().search_mut().clear();
            app.apply_search();
            navigate_back(app);
        }

        // Execute search and go back
        KeyCode::Enter => {
            app.apply_search();
            navigate_back(app);
        }

        // Edit query
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.query_mut().search_mut().insert_char(c);
        }
        KeyCode::Backspace => {
            app.query_mut().search_mut().delete_char();
        }
        KeyCode::Delete => {
            app.query_mut().search_mut().delete_char_forward();
        }
        KeyCode::Left => {
            app.query_mut().search_mut().move_cursor_left();
        }
        KeyCode::Right => {
            app.query_mut().search_mut().move_cursor_right();
        }
        KeyCode::Home => {
            app.query_mut().search_mut().move_cursor_home();
        }
        KeyCode::End => {
            app.query_mut().search_mut().move_cursor_end();
        }

        _ => {}
    }

    Ok(false)
}

/// Handle keys in sort menu
fn handle_sort_menu_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match key.code {
        // Back - use history-based navigation
        KeyCode::Esc | KeyCode::Char('q') => {
            navigate_back(app);
        }

        KeyCode::Char('1') => {
            app.set_sort_by(SortCriteria::Score);
            navigate_back(app);
        }
        KeyCode::Char('2') => {
            app.set_sort_by(SortCriteria::Coverage);
            navigate_back(app);
        }
        KeyCode::Char('3') => {
            app.set_sort_by(SortCriteria::Complexity);
            navigate_back(app);
        }
        KeyCode::Char('4') => {
            app.set_sort_by(SortCriteria::FilePath);
            navigate_back(app);
        }
        KeyCode::Char('5') => {
            app.set_sort_by(SortCriteria::FunctionName);
            navigate_back(app);
        }

        _ => {}
    }

    Ok(false)
}

/// Handle keys in filter menu
fn handle_filter_menu_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match key.code {
        // Back - use history-based navigation
        KeyCode::Esc | KeyCode::Char('q') => {
            navigate_back(app);
        }

        // Severity filters
        KeyCode::Char('1') => {
            app.add_filter(Filter::Severity(SeverityFilter::Critical));
            navigate_back(app);
        }
        KeyCode::Char('2') => {
            app.add_filter(Filter::Severity(SeverityFilter::High));
            navigate_back(app);
        }
        KeyCode::Char('3') => {
            app.add_filter(Filter::Severity(SeverityFilter::Medium));
            navigate_back(app);
        }
        KeyCode::Char('4') => {
            app.add_filter(Filter::Severity(SeverityFilter::Low));
            navigate_back(app);
        }

        // Coverage filters
        KeyCode::Char('n') => {
            app.add_filter(Filter::Coverage(CoverageFilter::None));
            navigate_back(app);
        }
        KeyCode::Char('l') => {
            app.add_filter(Filter::Coverage(CoverageFilter::Low));
            navigate_back(app);
        }
        KeyCode::Char('m') => {
            app.add_filter(Filter::Coverage(CoverageFilter::Medium));
            navigate_back(app);
        }
        KeyCode::Char('h') => {
            app.add_filter(Filter::Coverage(CoverageFilter::High));
            navigate_back(app);
        }

        // Clear filters
        KeyCode::Char('c') => {
            app.clear_filters();
            navigate_back(app);
        }

        _ => {}
    }

    Ok(false)
}

/// Handle keys in help overlay
fn handle_help_key(app: &mut ResultsApp, _key: KeyEvent) -> Result<bool> {
    // Any key exits help - use history-based navigation
    navigate_back(app);
    Ok(false)
}

/// Move selection by delta (can be negative)
fn move_selection(app: &mut ResultsApp, delta: isize) {
    let count = app.item_count();
    if count == 0 {
        return;
    }

    let current = app.list().selected_index() as isize;
    let new_index = (current + delta).max(0).min(count as isize - 1) as usize;

    app.list_mut().set_selected_index(new_index, count);
    adjust_scroll(app);
}

/// Adjust scroll offset to keep selection visible
fn adjust_scroll(app: &mut ResultsApp) {
    let selected = app.list().selected_index();
    let scroll = app.list().scroll_offset();
    let (_, height) = app.terminal_size();

    // Visible area (accounting for header and footer)
    let visible_rows = height.saturating_sub(6) as usize; // 3 lines header, 3 lines footer

    if selected < scroll {
        // Selection above visible area
        app.list_mut().set_scroll_offset(selected);
    } else if selected >= scroll + visible_rows {
        // Selection below visible area
        app.list_mut()
            .set_scroll_offset(selected.saturating_sub(visible_rows - 1));
    }
}

/// Navigate back using history-based navigation.
///
/// Uses the navigation history to return to the previous view.
/// If no history exists and not at root (List), returns to List.
/// If already at root with no history, does nothing.
fn navigate_back(app: &mut ResultsApp) {
    if app.nav_mut().go_back().is_none() && app.nav().view_mode != ViewMode::List {
        app.nav_mut().view_mode = ViewMode::List;
    }
    // Already at root with no history - do nothing
}

/// Ensure current page is valid for the selected item (coordination helper)
fn ensure_valid_page(app: &mut ResultsApp) {
    let new_page = page_availability::ensure_valid_page(
        app.nav().detail_page,
        app.selected_item(),
        &app.analysis().data_flow_graph,
    );
    app.nav_mut().detail_page = new_page;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::priority::call_graph::CallGraph;
    use crate::priority::semantic_classifier::FunctionRole;
    use crate::priority::unified_scorer::{Location, UnifiedScore};
    use crate::priority::{
        ActionableRecommendation, DebtType, ImpactMetrics, UnifiedAnalysis, UnifiedDebtItem,
    };

    /// Create a test UnifiedDebtItem with minimal required fields.
    fn create_test_item(file: &str, function: &str, line: usize) -> UnifiedDebtItem {
        UnifiedDebtItem {
            location: Location {
                file: file.into(),
                function: function.into(),
                line,
            },
            debt_type: DebtType::Complexity {
                cyclomatic: 5,
                cognitive: 3,
            },
            unified_score: UnifiedScore {
                complexity_factor: 0.0,
                coverage_factor: 10.0,
                dependency_factor: 0.0,
                role_multiplier: 1.0,
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
                primary_action: "Test".into(),
                rationale: "Test".into(),
                implementation_steps: vec![],
                related_items: vec![],
                steps: None,
                estimated_effort_hours: None,
            },
            expected_impact: ImpactMetrics {
                risk_reduction: 0.0,
                complexity_reduction: 0.0,
                coverage_improvement: 0.0,
                lines_reduction: 0,
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
            cognitive_complexity: 3,
            is_pure: Some(false),
            purity_confidence: Some(0.0),
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
        }
    }

    /// Create a test ResultsApp with the given number of items.
    fn create_test_app(item_count: usize) -> ResultsApp {
        let mut analysis = UnifiedAnalysis::new(CallGraph::new());
        for i in 0..item_count {
            analysis.items.push_back(create_test_item(
                &format!("test_{}.rs", i),
                &format!("fn_{}", i),
                i + 1,
            ));
        }
        ResultsApp::new(analysis)
    }

    // ============================================================================
    // execute_list_action tests
    // ============================================================================

    #[test]
    fn test_execute_list_action_quit_returns_true() {
        let mut app = create_test_app(5);
        let result = execute_list_action(&mut app, ListAction::Quit).unwrap();
        assert!(result, "Quit action should return true");
    }

    #[test]
    fn test_execute_list_action_move_down_increments_selection() {
        let mut app = create_test_app(5);
        assert_eq!(app.list().selected_index(), 0);

        execute_list_action(&mut app, ListAction::MoveDown).unwrap();
        assert_eq!(app.list().selected_index(), 1);

        execute_list_action(&mut app, ListAction::MoveDown).unwrap();
        assert_eq!(app.list().selected_index(), 2);
    }

    #[test]
    fn test_execute_list_action_move_up_decrements_selection() {
        let mut app = create_test_app(5);
        // Move down first
        execute_list_action(&mut app, ListAction::MoveDown).unwrap();
        execute_list_action(&mut app, ListAction::MoveDown).unwrap();
        assert_eq!(app.list().selected_index(), 2);

        execute_list_action(&mut app, ListAction::MoveUp).unwrap();
        assert_eq!(app.list().selected_index(), 1);
    }

    #[test]
    fn test_execute_list_action_move_up_at_top_stays_at_zero() {
        let mut app = create_test_app(5);
        assert_eq!(app.list().selected_index(), 0);

        execute_list_action(&mut app, ListAction::MoveUp).unwrap();
        assert_eq!(app.list().selected_index(), 0);
    }

    #[test]
    fn test_execute_list_action_move_down_at_bottom_stays_at_last() {
        let mut app = create_test_app(3);
        // Move to last
        execute_list_action(&mut app, ListAction::JumpToBottom).unwrap();
        let last_idx = app.list().selected_index();

        execute_list_action(&mut app, ListAction::MoveDown).unwrap();
        assert_eq!(app.list().selected_index(), last_idx);
    }

    #[test]
    fn test_execute_list_action_jump_to_top() {
        let mut app = create_test_app(5);
        // Move down first
        execute_list_action(&mut app, ListAction::MoveDown).unwrap();
        execute_list_action(&mut app, ListAction::MoveDown).unwrap();
        assert_eq!(app.list().selected_index(), 2);

        execute_list_action(&mut app, ListAction::JumpToTop).unwrap();
        assert_eq!(app.list().selected_index(), 0);
        assert_eq!(app.list().scroll_offset(), 0);
    }

    #[test]
    fn test_execute_list_action_jump_to_bottom() {
        let mut app = create_test_app(5);
        assert_eq!(app.list().selected_index(), 0);

        execute_list_action(&mut app, ListAction::JumpToBottom).unwrap();
        // Items are grouped, so we get the actual item count
        let expected_last = app.item_count().saturating_sub(1);
        assert_eq!(app.list().selected_index(), expected_last);
    }

    #[test]
    fn test_execute_list_action_page_down() {
        let mut app = create_test_app(50);
        assert_eq!(app.list().selected_index(), 0);

        execute_list_action(&mut app, ListAction::PageDown).unwrap();
        // Page size is 20, so should move to 20
        assert_eq!(app.list().selected_index(), 20);
    }

    #[test]
    fn test_execute_list_action_page_up() {
        let mut app = create_test_app(50);
        // Move to position 25
        for _ in 0..25 {
            execute_list_action(&mut app, ListAction::MoveDown).unwrap();
        }
        assert_eq!(app.list().selected_index(), 25);

        execute_list_action(&mut app, ListAction::PageUp).unwrap();
        // Page size is 20, so should move to 5
        assert_eq!(app.list().selected_index(), 5);
    }

    #[test]
    fn test_execute_list_action_enter_detail_changes_view_mode() {
        let mut app = create_test_app(5);
        assert_eq!(app.nav().view_mode, ViewMode::List);

        execute_list_action(&mut app, ListAction::EnterDetail).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::Detail);
    }

    #[test]
    fn test_execute_list_action_enter_search_changes_view_mode() {
        let mut app = create_test_app(5);
        assert_eq!(app.nav().view_mode, ViewMode::List);

        execute_list_action(&mut app, ListAction::EnterSearch).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::Search);
    }

    #[test]
    fn test_execute_list_action_open_sort_menu_changes_view_mode() {
        let mut app = create_test_app(5);
        assert_eq!(app.nav().view_mode, ViewMode::List);

        execute_list_action(&mut app, ListAction::OpenSortMenu).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::SortMenu);
    }

    #[test]
    fn test_execute_list_action_open_filter_menu_changes_view_mode() {
        let mut app = create_test_app(5);
        assert_eq!(app.nav().view_mode, ViewMode::List);

        execute_list_action(&mut app, ListAction::OpenFilterMenu).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::FilterMenu);
    }

    #[test]
    fn test_execute_list_action_show_help_changes_view_mode() {
        let mut app = create_test_app(5);
        assert_eq!(app.nav().view_mode, ViewMode::List);

        execute_list_action(&mut app, ListAction::ShowHelp).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::Help);
    }

    #[test]
    fn test_execute_list_action_non_quit_returns_false() {
        let mut app = create_test_app(5);

        let actions = [
            ListAction::MoveUp,
            ListAction::MoveDown,
            ListAction::JumpToTop,
            ListAction::JumpToBottom,
            ListAction::PageUp,
            ListAction::PageDown,
            ListAction::EnterDetail,
            ListAction::EnterSearch,
            ListAction::OpenSortMenu,
            ListAction::OpenFilterMenu,
            ListAction::ShowHelp,
        ];

        for action in actions {
            let result = execute_list_action(&mut app, action).unwrap();
            assert!(!result, "Action {:?} should return false", action);
            // Reset to list view for next iteration
            app.nav_mut().view_mode = ViewMode::List;
        }
    }

    #[test]
    fn test_execute_list_action_on_empty_list() {
        let mut app = create_test_app(0);
        assert_eq!(app.item_count(), 0);

        // Navigation should not panic on empty list
        let result = execute_list_action(&mut app, ListAction::MoveDown).unwrap();
        assert!(!result);

        let result = execute_list_action(&mut app, ListAction::MoveUp).unwrap();
        assert!(!result);

        let result = execute_list_action(&mut app, ListAction::JumpToTop).unwrap();
        assert!(!result);

        let result = execute_list_action(&mut app, ListAction::JumpToBottom).unwrap();
        assert!(!result);
    }

    // ============================================================================
    // execute_detail_action tests
    // ============================================================================

    #[test]
    fn test_execute_detail_action_navigate_back_returns_to_list() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);
        assert_eq!(app.nav().view_mode, ViewMode::Detail);

        execute_detail_action(&mut app, DetailAction::NavigateBack).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::List);
    }

    #[test]
    fn test_execute_detail_action_move_selection_changes_index() {
        let mut app = create_test_app(10);
        app.nav_mut().push_and_set_view(ViewMode::Detail);
        assert_eq!(app.list().selected_index(), 0);

        execute_detail_action(&mut app, DetailAction::MoveSelection(1)).unwrap();
        assert_eq!(app.list().selected_index(), 1);

        execute_detail_action(&mut app, DetailAction::MoveSelection(1)).unwrap();
        assert_eq!(app.list().selected_index(), 2);

        execute_detail_action(&mut app, DetailAction::MoveSelection(-1)).unwrap();
        assert_eq!(app.list().selected_index(), 1);
    }

    #[test]
    fn test_execute_detail_action_show_help_from_detail() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);
        assert_eq!(app.nav().view_mode, ViewMode::Detail);

        execute_detail_action(&mut app, DetailAction::ShowHelp).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::Help);
    }

    #[test]
    fn test_execute_detail_action_non_quit_returns_false() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        let actions = [
            DetailAction::NavigateBack,
            DetailAction::MoveSelection(1),
            DetailAction::ShowHelp,
        ];

        for action in actions {
            let result = execute_detail_action(&mut app, action).unwrap();
            assert!(!result, "Action {:?} should return false", action);
            // Reset view for next iteration
            app.nav_mut().view_mode = ViewMode::Detail;
        }
    }

    #[test]
    fn test_execute_detail_action_scroll_up() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        // Scroll down first to have something to scroll up from
        app.nav_mut().detail_scroll.scroll_down();
        app.nav_mut().detail_scroll.scroll_down();

        let result = execute_detail_action(&mut app, DetailAction::ScrollUp).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_scroll_down() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        let result = execute_detail_action(&mut app, DetailAction::ScrollDown).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_scroll_half_page_up() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        // Scroll down first
        for _ in 0..20 {
            app.nav_mut().detail_scroll.scroll_down();
        }

        let result = execute_detail_action(&mut app, DetailAction::ScrollHalfPageUp).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_scroll_half_page_down() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        let result = execute_detail_action(&mut app, DetailAction::ScrollHalfPageDown).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_scroll_page_up() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        let result = execute_detail_action(&mut app, DetailAction::ScrollPageUp).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_scroll_page_down() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        let result = execute_detail_action(&mut app, DetailAction::ScrollPageDown).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_scroll_to_top() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        // Scroll down first
        for _ in 0..10 {
            app.nav_mut().detail_scroll.scroll_down();
        }

        let result = execute_detail_action(&mut app, DetailAction::ScrollToTop).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_scroll_to_bottom() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        let result = execute_detail_action(&mut app, DetailAction::ScrollToBottom).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_next_page() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        let result = execute_detail_action(&mut app, DetailAction::NextPage).unwrap();
        assert!(!result);
        // Page should have changed (or stayed if no next page available)
        // Just verify no crash
    }

    #[test]
    fn test_execute_detail_action_prev_page() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        // First move to a page that isn't the first
        execute_detail_action(&mut app, DetailAction::NextPage).unwrap();

        let result = execute_detail_action(&mut app, DetailAction::PrevPage).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_jump_to_page() {
        use crate::tui::results::detail_page::DetailPage;

        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        // Jump to a specific page
        let result =
            execute_detail_action(&mut app, DetailAction::JumpToPage(DetailPage::Overview))
                .unwrap();
        assert!(!result);
        assert_eq!(app.nav().detail_page, DetailPage::Overview);
    }

    #[test]
    fn test_execute_detail_action_copy_page_sets_status() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        // Ensure we have a selected item
        assert!(app.selected_item().is_some());

        // CopyPage should not return quit signal and should set status
        let result = execute_detail_action(&mut app, DetailAction::CopyPage).unwrap();
        assert!(!result);

        // Status message should be set (either success or clipboard error)
        let status = app.status_message();
        assert!(status.is_some(), "CopyPage should set a status message");
    }

    #[test]
    fn test_execute_detail_action_copy_page_with_no_selection() {
        let mut app = create_test_app(0);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        // No items, so no selection
        assert!(app.selected_item().is_none());

        // Should not panic or error with no selection
        let result = execute_detail_action(&mut app, DetailAction::CopyPage).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_copy_item_as_llm_sets_status() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        assert!(app.selected_item().is_some());

        let result = execute_detail_action(&mut app, DetailAction::CopyItemAsLlm).unwrap();
        assert!(!result);

        // Status message should be set
        let status = app.status_message();
        assert!(
            status.is_some(),
            "CopyItemAsLlm should set a status message"
        );
    }

    #[test]
    fn test_execute_detail_action_copy_item_as_llm_with_no_selection() {
        let mut app = create_test_app(0);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        assert!(app.selected_item().is_none());

        let result = execute_detail_action(&mut app, DetailAction::CopyItemAsLlm).unwrap();
        assert!(!result);
    }

    #[test]
    #[ignore] // Requires terminal in raw mode for editor suspension
    fn test_execute_detail_action_open_in_editor() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        assert!(app.selected_item().is_some());

        // This test is ignored by default since it requires terminal context
        // Run with --ignored to test editor integration manually
        let result = execute_detail_action(&mut app, DetailAction::OpenInEditor).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_execute_detail_action_open_in_editor_with_no_selection() {
        let mut app = create_test_app(0);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        assert!(app.selected_item().is_none());

        // Should not panic or error with no selection
        let result = execute_detail_action(&mut app, DetailAction::OpenInEditor).unwrap();
        assert!(!result);
    }

    // ============================================================================
    // move_selection tests
    // ============================================================================

    #[test]
    fn test_move_selection_positive_delta() {
        let mut app = create_test_app(10);
        assert_eq!(app.list().selected_index(), 0);

        move_selection(&mut app, 3);
        assert_eq!(app.list().selected_index(), 3);
    }

    #[test]
    fn test_move_selection_negative_delta() {
        let mut app = create_test_app(10);
        move_selection(&mut app, 5);
        assert_eq!(app.list().selected_index(), 5);

        move_selection(&mut app, -2);
        assert_eq!(app.list().selected_index(), 3);
    }

    #[test]
    fn test_move_selection_clamps_at_zero() {
        let mut app = create_test_app(10);
        assert_eq!(app.list().selected_index(), 0);

        move_selection(&mut app, -5);
        assert_eq!(app.list().selected_index(), 0);
    }

    #[test]
    fn test_move_selection_clamps_at_max() {
        let mut app = create_test_app(5);
        let max_idx = app.item_count().saturating_sub(1);

        move_selection(&mut app, 100);
        assert_eq!(app.list().selected_index(), max_idx);
    }

    #[test]
    fn test_move_selection_empty_list_no_panic() {
        let mut app = create_test_app(0);
        // Should not panic
        move_selection(&mut app, 1);
        move_selection(&mut app, -1);
    }

    // ============================================================================
    // navigate_back tests
    // ============================================================================

    #[test]
    fn test_navigate_back_with_history() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);
        assert_eq!(app.nav().view_mode, ViewMode::Detail);

        navigate_back(&mut app);
        assert_eq!(app.nav().view_mode, ViewMode::List);
    }

    #[test]
    fn test_navigate_back_without_history_at_root() {
        let mut app = create_test_app(5);
        assert_eq!(app.nav().view_mode, ViewMode::List);

        navigate_back(&mut app);
        // Should stay at list
        assert_eq!(app.nav().view_mode, ViewMode::List);
    }

    #[test]
    fn test_navigate_back_without_history_not_at_root() {
        let mut app = create_test_app(5);
        // Set view mode directly without history
        app.nav_mut().view_mode = ViewMode::Detail;

        navigate_back(&mut app);
        // Should go back to List since no history
        assert_eq!(app.nav().view_mode, ViewMode::List);
    }

    // ============================================================================
    // handle_key integration tests
    // ============================================================================

    #[test]
    fn test_handle_key_q_quits_from_list() {
        let mut app = create_test_app(5);
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);

        let result = handle_key(&mut app, key).unwrap();
        assert!(result, "'q' should quit the application");
    }

    #[test]
    fn test_handle_key_navigation_in_list_view() {
        let mut app = create_test_app(5);
        assert_eq!(app.nav().view_mode, ViewMode::List);
        assert_eq!(app.list().selected_index(), 0);

        // Press 'j' to move down
        let j_key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        handle_key(&mut app, j_key).unwrap();
        assert_eq!(app.list().selected_index(), 1);

        // Press 'k' to move up
        let k_key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        handle_key(&mut app, k_key).unwrap();
        assert_eq!(app.list().selected_index(), 0);
    }

    #[test]
    fn test_handle_key_enter_to_detail_view() {
        let mut app = create_test_app(5);
        assert_eq!(app.nav().view_mode, ViewMode::List);

        let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        handle_key(&mut app, enter_key).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::Detail);
    }

    #[test]
    fn test_handle_key_esc_from_detail_returns_to_list() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Detail);

        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        handle_key(&mut app, esc_key).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::List);
    }

    #[test]
    fn test_handle_key_any_key_exits_help() {
        let mut app = create_test_app(5);
        app.nav_mut().push_and_set_view(ViewMode::Help);
        assert_eq!(app.nav().view_mode, ViewMode::Help);

        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        handle_key(&mut app, key).unwrap();
        assert_eq!(app.nav().view_mode, ViewMode::List);
    }
}
