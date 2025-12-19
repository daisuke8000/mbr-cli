//! Content panel component.
//!
//! Displays the main content area (query results, question details, etc.).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::{Component, ScrollState};

/// Content view types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentView {
    #[default]
    Welcome,
    Questions,
    Collections,
    Databases,
    Settings,
}

/// Content panel showing main content.
pub struct ContentPanel {
    view: ContentView,
    scroll: ScrollState,
}

impl Default for ContentPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentPanel {
    /// Create a new content panel.
    pub fn new() -> Self {
        Self {
            view: ContentView::Welcome,
            scroll: ScrollState::default(),
        }
    }

    /// Set the current view.
    pub fn set_view(&mut self, view: ContentView) {
        self.view = view;
        self.scroll = ScrollState::default();
    }

    /// Render welcome view content.
    fn render_welcome(&self, _area: Rect, focused: bool) -> Paragraph<'static> {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let banner = r#"
                 _                  _           _
  _ __ ___  | |__  _ __       | |_ _   _ (_)
 | '_ ` _ \ | '_ \| '__|_____ | __| | | || |
 | | | | | || |_) | |  |_____|| |_| |_| || |
 |_| |_| |_||_.__/|_|         \__|\__,_||_|
"#;

        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));

        for banner_line in banner.lines() {
            lines.push(Line::from(Span::styled(
                banner_line.to_string(),
                Style::default().fg(Color::Cyan),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Welcome to mbr-tui!",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Select an item from the navigation panel to get started.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from("  Quick Keys:"));
        lines.push(Line::from(Span::styled(
            "    Tab       - Switch panels",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "    ↑/↓ j/k   - Navigate items",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "    Enter     - Select item",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            "    q         - Quit",
            Style::default().fg(Color::Yellow),
        )));

        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Welcome ")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: false })
    }

    /// Render placeholder for unimplemented views.
    fn render_placeholder(&self, title: &str, focused: bool) -> Paragraph<'static> {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {} view", title),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Coming soon in Phase 3...",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(format!(" {} ", title))
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: false })
    }
}

impl Component for ContentPanel {
    fn draw(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let widget = match self.view {
            ContentView::Welcome => self.render_welcome(area, focused),
            ContentView::Questions => self.render_placeholder("Questions", focused),
            ContentView::Collections => self.render_placeholder("Collections", focused),
            ContentView::Databases => self.render_placeholder("Databases", focused),
            ContentView::Settings => self.render_placeholder("Settings", focused),
        };

        frame.render_widget(widget, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll.scroll_up();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll.scroll_down();
                true
            }
            KeyCode::PageUp => {
                for _ in 0..self.scroll.visible {
                    self.scroll.scroll_up();
                }
                true
            }
            KeyCode::PageDown => {
                for _ in 0..self.scroll.visible {
                    self.scroll.scroll_down();
                }
                true
            }
            _ => false,
        }
    }

    fn title(&self) -> &str {
        match self.view {
            ContentView::Welcome => "Welcome",
            ContentView::Questions => "Questions",
            ContentView::Collections => "Collections",
            ContentView::Databases => "Databases",
            ContentView::Settings => "Settings",
        }
    }
}
