//! Sort functionality for query results.
//!
//! Provides sort modal handling, sort order cycling, and index-based sorting
//! for memory-efficient sorting of large result sets.

use super::ContentPanel;
use super::types::SortOrder;

impl ContentPanel {
    /// Check if sort modal is active.
    pub fn is_sort_mode_active(&self) -> bool {
        self.sort_mode_active
    }

    /// Open sort column selection modal.
    pub fn open_sort_modal(&mut self) {
        if let Some(ref result) = self.query_result {
            if !result.columns.is_empty() {
                self.sort_mode_active = true;
                // Start at currently sorted column or first column
                self.sort_modal_selection = self.sort_column_index.unwrap_or(0);
            }
        }
    }

    /// Close sort modal without applying.
    pub fn close_sort_modal(&mut self) {
        self.sort_mode_active = false;
    }

    /// Move selection up in sort modal.
    pub fn sort_modal_up(&mut self) {
        if self.sort_modal_selection > 0 {
            self.sort_modal_selection -= 1;
        }
    }

    /// Move selection down in sort modal.
    pub fn sort_modal_down(&mut self) {
        if let Some(ref result) = self.query_result {
            if self.sort_modal_selection < result.columns.len().saturating_sub(1) {
                self.sort_modal_selection += 1;
            }
        }
    }

    /// Apply sort on selected column.
    /// If same column is selected, toggles between Ascending -> Descending -> None.
    pub fn apply_sort(&mut self) {
        let selected_col = self.sort_modal_selection;

        // Toggle sort order
        if self.sort_column_index == Some(selected_col) {
            // Same column - cycle through orders
            self.sort_order = match self.sort_order {
                SortOrder::None => SortOrder::Ascending,
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::None,
            };
            if self.sort_order == SortOrder::None {
                // Restore original order by clearing indices
                self.sort_column_index = None;
                self.sort_indices = None;
            } else {
                // Sort indices (not data)
                self.update_sort_indices();
            }
        } else {
            // New column - start with ascending
            self.sort_column_index = Some(selected_col);
            self.sort_order = SortOrder::Ascending;
            // Sort indices (not data)
            self.update_sort_indices();
        }

        // Close modal
        self.sort_mode_active = false;

        // Reset to first page and first row after sort
        self.result_page = 0;
        self.result_table_state.select(Some(0));
    }

    /// Clear sort and restore original order.
    #[allow(dead_code)] // Designed for future features
    pub fn clear_sort(&mut self) {
        self.sort_order = SortOrder::None;
        self.sort_column_index = None;
        self.sort_indices = None;
    }

    /// Update sort indices based on current sort column and order.
    /// Uses index-based sorting for memory efficiency (no data cloning).
    pub(super) fn update_sort_indices(&mut self) {
        if self.sort_order == SortOrder::None {
            self.sort_indices = None;
            return;
        }

        let col_idx = match self.sort_column_index {
            Some(idx) => idx,
            None => {
                self.sort_indices = None;
                return;
            }
        };

        if let Some(ref result) = self.query_result {
            if col_idx >= result.columns.len() {
                self.sort_indices = None;
                return;
            }

            // Create index array based on filtered or full row count
            let row_count = self.visible_row_count();
            let mut indices: Vec<usize> = (0..row_count).collect();

            // Sort indices based on row values at col_idx
            // When filter is active, we need to look up actual row through filter_indices
            let order = self.sort_order;
            let filter_indices = &self.filter_indices;
            indices.sort_by(|&a, &b| {
                // Get actual row index (through filter if active)
                let actual_a = if let Some(fi) = filter_indices {
                    fi[a]
                } else {
                    a
                };
                let actual_b = if let Some(fi) = filter_indices {
                    fi[b]
                } else {
                    b
                };

                let val_a = result.rows[actual_a]
                    .get(col_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let val_b = result.rows[actual_b]
                    .get(col_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("");

                // Try numeric comparison first
                let cmp = match (val_a.parse::<f64>(), val_b.parse::<f64>()) {
                    (Ok(num_a), Ok(num_b)) => num_a
                        .partial_cmp(&num_b)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    _ => val_a.cmp(val_b), // Fall back to string comparison
                };

                match order {
                    SortOrder::Ascending => cmp,
                    SortOrder::Descending => cmp.reverse(),
                    SortOrder::None => std::cmp::Ordering::Equal,
                }
            });

            self.sort_indices = Some(indices);
        }
    }

    /// Get current sort info for display.
    pub fn get_sort_info(&self) -> Option<(String, SortOrder)> {
        if let (Some(col_idx), Some(result)) = (self.sort_column_index, &self.query_result) {
            if col_idx < result.columns.len() && self.sort_order != SortOrder::None {
                return Some((result.columns[col_idx].clone(), self.sort_order));
            }
        }
        None
    }
}
