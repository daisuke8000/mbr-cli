//! Navigation panel component.
//!
//! Displays a list of menu items (Questions, Collections, etc.) for navigation.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use super::{Component, ScrollState};

/// Menu item in the navigation panel.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub icon: &'static str,
}

impl MenuItem {
    pub fn new(label: impl Into<String>, icon: &'static str) -> Self {
        Self {
            label: label.into(),
            icon,
        }
    }
}

/// Navigation panel showing menu items.
pub struct NavigationPanel {
    items: Vec<MenuItem>,
    selected: usize,
    list_state: ListState,
    scroll: ScrollState,
}

impl Default for NavigationPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl NavigationPanel {
    /// Create a new navigation panel with default menu items.
    pub fn new() -> Self {
        let items = vec![
            MenuItem::new("Questions", "󰋗"),
            MenuItem::new("Collections", ""),
            MenuItem::new("Databases", ""),
            MenuItem::new("Settings", ""),
        ];
        let total = items.len();

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            items,
            selected: 0,
            list_state,
            scroll: ScrollState::new(total, total), // All visible initially
        }
    }

    /// Get the currently selected menu item index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Get the currently selected menu item.
    pub fn selected_item(&self) -> Option<&MenuItem> {
        self.items.get(self.selected)
    }

    /// Move selection up.
    fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
            self.scroll.scroll_to(self.selected);
        }
    }

    /// Move selection down.
    fn select_next(&mut self) {
        if self.selected < self.items.len().saturating_sub(1) {
            self.selected += 1;
            self.list_state.select(Some(self.selected));
            self.scroll.scroll_to(self.selected);
        }
    }
}

impl Component for NavigationPanel {
    fn draw(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == self.selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if i == self.selected { "▶ " } else { "  " };
                let content = Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(format!("{} {}", item.icon, item.label), style),
                ]);
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!(" {} ", self.title()))
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(list, area, &mut self.list_state.clone());
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                true
            }
            KeyCode::Home => {
                self.selected = 0;
                self.list_state.select(Some(0));
                self.scroll.scroll_to(0);
                true
            }
            KeyCode::End => {
                self.selected = self.items.len().saturating_sub(1);
                self.list_state.select(Some(self.selected));
                self.scroll.scroll_to(self.selected);
                true
            }
            _ => false,
        }
    }

    fn title(&self) -> &str {
        "Navigation"
    }
}
