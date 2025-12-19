//! Content panel component.
//!
//! Displays the main content area (query results, question details, etc.).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use mbr_core::api::models::Question;

use super::{Component, ScrollState};
use crate::service::LoadState;

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
    /// Questions data for the Questions view
    questions: LoadState<Vec<Question>>,
    /// Selected question index in list view
    selected_index: usize,
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
            questions: LoadState::default(),
            selected_index: 0,
        }
    }

    /// Set the current view.
    pub fn set_view(&mut self, view: ContentView) {
        self.view = view;
        self.scroll = ScrollState::default();
        self.selected_index = 0;
    }

    /// Update questions data from AppData.
    pub fn update_questions(&mut self, questions: &LoadState<Vec<Question>>) {
        self.questions = questions.clone();
    }

    /// Select next question in list.
    pub fn select_next(&mut self) {
        if let LoadState::Loaded(questions) = &self.questions {
            if !questions.is_empty() {
                self.selected_index = (self.selected_index + 1).min(questions.len() - 1);
            }
        }
    }

    /// Select previous question in list.
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
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

    /// Render questions view with table.
    fn render_questions(&self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        match &self.questions {
            LoadState::Idle => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press 'r' to load questions",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Questions ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loading => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  ⏳ Loading questions...",
                        Style::default().fg(Color::Yellow),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Questions ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Error(msg) => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  ❌ Error: {}", msg),
                        Style::default().fg(Color::Red),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press 'r' to retry",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Questions ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loaded(questions) => {
                if questions.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  No questions found",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                    .block(
                        Block::default()
                            .title(" Questions (0) ")
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
                    // Create table rows
                    let rows: Vec<Row> = questions
                        .iter()
                        .enumerate()
                        .map(|(i, q)| {
                            let style = if i == self.selected_index {
                                Style::default()
                                    .fg(Color::Black)
                                    .bg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            };

                            let collection_name = q
                                .collection
                                .as_ref()
                                .map(|c| c.name.as_str())
                                .unwrap_or("—");

                            Row::new(vec![
                                Cell::from(format!("{}", q.id)),
                                Cell::from(q.name.clone()),
                                Cell::from(collection_name.to_string()),
                            ])
                            .style(style)
                        })
                        .collect();

                    let table = Table::new(
                        rows,
                        [
                            ratatui::layout::Constraint::Length(6),  // ID
                            ratatui::layout::Constraint::Min(20),    // Name
                            ratatui::layout::Constraint::Length(20), // Collection
                        ],
                    )
                    .header(
                        Row::new(vec!["ID", "Name", "Collection"])
                            .style(
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .bottom_margin(1),
                    )
                    .block(
                        Block::default()
                            .title(format!(" Questions ({}) ", questions.len()))
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    )
                    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

                    frame.render_widget(table, area);
                }
            }
        }
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
        // Questions view renders directly (uses Table widget)
        if self.view == ContentView::Questions {
            self.render_questions(area, frame, focused);
            return;
        }

        // Other views return Paragraph widgets
        let widget = match self.view {
            ContentView::Welcome => self.render_welcome(area, focused),
            ContentView::Questions => unreachable!(), // Handled above
            ContentView::Collections => self.render_placeholder("Collections", focused),
            ContentView::Databases => self.render_placeholder("Databases", focused),
            ContentView::Settings => self.render_placeholder("Settings", focused),
        };

        frame.render_widget(widget, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Questions view has list navigation
        if self.view == ContentView::Questions {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_next();
                    true
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.selected_index = 0;
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    if let LoadState::Loaded(questions) = &self.questions {
                        if !questions.is_empty() {
                            self.selected_index = questions.len() - 1;
                        }
                    }
                    true
                }
                _ => false,
            }
        } else {
            // Other views use scroll
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
