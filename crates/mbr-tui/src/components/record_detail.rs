//! Record detail overlay component.
//!
//! Displays a modal overlay showing all fields of a selected record.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use unicode_width::UnicodeWidthStr;

/// Record detail overlay showing all fields of a selected record.
pub struct RecordDetailOverlay {
    /// Column names
    columns: Vec<String>,
    /// Row values
    values: Vec<String>,
    /// Currently selected field index (cursor position)
    selected_index: usize,
    /// Scroll offset for viewport management
    scroll_offset: usize,
}

impl RecordDetailOverlay {
    /// Create a new record detail overlay.
    pub fn new(columns: Vec<String>, values: Vec<String>) -> Self {
        Self {
            columns,
            values,
            selected_index: 0,
            scroll_offset: 0,
        }
    }

    /// Move cursor up by one line.
    pub fn scroll_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move cursor down by one line.
    pub fn scroll_down(&mut self) {
        let max_index = self.columns.len().saturating_sub(1);
        if self.selected_index < max_index {
            self.selected_index += 1;
        }
    }

    /// Render the record detail overlay centered on screen.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Calculate centered popup area (70% width, 80% height)
        let popup_area = Self::centered_rect(70, 80, area);

        // Clear the background
        frame.render_widget(Clear, popup_area);

        // Calculate visible area height (subtract borders, title, footer, help)
        // Border top (1) + empty line (1) + fields + empty line (1) + info (1) + empty line (1) + help (1) + border bottom (1)
        let content_height = popup_area.height.saturating_sub(8) as usize;
        let visible_fields = content_height.max(1);

        // Adjust scroll offset to keep selected item visible
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_fields {
            self.scroll_offset = self.selected_index.saturating_sub(visible_fields - 1);
        }

        // Build content lines
        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(""));

        // Find max column name display width for alignment (CJK chars = 2 width)
        let max_col_width = self
            .columns
            .iter()
            .map(|c| c.width())
            .max()
            .unwrap_or(0)
            .min(24); // Cap at 24 display width

        // Add each field as a line (only visible range)
        let end_index = (self.scroll_offset + visible_fields).min(self.columns.len());
        for i in self.scroll_offset..end_index {
            let col = &self.columns[i];
            let val = &self.values[i];
            let is_selected = i == self.selected_index;

            // Truncate column name if too long (use display width for Unicode)
            let col_width = col.width();
            let col_display = if col_width > max_col_width {
                Self::truncate_to_width(col, max_col_width - 3)
            } else {
                col.clone()
            };

            // Calculate padding for right-alignment (based on display width)
            let display_width = col_display.width();
            let padding = max_col_width.saturating_sub(display_width);
            let padded_col = format!("{}{}", " ".repeat(padding), col_display);

            // Format value - show full content
            let val_display = if val.is_empty() {
                "(empty)".to_string()
            } else {
                // Truncate long values for display
                let max_val_width = popup_area.width.saturating_sub(max_col_width as u16 + 10) as usize;
                if val.width() > max_val_width {
                    Self::truncate_to_width(val, max_val_width)
                } else {
                    val.clone()
                }
            };

            // Apply styles based on selection
            let (prefix, col_style, separator_style, val_style) = if is_selected {
                (
                    "► ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::Black).bg(Color::Cyan),
                    Style::default().fg(Color::Black).bg(Color::Cyan),
                )
            } else {
                (
                    "  ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                    Style::default().fg(Color::DarkGray),
                    Style::default(),
                )
            };

            lines.push(Line::from(vec![
                Span::styled(prefix, if is_selected { Style::default().bg(Color::Cyan) } else { Style::default() }),
                Span::styled(padded_col, col_style),
                Span::styled(" : ", separator_style),
                Span::styled(val_display, val_style),
            ]));
        }

        lines.push(Line::from(""));

        // Scroll/position indicator
        let total_fields = self.columns.len();
        let scroll_info = if total_fields > 0 {
            let scroll_indicator = if total_fields > visible_fields {
                format!(
                    " (scroll {}-{}/{})",
                    self.scroll_offset + 1,
                    end_index,
                    total_fields
                )
            } else {
                String::new()
            };
            format!(
                "  Field {}/{}{}",
                self.selected_index + 1,
                total_fields,
                scroll_indicator
            )
        } else {
            "  No fields".to_string()
        };

        lines.push(Line::from(Span::styled(
            scroll_info,
            Style::default().fg(Color::DarkGray),
        )));

        // Help line
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  [", Style::default().fg(Color::DarkGray)),
            Span::styled("↑↓/jk", Style::default().fg(Color::Yellow)),
            Span::styled(" Move] [", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc/Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" Close]", Style::default().fg(Color::DarkGray)),
        ]));

        // Create the paragraph widget
        let detail_text = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Record Detail ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left);

        frame.render_widget(detail_text, popup_area);
    }

    /// Truncate string to fit within max display width, adding "..." suffix.
    fn truncate_to_width(s: &str, max_width: usize) -> String {
        let mut result = String::new();
        let mut current_width = 0;

        for ch in s.chars() {
            let char_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + char_width > max_width {
                break;
            }
            result.push(ch);
            current_width += char_width;
        }

        format!("{}...", result)
    }

    /// Calculate a centered rect with percentage-based dimensions.
    fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);

        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }
}
