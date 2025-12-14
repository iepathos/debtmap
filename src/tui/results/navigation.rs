//! Keyboard navigation handling.
//!
//! This module handles keyboard input and delegates to the navigation state
//! machine (nav_state.rs) for state transitions. Guards are used to validate
//! transitions, and history is tracked for proper back navigation.
//!
//! Following Stillwater philosophy:
//! - Pure action determination in `list_actions` module
//! - Imperative shell (this module) executes the actions

use super::filter::{CoverageFilter, Filter, SeverityFilter};
use super::list_actions::{determine_list_action, ListAction, ListActionContext};
use super::nav_state::can_enter_help;
use super::sort::SortCriteria;
use super::{app::ResultsApp, detail_page::DetailPage, page_availability, view_mode::ViewMode};
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

        ListAction::ToggleGrouping => {
            app.list_mut().toggle_grouping();
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

        ListAction::OpenInEditor => {
            if let Some(item) = app.selected_item() {
                super::actions::open_in_editor(&item.location.file, Some(item.location.line))?;
                app.request_redraw();
            }
        }
    }

    Ok(false)
}

/// Handle keys in detail view
fn handle_detail_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match key.code {
        // Back - use history-based navigation
        KeyCode::Esc | KeyCode::Char('q') => {
            navigate_back(app);
        }

        // Page navigation (only cycles through available pages)
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
            let new_page = page_availability::next_available_page(
                app.nav().detail_page,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            );
            app.nav_mut().detail_page = new_page;
        }
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
            let new_page = page_availability::prev_available_page(
                app.nav().detail_page,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            );
            app.nav_mut().detail_page = new_page;
        }

        // Jump to specific page (only if available)
        KeyCode::Char('1') => {
            if page_availability::is_page_available(
                DetailPage::Overview,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            ) {
                app.nav_mut().detail_page = DetailPage::Overview;
            }
        }
        KeyCode::Char('2') => {
            if page_availability::is_page_available(
                DetailPage::Dependencies,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            ) {
                app.nav_mut().detail_page = DetailPage::Dependencies;
            }
        }
        KeyCode::Char('3') => {
            if page_availability::is_page_available(
                DetailPage::GitContext,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            ) {
                app.nav_mut().detail_page = DetailPage::GitContext;
            }
        }
        KeyCode::Char('4') => {
            if page_availability::is_page_available(
                DetailPage::Patterns,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            ) {
                app.nav_mut().detail_page = DetailPage::Patterns;
            }
        }
        KeyCode::Char('5') => {
            if page_availability::is_page_available(
                DetailPage::DataFlow,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            ) {
                app.nav_mut().detail_page = DetailPage::DataFlow;
            }
        }
        KeyCode::Char('6') => {
            if page_availability::is_page_available(
                DetailPage::Responsibilities,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            ) {
                app.nav_mut().detail_page = DetailPage::Responsibilities;
            }
        }

        // Navigate to next/previous item (preserve page if available)
        KeyCode::Down | KeyCode::Char('j') => {
            move_selection(app, 1);
            ensure_valid_page(app);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            move_selection(app, -1);
            ensure_valid_page(app);
        }

        // Actions
        KeyCode::Char('c') => {
            if let Some(item) = app.selected_item() {
                let detail_page = app.nav().detail_page;
                let message = super::actions::copy_page_to_clipboard(item, detail_page, app)?;
                app.set_status_message(message);
            }
        }
        KeyCode::Char('e') | KeyCode::Char('o') => {
            if let Some(item) = app.selected_item() {
                super::actions::open_in_editor(&item.location.file, Some(item.location.line))?;
                app.request_redraw(); // Force redraw after editor suspends/resumes TUI
            }
        }

        // Help - guarded transition with history
        KeyCode::Char('?') => {
            if can_enter_help(app.nav().view_mode) {
                app.nav_mut().push_and_set_view(ViewMode::Help);
            }
        }

        _ => {}
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
