//! Result search functionality (all-column search).
//!
//! Provides real-time search across all columns in query results,
//! with case-insensitive matching and integration with filter/sort.

use super::ContentPanel;
use super::types::SortOrder;

impl ContentPanel {
    /// Check if result search mode is active.
    pub fn is_result_search_active(&self) -> bool {
        self.result_search_active
    }

    /// Open result search input.
    pub fn open_result_search(&mut self) {
        self.result_search_active = true;
        // Don't clear previous search text, allow editing
    }

    /// Close result search input without applying.
    pub fn close_result_search(&mut self) {
        self.result_search_active = false;
    }

    /// Add a character to result search text (real-time search).
    pub fn result_search_input_char(&mut self, c: char) {
        self.result_search_text.push(c);
        self.update_result_search_indices();
        // If filter is active, re-apply filter on new search results
        if self.filter_column_index.is_some() && !self.filter_text.is_empty() {
            self.update_filter_indices();
        }
        // Re-apply sort on new filtered/searched data
        if self.sort_order != SortOrder::None {
            self.update_sort_indices();
        }
    }

    /// Delete the last character from result search text.
    pub fn result_search_delete_char(&mut self) {
        self.result_search_text.pop();
        self.update_result_search_indices();
        // If filter is active, re-apply filter on new search results
        if self.filter_column_index.is_some() && !self.filter_text.is_empty() {
            self.update_filter_indices();
        }
        // Re-apply sort on new filtered/searched data
        if self.sort_order != SortOrder::None {
            self.update_sort_indices();
        }
    }

    /// Apply result search and close input.
    pub fn apply_result_search(&mut self) {
        self.result_search_active = false;
        // Reset to first page
        self.result_page = 0;
        self.result_table_state.select(Some(0));
    }

    /// Clear result search and restore all rows.
    pub fn clear_result_search(&mut self) {
        self.result_search_text.clear();
        self.result_search_indices = None;
        self.result_search_active = false;
        // If filter is active, re-apply filter on full data
        if self.filter_column_index.is_some() && !self.filter_text.is_empty() {
            self.update_filter_indices();
        }
        // Re-apply sort
        if self.sort_order != SortOrder::None {
            self.update_sort_indices();
        }
    }

    /// Update result search indices based on current search text.
    /// Searches all columns with case-insensitive contains match.
    fn update_result_search_indices(&mut self) {
        if self.result_search_text.is_empty() {
            self.result_search_indices = None;
            return;
        }

        if let Some(ref result) = self.query_result {
            let search_lower = self.result_search_text.to_lowercase();
            let indices: Vec<usize> = result
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    // Match if any column contains the search text
                    row.iter()
                        .any(|cell| cell.to_lowercase().contains(&search_lower))
                })
                .map(|(i, _)| i)
                .collect();

            self.result_search_indices = Some(indices);
        }
    }

    /// Get current result search info for display.
    /// Returns (search_text, matched_count, total_count) if search is active.
    #[allow(dead_code)] // Designed for status bar display
    pub fn get_result_search_info(&self) -> Option<(String, usize, usize)> {
        if !self.result_search_text.is_empty() {
            let total = self
                .query_result
                .as_ref()
                .map(|r| r.rows.len())
                .unwrap_or(0);
            let matched = self.visible_row_count();
            return Some((self.result_search_text.clone(), matched, total));
        }
        None
    }
}
