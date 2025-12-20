//! View rendering functions for ContentPanel.
//!
//! This module contains all the render_* functions that draw different
//! views (Welcome, Questions, Collections, Databases, QueryResult, etc.).

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::components::state_renderer::{
    LoadStateConfig, render_empty, render_empty_with_hint, render_non_loaded_state,
};
use crate::components::styles::{
    HIGHLIGHT_SYMBOL, border_style, header_style, result_row_highlight_style, row_highlight_style,
};
use crate::layout::questions_table::{COLLECTION_WIDTH, ID_WIDTH, NAME_MIN_WIDTH};
use crate::service::LoadState;

use super::{ContentPanel, ContentView, InputMode, SortOrder};

impl ContentPanel {
    /// Render welcome view content.
    pub(super) fn render_welcome(&self, _area: Rect, focused: bool) -> Paragraph<'static> {
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

    /// Render questions view with table and search bar.
    pub(super) fn render_questions(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
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
                        .border_style(border_style),
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
                        .border_style(border_style),
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
                        .border_style(border_style),
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
                            .border_style(border_style),
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
                            .border_style(border_style),
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

    /// Render placeholder for unimplemented views.
    #[allow(dead_code)]
    pub(super) fn render_placeholder(&self, title: &str, focused: bool) -> Paragraph<'static> {
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
                "  ⚠ Not implemented yet",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  This feature is planned for future releases.",
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

    /// Render collections view with table.
    pub(super) fn render_collections(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
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
    pub(super) fn render_databases(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
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

    /// Render collection questions view with table.
    /// Shows questions filtered by a specific collection.
    pub(super) fn render_collection_questions(
        &mut self,
        area: Rect,
        frame: &mut Frame,
        focused: bool,
    ) {
        // Get collection name from ContentView variant
        let collection_name = match &self.view {
            ContentView::CollectionQuestions { name, .. } => name.as_str(),
            _ => "Unknown",
        };

        let title = format!(" {} ", collection_name);
        let idle_msg = format!("Loading questions from '{}'...", collection_name);
        let loading_msg = format!("Loading questions from '{}'...", collection_name);
        let config = LoadStateConfig::new(&title, focused)
            .with_idle_message(&idle_msg)
            .with_loading_message(&loading_msg);

        // Handle non-loaded states with helper
        if render_non_loaded_state(frame, area, &self.questions, &config) {
            return;
        }

        // Handle Loaded state
        let questions = match &self.questions {
            LoadState::Loaded(q) => q,
            _ => return,
        };

        if questions.is_empty() {
            let empty_title = format!(" {} (0) ", collection_name);
            let empty_msg = format!("No questions found in '{}'", collection_name);
            render_empty_with_hint(
                frame,
                area,
                &LoadStateConfig::new(&empty_title, focused),
                &empty_msg,
                "Press Esc to go back",
            );
            return;
        }

        // Create table rows
        let rows: Vec<Row> = questions
            .iter()
            .map(|q| {
                Row::new(vec![
                    Cell::from(format!("{}", q.id)),
                    Cell::from(q.name.clone()),
                    Cell::from(q.description.as_deref().unwrap_or("—").to_string()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(ID_WIDTH),
                Constraint::Percentage(35), // Name
                Constraint::Percentage(55), // Description (more space)
            ],
        )
        .header(
            Row::new(vec!["ID", "Name", "Description"])
                .style(header_style())
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(format!(
                    " {} ({}) - Press Esc to go back ",
                    collection_name,
                    questions.len()
                ))
                .borders(Borders::ALL)
                .border_style(border_style(focused)),
        )
        .row_highlight_style(row_highlight_style())
        .highlight_symbol(HIGHLIGHT_SYMBOL);

        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    /// Render database schemas view with table.
    /// Shows schemas in a specific database.
    pub(super) fn render_database_schemas(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        // Get database name from ContentView variant
        let database_name = match &self.view {
            ContentView::DatabaseSchemas { db_name, .. } => db_name.as_str(),
            _ => "Unknown",
        };

        let title = format!(" {} - Schemas ", database_name);
        let idle_msg = format!("Loading schemas from '{}'...", database_name);
        let loading_msg = format!("Loading schemas from '{}'...", database_name);
        let config = LoadStateConfig::new(&title, focused)
            .with_idle_message(&idle_msg)
            .with_loading_message(&loading_msg);

        // Handle non-loaded states with helper
        if render_non_loaded_state(frame, area, &self.schemas, &config) {
            return;
        }

        // Handle Loaded state
        let schemas = match &self.schemas {
            LoadState::Loaded(s) => s,
            _ => return,
        };

        if schemas.is_empty() {
            let empty_title = format!(" {} - Schemas (0) ", database_name);
            let empty_msg = format!("No schemas found in '{}'", database_name);
            render_empty_with_hint(
                frame,
                area,
                &LoadStateConfig::new(&empty_title, focused),
                &empty_msg,
                "Press Esc to go back",
            );
            return;
        }

        // Create table rows
        let rows: Vec<Row> = schemas
            .iter()
            .enumerate()
            .map(|(i, schema)| {
                Row::new(vec![
                    Cell::from(format!("{}", i + 1)),
                    Cell::from(schema.clone()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(ID_WIDTH),
                Constraint::Min(NAME_MIN_WIDTH),
            ],
        )
        .header(
            Row::new(vec!["#", "Schema Name"])
                .style(header_style())
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(format!(
                    " {} - Schemas ({}) - Press Esc to go back ",
                    database_name,
                    schemas.len()
                ))
                .borders(Borders::ALL)
                .border_style(border_style(focused)),
        )
        .row_highlight_style(row_highlight_style())
        .highlight_symbol(HIGHLIGHT_SYMBOL);

        frame.render_stateful_widget(table, area, &mut self.schemas_table_state);
    }

    /// Render schema tables view with table.
    /// Shows tables in a specific schema.
    pub(super) fn render_schema_tables(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        // Get schema name from ContentView variant
        let schema_name = match &self.view {
            ContentView::SchemaTables { schema_name, .. } => schema_name.as_str(),
            _ => "Unknown",
        };

        let title = format!(" {} - Tables ", schema_name);
        let idle_msg = format!("Loading tables from '{}'...", schema_name);
        let loading_msg = format!("Loading tables from '{}'...", schema_name);
        let config = LoadStateConfig::new(&title, focused)
            .with_idle_message(&idle_msg)
            .with_loading_message(&loading_msg);

        // Handle non-loaded states with helper
        if render_non_loaded_state(frame, area, &self.tables, &config) {
            return;
        }

        // Handle Loaded state
        let tables = match &self.tables {
            LoadState::Loaded(t) => t,
            _ => return,
        };

        if tables.is_empty() {
            let empty_title = format!(" {} - Tables (0) ", schema_name);
            let empty_msg = format!("No tables found in '{}'", schema_name);
            render_empty_with_hint(
                frame,
                area,
                &LoadStateConfig::new(&empty_title, focused),
                &empty_msg,
                "Press Esc to go back",
            );
            return;
        }

        // Create table rows
        let rows: Vec<Row> = tables
            .iter()
            .map(|t| {
                let display_name = t.display_name.as_deref().unwrap_or(t.name.as_str());
                let desc = t.description.as_deref().unwrap_or("—");
                Row::new(vec![
                    Cell::from(format!("{}", t.id)),
                    Cell::from(t.name.clone()),
                    Cell::from(display_name.to_string()),
                    Cell::from(desc.to_string()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(ID_WIDTH),
                Constraint::Min(NAME_MIN_WIDTH),
                Constraint::Length(20), // Display Name
                Constraint::Min(20),    // Description
            ],
        )
        .header(
            Row::new(vec!["ID", "Name", "Display Name", "Description"])
                .style(header_style())
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(format!(
                    " {} - Tables ({}) - Press Esc to go back ",
                    schema_name,
                    tables.len()
                ))
                .borders(Borders::ALL)
                .border_style(border_style(focused)),
        )
        .row_highlight_style(row_highlight_style())
        .highlight_symbol(HIGHLIGHT_SYMBOL);

        frame.render_stateful_widget(table, area, &mut self.tables_table_state);
    }

    /// Render table preview view with query result table.
    /// Shows sample data from a table (reuses query result rendering).
    pub(super) fn render_table_preview(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        // Get table name from ContentView variant (clone to avoid borrow conflict)
        let table_name = match &self.view {
            ContentView::TablePreview { table_name, .. } => table_name.clone(),
            _ => "Unknown".to_string(),
        };

        // Check if we have result data and if it's non-empty
        let has_data = self
            .query_result
            .as_ref()
            .map(|r| !r.rows.is_empty())
            .unwrap_or(false);
        let is_empty = self
            .query_result
            .as_ref()
            .map(|r| r.rows.is_empty())
            .unwrap_or(false);

        if self.query_result.is_none() {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  Loading preview for '{}'...", table_name),
                    Style::default().fg(Color::Yellow),
                )),
            ])
            .block(
                Block::default()
                    .title(format!(" {} - Preview ", table_name))
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            );
            frame.render_widget(paragraph, area);
        } else if is_empty {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  Table: {}", table_name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  No data in table",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Esc to go back",
                    Style::default().fg(Color::Yellow),
                )),
            ])
            .block(
                Block::default()
                    .title(format!(" {} - Preview (0 rows) ", table_name))
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            );
            frame.render_widget(paragraph, area);
        } else if has_data {
            self.render_result_table(frame, area, focused, &table_name);
        }
    }

    /// Render query result view with table.
    pub(super) fn render_query_result(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        // Extract info before mutable borrow
        let (has_result, is_empty, question_name) = match &self.query_result {
            None => (false, false, String::new()),
            Some(result) => (true, result.rows.is_empty(), result.question_name.clone()),
        };

        if !has_result {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No query result available",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(
                Block::default()
                    .title(" Query Result ")
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            );
            frame.render_widget(paragraph, area);
        } else if is_empty {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  Query: {}", question_name),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  No data returned",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Esc to go back",
                    Style::default().fg(Color::Yellow),
                )),
            ])
            .block(
                Block::default()
                    .title(format!(" Query Result: {} (0 rows) ", question_name))
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            );
            frame.render_widget(paragraph, area);
        } else {
            self.render_result_table(frame, area, focused, &question_name);
        }
    }

