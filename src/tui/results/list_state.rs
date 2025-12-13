//! List selection, scroll position, and grouping state.
//!
//! This module manages the list-related UI state for the TUI,
//! following the single responsibility principle. It handles:
//! - Selection index tracking
//! - Scroll offset for viewport
//! - Grouping toggle state
//!
//! All methods are pure or have minimal side effects.

/// Manages list selection, scroll position, and grouping state.
///
/// Pure state container with no I/O operations.
#[derive(Debug, Clone)]
pub struct ListState {
    selected_index: usize,
    scroll_offset: usize,
    show_grouped: bool,
}

impl Default for ListState {
    fn default() -> Self {
        Self {
            selected_index: 0,
            scroll_offset: 0,
            show_grouped: true, // Default: grouping enabled
        }
    }
}

impl ListState {
    /// Create new list state with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get selected index.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Set selected index with bounds checking.
    ///
    /// Pure function - validates against provided item count.
    pub fn set_selected_index(&mut self, index: usize, item_count: usize) {
        self.selected_index = clamp_selection(index, item_count);
    }

    /// Get scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    /// Check if grouping is enabled.
    pub fn is_grouped(&self) -> bool {
        self.show_grouped
    }

    /// Set grouping state.
    pub fn set_grouped(&mut self, grouped: bool) {
        self.show_grouped = grouped;
    }

    /// Toggle grouping on/off.
    pub fn toggle_grouping(&mut self) {
        self.show_grouped = !self.show_grouped;
    }

    /// Reset selection and scroll to top.
    pub fn reset(&mut self) {
        self.selected_index = 0;
        self.scroll_offset = 0;
    }
}

// ============================================================================
// PURE FUNCTIONS
// ============================================================================

/// Clamps selection index to valid range (pure).
pub fn clamp_selection(index: usize, item_count: usize) -> usize {
    if item_count == 0 {
        0
    } else {
        index.min(item_count - 1)
    }
}

