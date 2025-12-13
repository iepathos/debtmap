//! Search functionality for filtering results.

use crate::priority::UnifiedAnalysis;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// Search state
#[derive(Debug)]
pub struct SearchState {
    query: String,
    cursor_position: usize,
}

impl SearchState {
    /// Create new search state
    pub fn new() -> Self {
        Self {
            query: String::new(),
            cursor_position: 0,
        }
    }

    /// Get current search query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Set search query
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.cursor_position = self.query.len();
    }

    /// Clear search query
    pub fn clear(&mut self) {
        self.query.clear();
        self.cursor_position = 0;
    }

    /// Insert character at cursor
    pub fn insert_char(&mut self, c: char) {
        self.query.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    /// Delete character before cursor (backspace)
    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            self.query.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
        }
    }

    /// Delete character at cursor (delete key)
    pub fn delete_char_forward(&mut self) {
        if self.cursor_position < self.query.len() {
            self.query.remove(self.cursor_position);
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.query.len() {
            self.cursor_position += 1;
        }
    }

    /// Move cursor to start
    pub fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    /// Move cursor to end
    pub fn move_cursor_end(&mut self) {
        self.cursor_position = self.query.len();
    }

    /// Get cursor position
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter items based on search query
pub fn filter_items(analysis: &UnifiedAnalysis, query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..analysis.items.len()).collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut matches: Vec<(usize, i64)> = Vec::new();

    for (idx, item) in analysis.items.iter().enumerate() {
        // Search in file path
        let file_path = item.location.file.to_string_lossy();
        let file_score = matcher.fuzzy_match(&file_path, query);

        // Search in function name
        let function_score = matcher.fuzzy_match(&item.location.function, query);

        // Search in recommendation text
        let recommendation_text = &item.recommendation.primary_action;
        let rec_score = matcher.fuzzy_match(recommendation_text, query);

        // Take best match score
        let best_score = file_score.or(function_score).or(rec_score);

        if let Some(score) = best_score {
            matches.push((idx, score));
        }
    }

    // Sort by match score (best matches first)
    matches.sort_by(|a, b| b.1.cmp(&a.1));

    // Extract indices
    matches.into_iter().map(|(idx, _)| idx).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_state_insert_char() {
        let mut state = SearchState::new();
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');
        assert_eq!(state.query(), "abc");
        assert_eq!(state.cursor_position(), 3);
    }

    #[test]
    fn test_search_state_delete_char() {
        let mut state = SearchState::new();
        state.set_query("abc".to_string());
        state.delete_char();
        assert_eq!(state.query(), "ab");
        assert_eq!(state.cursor_position(), 2);
    }

    #[test]
    fn test_search_state_cursor_movement() {
        let mut state = SearchState::new();
        state.set_query("abc".to_string());
        state.move_cursor_home();
        assert_eq!(state.cursor_position(), 0);
        state.move_cursor_end();
        assert_eq!(state.cursor_position(), 3);
        state.move_cursor_left();
        assert_eq!(state.cursor_position(), 2);
        state.move_cursor_right();
        assert_eq!(state.cursor_position(), 3);
    }

    #[test]
    fn test_search_state_clear() {
        let mut state = SearchState::new();
        state.set_query("test".to_string());
        state.clear();
        assert_eq!(state.query(), "");
        assert_eq!(state.cursor_position(), 0);
    }
}
