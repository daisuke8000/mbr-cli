//! Query result and table preview rendering.

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::components::content::{ContentPanel, ContentView, SortOrder};
use crate::components::styles::{
    HIGHLIGHT_SYMBOL, border_style, header_style, multi_selected_style, result_row_highlight_style,
};

impl ContentPanel {
    /// Render table preview view with query result table.
    /// Shows sample data from a table (reuses query result rendering).
    pub(in crate::components::content) fn render_table_preview(
        &mut self,
        area: Rect,
        frame: &mut Frame,
        focused: bool,
    ) {
        // Get table name from ContentView variant (clone to avoid borrow conflict)
        let table_name = match &self.view {
            ContentView::TablePreview { table_name, .. } => table_name.clone(),
            _ => "Unknown".to_string(),
        };

        // Check if we have result data and if it's non-empty
        let has_data = self
            .query_result
            .as_ref()
            .map(|r| !r.rows.is_empty())
            .unwrap_or(false);
        let is_empty = self
            .query_result
            .as_ref()
            .map(|r| r.rows.is_empty())
            .unwrap_or(false);

        if self.query_result.is_none() {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  Loading preview for '{}'...", table_name),
                    Style::default().fg(Color::Yellow),
                )),
            ])
            .block(
                Block::default()
                    .title(format!(" {} - Preview ", table_name))
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            );
            frame.render_widget(paragraph, area);
        } else if is_empty {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  Table: {}", table_name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  No data in table",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Esc to go back",
                    Style::default().fg(Color::Yellow),
                )),
            ])
            .block(
                Block::default()
                    .title(format!(" {} - Preview (0 rows) ", table_name))
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            );
            frame.render_widget(paragraph, area);
        } else if has_data {
            self.render_result_table(frame, area, focused, &table_name);
        }
    }

    /// Render query result view with table.
    pub(in crate::components::content) fn render_query_result(
        &mut self,
        area: Rect,
        frame: &mut Frame,
        focused: bool,
    ) {
        // Extract info before mutable borrow
        let (has_result, is_empty, question_name) = match &self.query_result {
            None => (false, false, String::new()),
            Some(result) => (true, result.rows.is_empty(), result.question_name.clone()),
        };

        if !has_result {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No query result available",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(
                Block::default()
                    .title(" Query Result ")
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            );
            frame.render_widget(paragraph, area);
        } else if is_empty {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  Query: {}", question_name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  No data returned",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Esc to go back",
                    Style::default().fg(Color::Yellow),
                )),
            ])
            .block(
                Block::default()
                    .title(format!(" Query Result: {} (0 rows) ", question_name))
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            );
            frame.render_widget(paragraph, area);
        } else {
            self.render_result_table(frame, area, focused, &question_name);
        }
    }

    /// Common result table rendering logic for both TablePreview and QueryResult.
    /// Accesses self.query_result internally to avoid borrow conflicts.
    pub(in crate::components::content) fn render_result_table(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        focused: bool,
        title_prefix: &str,
    ) {
        // Process dirty flags before rendering (lazy recomputation)
        if self.search_dirty {
            self.update_result_search_indices();
            self.search_dirty = false;
        }
        if self.filter_dirty {
            self.update_filter_indices();
            self.filter_dirty = false;
        }
        if self.sort_dirty {
            self.update_sort_indices();
            self.sort_dirty = false;
        }

        // Get result reference - caller guarantees query_result is Some
        let result = match &self.query_result {
            Some(r) => r,
            None => return,
        };

        // Pagination: calculate row range for current page
        let total_rows = result.rows.len();
        let total_pages = self.total_pages();
        let page_start = self.result_page * self.rows_per_page;
        let page_end = (page_start + self.rows_per_page).min(total_rows);

        // Calculate visible columns based on scroll_x
        let total_cols = result.columns.len();
        let scroll_x = self.scroll_x.min(total_cols.saturating_sub(1));

        // Calculate how many columns can fit (estimate based on min width)
        let available_width = area.width.saturating_sub(4) as usize;
        let min_col_width = 15usize;
        let visible_cols = (available_width / min_col_width).max(1).min(total_cols);
        let end_col = (scroll_x + visible_cols).min(total_cols);

        // Slice columns based on scroll position (use reference, avoid clone)
        let visible_columns = &result.columns[scroll_x..end_col];
        let visible_col_count = visible_columns.len();

        // Always include selection gutter column to prevent layout shift
        // The gutter is always present but only shows ✓ for selected rows
        let mut constraints: Vec<Constraint> = Vec::new();
        constraints.push(Constraint::Length(2)); // Selection indicator column (always present)

        // Use pre-computed column widths if available, otherwise fall back to heuristic
        if let Some(ref cached_widths) = self.cached_column_widths {
            constraints.extend((scroll_x..end_col).map(|col_idx| {
                let width = cached_widths.get(col_idx).copied().unwrap_or(15);
                // Add 2 for padding
                Constraint::Min(width.max(8) + 2)
            }));
        } else if visible_col_count <= 3 {
            constraints.extend(
                visible_columns
                    .iter()
                    .map(|_| Constraint::Ratio(1, visible_col_count as u32)),
            );
        } else {
            constraints.extend(visible_columns.iter().map(|_| Constraint::Min(15)));
        }

        // Pre-collect row metadata to avoid borrow conflicts with render_stateful_widget.
        // We extract (original_idx, is_selected, sliced cells as owned) to release
        // the immutable borrow on self before render_stateful_widget borrows mutably.
        struct RowInfo {
            is_selected: bool,
            cells: Vec<String>,
        }
        let row_infos: Vec<RowInfo> = (page_start..page_end)
            .filter_map(|logical_idx| {
                let row = self.get_visible_row(logical_idx)?;
                let original_idx = self.get_original_row_index(logical_idx);
                let is_selected = original_idx
                    .map(|idx| self.is_row_selected(idx))
                    .unwrap_or(false);
                let end = end_col.min(row.len());
                // Use as_str() + to_owned() for the visible slice only (not all columns)
                let cells: Vec<String> = row[scroll_x..end]
                    .iter()
                    .map(|cell| cell.to_owned())
                    .collect();
                Some(RowInfo { is_selected, cells })
            })
            .collect();

        // Build Row widgets from pre-collected data (no borrow on self.query_result)
        let rows: Vec<Row> = row_infos
            .into_iter()
            .map(|info| {
                let mut cells: Vec<Cell> = Vec::with_capacity(info.cells.len() + 1);
                // Always add selection indicator column (gutter)
                let indicator = if info.is_selected { "✓ " } else { "  " };
                cells.push(Cell::from(indicator));
                // Add data cells
                cells.extend(info.cells.into_iter().map(Cell::from));

                let mut row_widget = Row::new(cells);
                if info.is_selected {
                    row_widget = row_widget.style(multi_selected_style());
                }
                row_widget
            })
            .collect();

        // Create header row with sort indicators
        let mut header_cells: Vec<Cell> = Vec::new();

        // Always add empty gutter column header (matches data rows)
        header_cells.push(Cell::from("  "));

        // Add data column headers (avoid clone when no sort indicator needed)
        header_cells.extend(
            visible_columns
                .iter()
                .enumerate()
                .map(|(visible_idx, col)| {
                    let actual_col_idx = scroll_x + visible_idx;
                    let is_sorted = self.sort_column_index == Some(actual_col_idx);

                    if is_sorted {
                        let indicator = match self.sort_order {
                            SortOrder::Ascending => " ↑",
                            SortOrder::Descending => " ↓",
                            SortOrder::None => "",
                        };
                        Cell::from(format!("{}{}", col, indicator))
                    } else {
                        Cell::from(col.as_str())
                    }
                }),
        );

        // Build column indicator
        let col_indicator = if total_cols > visible_cols {
            let left_arrow = if scroll_x > 0 { "← " } else { "  " };
            let right_arrow = if end_col < total_cols { " →" } else { "  " };
            format!(
                " {}Col {}-{}/{}{}",
                left_arrow,
                scroll_x + 1,
                end_col,
                total_cols,
                right_arrow
            )
        } else {
            String::new()
        };

        // Build page indicator
        let page_indicator = if total_pages > 1 {
            format!(
                " Page {}/{} (rows {}-{} of {})",
                self.result_page + 1,
                total_pages,
                page_start + 1,
                page_end,
                total_rows
            )
        } else {
            format!(" {} rows", total_rows)
        };

        // Build sort indicator for title
        let sort_indicator = if let Some((col_name, order)) = self.get_sort_info() {
            let arrow = match order {
                SortOrder::Ascending => "↑",
                SortOrder::Descending => "↓",
                SortOrder::None => "",
            };
            format!(" [Sort: {} {}]", col_name, arrow)
        } else {
            String::new()
        };

        let table = Table::new(rows, constraints)
            .header(
                Row::new(header_cells)
                    .style(header_style())
                    .bottom_margin(1),
            )
            .block(
                Block::default()
                    .title(format!(
                        " {}{}{}{}",
                        title_prefix, page_indicator, col_indicator, sort_indicator
                    ))
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            )
            .row_highlight_style(result_row_highlight_style())
            .highlight_symbol(HIGHLIGHT_SYMBOL);

        frame.render_stateful_widget(table, area, &mut self.result_table_state);

        // Render overlays
        if self.sort_mode_active {
            self.render_sort_modal(frame, area);
        }
        if self.filter_mode_active {
            self.render_filter_modal(frame, area);
        }
        if self.result_search_active {
            self.render_result_search_bar(frame, area);
        }
    }
}
