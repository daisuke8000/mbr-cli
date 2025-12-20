//! Copy menu modal component.
//!
//! Displays format selection menu for copying record data to clipboard.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use super::clipboard::CopyFormat;

/// Copy menu state for format selection.
pub struct CopyMenu {
    /// Currently selected menu item (0=JSON, 1=CSV, 2=TSV)
    selected: usize,
    /// Whether to include header row (for CSV/TSV)
    include_header: bool,
    /// Multiple records for multi-select copy
    records: Vec<(Vec<String>, Vec<String>)>,
}

/// Available format options
const FORMATS: [CopyFormat; 3] = [CopyFormat::Json, CopyFormat::Csv, CopyFormat::Tsv];

impl CopyMenu {
    /// Create a new copy menu with single record data.
    pub fn new(columns: Vec<String>, values: Vec<String>) -> Self {
        Self {
            selected: 0,
            include_header: true,
            records: vec![(columns, values)],
        }
    }

    /// Create a new copy menu with multiple records.
    pub fn new_multi(records: Vec<(Vec<String>, Vec<String>)>) -> Self {
        Self {
            selected: 0,
            include_header: true,
            records,
        }
    }

    /// Get the number of records.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Check if this is a multi-record copy.
    pub fn is_multi(&self) -> bool {
        self.records.len() > 1
    }

    /// Move selection up.
    pub fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down.
    pub fn select_down(&mut self) {
        if self.selected < FORMATS.len() - 1 {
            self.selected += 1;
        }
    }

    /// Toggle header inclusion.
    pub fn toggle_header(&mut self) {
        self.include_header = !self.include_header;
    }

    /// Get currently selected format.
    pub fn selected_format(&self) -> CopyFormat {
        FORMATS[self.selected]
    }

    /// Get header inclusion setting.
    pub fn include_header(&self) -> bool {
        self.include_header
    }

    /// Get single record data (columns and values).
    /// For multi-record, returns the first record.
    pub fn record_data(&self) -> (&[String], &[String]) {
        self.records
            .first()
            .map(|(cols, vals)| (cols.as_slice(), vals.as_slice()))
            .unwrap_or((&[], &[]))
    }

    /// Get all records data (for multi-record copy).
    pub fn records_data(&self) -> &[(Vec<String>, Vec<String>)] {
        &self.records
    }

    /// Render the copy menu as a centered overlay.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Calculate centered popup (40% width, 35% height)
        let popup_area = Self::centered_rect(40, 35, area);

        // Clear background
        frame.render_widget(Clear, popup_area);

        // Build menu items
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        // Format selection
        lines.push(Line::from(Span::styled(
            "  Format:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        for (i, format) in FORMATS.iter().enumerate() {
            let is_selected = i == self.selected;
            let prefix = if is_selected { "  ► " } else { "    " };
            let key_hint = format!("[{}] ", format.key());

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(key_hint, Style::default().fg(Color::Yellow)),
                Span::styled(format.label(), style),
            ]));
        }

        lines.push(Line::from(""));

        // Header toggle (note: doesn't affect JSON)
        let header_indicator = if self.include_header { "[x]" } else { "[ ]" };
        lines.push(Line::from(vec![
            Span::styled("  [", Style::default().fg(Color::DarkGray)),
            Span::styled("h", Style::default().fg(Color::Yellow)),
            Span::styled("] ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} Include header", header_indicator),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        lines.push(Line::from(""));

        // Help line
        lines.push(Line::from(vec![
            Span::styled("  [", Style::default().fg(Color::DarkGray)),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::styled(" Move] [", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::styled(" Copy] [", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::styled(" Cancel]", Style::default().fg(Color::DarkGray)),
        ]));

        // Dynamic title based on record count
        let title = if self.is_multi() {
            format!(" Copy {} Records ", self.record_count())
        } else {
            " Copy Record ".to_string()
        };

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, popup_area);
    }

    /// Calculate centered rect with percentage-based dimensions.
    fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }
}
