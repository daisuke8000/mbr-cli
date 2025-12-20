//! Search functionality for Questions view.
//!
//! Handles search mode input, query execution, and search state management.

use super::ContentPanel;
use super::types::InputMode;

impl ContentPanel {
    /// Get the current input mode.
    pub fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    /// Enter search mode.
    pub fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Search;
        self.search_query.clear();
    }

    /// Exit search mode without executing search.
    pub fn exit_search_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
    }

    /// Get the current search query (for debugging/future use).
    #[allow(dead_code)]
    pub fn get_search_query(&self) -> &str {
        &self.search_query
    }

    /// Get the active search query (after execution).
    pub fn get_active_search(&self) -> Option<&str> {
        self.active_search.as_deref()
    }

    /// Execute the current search query and return it for API call.
    /// Returns Some(query) if there's a query to search, None if empty.
    pub fn execute_search(&mut self) -> Option<String> {
        self.input_mode = InputMode::Normal;
        let query = self.search_query.trim().to_string();
        if query.is_empty() {
            self.active_search = None;
            None
        } else {
            self.active_search = Some(query.clone());
            // Reset selection for new results
            self.table_state.select(Some(0));
            Some(query)
        }
    }

    /// Clear the active search and return to showing all questions.
    pub fn clear_search(&mut self) {
        self.active_search = None;
        self.search_query.clear();
        self.table_state.select(Some(0));
    }

    /// Handle character input in search mode.
    pub fn handle_search_input(&mut self, c: char) {
        self.search_query.push(c);
    }

    /// Handle backspace in search mode.
    pub fn handle_search_backspace(&mut self) {
        self.search_query.pop();
    }
}
