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
                KeyBinding::new("Tab", "Switch"),
                KeyBinding::new("↑↓", "Navigate"),
                KeyBinding::new("Enter", "Select"),
                KeyBinding::new("q", "Quit"),
                KeyBinding::new("?", "Help"),
            ],
        }
    }

    /// Set a status message.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

}

impl Component for StatusBar {
    fn draw(&self, frame: &mut Frame, area: Rect, _focused: bool) {
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

    fn title(&self) -> &str {
        ""
    }
}
