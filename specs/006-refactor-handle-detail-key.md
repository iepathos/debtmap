---
number: 6
title: Refactor handle_detail_key Event Handler
category: optimization
priority: high
status: draft
dependencies: []
created: 2025-12-21
---

# Specification 6: Refactor handle_detail_key Event Handler

**Category**: optimization
**Priority**: high
**Status**: draft
**Dependencies**: none

## Context

The `handle_detail_key` function in `src/tui/results/navigation.rs:134` has concerning metrics:
- Cyclomatic complexity: 41
- Cognitive complexity: 74 → 39 (entropy-adjusted)
- Nesting depth: 4
- Lines of code: 168
- Coverage: 23%
- Critical path: Yes
- Score: 74.39

This function is a TUI event handler that processes keyboard input for the detail view. It handles:
- Navigation (back, page switching, item up/down)
- Page jumping (1-8 keys)
- Actions (copy, open in editor, help)

Following Stillwater's "Composition Over Complexity" principle, this monolithic handler should be decomposed into focused, testable functions.

## Objective

Refactor `handle_detail_key` to:
1. Separate pure key-to-action mapping from effectful action execution
2. Extract repetitive page-jump logic into a data-driven approach
3. Reduce nesting depth from 4 to ≤2
4. Enable unit testing of navigation logic without TUI state

## Requirements

### Functional Requirements

1. **Key classification**: Pure function to classify KeyEvent into action type
2. **Page jump consolidation**: Single function handling all number-key page jumps
3. **Action execution**: Thin shell that executes classified actions
4. **Preserve behavior**: All existing key bindings must work identically

### Non-Functional Requirements

1. **Testability**: Navigation logic testable without mocking TUI
2. **Maintainability**: Adding new keybindings should be straightforward
3. **Performance**: No overhead vs current implementation

## Acceptance Criteria

- [ ] Cyclomatic complexity reduced from 41 to ≤15
- [ ] Nesting depth reduced from 4 to ≤2
- [ ] Test coverage reaches ≥60%
- [ ] All existing keybindings preserved
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean

## Technical Details

### Implementation Approach: Action Enum + Handler Pattern

Following Stillwater's "Pure Core, Imperative Shell":

```rust
/// Pure: Classify key into action (no side effects)
#[derive(Debug, Clone, PartialEq)]
enum DetailAction {
    NavigateBack,
    NextPage,
    PrevPage,
    JumpToPage(DetailPage),
    MoveSelection(i32),
    CopyContext,
    CopyPrimary,
    CopyPage,
    OpenInEditor,
    ShowHelp,
    NoOp,
}

/// Pure: Map key event to action
fn classify_key(key: KeyEvent, current_page: DetailPage) -> DetailAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => DetailAction::NavigateBack,
        KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => DetailAction::NextPage,
        KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') => DetailAction::PrevPage,
        KeyCode::Char(c @ '1'..='8') => {
            let page = page_from_digit(c);
            DetailAction::JumpToPage(page)
        }
        KeyCode::Down | KeyCode::Char('j') => DetailAction::MoveSelection(1),
        KeyCode::Up | KeyCode::Char('k') => DetailAction::MoveSelection(-1),
        KeyCode::Char('c') if current_page == DetailPage::Context => DetailAction::CopyContext,
        KeyCode::Char('c') => DetailAction::CopyPage,
        KeyCode::Char('p') if current_page == DetailPage::Context => DetailAction::CopyPrimary,
        KeyCode::Char('e') | KeyCode::Char('o') => DetailAction::OpenInEditor,
        KeyCode::Char('?') => DetailAction::ShowHelp,
        _ => DetailAction::NoOp,
    }
}

/// Pure: Map digit to page
fn page_from_digit(c: char) -> DetailPage {
    match c {
        '1' => DetailPage::Overview,
        '2' => DetailPage::ScoreBreakdown,
        '3' => DetailPage::Context,
        '4' => DetailPage::Dependencies,
        '5' => DetailPage::GitContext,
        '6' => DetailPage::Patterns,
        '7' => DetailPage::DataFlow,
        '8' => DetailPage::Responsibilities,
        _ => DetailPage::Overview, // fallback
    }
}

/// Effectful: Execute action (thin shell)
fn execute_action(app: &mut ResultsApp, action: DetailAction) -> Result<bool> {
    match action {
        DetailAction::NavigateBack => navigate_back(app),
        DetailAction::NextPage => {
            let new_page = page_availability::next_available_page(
                app.nav().detail_page,
                app.selected_item(),
                &app.analysis().data_flow_graph,
            );
            app.nav_mut().detail_page = new_page;
        }
        DetailAction::JumpToPage(page) => {
            if page_availability::is_page_available(page, app.selected_item(), &app.analysis().data_flow_graph) {
                app.nav_mut().detail_page = page;
            }
        }
        // ... other actions
        DetailAction::NoOp => {}
    }
    Ok(false)
}

/// Entry point: compose pure classification with effectful execution
fn handle_detail_key(app: &mut ResultsApp, key: KeyEvent) -> Result<bool> {
    let action = classify_key(key, app.nav().detail_page);
    execute_action(app, action)
}
```

### Benefits

1. **Pure `classify_key`**: 100% testable without app state
2. **Data-driven page mapping**: `page_from_digit` is a simple lookup
3. **Action enum**: Documents all possible actions in one place
4. **Thin shell**: `execute_action` does minimal work

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_navigates_back() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(classify_key(key, DetailPage::Overview), DetailAction::NavigateBack);
    }

    #[test]
    fn test_number_keys_jump_to_pages() {
        for (c, expected) in [('1', DetailPage::Overview), ('2', DetailPage::ScoreBreakdown)] {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            assert_eq!(classify_key(key, DetailPage::Overview), DetailAction::JumpToPage(expected));
        }
    }

    #[test]
    fn test_c_on_context_page_copies_context() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(classify_key(key, DetailPage::Context), DetailAction::CopyContext);
    }

    #[test]
    fn test_c_on_other_page_copies_page() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(classify_key(key, DetailPage::Overview), DetailAction::CopyPage);
    }
}
```

## Dependencies

- **Prerequisites**: None
- **Affected Components**: `src/tui/results/navigation.rs`
- **External Dependencies**: None

## Testing Strategy

- **Unit Tests**: Test `classify_key` pure function exhaustively
- **Integration Tests**: Verify action execution modifies app state correctly
- **Edge Cases**: Unknown keys, modifier combinations

## Documentation Requirements

- **Code Documentation**: Document `DetailAction` variants and their triggers
- **User Documentation**: None (preserves existing keybindings)

## Implementation Notes

The context-sensitive 'c' and 'p' keys (different behavior on Context page vs others) require passing `current_page` to the classifier. This keeps the pure function informed without needing app state.

Consider making `DetailAction` public for use in help screen generation - the action enum becomes documentation.

## Migration and Compatibility

No breaking changes. All keybindings preserved exactly.