/// Calculates visible range for scrolling (pure).
pub fn calculate_visible_range(
    scroll_offset: usize,
    viewport_height: usize,
    total_items: usize,
) -> std::ops::Range<usize> {
    let start = scroll_offset;
    let end = (scroll_offset + viewport_height).min(total_items);
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = ListState::default();
        assert_eq!(state.selected_index(), 0);
        assert_eq!(state.scroll_offset(), 0);
        assert!(state.is_grouped());
    }

    #[test]
    fn test_clamp_selection_empty() {
        assert_eq!(clamp_selection(5, 0), 0);
    }

    #[test]
    fn test_clamp_selection_within_bounds() {
        assert_eq!(clamp_selection(3, 10), 3);
    }

    #[test]
    fn test_clamp_selection_exceeds_bounds() {
        assert_eq!(clamp_selection(15, 10), 9);
    }

    #[test]
    fn test_clamp_selection_at_boundary() {
        assert_eq!(clamp_selection(9, 10), 9);
        assert_eq!(clamp_selection(10, 10), 9);
    }

    #[test]
    fn test_set_selected_index_clamps() {
        let mut state = ListState::new();
        state.set_selected_index(100, 10);
        assert_eq!(state.selected_index(), 9);
    }

    #[test]
    fn test_set_selected_index_empty_list() {
        let mut state = ListState::new();
        state.set_selected_index(5, 0);
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_toggle_grouping() {
        let mut state = ListState::default();
        assert!(state.is_grouped());
        state.toggle_grouping();
        assert!(!state.is_grouped());
        state.toggle_grouping();
        assert!(state.is_grouped());
    }

    #[test]
    fn test_set_grouped() {
        let mut state = ListState::default();
        state.set_grouped(false);
        assert!(!state.is_grouped());
        state.set_grouped(true);
        assert!(state.is_grouped());
    }

    #[test]
    fn test_reset() {
        let mut state = ListState::new();
        state.set_selected_index(5, 10);
        state.set_scroll_offset(3);
        state.reset();
        assert_eq!(state.selected_index(), 0);
        assert_eq!(state.scroll_offset(), 0);
    }

    #[test]
    fn test_scroll_offset() {
        let mut state = ListState::new();
        state.set_scroll_offset(10);
        assert_eq!(state.scroll_offset(), 10);
    }

    #[test]
    fn test_calculate_visible_range_normal() {
        let range = calculate_visible_range(0, 10, 100);
        assert_eq!(range, 0..10);
    }

    #[test]
    fn test_calculate_visible_range_with_offset() {
        let range = calculate_visible_range(5, 10, 100);
        assert_eq!(range, 5..15);
    }

    #[test]
    fn test_calculate_visible_range_clamped() {
        let range = calculate_visible_range(95, 10, 100);
        assert_eq!(range, 95..100);
    }

    #[test]
    fn test_calculate_visible_range_empty() {
        let range = calculate_visible_range(0, 10, 0);
        assert_eq!(range, 0..0);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: selection is always valid after set_selected_index.
        ///
        /// No matter what index we try to set, the resulting selection must be
        /// within valid bounds (0 to item_count-1, or 0 if empty).
        #[test]
        fn selection_always_valid(
            input_index in 0usize..10000,
            item_count in 0usize..1000
        ) {
            let mut state = ListState::new();
            state.set_selected_index(input_index, item_count);

            let selected = state.selected_index();
            if item_count == 0 {
                prop_assert_eq!(selected, 0);
            } else {
                prop_assert!(selected < item_count, "Selected {} >= count {}", selected, item_count);
            }
        }

        /// Property: clamp_selection is idempotent.
        ///
        /// Applying clamp twice gives the same result as applying once.
        #[test]
        fn clamp_is_idempotent(
            index in 0usize..10000,
            item_count in 0usize..1000
        ) {
            let once = clamp_selection(index, item_count);
            let twice = clamp_selection(once, item_count);
            prop_assert_eq!(once, twice);
        }

        /// Property: clamp_selection output is always valid.
        ///
        /// For any input, the result is within [0, max(0, item_count-1)].
        #[test]
        fn clamp_output_always_valid(
            index in 0usize..10000,
            item_count in 0usize..1000
        ) {
            let result = clamp_selection(index, item_count);
            if item_count == 0 {
                prop_assert_eq!(result, 0);
            } else {
                prop_assert!(result < item_count);
            }
        }

        /// Property: visible range end is capped at total_items.
        ///
        /// This property verifies that the end of the range never exceeds total_items.
        /// Note: When scroll_offset > total_items, the range will be inverted (start > end),
        /// which is an empty range. This is expected behavior - the caller is responsible
        /// for keeping scroll_offset within bounds.
        #[test]
        fn visible_range_end_capped(
            scroll_offset in 0usize..100,
            viewport_height in 0usize..100,
            total_items in 0usize..200
        ) {
            let range = calculate_visible_range(scroll_offset, viewport_height, total_items);

            // End should not exceed total_items (this is the contract the function guarantees)
            prop_assert!(range.end <= total_items);
        }

        /// Property: visible range is valid when scroll_offset is within bounds.
        ///
        /// When scroll_offset <= total_items, the range should be valid (start <= end).
        #[test]
        fn visible_range_valid_with_bounded_scroll(
            total_items in 1usize..200,
            viewport_height in 0usize..100,
            scroll_factor in 0.0f64..1.0
        ) {
            // Generate scroll_offset bounded by total_items
            let scroll_offset = (scroll_factor * total_items as f64) as usize;

            let range = calculate_visible_range(scroll_offset, viewport_height, total_items);
            prop_assert!(range.start <= range.end, "start {} > end {} with scroll={}, total={}", range.start, range.end, scroll_offset, total_items);
            prop_assert!(range.end <= total_items);
        }

        /// Property: toggle_grouping is reversible.
        ///
        /// Two toggles should return to original state.
        #[test]
        fn toggle_grouping_reversible(initial_grouped: bool) {
            let mut state = ListState::default();
            state.set_grouped(initial_grouped);

            let before = state.is_grouped();
            state.toggle_grouping();
            state.toggle_grouping();
            let after = state.is_grouped();

            prop_assert_eq!(before, after);
        }

        /// Property: reset always results in zero state.
        #[test]
        fn reset_always_zeros(
            selected in 0usize..1000,
            scroll in 0usize..1000
        ) {
            let mut state = ListState::new();
            state.set_selected_index(selected, 1000);
            state.set_scroll_offset(scroll);

            state.reset();

            prop_assert_eq!(state.selected_index(), 0);
            prop_assert_eq!(state.scroll_offset(), 0);
        }
    }
}
