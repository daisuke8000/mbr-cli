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
}
