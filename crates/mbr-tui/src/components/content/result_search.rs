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

    /// Add a character to result search text.
    /// Uses debounce: only triggers search when 3+ characters are typed.
    /// Actual index recomputation is deferred via dirty flags until next render.
    pub fn result_search_input_char(&mut self, c: char) {
        self.result_search_text.push(c);
        // Only trigger search with 3+ characters (debounce)
        if self.result_search_text.len() >= 3 {
            self.search_dirty = true;
            // Cascade: filter and sort also need recomputation
            if self.filter_column_index.is_some() && !self.filter_text.is_empty() {
                self.filter_dirty = true;
            }
            if self.sort_order != SortOrder::None {
                self.sort_dirty = true;
            }
        }
    }

    /// Delete the last character from result search text.
    /// Uses dirty flags for deferred recomputation.
    pub fn result_search_delete_char(&mut self) {
        self.result_search_text.pop();
        // Mark dirty for recomputation (including when dropping below 3 chars to clear results)
        self.search_dirty = true;
        if self.filter_column_index.is_some() && !self.filter_text.is_empty() {
            self.filter_dirty = true;
        }
        if self.sort_order != SortOrder::None {
            self.sort_dirty = true;
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
        self.search_dirty = false; // Already cleared manually
        // Cascade: filter and sort need recomputation on full data
        if self.filter_column_index.is_some() && !self.filter_text.is_empty() {
            self.filter_dirty = true;
        }
        if self.sort_order != SortOrder::None {
            self.sort_dirty = true;
        }
    }

    /// Update result search indices based on current search text.
    /// Searches all columns with case-insensitive contains match.
    pub(super) fn update_result_search_indices(&mut self) {
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
}
