//! Application state and logic for the TUI.
//!
//! This module contains the core application state and the main run loop.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::event::{Event, EventHandler};

/// ASCII art banner for mbr-tui
const BANNER: &str = r#"
                 _                  _           _
  _ __ ___  | |__  _ __       | |_ _   _ (_)
 | '_ ` _ \ | '_ \| '__|_____ | __| | | || |
 | | | | | || |_) | |  |_____|| |_| |_| || |
 |_| |_| |_||_.__/|_|         \__|\__,_||_|
"#;

/// The main application state.
pub struct App {
    /// Whether the application should quit
    pub should_quit: bool,
    /// Current status message
    pub status: String,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new application instance.
    pub fn new() -> Self {
        Self {
            should_quit: false,
            status: String::from("Welcome to mbr-tui! Press 'q' to quit."),
        }
    }

    /// Run the main application loop.
    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<impl ratatui::backend::Backend>,
    ) -> std::io::Result<()> {
        let event_handler = EventHandler::new(250);

        while !self.should_quit {
            // Draw the UI
            terminal.draw(|frame| self.draw(frame))?;

            // Handle events
            match event_handler.next()? {
                Event::Key(key) => self.handle_key(key.code, key.modifiers),
                Event::Resize(_, _) => {} // Terminal will redraw automatically
                Event::Tick => {}         // Can be used for animations/updates
            }
        }

        Ok(())
    }

    /// Handle keyboard input.
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                self.should_quit = true;
            }
            _ => {}
        }
    }

    /// Draw the UI.
    fn draw(&self, frame: &mut Frame) {
        let size = frame.area();

        // Create layout with header, main content, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer
            ])
            .split(size);

        // Header
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " mbr-tui ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("- Metabase Terminal UI"),
        ]))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, chunks[0]);

        // Main content area with banner
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        // Add banner with cyan color
        for banner_line in BANNER.lines() {
            lines.push(Line::from(Span::styled(
                banner_line.to_string(),
                Style::default().fg(Color::Cyan),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Metabase Terminal UI",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "  Interactive interface for your data",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        lines.push(Line::from("  Features:"));
        lines.push(Line::from(Span::styled(
            "    ◆ Question browser",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "    ◆ Query execution",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "    ◆ Result visualization",
            Style::default().fg(Color::Yellow),
        )));

        let main_content =
            Paragraph::new(lines).block(Block::default().title(" Welcome ").borders(Borders::ALL));
        frame.render_widget(main_content, chunks[1]);

        // Footer with status and keybindings
        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" q ", Style::default().fg(Color::Yellow)),
            Span::raw("Quit  "),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("  {}", self.status)),
        ]))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, chunks[2]);
    }
}
