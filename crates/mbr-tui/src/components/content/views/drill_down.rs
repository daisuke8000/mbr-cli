//! Drill-down view rendering for nested data exploration.
//!
//! - Collection → Questions
//! - Database → Schemas → Tables

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Cell, Row, Table},
};

use crate::components::content::{ContentPanel, ContentView};
use crate::components::state_renderer::{
    LoadStateConfig, render_empty_with_hint, render_non_loaded_state,
};
use crate::components::styles::{
    HIGHLIGHT_SYMBOL, border_style, header_style, row_highlight_style,
};
use crate::layout::questions_table::{ID_WIDTH, NAME_MIN_WIDTH};
use crate::service::LoadState;

impl ContentPanel {
    /// Render collection questions view with table.
    /// Shows questions filtered by a specific collection.
    pub(in crate::components::content) fn render_collection_questions(
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

        // Client-side pagination: show only current page slice
        let offset = self.questions_offset as usize;
        let page_size = self.questions_page_size as usize;
        let end = (offset + page_size).min(questions.len());
        let page_questions = &questions[offset..end];

        let rows: Vec<Row> = page_questions
            .iter()
            .map(|q| {
                Row::new(vec![
                    Cell::from(q.id.to_string()),
                    Cell::from(q.name.as_str()),
                    Cell::from(q.description.as_deref().unwrap_or("—")),
                ])
            })
            .collect();

        let pagination_hint = self
            .questions_pagination_info()
            .map(|(page, total_pages, _)| format!(" [{}/{}]", page, total_pages))
            .unwrap_or_default();

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
                    " {} ({}){} - Esc: back, n/p: page ",
                    collection_name,
                    questions.len(),
                    pagination_hint,
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
    pub(in crate::components::content) fn render_database_schemas(
        &mut self,
        area: Rect,
        frame: &mut Frame,
        focused: bool,
    ) {
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

        // Create table rows (use as_str() to avoid cloning)
        let rows: Vec<Row> = schemas
            .iter()
            .enumerate()
            .map(|(i, schema)| {
                Row::new(vec![
                    Cell::from((i + 1).to_string()),
                    Cell::from(schema.as_str()),
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
    pub(in crate::components::content) fn render_schema_tables(
        &mut self,
        area: Rect,
        frame: &mut Frame,
        focused: bool,
    ) {
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

        // Create table rows (minimize clones)
        let rows: Vec<Row> = tables
            .iter()
            .map(|t| {
                let display_name = t.display_name.as_deref().unwrap_or(t.name.as_str());
                let desc = t.description.as_deref().unwrap_or("—");
                Row::new(vec![
                    Cell::from(t.id.to_string()),
                    Cell::from(t.name.as_str()),
                    Cell::from(display_name),
                    Cell::from(desc),
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
}
