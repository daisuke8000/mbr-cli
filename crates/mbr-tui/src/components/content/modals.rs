//! Modal rendering functions for ContentPanel.
//!
//! This module contains rendering functions for modal overlays:
//! - Sort column selection modal
//! - Filter column/text input modal
//! - Result search bar overlay

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::{ContentPanel, QueryResultData, SortOrder};

impl ContentPanel {
    /// Render result search bar as an overlay at the bottom.
    pub(super) fn render_result_search_bar(&self, frame: &mut Frame, area: Rect) {
        if !self.result_search_active && self.result_search_text.is_empty() {
            return;
        }

        // Search bar at bottom of content area
        let bar_height = 3u16;
        let bar_y = area.y + area.height.saturating_sub(bar_height);
        let bar_area = Rect::new(area.x, bar_y, area.width, bar_height);

        // Clear background for better visibility
        frame.render_widget(Clear, bar_area);

        // Build search display
        let total = self
            .query_result
            .as_ref()
            .map(|r| r.rows.len())
            .unwrap_or(0);
        let matched = self.visible_row_count();

        let (input_display, title_suffix) = if self.result_search_active {
            (format!("{}_", self.result_search_text), " (editing)")
        } else {
            (self.result_search_text.clone(), "")
        };

        let title = format!(" Search Results{} ", title_suffix);

        let lines = vec![
            Line::from(vec![
                Span::styled(" > ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    input_display,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!(" Found: {} / {} rows", matched, total),
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .title_style(
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            )
            .style(Style::default().bg(Color::Black));

        frame.render_widget(paragraph, bar_area);
    }

    /// Render filter column/text input modal as an overlay.
    pub(super) fn render_filter_modal(&self, frame: &mut Frame, area: Rect) {
        let result = match &self.query_result {
            Some(r) => r,
            None => return,
        };

        if self.filter_modal_step == 0 {
            // Step 0: Column selection (similar to sort modal)
            self.render_filter_column_selection(frame, area, result);
        } else {
            // Step 1: Text input
            self.render_filter_text_input(frame, area, result);
        }
    }

    /// Render filter column selection modal (step 0).
    fn render_filter_column_selection(
        &self,
        frame: &mut Frame,
        area: Rect,
        result: &QueryResultData,
    ) {
        // Calculate modal dimensions (40% width, centered)
        let modal_width = (area.width as f32 * 0.4).clamp(30.0, 60.0) as u16;
        let max_height = (result.columns.len() + 4).min(20) as u16;
        let modal_height = max_height.min(area.height.saturating_sub(4));

        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(
            area.x + modal_x,
            area.y + modal_y,
            modal_width,
            modal_height,
        );

        // Clear background for better visibility
        frame.render_widget(Clear, modal_area);

        // Build column list items
        let items: Vec<Line> = result
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let is_selected = i == self.filter_modal_selection;
                let is_filtered = self.filter_column_index == Some(i);

                // Build column text with filter indicator
                let filter_indicator = if is_filtered { " ⚡" } else { "" };
                let prefix = if is_selected { "► " } else { "  " };
                let text = format!("{}{}{}", prefix, col, filter_indicator);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Magenta)
                        .add_modifier(Modifier::BOLD)
                } else if is_filtered {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default().fg(Color::White)
                };

                Line::from(Span::styled(text, style))
            })
            .collect();

        let paragraph = Paragraph::new(items)
            .block(
                Block::default()
                    .title(" Filter by Column ")
                    .title_style(
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .style(Style::default().bg(Color::Black));

        frame.render_widget(paragraph, modal_area);

        // Render footer hint
        if modal_area.height > 2 {
            let footer_area = Rect::new(
                modal_area.x + 1,
                modal_area.y + modal_area.height.saturating_sub(1),
                modal_area.width.saturating_sub(2),
                1,
            );
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(": Next  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(": Cancel", Style::default().fg(Color::DarkGray)),
            ]))
            .style(Style::default().bg(Color::Black));
            frame.render_widget(hint, footer_area);
        }
    }

    /// Render filter text input modal (step 1).
    fn render_filter_text_input(&self, frame: &mut Frame, area: Rect, result: &QueryResultData) {
        // Calculate modal dimensions
        let modal_width = (area.width as f32 * 0.5).clamp(40.0, 70.0) as u16;
        let modal_height = 7_u16; // Fixed height for text input

        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(
            area.x + modal_x,
            area.y + modal_y,
            modal_width,
            modal_height,
        );

        // Clear background for better visibility
        frame.render_widget(Clear, modal_area);

        // Get column name for title
        let col_name = result
            .columns
            .get(self.filter_modal_selection)
            .map(|s| s.as_str())
            .unwrap_or("Column");

        let title = format!(" Filter: {} ", col_name);

        // Build input display with cursor
        let input_display = format!("{}_", self.filter_text);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Enter filter text (case-insensitive):",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}", input_display),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .title_style(
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .style(Style::default().bg(Color::Black));

        frame.render_widget(paragraph, modal_area);

        // Render footer hint
        if modal_area.height > 2 {
            let footer_area = Rect::new(
                modal_area.x + 1,
                modal_area.y + modal_area.height.saturating_sub(1),
                modal_area.width.saturating_sub(2),
                1,
            );
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(": Apply  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(": Back  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Backspace", Style::default().fg(Color::Yellow)),
                Span::styled(": Delete", Style::default().fg(Color::DarkGray)),
            ]))
            .style(Style::default().bg(Color::Black));
            frame.render_widget(hint, footer_area);
        }
    }

    /// Render sort column selection modal as an overlay.
    pub(super) fn render_sort_modal(&self, frame: &mut Frame, area: Rect) {
        let result = match &self.query_result {
            Some(r) => r,
            None => return,
        };

        // Calculate modal dimensions (40% width, centered, max height for columns + header/footer)
        let modal_width = (area.width as f32 * 0.4).clamp(30.0, 60.0) as u16;
        let max_height = (result.columns.len() + 4).min(20) as u16;
        let modal_height = max_height.min(area.height.saturating_sub(4));

        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(
            area.x + modal_x,
            area.y + modal_y,
            modal_width,
            modal_height,
        );

        // Clear background for better visibility
        frame.render_widget(Clear, modal_area);

        // Build column list items
        let items: Vec<Line> = result
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let is_selected = i == self.sort_modal_selection;
                let is_sorted = self.sort_column_index == Some(i);

                // Build column text with sort indicator
                let sort_indicator = if is_sorted {
                    match self.sort_order {
                        SortOrder::Ascending => " ↑",
                        SortOrder::Descending => " ↓",
                        SortOrder::None => "",
                    }
                } else {
                    ""
                };

                let prefix = if is_selected { "► " } else { "  " };
                let text = format!("{}{}{}", prefix, col, sort_indicator);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if is_sorted {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };

                Line::from(Span::styled(text, style))
            })
            .collect();

        let paragraph = Paragraph::new(items)
            .block(
                Block::default()
                    .title(" Sort by Column ")
                    .title_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().bg(Color::Black));

        frame.render_widget(paragraph, modal_area);

        // Render footer hint
        if modal_area.height > 2 {
            let footer_area = Rect::new(
                modal_area.x + 1,
                modal_area.y + modal_area.height.saturating_sub(1),
                modal_area.width.saturating_sub(2),
                1,
            );
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(": Select  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(": Cancel", Style::default().fg(Color::DarkGray)),
            ]))
            .style(Style::default().bg(Color::Black));
            frame.render_widget(hint, footer_area);
        }
    }
}
