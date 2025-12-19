//! Help overlay component.
//!
//! Displays a modal overlay showing all available keybindings.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Help overlay showing keybindings.
pub struct HelpOverlay;

impl HelpOverlay {
    /// Keybinding groups for display.
    const GLOBAL_KEYS: &'static [(&'static str, &'static str)] = &[
        ("q / Esc", "Quit application"),
        ("Tab", "Switch between panels"),
        ("Shift+Tab", "Switch panels (reverse)"),
        ("r", "Refresh data"),
        ("?", "Toggle help"),
    ];

    const NAVIGATION_KEYS: &'static [(&'static str, &'static str)] = &[
        ("↑ / k", "Move up"),
        ("↓ / j", "Move down"),
        ("Home / g", "Go to first item"),
        ("End / G", "Go to last item"),
        ("Enter", "Select item"),
    ];

    /// Render the help overlay centered on screen.
    pub fn render(frame: &mut Frame, area: Rect) {
        // Calculate centered popup area
        let popup_area = Self::centered_rect(60, 70, area);

        // Clear the background
        frame.render_widget(Clear, popup_area);

        // Build help content
        let mut lines: Vec<Line> = Vec::new();

        // Title section
        lines.push(Line::from(""));

        // Global keybindings section
        lines.push(Line::from(Span::styled(
            "  Global",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from("  ──────────────────────────────────"));
        for (key, action) in Self::GLOBAL_KEYS {
            lines.push(Self::format_keybinding(key, action));
        }

        lines.push(Line::from(""));

        // Navigation keybindings section
        lines.push(Line::from(Span::styled(
            "  Navigation",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from("  ──────────────────────────────────"));
        for (key, action) in Self::NAVIGATION_KEYS {
            lines.push(Self::format_keybinding(key, action));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )));

        // Create the paragraph widget
        let help_text = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Help ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left);

        frame.render_widget(help_text, popup_area);
    }

    /// Format a single keybinding line.
    fn format_keybinding(key: &str, action: &str) -> Line<'static> {
        Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{:<14}", key),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(action.to_string()),
        ])
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
