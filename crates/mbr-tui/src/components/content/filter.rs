//! Filter functionality for query results.
//!
//! Provides filter modal handling with two-step workflow (column selection → text input),
//! case-insensitive contains matching, and integration with search results.

use super::ContentPanel;
use super::types::SortOrder;

impl ContentPanel {
    /// Check if filter modal is active.
    pub fn is_filter_mode_active(&self) -> bool {
        self.filter_mode_active
    }

    /// Open filter column selection modal.
    pub fn open_filter_modal(&mut self) {
        if let Some(ref result) = self.query_result {
            if !result.columns.is_empty() {
                self.filter_mode_active = true;
                self.filter_modal_step = 0; // Start at column selection
                // Start at currently filtered column or first column
                self.filter_modal_selection = self.filter_column_index.unwrap_or(0);
            }
        }
    }

    /// Close filter modal without applying.
    pub fn close_filter_modal(&mut self) {
        self.filter_mode_active = false;
        self.filter_modal_step = 0;
        // Don't clear filter_text here to preserve it for re-editing
    }

    /// Move selection up in filter modal (column selection step).
    pub fn filter_modal_up(&mut self) {
        if self.filter_modal_step == 0 && self.filter_modal_selection > 0 {
            self.filter_modal_selection -= 1;
        }
    }

    /// Move selection down in filter modal (column selection step).
    pub fn filter_modal_down(&mut self) {
        if self.filter_modal_step == 0 {
            if let Some(ref result) = self.query_result {
                if self.filter_modal_selection < result.columns.len().saturating_sub(1) {
                    self.filter_modal_selection += 1;
                }
            }
        }
    }

    /// Move to next step in filter modal (column selection → text input).
    pub fn filter_modal_next_step(&mut self) {
        if self.filter_modal_step == 0 {
            self.filter_modal_step = 1;
            // Pre-fill with existing filter text if same column
            if self.filter_column_index != Some(self.filter_modal_selection) {
                self.filter_text.clear();
            }
        }
    }

    /// Move to previous step in filter modal (text input → column selection).
    pub fn filter_modal_prev_step(&mut self) {
        if self.filter_modal_step == 1 {
            self.filter_modal_step = 0;
        }
    }

    /// Handle character input in filter modal (text input step).
    pub fn filter_modal_input_char(&mut self, c: char) {
        if self.filter_modal_step == 1 {
            self.filter_text.push(c);
        }
    }

    /// Handle backspace in filter modal (text input step).
    pub fn filter_modal_delete_char(&mut self) {
        if self.filter_modal_step == 1 {
            self.filter_text.pop();
        }
    }

    /// Apply filter on selected column with current filter text.
    pub fn apply_filter(&mut self) {
        let selected_col = self.filter_modal_selection;

        if self.filter_text.is_empty() {
            // Empty filter text - clear filter
            self.clear_filter();
        } else {
            // Apply filter
            self.filter_column_index = Some(selected_col);
            self.update_filter_indices();
            // Re-apply sort on filtered data
            if self.sort_order != SortOrder::None {
                self.update_sort_indices();
            }
        }

        // Close modal
        self.filter_mode_active = false;
        self.filter_modal_step = 0;

        // Reset to first page and first row after filter
        self.result_page = 0;
        self.result_table_state.select(Some(0));
    }

    /// Clear filter and restore all rows.
    pub fn clear_filter(&mut self) {
        self.filter_column_index = None;
        self.filter_text.clear();
        self.filter_indices = None;
        // Re-apply sort on full data
        if self.sort_order != SortOrder::None {
            self.update_sort_indices();
        }
    }

    /// Update filter indices based on current filter column and text.
    /// If search is active, filters within search results.
    pub(super) fn update_filter_indices(&mut self) {
        let col_idx = match self.filter_column_index {
            Some(idx) => idx,
            None => {
                self.filter_indices = None;
                return;
            }
        };

        if self.filter_text.is_empty() {
            self.filter_indices = None;
            return;
        }

        if let Some(ref result) = self.query_result {
            if col_idx >= result.columns.len() {
                self.filter_indices = None;
                return;
            }

            // Case-insensitive contains match
            let filter_lower = self.filter_text.to_lowercase();

            // Determine the base set of indices to filter from
            let base_indices: Box<dyn Iterator<Item = usize>> =
                if let Some(ref search_indices) = self.result_search_indices {
                    // Filter within search results
                    Box::new(search_indices.iter().copied())
                } else {
                    // Filter all rows
                    Box::new(0..result.rows.len())
                };

            let indices: Vec<usize> = base_indices
                .filter(|&i| {
                    result
                        .rows
                        .get(i)
                        .and_then(|row| row.get(col_idx))
                        .map(|cell| cell.to_lowercase().contains(&filter_lower))
                        .unwrap_or(false)
                })
                .collect();

            self.filter_indices = Some(indices);
        }
    }

    /// Get current filter info for display.
    /// Returns (column_name, filter_text, visible_row_count) if filter is active.
    #[allow(dead_code)] // Designed for status bar display in future
    pub fn get_filter_info(&self) -> Option<(String, String, usize)> {
        if let (Some(col_idx), Some(result)) = (self.filter_column_index, &self.query_result) {
            if col_idx < result.columns.len() && !self.filter_text.is_empty() {
                let visible = self.visible_row_count();
                return Some((
                    result.columns[col_idx].clone(),
                    self.filter_text.clone(),
                    visible,
                ));
            }
        }
        None
    }
}
