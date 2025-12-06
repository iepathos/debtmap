//! Keyboard navigation handling.

use super::app::{ResultsApp, ViewMode};
use super::filter::{CoverageFilter, Filter, SeverityFilter};
use super::sort::SortCriteria;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Handle keyboard input and return true if should quit
pub fn handle_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match app.view_mode() {
        ViewMode::List => handle_list_key(app, key),
        ViewMode::Detail => handle_detail_key(app, key),
        ViewMode::Search => handle_search_key(app, key),
        ViewMode::SortMenu => handle_sort_menu_key(app, key),
        ViewMode::FilterMenu => handle_filter_menu_key(app, key),
        ViewMode::Help => handle_help_key(app, key),
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

        // Enter detail view
        KeyCode::Enter => {
            if app.item_count() > 0 {
                app.set_view_mode(ViewMode::Detail);
            }
        }

        // Search
        KeyCode::Char('/') => {
            app.search_mut().clear();
            app.set_view_mode(ViewMode::Search);
        }

        // Sort menu
        KeyCode::Char('s') => {
            app.set_view_mode(ViewMode::SortMenu);
        }

        // Filter menu
        KeyCode::Char('f') => {
            app.set_view_mode(ViewMode::FilterMenu);
        }

        // Help
        KeyCode::Char('?') => {
            app.set_view_mode(ViewMode::Help);
        }

        // Actions (clipboard, editor)
        KeyCode::Char('c') => {
            if let Some(item) = app.selected_item() {
                super::actions::copy_path_to_clipboard(&item.location.file)?;
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
        // Back to list
        KeyCode::Esc | KeyCode::Char('q') => {
            app.set_view_mode(ViewMode::List);
        }

        // Page navigation
        KeyCode::Tab | KeyCode::Right => {
            let next_page = app.detail_page().next();
            app.set_detail_page(next_page);
        }
        KeyCode::BackTab | KeyCode::Left => {
            let prev_page = app.detail_page().prev();
            app.set_detail_page(prev_page);
        }

        // Jump to specific page
        KeyCode::Char('1') => {
            app.set_detail_page(super::app::DetailPage::Overview);
        }
        KeyCode::Char('2') => {
            app.set_detail_page(super::app::DetailPage::Dependencies);
        }
        KeyCode::Char('3') => {
            app.set_detail_page(super::app::DetailPage::GitContext);
        }
        KeyCode::Char('4') => {
            app.set_detail_page(super::app::DetailPage::Patterns);
        }

        // Navigate to next/previous item (preserve page)
        KeyCode::Char('n') | KeyCode::Down | KeyCode::Char('j') => {
            move_selection(app, 1);
        }
        KeyCode::Char('p') | KeyCode::Up | KeyCode::Char('k') => {
            move_selection(app, -1);
        }

        // Actions
        KeyCode::Char('c') => {
            if let Some(item) = app.selected_item() {
                super::actions::copy_path_to_clipboard(&item.location.file)?;
            }
        }
        KeyCode::Char('e') | KeyCode::Char('o') => {
            if let Some(item) = app.selected_item() {
                super::actions::open_in_editor(&item.location.file, Some(item.location.line))?;
                app.request_redraw(); // Force redraw after editor suspends/resumes TUI
            }
        }

        // Help
        KeyCode::Char('?') => {
            app.set_view_mode(ViewMode::Help);
        }

        _ => {}
    }

    Ok(false)
}

/// Handle keys in search mode
fn handle_search_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match key.code {
        // Exit search
        KeyCode::Esc => {
            app.search_mut().clear();
            app.apply_search();
            app.set_view_mode(ViewMode::List);
        }

        // Execute search
        KeyCode::Enter => {
            app.apply_search();
            app.set_view_mode(ViewMode::List);
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
        KeyCode::Esc | KeyCode::Char('q') => {
            app.set_view_mode(ViewMode::List);
        }

        KeyCode::Char('1') => {
            app.set_sort_by(SortCriteria::Score);
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('2') => {
            app.set_sort_by(SortCriteria::Coverage);
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('3') => {
            app.set_sort_by(SortCriteria::Complexity);
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('4') => {
            app.set_sort_by(SortCriteria::FilePath);
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('5') => {
            app.set_sort_by(SortCriteria::FunctionName);
            app.set_view_mode(ViewMode::List);
        }

        _ => {}
    }

    Ok(false)
}

/// Handle keys in filter menu
fn handle_filter_menu_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.set_view_mode(ViewMode::List);
        }

        // Severity filters
        KeyCode::Char('1') => {
            app.add_filter(Filter::Severity(SeverityFilter::Critical));
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('2') => {
            app.add_filter(Filter::Severity(SeverityFilter::High));
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('3') => {
            app.add_filter(Filter::Severity(SeverityFilter::Medium));
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('4') => {
            app.add_filter(Filter::Severity(SeverityFilter::Low));
            app.set_view_mode(ViewMode::List);
        }

        // Coverage filters
        KeyCode::Char('n') => {
            app.add_filter(Filter::Coverage(CoverageFilter::None));
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('l') => {
            app.add_filter(Filter::Coverage(CoverageFilter::Low));
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('m') => {
            app.add_filter(Filter::Coverage(CoverageFilter::Medium));
            app.set_view_mode(ViewMode::List);
        }
        KeyCode::Char('h') => {
            app.add_filter(Filter::Coverage(CoverageFilter::High));
            app.set_view_mode(ViewMode::List);
        }

        // Clear filters
        KeyCode::Char('c') => {
            app.clear_filters();
            app.set_view_mode(ViewMode::List);
        }

        _ => {}
    }

    Ok(false)
}

/// Handle keys in help overlay
fn handle_help_key(app: &mut ResultsApp, _key: KeyEvent) -> Result<bool> {
    // Any key exits help
    app.set_view_mode(ViewMode::List);
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