    /// Common result table rendering logic for both TablePreview and QueryResult.
    /// Accesses self.query_result internally to avoid borrow conflicts.
    fn render_result_table(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        focused: bool,
        title_prefix: &str,
    ) {
        // Get result reference - caller guarantees query_result is Some
        let result = match &self.query_result {
            Some(r) => r,
            None => return,
        };

        // Pagination: calculate row range for current page
        let total_rows = result.rows.len();
        let total_pages = self.total_pages();
        let page_start = self.result_page * self.rows_per_page;
        let page_end = (page_start + self.rows_per_page).min(total_rows);

        // Calculate visible columns based on scroll_x
        let total_cols = result.columns.len();
        let scroll_x = self.scroll_x.min(total_cols.saturating_sub(1));

        // Calculate how many columns can fit (estimate based on min width)
        let available_width = area.width.saturating_sub(4) as usize;
        let min_col_width = 15usize;
        let visible_cols = (available_width / min_col_width).max(1).min(total_cols);
        let end_col = (scroll_x + visible_cols).min(total_cols);

        // Slice columns based on scroll position
        let visible_columns: Vec<String> = result.columns[scroll_x..end_col].to_vec();
        let visible_col_count = visible_columns.len();

        // Create dynamic column widths
        let constraints: Vec<Constraint> = if visible_col_count <= 3 {
            visible_columns
                .iter()
                .map(|_| Constraint::Ratio(1, visible_col_count as u32))
                .collect()
        } else {
            visible_columns
                .iter()
                .map(|_| Constraint::Min(15))
                .collect()
        };

        // Create table rows with sliced cells (only current page)
        let rows: Vec<Row> = (page_start..page_end)
            .filter_map(|logical_idx| self.get_visible_row(logical_idx))
            .map(|row| {
                let cells: Vec<Cell> = row[scroll_x..end_col.min(row.len())]
                    .iter()
                    .map(|cell| Cell::from(cell.clone()))
                    .collect();
                Row::new(cells)
            })
            .collect();

        // Create header row with sort indicators
        let header_cells: Vec<Cell> = visible_columns
            .iter()
            .enumerate()
            .map(|(visible_idx, col)| {
                let actual_col_idx = scroll_x + visible_idx;
                let is_sorted = self.sort_column_index == Some(actual_col_idx);

                let header_text = if is_sorted {
                    let indicator = match self.sort_order {
                        SortOrder::Ascending => " ↑",
                        SortOrder::Descending => " ↓",
                        SortOrder::None => "",
                    };
                    format!("{}{}", col, indicator)
                } else {
                    col.clone()
                };

                Cell::from(header_text)
            })
            .collect();

        // Build column indicator
        let col_indicator = if total_cols > visible_cols {
            let left_arrow = if scroll_x > 0 { "← " } else { "  " };
            let right_arrow = if end_col < total_cols { " →" } else { "  " };
            format!(
                " {}Col {}-{}/{}{}",
                left_arrow,
                scroll_x + 1,
                end_col,
                total_cols,
                right_arrow
            )
        } else {
            String::new()
        };

        // Build page indicator
        let page_indicator = if total_pages > 1 {
            format!(
                " Page {}/{} (rows {}-{} of {})",
                self.result_page + 1,
                total_pages,
                page_start + 1,
                page_end,
                total_rows
            )
        } else {
            format!(" {} rows", total_rows)
        };

        // Build sort indicator for title
        let sort_indicator = if let Some((col_name, order)) = self.get_sort_info() {
            let arrow = match order {
                SortOrder::Ascending => "↑",
                SortOrder::Descending => "↓",
                SortOrder::None => "",
            };
            format!(" [Sort: {} {}]", col_name, arrow)
        } else {
            String::new()
        };

        let table = Table::new(rows, constraints)
            .header(
                Row::new(header_cells)
                    .style(header_style())
                    .bottom_margin(1),
            )
            .block(
                Block::default()
                    .title(format!(
                        " {}{}{}{}",
                        title_prefix, page_indicator, col_indicator, sort_indicator
                    ))
                    .borders(Borders::ALL)
                    .border_style(border_style(focused)),
            )
            .row_highlight_style(result_row_highlight_style())
            .highlight_symbol(HIGHLIGHT_SYMBOL);

        frame.render_stateful_widget(table, area, &mut self.result_table_state);

        // Render overlays
        if self.sort_mode_active {
            self.render_sort_modal(frame, area);
        }
        if self.filter_mode_active {
            self.render_filter_modal(frame, area);
        }
        if self.result_search_active {
            self.render_result_search_bar(frame, area);
        }
    }
}
