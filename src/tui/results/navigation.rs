//! Keyboard navigation handling.
//!
//! This module handles keyboard input and delegates to the navigation state
//! machine (nav_state.rs) for state transitions. Guards are used to validate
//! transitions, and history is tracked for proper back navigation.

use super::app::{ResultsApp, ViewMode};
use super::filter::{CoverageFilter, Filter, SeverityFilter};
use super::nav_state::{self, can_enter_detail, can_enter_dsm, can_enter_help};
use super::sort::SortCriteria;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keyboard input and return true if should quit
pub fn handle_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    // Clear status message on any key press (except on first press that sets it)
    app.clear_status_message();

    match app.view_mode() {
        ViewMode::List => handle_list_key(app, key),
        ViewMode::Detail => handle_detail_key(app, key),
        ViewMode::Search => handle_search_key(app, key),
        ViewMode::SortMenu => handle_sort_menu_key(app, key),
        ViewMode::FilterMenu => handle_filter_menu_key(app, key),
        ViewMode::Help => handle_help_key(app, key),
        ViewMode::Dsm => handle_dsm_key(app, key),
    }
}

/// Handle keys in list view
fn handle_list_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match key.code {
        // Quit
        KeyCode::Char('q') => return Ok(true),

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            move_selection(app, -1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            move_selection(app, 1);
        }
        KeyCode::Char('g') | KeyCode::Home => {
            // Go to top
            app.set_selected_index(0);
            app.set_scroll_offset(0);
        }
        KeyCode::Char('G') => {
            // Toggle grouping
            app.toggle_grouping();
        }
        KeyCode::End => {
            // Go to bottom
            let last = app.item_count().saturating_sub(1);
            app.set_selected_index(last);
            adjust_scroll(app);
        }
        KeyCode::PageUp => {
            let page_size = 20; // Approximate page size
            move_selection(app, -(page_size as isize));
        }
        KeyCode::PageDown => {
            let page_size = 20;
            move_selection(app, page_size);
        }

        // Enter detail view - guarded transition
        KeyCode::Enter => {
            if can_enter_detail(app.view_mode(), app.has_items(), app.has_selection()) {
                app.push_nav_history(ViewMode::List);
                app.set_view_mode(ViewMode::Detail);
            }
        }

        // Search - guarded transition
        KeyCode::Char('/') => {
            if nav_state::can_enter_search(app.view_mode()) {
                app.search_mut().clear();
                app.push_nav_history(ViewMode::List);
                app.set_view_mode(ViewMode::Search);
            }
        }

        // Sort menu - guarded transition
        KeyCode::Char('s') => {
            if nav_state::can_enter_sort_menu(app.view_mode()) {
                app.push_nav_history(ViewMode::List);
                app.set_view_mode(ViewMode::SortMenu);
            }
        }

        // Filter menu - guarded transition
        KeyCode::Char('f') => {
            if nav_state::can_enter_filter_menu(app.view_mode()) {
                app.push_nav_history(ViewMode::List);
                app.set_view_mode(ViewMode::FilterMenu);
            }
        }

        // Help - guarded transition
        KeyCode::Char('?') => {
            if can_enter_help(app.view_mode()) {
                app.push_nav_history(ViewMode::List);
                app.set_view_mode(ViewMode::Help);
            }
        }

        // DSM view (Spec 205) - guarded transition
        KeyCode::Char('m') => {
            if can_enter_dsm(app.view_mode(), app.dsm_enabled()) {
                app.push_nav_history(ViewMode::List);
                app.set_view_mode(ViewMode::Dsm);
            }
        }

        // Actions (clipboard, editor)
        KeyCode::Char('c') => {
            if let Some(item) = app.selected_item() {
                let message = super::actions::copy_path_to_clipboard(&item.location.file)?;
                app.set_status_message(message);
            }
        }
        KeyCode::Char('e') => {
            if let Some(item) = app.selected_item() {
                super::actions::open_in_editor(&item.location.file, Some(item.location.line))?;
                app.request_redraw(); // Force redraw after editor suspends/resumes TUI
            }
        }
        KeyCode::Char('o') => {
            if let Some(item) = app.selected_item() {
                super::actions::open_in_editor(&item.location.file, Some(item.location.line))?;
                app.request_redraw(); // Force redraw after editor suspends/resumes TUI
            }
        }

        _ => {}
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
            app.next_available_page();
        }
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => {
            app.prev_available_page();
        }

        // Jump to specific page (only if available)
        KeyCode::Char('1') => {
            if app.is_page_available(super::app::DetailPage::Overview) {
                app.set_detail_page(super::app::DetailPage::Overview);
            }
        }
        KeyCode::Char('2') => {
            if app.is_page_available(super::app::DetailPage::Dependencies) {
                app.set_detail_page(super::app::DetailPage::Dependencies);
            }
        }
        KeyCode::Char('3') => {
            if app.is_page_available(super::app::DetailPage::GitContext) {
                app.set_detail_page(super::app::DetailPage::GitContext);
            }
        }
        KeyCode::Char('4') => {
            if app.is_page_available(super::app::DetailPage::Patterns) {
                app.set_detail_page(super::app::DetailPage::Patterns);
            }
        }
        KeyCode::Char('5') => {
            if app.is_page_available(super::app::DetailPage::DataFlow) {
                app.set_detail_page(super::app::DetailPage::DataFlow);
            }
        }
        KeyCode::Char('6') => {
            if app.is_page_available(super::app::DetailPage::Responsibilities) {
                app.set_detail_page(super::app::DetailPage::Responsibilities);
            }
        }

        // Navigate to next/previous item (preserve page if available)
        KeyCode::Down | KeyCode::Char('j') => {
            move_selection(app, 1);
            app.ensure_valid_page();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            move_selection(app, -1);
            app.ensure_valid_page();
        }

        // Actions
        KeyCode::Char('c') => {
            if let Some(item) = app.selected_item() {
                let message = super::actions::copy_page_to_clipboard(item, app.detail_page(), app)?;
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
            if can_enter_help(app.view_mode()) {
                app.push_nav_history(ViewMode::Detail);
                app.set_view_mode(ViewMode::Help);
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
            app.search_mut().clear();
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
            app.search_mut().insert_char(c);
        }
        KeyCode::Backspace => {
            app.search_mut().delete_char();
        }
        KeyCode::Delete => {
            app.search_mut().delete_char_forward();
        }
        KeyCode::Left => {
            app.search_mut().move_cursor_left();
        }
        KeyCode::Right => {
            app.search_mut().move_cursor_right();
        }
        KeyCode::Home => {
            app.search_mut().move_cursor_home();
        }
        KeyCode::End => {
            app.search_mut().move_cursor_end();
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

/// Handle keys in DSM view (Spec 205)
fn handle_dsm_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match key.code {
        // Back - use history-based navigation
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('m') => {
            navigate_back(app);
        }

        // Help - guarded transition with history
        KeyCode::Char('?') => {
            if can_enter_help(app.view_mode()) {
                app.push_nav_history(ViewMode::Dsm);
                app.set_view_mode(ViewMode::Help);
            }
        }

        // Navigation within DSM (scroll if matrix is large)
        KeyCode::Up | KeyCode::Char('k') => {
            let current = app.dsm_scroll_y();
            if current > 0 {
                app.set_dsm_scroll_y(current - 1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.set_dsm_scroll_y(app.dsm_scroll_y() + 1);
        }
        KeyCode::Left | KeyCode::Char('h') => {
            let current = app.dsm_scroll_x();
            if current > 0 {
                app.set_dsm_scroll_x(current - 1);
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.set_dsm_scroll_x(app.dsm_scroll_x() + 1);
        }

        // Reset scroll
        KeyCode::Home | KeyCode::Char('g') => {
            app.set_dsm_scroll_x(0);
            app.set_dsm_scroll_y(0);
        }

        _ => {}
    }

    Ok(false)
}

/// Move selection by delta (can be negative)
fn move_selection(app: &mut ResultsApp, delta: isize) {
    if app.item_count() == 0 {
        return;
    }

    let current = app.selected_index() as isize;
    let new_index = (current + delta).max(0).min(app.item_count() as isize - 1) as usize;

    app.set_selected_index(new_index);
    adjust_scroll(app);
}

/// Adjust scroll offset to keep selection visible
fn adjust_scroll(app: &mut ResultsApp) {
    let selected = app.selected_index();
    let scroll = app.scroll_offset();
    let (_, height) = app.terminal_size();

    // Visible area (accounting for header and footer)
    let visible_rows = height.saturating_sub(6) as usize; // 3 lines header, 3 lines footer

    if selected < scroll {
        // Selection above visible area
        app.set_scroll_offset(selected);
    } else if selected >= scroll + visible_rows {
        // Selection below visible area
        app.set_scroll_offset(selected.saturating_sub(visible_rows - 1));
    }
}

/// Navigate back using history-based navigation.
///
/// Uses the navigation history to return to the previous view.
/// If no history exists and not at root (List), returns to List.
/// If already at root with no history, does nothing.
fn navigate_back(app: &mut ResultsApp) {
    if let Some(previous) = app.pop_nav_history() {
        app.set_view_mode(previous);
    } else if app.view_mode() != ViewMode::List {
        app.set_view_mode(ViewMode::List);
    }
    // Already at root with no history - do nothing
}
