//! Content panel component.
//!
//! Displays the main content area (query results, question details, etc.).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

use mbr_core::api::models::{CollectionItem, Database, Question, TableInfo};

use super::{Component, ScrollState};
use crate::layout::questions_table::{COLLECTION_WIDTH, ID_WIDTH, NAME_MIN_WIDTH};
use crate::service::LoadState;

/// Input mode for text input fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal navigation mode
    #[default]
    Normal,
    /// Search input mode
    Search,
}

/// Content view types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentView {
    #[default]
    Welcome,
    Questions,
    Collections,
    Databases,
    QueryResult,
    /// Questions filtered by a specific collection
    CollectionQuestions,
    /// Schemas in a specific database
    DatabaseSchemas,
    /// Tables in a specific schema
    SchemaTables,
    /// Table data preview
    TablePreview,
}

/// Query result data for display in TUI.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResultData {
    /// Question ID that was executed
    pub question_id: u32,
    /// Question name for display
    pub question_name: String,
    /// Column headers
    pub columns: Vec<String>,
    /// Row data (each cell as string)
    pub rows: Vec<Vec<String>>,
}

/// Default rows per page for query result pagination.
const DEFAULT_ROWS_PER_PAGE: usize = 100;

/// Content panel showing main content.
pub struct ContentPanel {
    view: ContentView,
    scroll: ScrollState,
    /// Horizontal scroll offset (column index)
    scroll_x: usize,
    /// Questions data for the Questions view
    questions: LoadState<Vec<Question>>,
    /// Table state for Questions view (manages selection and scroll)
    table_state: TableState,
    /// Collections data for the Collections view
    collections: LoadState<Vec<CollectionItem>>,
    /// Table state for Collections view
    collections_table_state: TableState,
    /// Databases data for the Databases view
    databases: LoadState<Vec<Database>>,
    /// Table state for Databases view
    databases_table_state: TableState,
    /// Schemas data for the DatabaseSchemas view
    schemas: LoadState<Vec<String>>,
    /// Table state for Schemas view
    schemas_table_state: TableState,
    /// Tables data for the SchemaTables view
    tables: LoadState<Vec<TableInfo>>,
    /// Table state for Tables view
    tables_table_state: TableState,
    /// Query result data for QueryResult view
    query_result: Option<QueryResultData>,
    /// Table state for query result table
    result_table_state: TableState,
    /// Current page for query result pagination (0-indexed)
    result_page: usize,
    /// Rows per page for query result pagination
    rows_per_page: usize,
    /// Current input mode
    input_mode: InputMode,
    /// Current search query
    search_query: String,
    /// Active search query (used for display after search is executed)
    active_search: Option<String>,
    /// Current collection context for CollectionQuestions view (id, name)
    collection_context: Option<(u32, String)>,
    /// Current database context for DatabaseSchemas view (id, name)
    database_context: Option<(u32, String)>,
    /// Current schema context for SchemaTables view (database_id, schema_name)
    schema_context: Option<(u32, String)>,
    /// Current table context for TablePreview view (database_id, table_id, table_name)
    table_context: Option<(u32, u32, String)>,
    /// Navigation stack for multi-level drill-down (supports 4+ levels)
    /// Used for: Databases → Schemas → Tables → Preview
    ///           Collections → Questions → QueryResult
    navigation_stack: Vec<ContentView>,
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
            scroll_x: 0,
            questions: LoadState::default(),
            table_state: TableState::default(),
            collections: LoadState::default(),
            collections_table_state: TableState::default(),
            databases: LoadState::default(),
            databases_table_state: TableState::default(),
            schemas: LoadState::default(),
            schemas_table_state: TableState::default(),
            tables: LoadState::default(),
            tables_table_state: TableState::default(),
            query_result: None,
            result_table_state: TableState::default(),
            result_page: 0,
            rows_per_page: DEFAULT_ROWS_PER_PAGE,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            active_search: None,
            collection_context: None,
            database_context: None,
            schema_context: None,
            table_context: None,
            navigation_stack: Vec::new(),
        }
    }

    /// Set the current view (used for tab switching).
    /// Clears navigation stack since tab changes reset the navigation context.
    pub fn set_view(&mut self, view: ContentView) {
        self.view = view;
        self.scroll = ScrollState::default();
        self.scroll_x = 0;
        self.table_state = TableState::default();
        // Clear navigation stack on tab switch
        self.clear_navigation_stack();
    }

    /// Get the current view.
    pub fn current_view(&self) -> ContentView {
        self.view
    }

    // === Navigation Stack Methods ===

    /// Push current view to stack and navigate to new view.
    /// Used for drill-down navigation (e.g., Collections → Questions).
    pub fn push_view(&mut self, new_view: ContentView) {
        self.navigation_stack.push(self.view);
        self.view = new_view;
    }

    /// Pop from navigation stack and return to previous view.
    /// Returns the view that was popped to, or None if stack was empty.
    pub fn pop_view(&mut self) -> Option<ContentView> {
        if let Some(previous) = self.navigation_stack.pop() {
            self.view = previous;
            Some(previous)
        } else {
            None
        }
    }

    /// Get the depth of the navigation stack.
    #[allow(dead_code)] // Useful for debugging and future features
    pub fn navigation_depth(&self) -> usize {
        self.navigation_stack.len()
    }

    /// Clear the navigation stack (used when switching tabs).
    pub fn clear_navigation_stack(&mut self) {
        self.navigation_stack.clear();
    }

    /// Update questions data from AppData.
    /// Automatically selects first item when data is loaded.
    pub fn update_questions(&mut self, questions: &LoadState<Vec<Question>>) {
        self.questions = questions.clone();

        // Auto-select first item when data is loaded
        if let LoadState::Loaded(items) = questions {
            if !items.is_empty() && self.table_state.selected().is_none() {
                self.table_state.select(Some(0));
            }
        }
    }

    /// Update collections data from AppData.
    /// Automatically selects first item when data is loaded.
    pub fn update_collections(&mut self, collections: &LoadState<Vec<CollectionItem>>) {
        self.collections = collections.clone();

        // Auto-select first item when data is loaded
        if let LoadState::Loaded(items) = collections {
            if !items.is_empty() && self.collections_table_state.selected().is_none() {
                self.collections_table_state.select(Some(0));
            }
        }
    }

    /// Update databases data from AppData.
    /// Automatically selects first item when data is loaded.
    pub fn update_databases(&mut self, databases: &LoadState<Vec<Database>>) {
        self.databases = databases.clone();

        // Auto-select first item when data is loaded
        if let LoadState::Loaded(items) = databases {
            if !items.is_empty() && self.databases_table_state.selected().is_none() {
                self.databases_table_state.select(Some(0));
            }
        }
    }

    /// Update schemas data from AppData.
    /// Automatically selects first item when data is loaded.
    pub fn update_schemas(&mut self, schemas: &LoadState<Vec<String>>) {
        self.schemas = schemas.clone();

        // Auto-select first item when data is loaded
        if let LoadState::Loaded(items) = schemas {
            if !items.is_empty() && self.schemas_table_state.selected().is_none() {
                self.schemas_table_state.select(Some(0));
            }
        }
    }

    /// Update tables data from AppData.
    /// Automatically selects first item when data is loaded.
    pub fn update_tables(&mut self, tables: &LoadState<Vec<TableInfo>>) {
        self.tables = tables.clone();

        // Auto-select first item when data is loaded
        if let LoadState::Loaded(items) = tables {
            if !items.is_empty() && self.tables_table_state.selected().is_none() {
                self.tables_table_state.select(Some(0));
            }
        }
    }

    /// Select next question in list.
    pub fn select_next(&mut self) {
        if let LoadState::Loaded(questions) = &self.questions {
            if questions.is_empty() {
                return;
            }
            let current = self.table_state.selected().unwrap_or(0);
            let next = (current + 1).min(questions.len() - 1);
            self.table_state.select(Some(next));
        }
    }

    /// Select previous question in list.
    pub fn select_previous(&mut self) {
        let current = self.table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.table_state.select(Some(prev));
    }

    /// Select first question in list.
    pub fn select_first(&mut self) {
        self.table_state.select(Some(0));
    }

    /// Select last question in list.
    pub fn select_last(&mut self) {
        if let LoadState::Loaded(questions) = &self.questions {
            if !questions.is_empty() {
                self.table_state.select(Some(questions.len() - 1));
            }
        }
    }

    // === Collections view navigation ===

    /// Select next collection in list.
    fn select_collections_next(&mut self) {
        if let LoadState::Loaded(collections) = &self.collections {
            if collections.is_empty() {
                return;
            }
            let current = self.collections_table_state.selected().unwrap_or(0);
            let next = (current + 1).min(collections.len() - 1);
            self.collections_table_state.select(Some(next));
        }
    }

    /// Select previous collection in list.
    fn select_collections_previous(&mut self) {
        let current = self.collections_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.collections_table_state.select(Some(prev));
    }

    /// Select first collection in list.
    fn select_collections_first(&mut self) {
        self.collections_table_state.select(Some(0));
    }

    /// Select last collection in list.
    fn select_collections_last(&mut self) {
        if let LoadState::Loaded(collections) = &self.collections {
            if !collections.is_empty() {
                self.collections_table_state
                    .select(Some(collections.len() - 1));
            }
        }
    }

    // === Databases view navigation ===

    /// Select next database in list.
    fn select_databases_next(&mut self) {
        if let LoadState::Loaded(databases) = &self.databases {
            if databases.is_empty() {
                return;
            }
            let current = self.databases_table_state.selected().unwrap_or(0);
            let next = (current + 1).min(databases.len() - 1);
            self.databases_table_state.select(Some(next));
        }
    }

    /// Select previous database in list.
    fn select_databases_previous(&mut self) {
        let current = self.databases_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.databases_table_state.select(Some(prev));
    }

    /// Select first database in list.
    fn select_databases_first(&mut self) {
        self.databases_table_state.select(Some(0));
    }

    /// Select last database in list.
    fn select_databases_last(&mut self) {
        if let LoadState::Loaded(databases) = &self.databases {
            if !databases.is_empty() {
                self.databases_table_state.select(Some(databases.len() - 1));
            }
        }
    }

    // === Schemas view navigation ===

    /// Select next schema in list.
    fn select_schemas_next(&mut self) {
        if let LoadState::Loaded(schemas) = &self.schemas {
            if schemas.is_empty() {
                return;
            }
            let current = self.schemas_table_state.selected().unwrap_or(0);
            let next = (current + 1).min(schemas.len() - 1);
            self.schemas_table_state.select(Some(next));
        }
    }

    /// Select previous schema in list.
    fn select_schemas_previous(&mut self) {
        let current = self.schemas_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.schemas_table_state.select(Some(prev));
    }

    /// Select first schema in list.
    fn select_schemas_first(&mut self) {
        self.schemas_table_state.select(Some(0));
    }

    /// Select last schema in list.
    fn select_schemas_last(&mut self) {
        if let LoadState::Loaded(schemas) = &self.schemas {
            if !schemas.is_empty() {
                self.schemas_table_state.select(Some(schemas.len() - 1));
            }
        }
    }

    // === Tables view navigation ===

    /// Select next table in list.
    fn select_tables_next(&mut self) {
        if let LoadState::Loaded(tables) = &self.tables {
            if tables.is_empty() {
                return;
            }
            let current = self.tables_table_state.selected().unwrap_or(0);
            let next = (current + 1).min(tables.len() - 1);
            self.tables_table_state.select(Some(next));
        }
    }

    /// Select previous table in list.
    fn select_tables_previous(&mut self) {
        let current = self.tables_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.tables_table_state.select(Some(prev));
    }

    /// Select first table in list.
    fn select_tables_first(&mut self) {
        self.tables_table_state.select(Some(0));
    }

    /// Select last table in list.
    fn select_tables_last(&mut self) {
        if let LoadState::Loaded(tables) = &self.tables {
            if !tables.is_empty() {
                self.tables_table_state.select(Some(tables.len() - 1));
            }
        }
    }

    /// Get the currently selected question ID.
    /// Works in both Questions and CollectionQuestions views.
    pub fn get_selected_question_id(&self) -> Option<u32> {
        if self.view != ContentView::Questions && self.view != ContentView::CollectionQuestions {
            return None;
        }
        if let LoadState::Loaded(questions) = &self.questions {
            if let Some(selected) = self.table_state.selected() {
                return questions.get(selected).map(|q| q.id);
            }
        }
        None
    }

    /// Get the currently selected collection info (id, name).
    pub fn get_selected_collection_info(&self) -> Option<(u32, String)> {
        if self.view != ContentView::Collections {
            return None;
        }
        if let LoadState::Loaded(collections) = &self.collections {
            if let Some(selected) = self.collections_table_state.selected() {
                return collections
                    .get(selected)
                    .and_then(|c| c.id.map(|id| (id, c.name.clone())));
            }
        }
        None
    }

    // === Collection Questions View ===

    /// Enter collection questions view to show questions from a specific collection.
    /// Uses navigation stack for proper back navigation.
    pub fn enter_collection_questions(&mut self, collection_id: u32, collection_name: String) {
        self.collection_context = Some((collection_id, collection_name));
        // Reset questions state for new load
        self.questions = LoadState::Idle;
        self.table_state = TableState::default();
        // Push current view (Collections) to stack before switching
        self.push_view(ContentView::CollectionQuestions);
    }

    /// Exit collection questions view and return to previous view.
    /// Uses navigation stack to return to the correct originating view.
    pub fn exit_collection_questions(&mut self) {
        self.collection_context = None;
        // Reset questions state
        self.questions = LoadState::Idle;
        self.table_state = TableState::default();
        // Pop from navigation stack (defaults to Collections if stack is empty)
        if self.pop_view().is_none() {
            self.view = ContentView::Collections;
        }
    }

    /// Get the current collection context (id, name) for CollectionQuestions view.
    #[allow(dead_code)] // Designed for future features
    pub fn get_collection_context(&self) -> Option<&(u32, String)> {
        self.collection_context.as_ref()
    }

    // === Database Drill-down View ===

    /// Get the currently selected database info (id, name).
    pub fn get_selected_database_info(&self) -> Option<(u32, String)> {
        if self.view != ContentView::Databases {
            return None;
        }
        if let LoadState::Loaded(databases) = &self.databases {
            if let Some(selected) = self.databases_table_state.selected() {
                return databases.get(selected).map(|db| (db.id, db.name.clone()));
            }
        }
        None
    }

    /// Get the currently selected schema name.
    pub fn get_selected_schema(&self) -> Option<String> {
        if self.view != ContentView::DatabaseSchemas {
            return None;
        }
        if let LoadState::Loaded(schemas) = &self.schemas {
            if let Some(selected) = self.schemas_table_state.selected() {
                return schemas.get(selected).cloned();
            }
        }
        None
    }

    /// Get the currently selected table info (table_id, table_name).
    pub fn get_selected_table_info(&self) -> Option<(u32, String)> {
        if self.view != ContentView::SchemaTables {
            return None;
        }
        if let LoadState::Loaded(tables) = &self.tables {
            if let Some(selected) = self.tables_table_state.selected() {
                return tables.get(selected).map(|t| (t.id, t.name.clone()));
            }
        }
        None
    }

    /// Enter database schemas view to show schemas in a specific database.
    /// Uses navigation stack for proper back navigation.
    pub fn enter_database_schemas(&mut self, database_id: u32, database_name: String) {
        self.database_context = Some((database_id, database_name));
        // Reset schemas state for new load
        self.schemas = LoadState::Idle;
        self.schemas_table_state = TableState::default();
        // Push current view (Databases) to stack before switching
        self.push_view(ContentView::DatabaseSchemas);
    }

    /// Exit database schemas view and return to previous view.
    pub fn exit_database_schemas(&mut self) {
        self.database_context = None;
        self.schemas = LoadState::Idle;
        self.schemas_table_state = TableState::default();
        // Pop from navigation stack (defaults to Databases if stack is empty)
        if self.pop_view().is_none() {
            self.view = ContentView::Databases;
        }
    }

    /// Enter schema tables view to show tables in a specific schema.
    /// Uses navigation stack for proper back navigation.
    pub fn enter_schema_tables(&mut self, database_id: u32, schema_name: String) {
        self.schema_context = Some((database_id, schema_name));
        // Reset tables state for new load
        self.tables = LoadState::Idle;
        self.tables_table_state = TableState::default();
        // Push current view (DatabaseSchemas) to stack before switching
        self.push_view(ContentView::SchemaTables);
    }

    /// Exit schema tables view and return to previous view.
    pub fn exit_schema_tables(&mut self) {
        self.schema_context = None;
        self.tables = LoadState::Idle;
        self.tables_table_state = TableState::default();
        // Pop from navigation stack (defaults to DatabaseSchemas if stack is empty)
        if self.pop_view().is_none() {
            self.view = ContentView::DatabaseSchemas;
        }
    }

    /// Enter table preview view to show sample data from a table.
    /// Uses navigation stack for proper back navigation.
    pub fn enter_table_preview(&mut self, database_id: u32, table_id: u32, table_name: String) {
        self.table_context = Some((database_id, table_id, table_name));
        // Reset query result for new load
        self.query_result = None;
        self.result_table_state = TableState::default();
        self.result_page = 0;
        self.scroll_x = 0;
        // Push current view (SchemaTables) to stack before switching
        self.push_view(ContentView::TablePreview);
    }

    /// Exit table preview view and return to previous view.
    pub fn exit_table_preview(&mut self) {
        self.table_context = None;
        self.query_result = None;
        self.result_table_state = TableState::default();
        self.result_page = 0;
        self.scroll_x = 0;
        // Pop from navigation stack (defaults to SchemaTables if stack is empty)
        if self.pop_view().is_none() {
            self.view = ContentView::SchemaTables;
        }
    }

    /// Set table preview data (used when data is loaded after entering preview view).
    /// Does not change navigation state since enter_table_preview already handled that.
    pub fn set_table_preview_data(&mut self, data: QueryResultData) {
        self.query_result = Some(data);
        self.result_table_state = TableState::default();
        self.result_page = 0;
        // Auto-select first row if available
        if self
            .query_result
            .as_ref()
            .is_some_and(|r| !r.rows.is_empty())
        {
            self.result_table_state.select(Some(0));
        }
    }

    /// Get the current database context (id, name) for DatabaseSchemas view.
    pub fn get_database_context(&self) -> Option<&(u32, String)> {
        self.database_context.as_ref()
    }

    /// Get the current schema context (database_id, schema_name) for SchemaTables view.
    pub fn get_schema_context(&self) -> Option<&(u32, String)> {
        self.schema_context.as_ref()
    }

    /// Get the current table context (database_id, table_id, table_name) for TablePreview view.
    #[allow(dead_code)] // Designed for future features
    pub fn get_table_context(&self) -> Option<&(u32, u32, String)> {
        self.table_context.as_ref()
    }

    // === Search functionality ===

    /// Get the current input mode.
    pub fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    /// Enter search mode.
    pub fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Search;
        self.search_query.clear();
    }

    /// Exit search mode without executing search.
    pub fn exit_search_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
    }

    /// Get the current search query (for debugging/future use).
    #[allow(dead_code)]
    pub fn get_search_query(&self) -> &str {
        &self.search_query
    }

    /// Get the active search query (after execution).
    pub fn get_active_search(&self) -> Option<&str> {
        self.active_search.as_deref()
    }

    /// Execute the current search query and return it for API call.
    /// Returns Some(query) if there's a query to search, None if empty.
    pub fn execute_search(&mut self) -> Option<String> {
        self.input_mode = InputMode::Normal;
        let query = self.search_query.trim().to_string();
        if query.is_empty() {
            self.active_search = None;
            None
        } else {
            self.active_search = Some(query.clone());
            // Reset selection for new results
            self.table_state.select(Some(0));
            Some(query)
        }
    }

    /// Clear the active search and return to showing all questions.
    pub fn clear_search(&mut self) {
        self.active_search = None;
        self.search_query.clear();
        self.table_state.select(Some(0));
    }

    /// Handle character input in search mode.
    pub fn handle_search_input(&mut self, c: char) {
        self.search_query.push(c);
    }

    /// Handle backspace in search mode.
    pub fn handle_search_backspace(&mut self) {
        self.search_query.pop();
    }

    /// Get the currently selected record in QueryResult or TablePreview view.
    /// Returns (columns, values) tuple for the selected row.
    pub fn get_selected_record(&self) -> Option<(Vec<String>, Vec<String>)> {
        if self.view != ContentView::QueryResult && self.view != ContentView::TablePreview {
            return None;
        }
        if let Some(ref result) = self.query_result {
            if let Some(selected) = self.result_table_state.selected() {
                // Calculate actual row index considering pagination
                let page_start = self.result_page * self.rows_per_page;
                let actual_index = page_start + selected;

                if actual_index < result.rows.len() {
                    return Some((result.columns.clone(), result.rows[actual_index].clone()));
                }
            }
        }
        None
    }

    /// Set query result data and switch to QueryResult view.
    /// Uses navigation stack to enable returning to the originating view.
    pub fn set_query_result(&mut self, data: QueryResultData) {
        self.query_result = Some(data);
        self.result_table_state = TableState::default();
        self.result_page = 0; // Reset to first page
        self.scroll_x = 0;
        // Auto-select first row if available
        if self
            .query_result
            .as_ref()
            .is_some_and(|r| !r.rows.is_empty())
        {
            self.result_table_state.select(Some(0));
        }
        // Push current view to stack before switching
        self.push_view(ContentView::QueryResult);
    }

    /// Clear query result and return to previous view.
    /// Uses navigation stack to return to the correct originating view.
    pub fn back_to_questions(&mut self) {
        self.query_result = None;
        self.result_table_state = TableState::default();
        self.result_page = 0;
        self.scroll_x = 0;
        // Pop from navigation stack (defaults to Questions if stack is empty)
        if self.pop_view().is_none() {
            self.view = ContentView::Questions;
        }
    }

    /// Get total number of pages for query result.
    fn total_pages(&self) -> usize {
        self.query_result
            .as_ref()
            .map(|r| r.rows.len().div_ceil(self.rows_per_page))
            .unwrap_or(0)
    }

    /// Go to next page in query result.
    fn next_page(&mut self) {
        let total = self.total_pages();
        if total > 0 && self.result_page < total - 1 {
            self.result_page += 1;
            self.result_table_state.select(Some(0)); // Reset selection to first row of new page
        }
    }

    /// Go to previous page in query result.
    fn prev_page(&mut self) {
        if self.result_page > 0 {
            self.result_page -= 1;
            self.result_table_state.select(Some(0)); // Reset selection to first row of new page
        }
    }

    /// Go to first page in query result.
    fn first_page(&mut self) {
        self.result_page = 0;
        self.result_table_state.select(Some(0));
    }

    /// Go to last page in query result.
    fn last_page(&mut self) {
        let total = self.total_pages();
        if total > 0 {
            self.result_page = total - 1;
            self.result_table_state.select(Some(0));
        }
    }

    /// Scroll left (show previous columns).
    fn scroll_left(&mut self) {
        self.scroll_x = self.scroll_x.saturating_sub(1);
    }

    /// Scroll right (show next columns).
    fn scroll_right(&mut self) {
        let total_cols = self.get_total_columns();
        if total_cols > 0 && self.scroll_x < total_cols.saturating_sub(1) {
            self.scroll_x += 1;
        }
    }

    /// Get total number of columns for current view.
    fn get_total_columns(&self) -> usize {
        match self.view {
            ContentView::QueryResult => self
                .query_result
                .as_ref()
                .map(|r| r.columns.len())
                .unwrap_or(0),
            ContentView::Questions => 3, // ID, Name, Collection
            _ => 0,
        }
    }

    /// Navigate result table.
    fn select_result_next(&mut self) {
        if let Some(ref result) = self.query_result {
            if result.rows.is_empty() {
                return;
            }
            let current = self.result_table_state.selected().unwrap_or(0);
            let next = (current + 1).min(result.rows.len() - 1);
            self.result_table_state.select(Some(next));
        }
    }

    fn select_result_previous(&mut self) {
        let current = self.result_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.result_table_state.select(Some(prev));
    }

    /// Get the number of rows in the current page.
    fn current_page_row_count(&self) -> usize {
        self.query_result
            .as_ref()
            .map(|r| {
                let total_rows = r.rows.len();
                let page_start = self.result_page * self.rows_per_page;
                let page_end = (page_start + self.rows_per_page).min(total_rows);
                page_end - page_start
            })
            .unwrap_or(0)
    }

    /// Scroll result table up by multiple rows (PageUp).
    fn scroll_result_page_up(&mut self) {
        const SCROLL_AMOUNT: usize = 10;
        let current = self.result_table_state.selected().unwrap_or(0);
        let new = current.saturating_sub(SCROLL_AMOUNT);
        self.result_table_state.select(Some(new));
    }

    /// Scroll result table down by multiple rows (PageDown).
    fn scroll_result_page_down(&mut self) {
        const SCROLL_AMOUNT: usize = 10;
        let current = self.result_table_state.selected().unwrap_or(0);
        let page_row_count = self.current_page_row_count();
        let new = (current + SCROLL_AMOUNT).min(page_row_count.saturating_sub(1));
        self.result_table_state.select(Some(new));
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

    /// Render questions view with table and search bar.
    fn render_questions(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
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
    fn render_collections(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        match &self.collections {
            LoadState::Idle => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press 'r' to load collections",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Collections ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loading => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  ⏳ Loading collections...",
                        Style::default().fg(Color::Yellow),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Collections ")
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
                        .title(" Collections ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loaded(collections) => {
                if collections.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  No collections found",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                    .block(
                        Block::default()
                            .title(" Collections (0) ")
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
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
                            .style(
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .bottom_margin(1),
                    )
                    .block(
                        Block::default()
                            .title(format!(" Collections ({}) ", collections.len()))
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

                    frame.render_stateful_widget(table, area, &mut self.collections_table_state);
                }
            }
        }
    }

    /// Render databases view with table.
    fn render_databases(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        match &self.databases {
            LoadState::Idle => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  Press 'r' to load databases",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Databases ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loading => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        "  ⏳ Loading databases...",
                        Style::default().fg(Color::Yellow),
                    )),
                ])
                .block(
                    Block::default()
                        .title(" Databases ")
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
                        .title(" Databases ")
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loaded(databases) => {
                if databases.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            "  No databases found",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                    .block(
                        Block::default()
                            .title(" Databases (0) ")
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
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
                            .style(
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .bottom_margin(1),
                    )
                    .block(
                        Block::default()
                            .title(format!(" Databases ({}) ", databases.len()))
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

                    frame.render_stateful_widget(table, area, &mut self.databases_table_state);
                }
            }
        }
    }

    /// Render collection questions view with table.
    /// Shows questions filtered by a specific collection.
    fn render_collection_questions(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Get collection name for title
        let collection_name = self
            .collection_context
            .as_ref()
            .map(|(_, name)| name.as_str())
            .unwrap_or("Unknown");

        match &self.questions {
            LoadState::Idle => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  Loading questions from '{}'...", collection_name),
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(format!(" {} ", collection_name))
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loading => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  ⏳ Loading questions from '{}'...", collection_name),
                        Style::default().fg(Color::Yellow),
                    )),
                ])
                .block(
                    Block::default()
                        .title(format!(" {} ", collection_name))
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
                        "  Press 'r' to retry or Esc to go back",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(format!(" {} ", collection_name))
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
                            format!("  No questions found in '{}'", collection_name),
                            Style::default().fg(Color::DarkGray),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Press Esc to go back",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                    .block(
                        Block::default()
                            .title(format!(" {} (0) ", collection_name))
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
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
                            .style(
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )
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
                            .border_style(border_style),
                    )
                    .row_highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("► ");

                    frame.render_stateful_widget(table, area, &mut self.table_state);
                }
            }
        }
    }

    /// Render database schemas view with table.
    /// Shows schemas in a specific database.
    fn render_database_schemas(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Get database name for title
        let database_name = self
            .database_context
            .as_ref()
            .map(|(_, name)| name.as_str())
            .unwrap_or("Unknown");

        match &self.schemas {
            LoadState::Idle => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  Loading schemas from '{}'...", database_name),
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(format!(" {} - Schemas ", database_name))
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loading => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  ⏳ Loading schemas from '{}'...", database_name),
                        Style::default().fg(Color::Yellow),
                    )),
                ])
                .block(
                    Block::default()
                        .title(format!(" {} - Schemas ", database_name))
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
                        "  Press 'r' to retry or Esc to go back",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(format!(" {} - Schemas ", database_name))
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loaded(schemas) => {
                if schemas.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("  No schemas found in '{}'", database_name),
                            Style::default().fg(Color::DarkGray),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Press Esc to go back",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                    .block(
                        Block::default()
                            .title(format!(" {} - Schemas (0) ", database_name))
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
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
                            .style(
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )
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
                            .border_style(border_style),
                    )
                    .row_highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("► ");

                    frame.render_stateful_widget(table, area, &mut self.schemas_table_state);
                }
            }
        }
    }

    /// Render schema tables view with table.
    /// Shows tables in a specific schema.
    fn render_schema_tables(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Get schema name for title
        let schema_name = self
            .schema_context
            .as_ref()
            .map(|(_, name)| name.as_str())
            .unwrap_or("Unknown");

        match &self.tables {
            LoadState::Idle => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  Loading tables from '{}'...", schema_name),
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(format!(" {} - Tables ", schema_name))
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loading => {
                let paragraph = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  ⏳ Loading tables from '{}'...", schema_name),
                        Style::default().fg(Color::Yellow),
                    )),
                ])
                .block(
                    Block::default()
                        .title(format!(" {} - Tables ", schema_name))
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
                        "  Press 'r' to retry or Esc to go back",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(
                    Block::default()
                        .title(format!(" {} - Tables ", schema_name))
                        .borders(Borders::ALL)
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            LoadState::Loaded(tables) => {
                if tables.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("  No tables found in '{}'", schema_name),
                            Style::default().fg(Color::DarkGray),
                        )),
                        Line::from(""),
                        Line::from(Span::styled(
                            "  Press Esc to go back",
                            Style::default().fg(Color::DarkGray),
                        )),
                    ])
                    .block(
                        Block::default()
                            .title(format!(" {} - Tables (0) ", schema_name))
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
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
                            .style(
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )
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
                            .border_style(border_style),
                    )
                    .row_highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("► ");

                    frame.render_stateful_widget(table, area, &mut self.tables_table_state);
                }
            }
        }
    }

    /// Render table preview view with query result table.
    /// Shows sample data from a table (reuses query result rendering).
    fn render_table_preview(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Get table name for title
        let table_name = self
            .table_context
            .as_ref()
            .map(|(_, _, name)| name.as_str())
            .unwrap_or("Unknown");

        match &self.query_result {
            None => {
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
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            Some(result) => {
                if result.rows.is_empty() {
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
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
                    // Reuse query result rendering logic
                    let total_rows = result.rows.len();
                    let total_pages = self.total_pages();
                    let page_start = self.result_page * self.rows_per_page;
                    let page_end = (page_start + self.rows_per_page).min(total_rows);
                    let page_rows = &result.rows[page_start..page_end];

                    let total_cols = result.columns.len();
                    let scroll_x = self.scroll_x.min(total_cols.saturating_sub(1));

                    let available_width = area.width.saturating_sub(4) as usize;
                    let min_col_width = 15usize;
                    let visible_cols = (available_width / min_col_width).max(1).min(total_cols);
                    let end_col = (scroll_x + visible_cols).min(total_cols);

                    let visible_columns: Vec<String> = result.columns[scroll_x..end_col].to_vec();
                    let visible_col_count = visible_columns.len();

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

                    let rows: Vec<Row> = page_rows
                        .iter()
                        .map(|row| {
                            let cells: Vec<Cell> = row[scroll_x..end_col.min(row.len())]
                                .iter()
                                .map(|cell| Cell::from(cell.clone()))
                                .collect();
                            Row::new(cells)
                        })
                        .collect();

                    let header_cells: Vec<Cell> = visible_columns
                        .iter()
                        .map(|col| Cell::from(col.clone()))
                        .collect();

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

                    let table = Table::new(rows, constraints)
                        .header(
                            Row::new(header_cells)
                                .style(
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                )
                                .bottom_margin(1),
                        )
                        .block(
                            Block::default()
                                .title(format!(
                                    " {} - Preview{}{}",
                                    table_name, page_indicator, col_indicator
                                ))
                                .borders(Borders::ALL)
                                .border_style(border_style),
                        )
                        .row_highlight_style(
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("► ");

                    frame.render_stateful_widget(table, area, &mut self.result_table_state);
                }
            }
        }
    }

    /// Render query result view with table.
    fn render_query_result(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        match &self.query_result {
            None => {
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
                        .border_style(border_style),
                );
                frame.render_widget(paragraph, area);
            }
            Some(result) => {
                if result.rows.is_empty() {
                    let paragraph = Paragraph::new(vec![
                        Line::from(""),
                        Line::from(Span::styled(
                            format!("  Query: {}", result.question_name),
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
                            .title(format!(" Query Result: {} (0 rows) ", result.question_name))
                            .borders(Borders::ALL)
                            .border_style(border_style),
                    );
                    frame.render_widget(paragraph, area);
                } else {
                    // Pagination: calculate row range for current page
                    let total_rows = result.rows.len();
                    let total_pages = self.total_pages();
                    let page_start = self.result_page * self.rows_per_page;
                    let page_end = (page_start + self.rows_per_page).min(total_rows);
                    let page_rows = &result.rows[page_start..page_end];

                    // Calculate visible columns based on scroll_x
                    let total_cols = result.columns.len();
                    let scroll_x = self.scroll_x.min(total_cols.saturating_sub(1));

                    // Calculate how many columns can fit (estimate based on min width)
                    let available_width = area.width.saturating_sub(4) as usize; // borders + highlight symbol
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
                    let rows: Vec<Row> = page_rows
                        .iter()
                        .map(|row| {
                            let cells: Vec<Cell> = row[scroll_x..end_col.min(row.len())]
                                .iter()
                                .map(|cell| Cell::from(cell.clone()))
                                .collect();
                            Row::new(cells)
                        })
                        .collect();

                    // Create header row
                    let header_cells: Vec<Cell> = visible_columns
                        .iter()
                        .map(|col| Cell::from(col.clone()))
                        .collect();

                    // Build column indicator (e.g., "← 1-5 of 12 →")
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

                    let table = Table::new(rows, constraints)
                        .header(
                            Row::new(header_cells)
                                .style(
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                )
                                .bottom_margin(1),
                        )
                        .block(
                            Block::default()
                                .title(format!(
                                    " {}{}{}",
                                    result.question_name, page_indicator, col_indicator
                                ))
                                .borders(Borders::ALL)
                                .border_style(border_style),
                        )
                        .row_highlight_style(
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Green)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("► ");

                    frame.render_stateful_widget(table, area, &mut self.result_table_state);
                }
            }
        }
    }
}

impl Component for ContentPanel {
    fn draw(&mut self, frame: &mut Frame, area: Rect, focused: bool) {
        // Table-based views render directly (uses stateful Table widget)
        match self.view {
            ContentView::Questions => {
                self.render_questions(area, frame, focused);
                return;
            }
            ContentView::Collections => {
                self.render_collections(area, frame, focused);
                return;
            }
            ContentView::Databases => {
                self.render_databases(area, frame, focused);
                return;
            }
            ContentView::QueryResult => {
                self.render_query_result(area, frame, focused);
                return;
            }
            ContentView::CollectionQuestions => {
                self.render_collection_questions(area, frame, focused);
                return;
            }
            ContentView::DatabaseSchemas => {
                self.render_database_schemas(area, frame, focused);
                return;
            }
            ContentView::SchemaTables => {
                self.render_schema_tables(area, frame, focused);
                return;
            }
            ContentView::TablePreview => {
                self.render_table_preview(area, frame, focused);
                return;
            }
            ContentView::Welcome => {}
        }

        // Welcome view returns Paragraph widget
        let widget = self.render_welcome(area, focused);
        frame.render_widget(widget, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Search mode input handling (takes priority in Questions view)
        if self.input_mode == InputMode::Search {
            match key.code {
                KeyCode::Char(c) => {
                    self.handle_search_input(c);
                    true
                }
                KeyCode::Backspace => {
                    self.handle_search_backspace();
                    true
                }
                // Enter and Esc are handled by App (to send actions)
                _ => false,
            }
        // Questions view has list navigation
        } else if self.view == ContentView::Questions {
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
                    self.select_first();
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.select_last();
                    true
                }
                KeyCode::Char('/') => {
                    self.enter_search_mode();
                    true
                }
                _ => false,
            }
        } else if self.view == ContentView::Collections {
            // Collections view has list navigation
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_collections_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_collections_next();
                    true
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.select_collections_first();
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.select_collections_last();
                    true
                }
                _ => false,
            }
        } else if self.view == ContentView::Databases {
            // Databases view has list navigation
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_databases_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_databases_next();
                    true
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.select_databases_first();
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.select_databases_last();
                    true
                }
                _ => false,
            }
        } else if self.view == ContentView::CollectionQuestions {
            // CollectionQuestions view has same navigation as Questions
            // Enter/Esc handled by App
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
                    self.select_first();
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.select_last();
                    true
                }
                _ => false,
            }
        } else if self.view == ContentView::QueryResult {
            // QueryResult view has result table navigation + horizontal scroll + pagination
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_result_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_result_next();
                    true
                }
                // Horizontal scroll with h/l or Left/Right arrows
                KeyCode::Left | KeyCode::Char('h') => {
                    self.scroll_left();
                    true
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.scroll_right();
                    true
                }
                // Pagination: n for next page, p for previous page (matches CLI)
                KeyCode::Char('n') => {
                    self.next_page();
                    true
                }
                KeyCode::Char('p') => {
                    self.prev_page();
                    true
                }
                // PageUp/PageDown for scrolling within page (matches CLI)
                KeyCode::PageUp => {
                    self.scroll_result_page_up();
                    true
                }
                KeyCode::PageDown => {
                    self.scroll_result_page_down();
                    true
                }
                // First/Last page with g/G
                KeyCode::Home | KeyCode::Char('g') => {
                    self.first_page();
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.last_page();
                    true
                }
                // Note: Esc is handled in App for returning to Questions
                _ => false,
            }
        } else if self.view == ContentView::DatabaseSchemas {
            // DatabaseSchemas view has list navigation
            // Enter/Esc handled by App
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_schemas_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_schemas_next();
                    true
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.select_schemas_first();
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.select_schemas_last();
                    true
                }
                _ => false,
            }
        } else if self.view == ContentView::SchemaTables {
            // SchemaTables view has list navigation
            // Enter/Esc handled by App
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_tables_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_tables_next();
                    true
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.select_tables_first();
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.select_tables_last();
                    true
                }
                _ => false,
            }
        } else if self.view == ContentView::TablePreview {
            // TablePreview view has result table navigation (same as QueryResult)
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    self.select_result_previous();
                    true
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.select_result_next();
                    true
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    self.scroll_left();
                    true
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.scroll_right();
                    true
                }
                KeyCode::Char('n') => {
                    self.next_page();
                    true
                }
                KeyCode::Char('p') => {
                    self.prev_page();
                    true
                }
                KeyCode::PageUp => {
                    self.scroll_result_page_up();
                    true
                }
                KeyCode::PageDown => {
                    self.scroll_result_page_down();
                    true
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    self.first_page();
                    true
                }
                KeyCode::End | KeyCode::Char('G') => {
                    self.last_page();
                    true
                }
                // Note: Esc is handled in App for returning to SchemaTables
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
}
