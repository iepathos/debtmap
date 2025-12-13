//! Filtering, sorting, and search state management.
//!
//! This module manages query-related state for the TUI,
//! following the single responsibility principle. It handles:
//! - Filtered indices tracking
//! - Search state management
//! - Filter collection management
//! - Sort criteria
//!
//! Operations require a reference to `UnifiedAnalysis` to
//! compute results without owning the data.

use super::{filter::Filter, search::SearchState, sort::SortCriteria};
use crate::priority::UnifiedAnalysis;

/// Manages filtering, sorting, and search state.
///
/// Operates on analysis data without owning it.
#[derive(Debug)]
pub struct QueryState {
    filtered_indices: Vec<usize>,
    search: SearchState,
    filters: Vec<Filter>,
    sort_by: SortCriteria,
}

impl QueryState {
    /// Create new query state for given item count.
    pub fn new(item_count: usize) -> Self {
        Self {
            filtered_indices: (0..item_count).collect(),
            search: SearchState::new(),
            filters: Vec::new(),
            sort_by: SortCriteria::Score,
        }
    }

    /// Get filtered indices.
    pub fn filtered_indices(&self) -> &[usize] {
        &self.filtered_indices
    }

    /// Get mutable filtered indices (for direct sort operations).
    pub fn filtered_indices_mut(&mut self) -> &mut Vec<usize> {
        &mut self.filtered_indices
    }

    /// Get search state reference.
    pub fn search(&self) -> &SearchState {
        &self.search
    }

    /// Get mutable search state reference.
    pub fn search_mut(&mut self) -> &mut SearchState {
        &mut self.search
    }

    /// Get active filters.
    pub fn filters(&self) -> &[Filter] {
        &self.filters
    }

    /// Get current sort criteria.
    pub fn sort_by(&self) -> SortCriteria {
        self.sort_by
    }

    /// Set sort criteria and re-sort.
    pub fn set_sort_by(&mut self, criteria: SortCriteria, analysis: &UnifiedAnalysis) {
        self.sort_by = criteria;
        self.apply_sort(analysis);
    }

    /// Apply current search and filters.
    pub fn apply_search(&mut self, analysis: &UnifiedAnalysis) {
        let query = self.search.query();
        self.filtered_indices = if query.is_empty() {
            (0..analysis.items.len()).collect()
        } else {
            super::search::filter_items(analysis, query)
        };
        self.apply_filters(analysis);
        self.apply_sort(analysis);
    }

    /// Add a filter and reapply.
    pub fn add_filter(&mut self, filter: Filter, analysis: &UnifiedAnalysis) {
        self.filters.push(filter);
        self.reapply_all(analysis);
    }

    /// Remove a filter by index and reapply.
    pub fn remove_filter(&mut self, index: usize, analysis: &UnifiedAnalysis) {
        if index < self.filters.len() {
            self.filters.remove(index);
            self.reapply_all(analysis);
        }
    }

    /// Clear all filters and reapply.
    pub fn clear_filters(&mut self, analysis: &UnifiedAnalysis) {
        self.filters.clear();
        self.reapply_all(analysis);
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    fn apply_filters(&mut self, analysis: &UnifiedAnalysis) {
        if self.filters.is_empty() {
            return;
        }
        self.filtered_indices.retain(|&idx| {
            analysis
                .items
                .get(idx)
                .map(|item| self.filters.iter().all(|f| f.matches(item)))
                .unwrap_or(false)
        });
    }

    fn apply_sort(&mut self, analysis: &UnifiedAnalysis) {
        super::sort::sort_indices(&mut self.filtered_indices, analysis, self.sort_by);
    }

    fn reapply_all(&mut self, analysis: &UnifiedAnalysis) {
        // Start from search results
        let query = self.search.query();
        self.filtered_indices = if query.is_empty() {
            (0..analysis.items.len()).collect()
        } else {
            super::search::filter_items(analysis, query)
        };
        self.apply_filters(analysis);
        self.apply_sort(analysis);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_empty_analysis() -> UnifiedAnalysis {
        UnifiedAnalysis::new(crate::priority::call_graph::CallGraph::new())
    }

    #[test]
    fn test_new_query_state() {
        let state = QueryState::new(10);
        assert_eq!(state.filtered_indices().len(), 10);
        assert_eq!(state.sort_by(), SortCriteria::Score);
        assert!(state.filters().is_empty());
    }

    #[test]
    fn test_new_query_state_empty() {
        let state = QueryState::new(0);
        assert!(state.filtered_indices().is_empty());
    }

    #[test]
    fn test_sort_by_change() {
        let analysis = create_empty_analysis();
        let mut state = QueryState::new(0);
        state.set_sort_by(SortCriteria::Complexity, &analysis);
        assert_eq!(state.sort_by(), SortCriteria::Complexity);
    }

    #[test]
    fn test_search_state_access() {
        let mut state = QueryState::new(5);
        state.search_mut().set_query("test".to_string());
        assert_eq!(state.search().query(), "test");
    }

    #[test]
    fn test_filtered_indices_mut() {
        let mut state = QueryState::new(10);
        state.filtered_indices_mut().clear();
        assert!(state.filtered_indices().is_empty());
    }
}
