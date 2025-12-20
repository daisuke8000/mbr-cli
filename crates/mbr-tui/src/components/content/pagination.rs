//! Pagination and scrolling for query results.
//!
//! Provides page navigation, horizontal scrolling, and row visibility calculations.

use super::ContentPanel;
use super::types::{ContentView, SortOrder};

impl ContentPanel {
    /// Get the number of visible rows (after search and filter are applied).
    ///
    /// Priority: Search first, then Filter. Both are applied if both active.
    pub(super) fn visible_row_count(&self) -> usize {
        match (&self.result_search_indices, &self.filter_indices) {
            // Both search and filter: use filter (which operates on search results)
            (Some(_), Some(filter)) => filter.len(),
            // Only filter: use filter indices
            (None, Some(filter)) => filter.len(),
            // Only search: use search indices
            (Some(search), None) => search.len(),
            // Neither: use all rows
            (None, None) => self
                .query_result
                .as_ref()
                .map(|r| r.rows.len())
                .unwrap_or(0),
        }
    }

    /// Get total number of pages for query result (considers filter).
    pub(super) fn total_pages(&self) -> usize {
        self.visible_row_count().div_ceil(self.rows_per_page)
    }

    /// Go to next page in query result.
    pub(super) fn next_page(&mut self) {
        let total = self.total_pages();
        if total > 0 && self.result_page < total - 1 {
            self.result_page += 1;
            self.result_table_state.select(Some(0)); // Reset selection to first row of new page
        }
    }

    /// Go to previous page in query result.
    pub(super) fn prev_page(&mut self) {
        if self.result_page > 0 {
            self.result_page -= 1;
            self.result_table_state.select(Some(0)); // Reset selection to first row of new page
        }
    }

    /// Go to first page in query result.
    pub(super) fn first_page(&mut self) {
        self.result_page = 0;
        self.result_table_state.select(Some(0));
    }

    /// Go to last page in query result.
    pub(super) fn last_page(&mut self) {
        let total = self.total_pages();
        if total > 0 {
            self.result_page = total - 1;
            self.result_table_state.select(Some(0));
        }
    }

    /// Scroll left (show previous columns).
    pub(super) fn scroll_left(&mut self) {
        self.scroll_x = self.scroll_x.saturating_sub(1);
    }

    /// Scroll right (show next columns).
    pub(super) fn scroll_right(&mut self) {
        let total_cols = self.get_total_columns();
        if total_cols > 0 && self.scroll_x < total_cols.saturating_sub(1) {
            self.scroll_x += 1;
        }
    }

    /// Get total number of columns for current view.
    fn get_total_columns(&self) -> usize {
        match self.view {
            ContentView::QueryResult => self
                .query_result
                .as_ref()
                .map(|r| r.columns.len())
                .unwrap_or(0),
            ContentView::Questions => 3, // ID, Name, Collection
            _ => 0,
        }
    }

    /// Get the number of rows in the current page.
    pub(super) fn current_page_row_count(&self) -> usize {
        self.query_result
            .as_ref()
            .map(|r| {
                let total_rows = r.rows.len();
                let page_start = self.result_page * self.rows_per_page;
                let page_end = (page_start + self.rows_per_page).min(total_rows);
                page_end - page_start
            })
            .unwrap_or(0)
    }

    /// Get row at logical index (respects search, filter, and sort).
    /// Returns the actual row from query_result based on search, filter, and sort indices.
    pub(super) fn get_visible_row(&self, logical_index: usize) -> Option<&Vec<String>> {
        self.query_result.as_ref().and_then(|result| {
            // Step 1: Apply sort (if active), get index into visible rows
            let sorted_index = if let Some(ref sort_idx) = self.sort_indices {
                *sort_idx.get(logical_index)?
            } else {
                logical_index
            };

            // Step 2: Get actual row index from filter or search indices
            // Priority: Filter (which may operate on search results) > Search > None
            let actual_index = match (&self.filter_indices, &self.result_search_indices) {
                // Filter is active (may be filtering search results)
                (Some(filter), _) => *filter.get(sorted_index)?,
                // Only search active
                (None, Some(search)) => *search.get(sorted_index)?,
                // Neither: direct index
                (None, None) => sorted_index,
            };

            result.rows.get(actual_index)
        })
    }

    /// Scroll result table up by multiple rows (PageUp).
    pub(super) fn scroll_result_page_up(&mut self) {
        const SCROLL_AMOUNT: usize = 10;
        let current = self.result_table_state.selected().unwrap_or(0);
        let new = current.saturating_sub(SCROLL_AMOUNT);
        self.result_table_state.select(Some(new));
    }

    /// Scroll result table down by multiple rows (PageDown).
    pub(super) fn scroll_result_page_down(&mut self) {
        const SCROLL_AMOUNT: usize = 10;
        let current = self.result_table_state.selected().unwrap_or(0);
        let page_row_count = self.current_page_row_count();
        let new = (current + SCROLL_AMOUNT).min(page_row_count.saturating_sub(1));
        self.result_table_state.select(Some(new));
    }

    /// Reset sort, filter, and search state (helper for view transitions).
    pub(super) fn reset_sort_filter_state(&mut self) {
        // Sort state
        self.sort_order = SortOrder::None;
        self.sort_column_index = None;
        self.sort_mode_active = false;
        // Filter state
        self.filter_column_index = None;
        self.filter_text.clear();
        self.filter_mode_active = false;
        self.filter_modal_step = 0;
        // Result search state
        self.result_search_active = false;
        self.result_search_text.clear();
        self.result_search_indices = None;
    }
}
