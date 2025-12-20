//! List view rendering for Questions, Collections, and Databases.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use crate::components::content::{ContentPanel, InputMode};
use crate::components::state_renderer::{LoadStateConfig, render_empty, render_non_loaded_state};
use crate::components::styles::{
    HIGHLIGHT_SYMBOL, border_style, header_style, row_highlight_style,
};
use crate::layout::questions_table::{COLLECTION_WIDTH, ID_WIDTH, NAME_MIN_WIDTH};
use crate::service::LoadState;

impl ContentPanel {
    /// Render questions view with table and search bar.
    pub(in crate::components::content) fn render_questions(
        &mut self,
        area: Rect,
        frame: &mut Frame,
        focused: bool,
    ) {
        let border_color = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Calculate layout: search bar (if visible) + table
        let show_search_bar = self.input_mode == InputMode::Search || self.active_search.is_some();
        let (search_area, table_area) = if show_search_bar {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(5)])
                .split(area);
            (Some(chunks[0]), chunks[1])
        } else {
            (None, area)
        };

        // Render search bar if visible
        if let Some(search_rect) = search_area {
            let search_text = if self.input_mode == InputMode::Search {
                format!("/{}", self.search_query)
            } else if let Some(ref query) = self.active_search {
                format!("Search: {} (Esc to clear)", query)
            } else {
                String::new()
            };

            let search_style = if self.input_mode == InputMode::Search {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let search_bar = Paragraph::new(search_text).style(search_style).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(if self.input_mode == InputMode::Search {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    })
                    .title(" Search (/ to start, Enter to search, Esc to cancel) "),
            );
            frame.render_widget(search_bar, search_rect);
        }

        // Build title with search indicator
        let title = if let Some(ref query) = self.active_search {
            match &self.questions {
                LoadState::Loaded(questions) => {
                    format!(" Questions ({}) - Search: \"{}\" ", questions.len(), query)
                }
                _ => format!(" Questions - Search: \"{}\" ", query),
            }
        } else {
            match &self.questions {
                LoadState::Loaded(questions) => format!(" Questions ({}) ", questions.len()),
                _ => " Questions ".to_string(),
            }
        };

        match &self.questions {
            LoadState::Idle => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press 'r' to load questions, '/' to search",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_color),
                );
                frame.render_widget(paragraph, table_area);
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
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_color),
                );
                frame.render_widget(paragraph, table_area);
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
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(border_color),
                );
                frame.render_widget(paragraph, table_area);
            }
            LoadState::Loaded(questions) => {
                if questions.is_empty() {
                    let empty_msg = if self.active_search.is_some() {
                        "  No questions found matching your search"
                    } else {
                        "  No questions found"
                    };
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            empty_msg,
                            Style::default().fg(Color::DarkGray),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Press '/' to search or Esc to clear search",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                    .block(
                        Block::default()
                            .title(title)
                            .borders(Borders::ALL)
                            .border_style(border_color),
                    );
                    frame.render_widget(paragraph, table_area);
                } else {
                    // Create table rows
                    let rows: Vec<Row> = questions
                        .iter()
                        .map(|q| {
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
                        })
                        .collect();

                    let table = Table::new(
                        rows,
                        [
                            Constraint::Length(ID_WIDTH),
                            Constraint::Min(NAME_MIN_WIDTH),
                            Constraint::Length(COLLECTION_WIDTH),
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
                            .title(title)
                            .borders(Borders::ALL)
                            .border_style(border_color),
                    )
                    .row_highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("► ");

                    frame.render_stateful_widget(table, table_area, &mut self.table_state);
                }
            }
        }
    }

    /// Render collections view with table.
    pub(in crate::components::content) fn render_collections(
        &mut self,
        area: Rect,
        frame: &mut Frame,
        focused: bool,
    ) {
        let config = LoadStateConfig::new(" Collections ", focused)
            .with_idle_message("Press 'r' to load collections")
            .with_loading_message("Loading collections...");

        // Handle non-loaded states with helper
        if render_non_loaded_state(frame, area, &self.collections, &config) {
            return;
        }

        // Handle Loaded state
        let collections = match &self.collections {
            LoadState::Loaded(c) => c,
            _ => return,
        };

        if collections.is_empty() {
            render_empty(
                frame,
                area,
                &LoadStateConfig::new(" Collections (0) ", focused),
                "No collections found",
            );
            return;
        }

        // Create table rows
        let rows: Vec<Row> = collections
            .iter()
            .map(|c| {
                let id_str =
                    c.id.map(|id| id.to_string())
                        .unwrap_or_else(|| "—".to_string());
                let desc = c.description.as_deref().unwrap_or("—");
                let location = c.location.as_deref().unwrap_or("/");

                Row::new(vec![
                    Cell::from(id_str),
                    Cell::from(c.name.clone()),
                    Cell::from(location.to_string()),
                    Cell::from(desc.to_string()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(ID_WIDTH),
                Constraint::Min(NAME_MIN_WIDTH),
                Constraint::Length(15), // Location
                Constraint::Min(20),    // Description
            ],
        )
        .header(
            Row::new(vec!["ID", "Name", "Location", "Description"])
                .style(header_style())
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(format!(" Collections ({}) ", collections.len()))
                .borders(Borders::ALL)
                .border_style(border_style(focused)),
        )
        .row_highlight_style(row_highlight_style())
        .highlight_symbol(HIGHLIGHT_SYMBOL);

        frame.render_stateful_widget(table, area, &mut self.collections_table_state);
    }

    /// Render databases view with table.
    pub(in crate::components::content) fn render_databases(
        &mut self,
        area: Rect,
        frame: &mut Frame,
        focused: bool,
    ) {
        let config = LoadStateConfig::new(" Databases ", focused)
            .with_idle_message("Press 'r' to load databases")
            .with_loading_message("Loading databases...");

        // Handle non-loaded states with helper
        if render_non_loaded_state(frame, area, &self.databases, &config) {
            return;
        }

        // Handle Loaded state
        let databases = match &self.databases {
            LoadState::Loaded(d) => d,
            _ => return,
        };

        if databases.is_empty() {
            render_empty(
                frame,
                area,
                &LoadStateConfig::new(" Databases (0) ", focused),
                "No databases found",
            );
            return;
        }

        // Create table rows
        let rows: Vec<Row> = databases
            .iter()
            .map(|db| {
                let engine = db.engine.as_deref().unwrap_or("—");
                let desc = db.description.as_deref().unwrap_or("—");
                let sample_marker = if db.is_sample { " (sample)" } else { "" };

                Row::new(vec![
                    Cell::from(format!("{}", db.id)),
                    Cell::from(format!("{}{}", db.name, sample_marker)),
                    Cell::from(engine.to_string()),
                    Cell::from(desc.to_string()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(ID_WIDTH),
                Constraint::Min(NAME_MIN_WIDTH),
                Constraint::Length(15), // Engine
                Constraint::Min(20),    // Description
            ],
        )
        .header(
            Row::new(vec!["ID", "Name", "Engine", "Description"])
                .style(header_style())
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(format!(" Databases ({}) ", databases.len()))
                .borders(Borders::ALL)
                .border_style(border_style(focused)),
        )
        .row_highlight_style(row_highlight_style())
        .highlight_symbol(HIGHLIGHT_SYMBOL);

        frame.render_stateful_widget(table, area, &mut self.databases_table_state);
    }
}
