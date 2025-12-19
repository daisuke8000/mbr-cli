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

use crate::components::{
    ActivePanel, Component, ContentPanel, ContentView, NavigationPanel, StatusBar,
};
use crate::event::{Event, EventHandler};

/// The main application state.
pub struct App {
    /// Whether the application should quit
    pub should_quit: bool,
    /// Currently active panel
    active_panel: ActivePanel,
    /// Navigation panel (left)
    navigation: NavigationPanel,
    /// Content panel (right)
    content: ContentPanel,
    /// Status bar (bottom)
    status_bar: StatusBar,
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
            active_panel: ActivePanel::Navigation,
            navigation: NavigationPanel::new(),
            content: ContentPanel::new(),
            status_bar: StatusBar::new(),
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
        // Global keybindings (always active)
        match code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
                return;
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return;
            }
            KeyCode::Esc => {
                self.should_quit = true;
                return;
            }
            KeyCode::Tab => {
                self.active_panel = if modifiers.contains(KeyModifiers::SHIFT) {
                    self.active_panel.previous()
                } else {
                    self.active_panel.next()
                };
                return;
            }
            KeyCode::BackTab => {
                self.active_panel = self.active_panel.previous();
                return;
            }
            _ => {}
        }

        // Panel-specific keybindings
        match self.active_panel {
            ActivePanel::Navigation => {
                // Handle Enter to switch content view
                if code == KeyCode::Enter {
                    self.handle_navigation_select();
                    return;
                }
                self.navigation
                    .handle_key(crossterm::event::KeyEvent::new(code, modifiers));
            }
            ActivePanel::Content => {
                self.content
                    .handle_key(crossterm::event::KeyEvent::new(code, modifiers));
            }
        }
    }

    /// Handle navigation item selection.
    fn handle_navigation_select(&mut self) {
        let view = match self.navigation.selected() {
            0 => ContentView::Questions,
            1 => ContentView::Collections,
            2 => ContentView::Databases,
            3 => ContentView::Settings,
            _ => ContentView::Welcome,
        };
        self.content.set_view(view);

        // Update status message
        if let Some(item) = self.navigation.selected_item() {
            self.status_bar
                .set_message(format!("Viewing: {}", item.label));
        }
    }

    /// Draw the UI.
    fn draw(&self, frame: &mut Frame) {
        let size = frame.area();

        // Create main layout: Header, Main, Footer
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Status bar
            ])
            .split(size);

        // Draw header
        self.draw_header(frame, main_chunks[0]);

        // Split main area into navigation (left) and content (right)
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Navigation panel
                Constraint::Percentage(75), // Content panel
            ])
            .split(main_chunks[1]);

        // Draw panels with focus state
        self.navigation.draw(
            frame,
            content_chunks[0],
            self.active_panel == ActivePanel::Navigation,
        );
        self.content.draw(
            frame,
            content_chunks[1],
            self.active_panel == ActivePanel::Content,
        );

        // Draw status bar
        self.status_bar.draw(frame, main_chunks[2], false);
    }

    /// Draw the header.
    fn draw_header(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " mbr-tui ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("- Metabase Terminal UI "),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" Active: {} ", self.active_panel_name()),
                Style::default().fg(Color::Yellow),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(header, area);
    }

    /// Get the name of the active panel.
    fn active_panel_name(&self) -> &str {
        match self.active_panel {
            ActivePanel::Navigation => "Navigation",
            ActivePanel::Content => "Content",
        }
    }
}
