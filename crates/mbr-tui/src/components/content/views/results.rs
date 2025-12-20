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
    HIGHLIGHT_SYMBOL, border_style, header_style, result_row_highlight_style,
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

        // Slice columns based on scroll position
        let visible_columns: Vec<String> = result.columns[scroll_x..end_col].to_vec();
        let visible_col_count = visible_columns.len();

        // Create dynamic column widths
        let constraints: Vec<Constraint> = if visible_col_count <= 3 {
            visible_columns
                .iter()
                .map(|_| Constraint::Ratio(1, visible_col_count as u32))
                .collect()
        } else {
            visible_columns
                .iter()
                .map(|_| Constraint::Min(15))
                .collect()
        };

        // Create table rows with sliced cells (only current page)
        let rows: Vec<Row> = (page_start..page_end)
            .filter_map(|logical_idx| self.get_visible_row(logical_idx))
            .map(|row| {
                let cells: Vec<Cell> = row[scroll_x..end_col.min(row.len())]
                    .iter()
                    .map(|cell| Cell::from(cell.clone()))
                    .collect();
                Row::new(cells)
            })
            .collect();

        // Create header row with sort indicators
        let header_cells: Vec<Cell> = visible_columns
            .iter()
            .enumerate()
            .map(|(visible_idx, col)| {
                let actual_col_idx = scroll_x + visible_idx;
                let is_sorted = self.sort_column_index == Some(actual_col_idx);

                let header_text = if is_sorted {
                    let indicator = match self.sort_order {
                        SortOrder::Ascending => " ↑",
                        SortOrder::Descending => " ↓",
                        SortOrder::None => "",
                    };
                    format!("{}{}", col, indicator)
                } else {
                    col.clone()
                };

                Cell::from(header_text)
            })
            .collect();

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
