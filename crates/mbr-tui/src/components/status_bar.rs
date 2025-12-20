//! Status bar component.
//!
//! Displays keybindings and status messages at the bottom of the screen.

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::Component;

/// Key binding display item.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: &'static str,
    pub action: &'static str,
}

impl KeyBinding {
    pub const fn new(key: &'static str, action: &'static str) -> Self {
        Self { key, action }
    }
}

/// Status bar showing keybindings and messages.
pub struct StatusBar {
    message: String,
    bindings: Vec<KeyBinding>,
    /// Number of selected rows (for multi-select display)
    selection_count: usize,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBar {
    /// Create a new status bar with default keybindings.
    pub fn new() -> Self {
        Self {
            message: String::new(),
            bindings: vec![
                KeyBinding::new("1/2/3", "Tab"),
                KeyBinding::new("↑↓", "Nav"),
                KeyBinding::new("←→", "Scroll"),
                KeyBinding::new("n/p", "Page"),
                KeyBinding::new("Enter", "Run"),
                KeyBinding::new("?", "Help"),
                KeyBinding::new("q", "Quit"),
            ],
            selection_count: 0,
        }
    }

    /// Set a status message.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Update the selection count for multi-select display.
    pub fn set_selection_count(&mut self, count: usize) {
        self.selection_count = count;
    }
}

impl Component for StatusBar {
    fn draw(&mut self, frame: &mut Frame, area: Rect, _focused: bool) {
        let mut spans: Vec<Span> = Vec::new();

        // Add keybindings
        for (i, binding) in self.bindings.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }
            spans.push(Span::styled(
                format!(" {} ", binding.key),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::raw(binding.action));
        }

        // Add selection count if any rows are selected
        if self.selection_count > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!(" {} selected ", self.selection_count),
                Style::default().fg(Color::Yellow),
            ));
        }

        // Add message if present
        if !self.message.is_empty() {
            spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                self.message.clone(),
                Style::default().fg(Color::Green),
            ));
        }

        let paragraph = Paragraph::new(Line::from(spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        frame.render_widget(paragraph, area);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        // Status bar doesn't handle keys
        false
    }
}
