//! Record detail overlay component.
//!
//! Displays a modal overlay showing all fields of a selected record.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

/// Record detail overlay showing all fields of a selected record.
pub struct RecordDetailOverlay {
    /// Column names
    columns: Vec<String>,
    /// Row values
    values: Vec<String>,
    /// Current scroll offset
    scroll_offset: usize,
}

impl RecordDetailOverlay {
    /// Create a new record detail overlay.
    pub fn new(columns: Vec<String>, values: Vec<String>) -> Self {
        Self {
            columns,
            values,
            scroll_offset: 0,
        }
    }

    /// Scroll up by one line.
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down by one line.
    pub fn scroll_down(&mut self) {
        let max_scroll = self.columns.len().saturating_sub(1);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Render the record detail overlay centered on screen.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Calculate centered popup area (70% width, 80% height)
        let popup_area = Self::centered_rect(70, 80, area);

        // Clear the background
        frame.render_widget(Clear, popup_area);

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

        // Add each field as a line
        for (i, (col, val)) in self.columns.iter().zip(self.values.iter()).enumerate() {
            // Skip if before scroll offset
            if i < self.scroll_offset {
                continue;
            }

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

            // Format value - show full content (Wrap handles line breaks)
            let val_display = if val.is_empty() {
                "(empty)".to_string()
            } else {
                val.clone()
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    padded_col,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" : ", Style::default().fg(Color::DarkGray)),
                Span::raw(val_display),
            ]));
        }

        lines.push(Line::from(""));

        // Scroll indicator
        let total_fields = self.columns.len();
        let scroll_info = if total_fields > 0 {
            format!(
                "  Field {}/{} ",
                (self.scroll_offset + 1).min(total_fields),
                total_fields
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
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::styled(" Scroll] [", Style::default().fg(Color::DarkGray)),
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
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });

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
