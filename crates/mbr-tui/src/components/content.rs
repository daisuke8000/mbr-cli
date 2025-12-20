//! Content panel component.
//!
//! Displays the main content area (query results, question details, etc.).

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
};

use mbr_core::api::models::{CollectionItem, Database, Question, TableInfo};

use super::state_renderer::{LoadStateConfig, render_non_loaded_state};
use super::styles::{HIGHLIGHT_SYMBOL, border_style, header_style, row_highlight_style};
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

/// Sort order for query results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// No sorting applied
    #[default]
    None,
    /// Ascending order (A-Z, 0-9)
    Ascending,
    /// Descending order (Z-A, 9-0)
    Descending,
}

/// Content view types with embedded navigation context.
///
/// Views that represent drill-down navigation carry their context data directly,
/// eliminating the need for separate context fields and ensuring consistency
/// between the navigation stack and the current state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ContentView {
    #[default]
    Welcome,
    Questions,
    Collections,
    Databases,
    QueryResult,
    /// Questions filtered by a specific collection (id, name)
    CollectionQuestions {
        id: u32,
        name: String,
    },
    /// Schemas in a specific database (db_id, db_name)
    DatabaseSchemas {
        db_id: u32,
        db_name: String,
    },
    /// Tables in a specific schema (db_id, schema_name)
    SchemaTables {
        db_id: u32,
        schema_name: String,
    },
    /// Table data preview (db_id, table_id, table_name)
    TablePreview {
        db_id: u32,
        table_id: u32,
        table_name: String,
    },
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
    /// Sorted row indices (None = original order, Some = sorted indices)
    /// Using indices instead of copying rows for memory efficiency
    sort_indices: Option<Vec<usize>>,
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
    /// Navigation stack for multi-level drill-down (supports 4+ levels)
    /// Context data is now embedded directly in ContentView variants.
    /// Used for: Databases → Schemas → Tables → Preview
    ///           Collections → Questions → QueryResult
    navigation_stack: Vec<ContentView>,
    /// Sort order for query results
    sort_order: SortOrder,
    /// Column index currently being sorted (None = no sort)
    sort_column_index: Option<usize>,
    /// Whether sort column selection modal is active
    sort_mode_active: bool,
    /// Selected column index in sort modal
    sort_modal_selection: usize,
    // === Filter state ===
    /// Filtered row indices (None = no filter, Some = filtered indices)
    filter_indices: Option<Vec<usize>>,
    /// Column index currently being filtered (None = no filter)
    filter_column_index: Option<usize>,
    /// Filter text (case-insensitive contains match)
    filter_text: String,
    /// Whether filter modal is active
    filter_mode_active: bool,
    /// Current step in filter modal (0 = column selection, 1 = text input)
    filter_modal_step: usize,
    /// Selected column index in filter modal
    filter_modal_selection: usize,
    // === Result Search state (all-column search) ===
    /// Whether result search mode is active
    result_search_active: bool,
    /// Search text for result search (all-column, case-insensitive)
    result_search_text: String,
    /// Searched row indices (None = no search, Some = matched indices)
    result_search_indices: Option<Vec<usize>>,
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
            sort_indices: None,
            result_table_state: TableState::default(),
            result_page: 0,
            rows_per_page: DEFAULT_ROWS_PER_PAGE,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            active_search: None,
            navigation_stack: Vec::new(),
            sort_order: SortOrder::None,
            sort_column_index: None,
            sort_mode_active: false,
            sort_modal_selection: 0,
            filter_indices: None,
            filter_column_index: None,
            filter_text: String::new(),
            filter_mode_active: false,
            filter_modal_step: 0,
            filter_modal_selection: 0,
            result_search_active: false,
            result_search_text: String::new(),
            result_search_indices: None,
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

    /// Get the current view (cloned since ContentView now contains data).
    pub fn current_view(&self) -> ContentView {
        self.view.clone()
    }

    /// Check if the current view is a specific type (ignoring embedded data).
    #[allow(dead_code)] // Utility method for future use
    pub fn is_view(&self, view_type: &ContentView) -> bool {
        std::mem::discriminant(&self.view) == std::mem::discriminant(view_type)
    }

    /// Check if current view is Questions or CollectionQuestions.
    pub fn is_questions_view(&self) -> bool {
        matches!(
            self.view,
            ContentView::Questions | ContentView::CollectionQuestions { .. }
        )
    }

    /// Check if current view is CollectionQuestions.
    pub fn is_collection_questions_view(&self) -> bool {
        matches!(self.view, ContentView::CollectionQuestions { .. })
    }

    /// Check if current view is DatabaseSchemas.
    pub fn is_database_schemas_view(&self) -> bool {
        matches!(self.view, ContentView::DatabaseSchemas { .. })
    }

    /// Check if current view is SchemaTables.
    pub fn is_schema_tables_view(&self) -> bool {
        matches!(self.view, ContentView::SchemaTables { .. })
    }

    /// Check if current view is TablePreview.
    pub fn is_table_preview_view(&self) -> bool {
        matches!(self.view, ContentView::TablePreview { .. })
    }

    /// Check if current view is QueryResult or TablePreview (both show query results).
    pub fn is_result_view(&self) -> bool {
        matches!(
            self.view,
            ContentView::QueryResult | ContentView::TablePreview { .. }
        )
    }

    // === Navigation Stack Methods ===

    /// Push current view to stack and navigate to new view.
    /// Used for drill-down navigation (e.g., Collections → Questions).
    pub fn push_view(&mut self, new_view: ContentView) {
        self.navigation_stack.push(self.view.clone());
        self.view = new_view;
    }

    /// Pop from navigation stack and return to previous view.
    /// Returns the view that was popped to, or None if stack was empty.
    pub fn pop_view(&mut self) -> Option<ContentView> {
        if let Some(previous) = self.navigation_stack.pop() {
            self.view = previous.clone();
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
        if !self.is_questions_view() {
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
        // Reset questions state for new load
        self.questions = LoadState::Idle;
        self.table_state = TableState::default();
        // Push new view with embedded context
        self.push_view(ContentView::CollectionQuestions {
            id: collection_id,
            name: collection_name,
        });
    }

    /// Exit collection questions view and return to previous view.
    /// Uses navigation stack to return to the correct originating view.
    pub fn exit_collection_questions(&mut self) {
        // Reset questions state
        self.questions = LoadState::Idle;
        self.table_state = TableState::default();
        // Pop from navigation stack (defaults to Collections if stack is empty)
        if self.pop_view().is_none() {
            self.view = ContentView::Collections;
        }
    }

    /// Get the current collection context (id, name) for CollectionQuestions view.
    /// Extracts context from the ContentView variant.
    #[allow(dead_code)] // Designed for future features
    pub fn get_collection_context(&self) -> Option<(u32, String)> {
        match &self.view {
            ContentView::CollectionQuestions { id, name } => Some((*id, name.clone())),
            _ => None,
        }
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
        if !self.is_database_schemas_view() {
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
        if !self.is_schema_tables_view() {
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
        // Reset schemas state for new load
        self.schemas = LoadState::Idle;
        self.schemas_table_state = TableState::default();
        // Push new view with embedded context
        self.push_view(ContentView::DatabaseSchemas {
            db_id: database_id,
            db_name: database_name,
        });
    }

    /// Exit database schemas view and return to previous view.
    pub fn exit_database_schemas(&mut self) {
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
        // Reset tables state for new load
        self.tables = LoadState::Idle;
        self.tables_table_state = TableState::default();
        // Push new view with embedded context
        self.push_view(ContentView::SchemaTables {
            db_id: database_id,
            schema_name,
        });
    }

    /// Exit schema tables view and return to previous view.
    pub fn exit_schema_tables(&mut self) {
        self.tables = LoadState::Idle;
        self.tables_table_state = TableState::default();
        // Pop from navigation stack (defaults to DatabaseSchemas if stack is empty)
        if self.pop_view().is_none() {
            // Fallback without context - should rarely happen
            self.view = ContentView::Databases;
        }
    }

    /// Enter table preview view to show sample data from a table.
    /// Uses navigation stack for proper back navigation.
    pub fn enter_table_preview(&mut self, database_id: u32, table_id: u32, table_name: String) {
        // Reset query result for new load
        self.query_result = None;
        self.sort_indices = None;
        self.filter_indices = None;
        self.result_table_state = TableState::default();
        self.result_page = 0;
        self.scroll_x = 0;
        // Reset sort/filter state
        self.reset_sort_filter_state();
        // Push new view with embedded context
        self.push_view(ContentView::TablePreview {
            db_id: database_id,
            table_id,
            table_name,
        });
    }

    /// Exit table preview view and return to previous view.
    pub fn exit_table_preview(&mut self) {
        self.query_result = None;
        self.sort_indices = None;
        self.filter_indices = None;
        self.result_table_state = TableState::default();
        self.result_page = 0;
        self.scroll_x = 0;
        // Reset sort/filter state
        self.reset_sort_filter_state();
        // Pop from navigation stack (defaults to SchemaTables if stack is empty)
        if self.pop_view().is_none() {
            // Fallback without context - should rarely happen
            self.view = ContentView::Databases;
        }
    }

    /// Set table preview data (used when data is loaded after entering preview view).
    /// Does not change navigation state since enter_table_preview already handled that.
    pub fn set_table_preview_data(&mut self, data: QueryResultData) {
        // Clear sort/filter indices for new data
        self.sort_indices = None;
        self.filter_indices = None;
        self.query_result = Some(data);
        self.result_table_state = TableState::default();
        self.result_page = 0;
        // Reset sort/filter state for new data
        self.reset_sort_filter_state();
        // Auto-select first row if available
        if self
            .query_result
            .as_ref()
            .is_some_and(|r| !r.rows.is_empty())
        {
            self.result_table_state.select(Some(0));
        }
    }

    /// Get the current database context (db_id, db_name) for DatabaseSchemas view.
    /// Extracts context from the ContentView variant.
    pub fn get_database_context(&self) -> Option<(u32, String)> {
        match &self.view {
            ContentView::DatabaseSchemas { db_id, db_name } => Some((*db_id, db_name.clone())),
            ContentView::SchemaTables { db_id, .. } => {
                // Also available in SchemaTables since we're drilling down from DatabaseSchemas
                // Need to look at navigation stack for the db_name
                for view in self.navigation_stack.iter().rev() {
                    if let ContentView::DatabaseSchemas { db_id: id, db_name } = view {
                        if *id == *db_id {
                            return Some((*id, db_name.clone()));
                        }
                    }
                }
                None
            }
            ContentView::TablePreview { db_id, .. } => {
                // Look at navigation stack for database context
                for view in self.navigation_stack.iter().rev() {
                    if let ContentView::DatabaseSchemas { db_id: id, db_name } = view {
                        if *id == *db_id {
                            return Some((*id, db_name.clone()));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Get the current schema context (db_id, schema_name) for SchemaTables view.
    /// Extracts context from the ContentView variant.
    pub fn get_schema_context(&self) -> Option<(u32, String)> {
        match &self.view {
            ContentView::SchemaTables { db_id, schema_name } => Some((*db_id, schema_name.clone())),
            ContentView::TablePreview { db_id, .. } => {
                // Look at navigation stack for schema context
                for view in self.navigation_stack.iter().rev() {
                    if let ContentView::SchemaTables {
                        db_id: id,
                        schema_name,
                    } = view
                    {
                        if *id == *db_id {
                            return Some((*id, schema_name.clone()));
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Get the current table context (db_id, table_id, table_name) for TablePreview view.
    /// Extracts context from the ContentView variant.
    #[allow(dead_code)] // Designed for future features
    pub fn get_table_context(&self) -> Option<(u32, u32, String)> {
        match &self.view {
            ContentView::TablePreview {
                db_id,
                table_id,
                table_name,
            } => Some((*db_id, *table_id, table_name.clone())),
            _ => None,
        }
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
    /// Respects sort order when sorting is active.
    pub fn get_selected_record(&self) -> Option<(Vec<String>, Vec<String>)> {
        if !self.is_result_view() {
            return None;
        }
        if let Some(ref result) = self.query_result {
            if let Some(selected) = self.result_table_state.selected() {
                // Calculate logical index considering pagination
                let page_start = self.result_page * self.rows_per_page;
                let logical_index = page_start + selected;

                // Get row using sorted index if sorting is active
                if let Some(row) = self.get_visible_row(logical_index) {
                    return Some((result.columns.clone(), row.clone()));
                }
            }
        }
        None
    }

    /// Set query result data and switch to QueryResult view.
    /// Uses navigation stack to enable returning to the originating view.
    pub fn set_query_result(&mut self, data: QueryResultData) {
        // Clear sort/filter indices for new data
        self.sort_indices = None;
        self.filter_indices = None;
        self.query_result = Some(data);
        self.result_table_state = TableState::default();
        self.result_page = 0; // Reset to first page
        self.scroll_x = 0;
        // Reset sort/filter state for new data
        self.reset_sort_filter_state();
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
        self.sort_indices = None;
        self.filter_indices = None;
        self.result_table_state = TableState::default();
        self.result_page = 0;
        self.scroll_x = 0;
        // Reset sort/filter state
        self.reset_sort_filter_state();
        // Pop from navigation stack (defaults to Questions if stack is empty)
        if self.pop_view().is_none() {
            self.view = ContentView::Questions;
        }
    }

    /// Reset sort, filter, and search state (helper for view transitions).
    fn reset_sort_filter_state(&mut self) {
        // Sort state
        self.sort_order = SortOrder::None;
        self.sort_column_index = None;
        self.sort_mode_active = false;
        // Filter state
        self.filter_column_index = None;
        self.filter_text.clear();
        self.filter_mode_active = false;
        self.filter_modal_step = 0;
        // Result search state
        self.result_search_active = false;
        self.result_search_text.clear();
        self.result_search_indices = None;
    }

    /// Get the number of visible rows (after search and filter are applied).
    ///
    /// Priority: Search first, then Filter. Both are applied if both active.
    fn visible_row_count(&self) -> usize {
        match (&self.result_search_indices, &self.filter_indices) {
            // Both search and filter: use filter (which operates on search results)
            (Some(_), Some(filter)) => filter.len(),
            // Only filter: use filter indices
            (None, Some(filter)) => filter.len(),
            // Only search: use search indices
            (Some(search), None) => search.len(),
            // Neither: use all rows
            (None, None) => self
                .query_result
                .as_ref()
                .map(|r| r.rows.len())
                .unwrap_or(0),
        }
    }

    /// Get total number of pages for query result (considers filter).
    fn total_pages(&self) -> usize {
        self.visible_row_count().div_ceil(self.rows_per_page)
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

    // === Sort functionality ===

    /// Check if sort modal is active.
    pub fn is_sort_mode_active(&self) -> bool {
        self.sort_mode_active
    }

    /// Open sort column selection modal.
    pub fn open_sort_modal(&mut self) {
        if let Some(ref result) = self.query_result {
            if !result.columns.is_empty() {
                self.sort_mode_active = true;
                // Start at currently sorted column or first column
                self.sort_modal_selection = self.sort_column_index.unwrap_or(0);
            }
        }
    }

    /// Close sort modal without applying.
    pub fn close_sort_modal(&mut self) {
        self.sort_mode_active = false;
    }

    /// Move selection up in sort modal.
    pub fn sort_modal_up(&mut self) {
        if self.sort_modal_selection > 0 {
            self.sort_modal_selection -= 1;
        }
    }

    /// Move selection down in sort modal.
    pub fn sort_modal_down(&mut self) {
        if let Some(ref result) = self.query_result {
            if self.sort_modal_selection < result.columns.len().saturating_sub(1) {
                self.sort_modal_selection += 1;
            }
        }
    }

    /// Apply sort on selected column.
    /// If same column is selected, toggles between Ascending -> Descending -> None.
    pub fn apply_sort(&mut self) {
        let selected_col = self.sort_modal_selection;

        // Toggle sort order
        if self.sort_column_index == Some(selected_col) {
            // Same column - cycle through orders
            self.sort_order = match self.sort_order {
                SortOrder::None => SortOrder::Ascending,
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::None,
            };
            if self.sort_order == SortOrder::None {
                // Restore original order by clearing indices
                self.sort_column_index = None;
                self.sort_indices = None;
            } else {
                // Sort indices (not data)
                self.update_sort_indices();
            }
        } else {
            // New column - start with ascending
            self.sort_column_index = Some(selected_col);
            self.sort_order = SortOrder::Ascending;
            // Sort indices (not data)
            self.update_sort_indices();
        }

        // Close modal
        self.sort_mode_active = false;

        // Reset to first page and first row after sort
        self.result_page = 0;
        self.result_table_state.select(Some(0));
    }

    /// Clear sort and restore original order.
    #[allow(dead_code)] // Designed for future features
    pub fn clear_sort(&mut self) {
        self.sort_order = SortOrder::None;
        self.sort_column_index = None;
        self.sort_indices = None;
    }

    /// Update sort indices based on current sort column and order.
    /// Uses index-based sorting for memory efficiency (no data cloning).
    fn update_sort_indices(&mut self) {
        if self.sort_order == SortOrder::None {
            self.sort_indices = None;
            return;
        }

        let col_idx = match self.sort_column_index {
            Some(idx) => idx,
            None => {
                self.sort_indices = None;
                return;
            }
        };

        if let Some(ref result) = self.query_result {
            if col_idx >= result.columns.len() {
                self.sort_indices = None;
                return;
            }

            // Create index array based on filtered or full row count
            let row_count = self.visible_row_count();
            let mut indices: Vec<usize> = (0..row_count).collect();

            // Sort indices based on row values at col_idx
            // When filter is active, we need to look up actual row through filter_indices
            let order = self.sort_order;
            let filter_indices = &self.filter_indices;
            indices.sort_by(|&a, &b| {
                // Get actual row index (through filter if active)
                let actual_a = if let Some(fi) = filter_indices {
                    fi[a]
                } else {
                    a
                };
                let actual_b = if let Some(fi) = filter_indices {
                    fi[b]
                } else {
                    b
                };

                let val_a = result.rows[actual_a]
                    .get(col_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let val_b = result.rows[actual_b]
                    .get(col_idx)
                    .map(|s| s.as_str())
                    .unwrap_or("");

                // Try numeric comparison first
                let cmp = match (val_a.parse::<f64>(), val_b.parse::<f64>()) {
                    (Ok(num_a), Ok(num_b)) => num_a
                        .partial_cmp(&num_b)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    _ => val_a.cmp(val_b), // Fall back to string comparison
                };

                match order {
                    SortOrder::Ascending => cmp,
                    SortOrder::Descending => cmp.reverse(),
                    SortOrder::None => std::cmp::Ordering::Equal,
                }
            });

            self.sort_indices = Some(indices);
        }
    }

    /// Get row at logical index (respects search, filter, and sort).
    /// Returns the actual row from query_result based on search, filter, and sort indices.
    fn get_visible_row(&self, logical_index: usize) -> Option<&Vec<String>> {
        self.query_result.as_ref().and_then(|result| {
            // Step 1: Apply sort (if active), get index into visible rows
            let sorted_index = if let Some(ref sort_idx) = self.sort_indices {
                *sort_idx.get(logical_index)?
            } else {
                logical_index
            };

            // Step 2: Get actual row index from filter or search indices
            // Priority: Filter (which may operate on search results) > Search > None
            let actual_index = match (&self.filter_indices, &self.result_search_indices) {
                // Filter is active (may be filtering search results)
                (Some(filter), _) => *filter.get(sorted_index)?,
                // Only search active
                (None, Some(search)) => *search.get(sorted_index)?,
                // Neither: direct index
                (None, None) => sorted_index,
            };

            result.rows.get(actual_index)
        })
    }

    /// Get current sort info for display.
    pub fn get_sort_info(&self) -> Option<(String, SortOrder)> {
        if let (Some(col_idx), Some(result)) = (self.sort_column_index, &self.query_result) {
            if col_idx < result.columns.len() && self.sort_order != SortOrder::None {
                return Some((result.columns[col_idx].clone(), self.sort_order));
            }
        }
        None
    }

    // === Filter Modal Methods ===

    /// Check if filter modal is active.
    pub fn is_filter_mode_active(&self) -> bool {
        self.filter_mode_active
    }

    /// Open filter column selection modal.
    pub fn open_filter_modal(&mut self) {
        if let Some(ref result) = self.query_result {
            if !result.columns.is_empty() {
                self.filter_mode_active = true;
                self.filter_modal_step = 0; // Start at column selection
                // Start at currently filtered column or first column
                self.filter_modal_selection = self.filter_column_index.unwrap_or(0);
            }
        }
    }

    /// Close filter modal without applying.
    pub fn close_filter_modal(&mut self) {
        self.filter_mode_active = false;
        self.filter_modal_step = 0;
        // Don't clear filter_text here to preserve it for re-editing
    }

    /// Move selection up in filter modal (column selection step).
    pub fn filter_modal_up(&mut self) {
        if self.filter_modal_step == 0 && self.filter_modal_selection > 0 {
            self.filter_modal_selection -= 1;
        }
    }

    /// Move selection down in filter modal (column selection step).
    pub fn filter_modal_down(&mut self) {
        if self.filter_modal_step == 0 {
            if let Some(ref result) = self.query_result {
                if self.filter_modal_selection < result.columns.len().saturating_sub(1) {
                    self.filter_modal_selection += 1;
                }
            }
        }
    }

    /// Move to next step in filter modal (column selection → text input).
    pub fn filter_modal_next_step(&mut self) {
        if self.filter_modal_step == 0 {
            self.filter_modal_step = 1;
            // Pre-fill with existing filter text if same column
            if self.filter_column_index != Some(self.filter_modal_selection) {
                self.filter_text.clear();
            }
        }
    }

    /// Move to previous step in filter modal (text input → column selection).
    pub fn filter_modal_prev_step(&mut self) {
        if self.filter_modal_step == 1 {
            self.filter_modal_step = 0;
        }
    }

    /// Handle character input in filter modal (text input step).
    pub fn filter_modal_input_char(&mut self, c: char) {
        if self.filter_modal_step == 1 {
            self.filter_text.push(c);
        }
    }

    /// Handle backspace in filter modal (text input step).
    pub fn filter_modal_delete_char(&mut self) {
        if self.filter_modal_step == 1 {
            self.filter_text.pop();
        }
    }

    /// Apply filter on selected column with current filter text.
    pub fn apply_filter(&mut self) {
        let selected_col = self.filter_modal_selection;

        if self.filter_text.is_empty() {
            // Empty filter text - clear filter
            self.clear_filter();
        } else {
            // Apply filter
            self.filter_column_index = Some(selected_col);
            self.update_filter_indices();
            // Re-apply sort on filtered data
            if self.sort_order != SortOrder::None {
                self.update_sort_indices();
            }
        }

        // Close modal
        self.filter_mode_active = false;
        self.filter_modal_step = 0;

        // Reset to first page and first row after filter
        self.result_page = 0;
        self.result_table_state.select(Some(0));
    }

    /// Clear filter and restore all rows.
    pub fn clear_filter(&mut self) {
        self.filter_column_index = None;
        self.filter_text.clear();
        self.filter_indices = None;
        // Re-apply sort on full data
        if self.sort_order != SortOrder::None {
            self.update_sort_indices();
        }
    }

    /// Update filter indices based on current filter column and text.
    /// If search is active, filters within search results.
    fn update_filter_indices(&mut self) {
        let col_idx = match self.filter_column_index {
            Some(idx) => idx,
            None => {
                self.filter_indices = None;
                return;
            }
        };

        if self.filter_text.is_empty() {
            self.filter_indices = None;
            return;
        }

        if let Some(ref result) = self.query_result {
            if col_idx >= result.columns.len() {
                self.filter_indices = None;
                return;
            }

            // Case-insensitive contains match
            let filter_lower = self.filter_text.to_lowercase();

            // Determine the base set of indices to filter from
            let base_indices: Box<dyn Iterator<Item = usize>> =
                if let Some(ref search_indices) = self.result_search_indices {
                    // Filter within search results
                    Box::new(search_indices.iter().copied())
                } else {
                    // Filter all rows
                    Box::new(0..result.rows.len())
                };

            let indices: Vec<usize> = base_indices
                .filter(|&i| {
                    result
                        .rows
                        .get(i)
                        .and_then(|row| row.get(col_idx))
                        .map(|cell| cell.to_lowercase().contains(&filter_lower))
                        .unwrap_or(false)
                })
                .collect();

            self.filter_indices = Some(indices);
        }
    }

    /// Get current filter info for display.
    /// Returns (column_name, filter_text, visible_row_count) if filter is active.
    #[allow(dead_code)] // Designed for status bar display in future
    pub fn get_filter_info(&self) -> Option<(String, String, usize)> {
        if let (Some(col_idx), Some(result)) = (self.filter_column_index, &self.query_result) {
            if col_idx < result.columns.len() && !self.filter_text.is_empty() {
                let visible = self.visible_row_count();
                return Some((
                    result.columns[col_idx].clone(),
                    self.filter_text.clone(),
                    visible,
                ));
            }
        }
        None
    }

    // === Result Search Methods (all-column search) ===

    /// Check if result search mode is active.
    pub fn is_result_search_active(&self) -> bool {
        self.result_search_active
    }

    /// Open result search input.
    pub fn open_result_search(&mut self) {
        self.result_search_active = true;
        // Don't clear previous search text, allow editing
    }

    /// Close result search input without applying.
    pub fn close_result_search(&mut self) {
        self.result_search_active = false;
    }

    /// Add a character to result search text (real-time search).
    pub fn result_search_input_char(&mut self, c: char) {
        self.result_search_text.push(c);
        self.update_result_search_indices();
        // If filter is active, re-apply filter on new search results
        if self.filter_column_index.is_some() && !self.filter_text.is_empty() {
            self.update_filter_indices();
        }
        // Re-apply sort on new filtered/searched data
        if self.sort_order != SortOrder::None {
            self.update_sort_indices();
        }
    }

    /// Delete the last character from result search text.
    pub fn result_search_delete_char(&mut self) {
        self.result_search_text.pop();
        self.update_result_search_indices();
        // If filter is active, re-apply filter on new search results
        if self.filter_column_index.is_some() && !self.filter_text.is_empty() {
            self.update_filter_indices();
        }
        // Re-apply sort on new filtered/searched data
        if self.sort_order != SortOrder::None {
            self.update_sort_indices();
        }
    }

    /// Apply result search and close input.
    pub fn apply_result_search(&mut self) {
        self.result_search_active = false;
        // Reset to first page
        self.result_page = 0;
        self.result_table_state.select(Some(0));
    }

    /// Clear result search and restore all rows.
    pub fn clear_result_search(&mut self) {
        self.result_search_text.clear();
        self.result_search_indices = None;
        self.result_search_active = false;
        // If filter is active, re-apply filter on full data
        if self.filter_column_index.is_some() && !self.filter_text.is_empty() {
            self.update_filter_indices();
        }
        // Re-apply sort
        if self.sort_order != SortOrder::None {
            self.update_sort_indices();
        }
    }

    /// Update result search indices based on current search text.
    /// Searches all columns with case-insensitive contains match.
    fn update_result_search_indices(&mut self) {
        if self.result_search_text.is_empty() {
            self.result_search_indices = None;
            return;
        }

        if let Some(ref result) = self.query_result {
            let search_lower = self.result_search_text.to_lowercase();
            let indices: Vec<usize> = result
                .rows
                .iter()
                .enumerate()
                .filter(|(_, row)| {
                    // Match if any column contains the search text
                    row.iter()
                        .any(|cell| cell.to_lowercase().contains(&search_lower))
                })
                .map(|(i, _)| i)
                .collect();

            self.result_search_indices = Some(indices);
        }
    }

    /// Get current result search info for display.
    /// Returns (search_text, matched_count, total_count) if search is active.
    #[allow(dead_code)] // Designed for status bar display
    pub fn get_result_search_info(&self) -> Option<(String, usize, usize)> {
        if !self.result_search_text.is_empty() {
            let total = self
                .query_result
                .as_ref()
                .map(|r| r.rows.len())
                .unwrap_or(0);
            let matched = self.visible_row_count();
            return Some((self.result_search_text.clone(), matched, total));
        }
        None
    }

    /// Render result search bar as an overlay at the bottom.
    fn render_result_search_bar(&self, frame: &mut Frame, area: Rect) {
        if !self.result_search_active && self.result_search_text.is_empty() {
            return;
        }

        // Search bar at bottom of content area
        let bar_height = 3u16;
        let bar_y = area.y + area.height.saturating_sub(bar_height);
        let bar_area = Rect::new(area.x, bar_y, area.width, bar_height);

        // Clear background for better visibility
        frame.render_widget(Clear, bar_area);

        // Build search display
        let total = self
            .query_result
            .as_ref()
            .map(|r| r.rows.len())
            .unwrap_or(0);
        let matched = self.visible_row_count();

        let (input_display, title_suffix) = if self.result_search_active {
            (format!("{}_", self.result_search_text), " (editing)")
        } else {
            (self.result_search_text.clone(), "")
        };

        let title = format!(" Search Results{} ", title_suffix);

        let lines = vec![
            Line::from(vec![
                Span::styled(" > ", Style::default().fg(Color::Yellow)),
                Span::styled(
                    input_display,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!(" Found: {} / {} rows", matched, total),
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .title_style(
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            )
            .style(Style::default().bg(Color::Black));

        frame.render_widget(paragraph, bar_area);
    }

    /// Render filter column/text input modal as an overlay.
    fn render_filter_modal(&self, frame: &mut Frame, area: Rect) {
        let result = match &self.query_result {
            Some(r) => r,
            None => return,
        };

        if self.filter_modal_step == 0 {
            // Step 0: Column selection (similar to sort modal)
            self.render_filter_column_selection(frame, area, result);
        } else {
            // Step 1: Text input
            self.render_filter_text_input(frame, area, result);
        }
    }

    /// Render filter column selection modal (step 0).
    fn render_filter_column_selection(
        &self,
        frame: &mut Frame,
        area: Rect,
        result: &QueryResultData,
    ) {
        // Calculate modal dimensions (40% width, centered)
        let modal_width = (area.width as f32 * 0.4).clamp(30.0, 60.0) as u16;
        let max_height = (result.columns.len() + 4).min(20) as u16;
        let modal_height = max_height.min(area.height.saturating_sub(4));

        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(
            area.x + modal_x,
            area.y + modal_y,
            modal_width,
            modal_height,
        );

        // Clear background for better visibility
        frame.render_widget(Clear, modal_area);

        // Build column list items
        let items: Vec<Line> = result
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let is_selected = i == self.filter_modal_selection;
                let is_filtered = self.filter_column_index == Some(i);

                // Build column text with filter indicator
                let filter_indicator = if is_filtered { " ⚡" } else { "" };
                let prefix = if is_selected { "► " } else { "  " };
                let text = format!("{}{}{}", prefix, col, filter_indicator);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Magenta)
                        .add_modifier(Modifier::BOLD)
                } else if is_filtered {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default().fg(Color::White)
                };

                Line::from(Span::styled(text, style))
            })
            .collect();

        let paragraph = Paragraph::new(items)
            .block(
                Block::default()
                    .title(" Filter by Column ")
                    .title_style(
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .style(Style::default().bg(Color::Black));

        frame.render_widget(paragraph, modal_area);

        // Render footer hint
        if modal_area.height > 2 {
            let footer_area = Rect::new(
                modal_area.x + 1,
                modal_area.y + modal_area.height.saturating_sub(1),
                modal_area.width.saturating_sub(2),
                1,
            );
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(": Next  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(": Cancel", Style::default().fg(Color::DarkGray)),
            ]))
            .style(Style::default().bg(Color::Black));
            frame.render_widget(hint, footer_area);
        }
    }

    /// Render filter text input modal (step 1).
    fn render_filter_text_input(&self, frame: &mut Frame, area: Rect, result: &QueryResultData) {
        // Calculate modal dimensions
        let modal_width = (area.width as f32 * 0.5).clamp(40.0, 70.0) as u16;
        let modal_height = 7_u16; // Fixed height for text input

        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(
            area.x + modal_x,
            area.y + modal_y,
            modal_width,
            modal_height,
        );

        // Clear background for better visibility
        frame.render_widget(Clear, modal_area);

        // Get column name for title
        let col_name = result
            .columns
            .get(self.filter_modal_selection)
            .map(|s| s.as_str())
            .unwrap_or("Column");

        let title = format!(" Filter: {} ", col_name);

        // Build input display with cursor
        let input_display = format!("{}_", self.filter_text);

        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Enter filter text (case-insensitive):",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}", input_display),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
        ];

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(title)
                    .title_style(
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta)),
            )
            .style(Style::default().bg(Color::Black));

        frame.render_widget(paragraph, modal_area);

        // Render footer hint
        if modal_area.height > 2 {
            let footer_area = Rect::new(
                modal_area.x + 1,
                modal_area.y + modal_area.height.saturating_sub(1),
                modal_area.width.saturating_sub(2),
                1,
            );
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(": Apply  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(": Back  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Backspace", Style::default().fg(Color::Yellow)),
                Span::styled(": Delete", Style::default().fg(Color::DarkGray)),
            ]))
            .style(Style::default().bg(Color::Black));
            frame.render_widget(hint, footer_area);
        }
    }

    /// Render sort column selection modal as an overlay.
    fn render_sort_modal(&self, frame: &mut Frame, area: Rect) {
        let result = match &self.query_result {
            Some(r) => r,
            None => return,
        };

        // Calculate modal dimensions (40% width, centered, max height for columns + header/footer)
        let modal_width = (area.width as f32 * 0.4).clamp(30.0, 60.0) as u16;
        let max_height = (result.columns.len() + 4).min(20) as u16; // +4 for borders and title/footer
        let modal_height = max_height.min(area.height.saturating_sub(4));

        let modal_x = (area.width.saturating_sub(modal_width)) / 2;
        let modal_y = (area.height.saturating_sub(modal_height)) / 2;

        let modal_area = Rect::new(
            area.x + modal_x,
            area.y + modal_y,
            modal_width,
            modal_height,
        );

        // Clear background for better visibility
        frame.render_widget(Clear, modal_area);

        // Build column list items
        let items: Vec<Line> = result
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let is_selected = i == self.sort_modal_selection;
                let is_sorted = self.sort_column_index == Some(i);

                // Build column text with sort indicator
                let sort_indicator = if is_sorted {
                    match self.sort_order {
                        SortOrder::Ascending => " ↑",
                        SortOrder::Descending => " ↓",
                        SortOrder::None => "",
                    }
                } else {
                    ""
                };

                let prefix = if is_selected { "► " } else { "  " };
                let text = format!("{}{}{}", prefix, col, sort_indicator);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if is_sorted {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };

                Line::from(Span::styled(text, style))
            })
            .collect();

        let paragraph = Paragraph::new(items)
            .block(
                Block::default()
                    .title(" Sort by Column ")
                    .title_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .style(Style::default().bg(Color::Black));

        frame.render_widget(paragraph, modal_area);

        // Render footer hint
        if modal_area.height > 2 {
            let footer_area = Rect::new(
                modal_area.x + 1,
                modal_area.y + modal_area.height.saturating_sub(1),
                modal_area.width.saturating_sub(2),
                1,
            );
            let hint = Paragraph::new(Line::from(vec![
                Span::styled("Enter", Style::default().fg(Color::Yellow)),
                Span::styled(": Select  ", Style::default().fg(Color::DarkGray)),
                Span::styled("Esc", Style::default().fg(Color::Yellow)),
                Span::styled(": Cancel", Style::default().fg(Color::DarkGray)),
            ]))
            .style(Style::default().bg(Color::Black));
            frame.render_widget(hint, footer_area);
        }
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
            super::state_renderer::render_empty(
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
    fn render_databases(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
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
            super::state_renderer::render_empty(
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
    fn render_collection_questions(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
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
            super::state_renderer::render_empty_with_hint(
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
    fn render_database_schemas(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
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
            super::state_renderer::render_empty_with_hint(
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
    fn render_schema_tables(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
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
            super::state_renderer::render_empty_with_hint(
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
    fn render_table_preview(&mut self, area: Rect, frame: &mut Frame, focused: bool) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Get table name from ContentView variant
        let table_name = match &self.view {
            ContentView::TablePreview { table_name, .. } => table_name.as_str(),
            _ => "Unknown",
        };

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

                    // Create table rows with sliced cells (only current page)
                    // Uses sort indices if sorting is active, otherwise original order
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
                            // Calculate the actual column index (accounting for scroll)
                            let actual_col_idx = scroll_x + visible_idx;
                            let is_sorted = self.sort_column_index == Some(actual_col_idx);

                            // Add sort indicator if this column is sorted
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
                                    " {} - Preview{}{}{}",
                                    table_name, page_indicator, col_indicator, sort_indicator
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

                    // Render sort modal overlay if active
                    if self.sort_mode_active {
                        self.render_sort_modal(frame, area);
                    }
                    // Render filter modal overlay if active
                    if self.filter_mode_active {
                        self.render_filter_modal(frame, area);
                    }
                    // Render result search bar overlay if active
                    if self.result_search_active {
                        self.render_result_search_bar(frame, area);
                    }
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
                    // Uses sort indices if sorting is active, otherwise original order
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
                            // Calculate the actual column index (accounting for scroll)
                            let actual_col_idx = scroll_x + visible_idx;
                            let is_sorted = self.sort_column_index == Some(actual_col_idx);

                            // Add sort indicator if this column is sorted
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
                                    " {}{}{}{}",
                                    result.question_name,
                                    page_indicator,
                                    col_indicator,
                                    sort_indicator
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

                    // Render sort modal overlay if active
                    if self.sort_mode_active {
                        self.render_sort_modal(frame, area);
                    }
                    // Render filter modal overlay if active
                    if self.filter_mode_active {
                        self.render_filter_modal(frame, area);
                    }
                    // Render result search bar overlay if active
                    if self.result_search_active {
                        self.render_result_search_bar(frame, area);
                    }
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
            ContentView::CollectionQuestions { .. } => {
                self.render_collection_questions(area, frame, focused);
                return;
            }
            ContentView::DatabaseSchemas { .. } => {
                self.render_database_schemas(area, frame, focused);
                return;
            }
            ContentView::SchemaTables { .. } => {
                self.render_schema_tables(area, frame, focused);
                return;
            }
            ContentView::TablePreview { .. } => {
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
        } else if self.is_collection_questions_view() {
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
            // Filter modal takes priority when active
            if self.filter_mode_active {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.filter_modal_up();
                        true
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.filter_modal_down();
                        true
                    }
                    KeyCode::Enter => {
                        if self.filter_modal_step == 0 {
                            // Column selection → text input
                            self.filter_modal_next_step();
                        } else {
                            // Text input → apply filter
                            self.apply_filter();
                        }
                        true
                    }
                    KeyCode::Esc => {
                        if self.filter_modal_step == 1 {
                            // Text input → column selection
                            self.filter_modal_prev_step();
                        } else {
                            // Column selection → close modal
                            self.close_filter_modal();
                        }
                        true
                    }
                    KeyCode::Char('f') if self.filter_modal_step == 0 => {
                        // Close modal if 'f' pressed in column selection
                        self.close_filter_modal();
                        true
                    }
                    KeyCode::Backspace => {
                        self.filter_modal_delete_char();
                        true
                    }
                    KeyCode::Char(c) if self.filter_modal_step == 1 => {
                        self.filter_modal_input_char(c);
                        true
                    }
                    _ => false,
                }
            } else if self.sort_mode_active {
                // Sort modal takes priority when active
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.sort_modal_up();
                        true
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.sort_modal_down();
                        true
                    }
                    KeyCode::Enter => {
                        self.apply_sort();
                        true
                    }
                    KeyCode::Esc | KeyCode::Char('s') => {
                        self.close_sort_modal();
                        true
                    }
                    _ => false,
                }
            } else if self.result_search_active {
                // Result search mode takes priority
                match key.code {
                    KeyCode::Char(c) => {
                        self.result_search_input_char(c);
                        true
                    }
                    KeyCode::Backspace => {
                        self.result_search_delete_char();
                        true
                    }
                    KeyCode::Enter => {
                        self.apply_result_search();
                        true
                    }
                    KeyCode::Esc => {
                        self.close_result_search();
                        true
                    }
                    _ => false,
                }
            } else {
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
                    // Sort: s to open sort modal
                    KeyCode::Char('s') => {
                        self.open_sort_modal();
                        true
                    }
                    // Filter: f to open filter modal
                    KeyCode::Char('f') => {
                        self.open_filter_modal();
                        true
                    }
                    // Clear filter: F (shift+f) to clear filter
                    KeyCode::Char('F') => {
                        self.clear_filter();
                        // Reset to first page after clearing filter
                        self.result_page = 0;
                        self.result_table_state.select(Some(0));
                        true
                    }
                    // Search: / to open search input
                    KeyCode::Char('/') => {
                        self.open_result_search();
                        true
                    }
                    // Clear search: Shift+S to clear search
                    KeyCode::Char('S') => {
                        self.clear_result_search();
                        // Reset to first page after clearing search
                        self.result_page = 0;
                        self.result_table_state.select(Some(0));
                        true
                    }
                    // Note: Esc is handled in App for returning to Questions
                    _ => false,
                }
            }
        } else if self.is_database_schemas_view() {
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
        } else if self.is_schema_tables_view() {
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
        } else if self.is_table_preview_view() {
            // Filter modal takes priority when active
            if self.filter_mode_active {
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.filter_modal_up();
                        true
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.filter_modal_down();
                        true
                    }
                    KeyCode::Enter => {
                        if self.filter_modal_step == 0 {
                            self.filter_modal_next_step();
                        } else {
                            self.apply_filter();
                        }
                        true
                    }
                    KeyCode::Esc => {
                        if self.filter_modal_step == 1 {
                            self.filter_modal_prev_step();
                        } else {
                            self.close_filter_modal();
                        }
                        true
                    }
                    KeyCode::Char('f') if self.filter_modal_step == 0 => {
                        self.close_filter_modal();
                        true
                    }
                    KeyCode::Backspace => {
                        self.filter_modal_delete_char();
                        true
                    }
                    KeyCode::Char(c) if self.filter_modal_step == 1 => {
                        self.filter_modal_input_char(c);
                        true
                    }
                    _ => false,
                }
            } else if self.sort_mode_active {
                // Sort modal takes priority when active
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.sort_modal_up();
                        true
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.sort_modal_down();
                        true
                    }
                    KeyCode::Enter => {
                        self.apply_sort();
                        true
                    }
                    KeyCode::Esc | KeyCode::Char('s') => {
                        self.close_sort_modal();
                        true
                    }
                    _ => false,
                }
            } else if self.result_search_active {
                // Result search mode takes priority
                match key.code {
                    KeyCode::Char(c) => {
                        self.result_search_input_char(c);
                        true
                    }
                    KeyCode::Backspace => {
                        self.result_search_delete_char();
                        true
                    }
                    KeyCode::Enter => {
                        self.apply_result_search();
                        true
                    }
                    KeyCode::Esc => {
                        self.close_result_search();
                        true
                    }
                    _ => false,
                }
            } else {
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
                    // Sort: s to open sort modal
                    KeyCode::Char('s') => {
                        self.open_sort_modal();
                        true
                    }
                    // Filter: f to open filter modal
                    KeyCode::Char('f') => {
                        self.open_filter_modal();
                        true
                    }
                    // Clear filter: F (shift+f) to clear filter
                    KeyCode::Char('F') => {
                        self.clear_filter();
                        self.result_page = 0;
                        self.result_table_state.select(Some(0));
                        true
                    }
                    // Search: / to open search input
                    KeyCode::Char('/') => {
                        self.open_result_search();
                        true
                    }
                    // Clear search: Shift+S to clear search
                    KeyCode::Char('S') => {
                        self.clear_result_search();
                        self.result_page = 0;
                        self.result_table_state.select(Some(0));
                        true
                    }
                    // Note: Esc is handled in App for returning to SchemaTables
                    _ => false,
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    // === Navigation Stack Tests ===

    #[test]
    fn test_push_pop_view() {
        let mut panel = ContentPanel::new();
        assert_eq!(panel.current_view(), ContentView::Welcome);
        assert_eq!(panel.navigation_depth(), 0);

        // Push to Questions view
        panel.push_view(ContentView::Questions);
        assert_eq!(panel.current_view(), ContentView::Questions);
        assert_eq!(panel.navigation_depth(), 1);

        // Push to QueryResult view
        panel.push_view(ContentView::QueryResult);
        assert_eq!(panel.current_view(), ContentView::QueryResult);
        assert_eq!(panel.navigation_depth(), 2);

        // Pop back to Questions
        let popped = panel.pop_view();
        assert_eq!(popped, Some(ContentView::Questions));
        assert_eq!(panel.current_view(), ContentView::Questions);
        assert_eq!(panel.navigation_depth(), 1);

        // Pop back to Welcome
        let popped = panel.pop_view();
        assert_eq!(popped, Some(ContentView::Welcome));
        assert_eq!(panel.current_view(), ContentView::Welcome);
        assert_eq!(panel.navigation_depth(), 0);

        // Pop on empty stack returns None
        let popped = panel.pop_view();
        assert_eq!(popped, None);
        assert_eq!(panel.current_view(), ContentView::Welcome);
    }

    #[test]
    fn test_navigation_stack_four_level_drill_down() {
        let mut panel = ContentPanel::new();

        // Simulate: Databases → Schemas → Tables → Preview (4 levels)
        panel.set_view(ContentView::Databases);
        panel.push_view(ContentView::DatabaseSchemas {
            db_id: 1,
            db_name: "TestDB".to_string(),
        });
        panel.push_view(ContentView::SchemaTables {
            db_id: 1,
            schema_name: "public".to_string(),
        });
        panel.push_view(ContentView::TablePreview {
            db_id: 1,
            table_id: 10,
            table_name: "users".to_string(),
        });

        assert!(panel.is_table_preview_view());
        assert_eq!(panel.navigation_depth(), 3);

        // Go back step by step
        panel.pop_view();
        assert!(panel.is_schema_tables_view());

        panel.pop_view();
        assert!(panel.is_database_schemas_view());

        panel.pop_view();
        assert_eq!(panel.current_view(), ContentView::Databases);
    }

    #[test]
    fn test_clear_navigation_stack() {
        let mut panel = ContentPanel::new();

        // Build up a navigation stack
        panel.push_view(ContentView::Questions);
        panel.push_view(ContentView::QueryResult);
        assert_eq!(panel.navigation_depth(), 2);

        // Clear stack
        panel.clear_navigation_stack();
        assert_eq!(panel.navigation_depth(), 0);
    }

    #[test]
    fn test_set_view_clears_navigation_stack() {
        let mut panel = ContentPanel::new();

        // Build up navigation
        panel.push_view(ContentView::Questions);
        panel.push_view(ContentView::QueryResult);
        assert_eq!(panel.navigation_depth(), 2);

        // Tab switch should clear stack
        panel.set_view(ContentView::Collections);
        assert_eq!(panel.current_view(), ContentView::Collections);
        assert_eq!(panel.navigation_depth(), 0);
    }

    // === Search Mode Tests ===

    #[test]
    fn test_search_mode_toggle() {
        let mut panel = ContentPanel::new();
        assert_eq!(panel.input_mode(), InputMode::Normal);

        // Enter search mode
        panel.enter_search_mode();
        assert_eq!(panel.input_mode(), InputMode::Search);

        // Exit search mode
        panel.exit_search_mode();
        assert_eq!(panel.input_mode(), InputMode::Normal);
    }

    #[test]
    fn test_search_query_input() {
        let mut panel = ContentPanel::new();
        panel.enter_search_mode();

        // Type characters
        panel.handle_search_input('h');
        panel.handle_search_input('e');
        panel.handle_search_input('l');
        panel.handle_search_input('l');
        panel.handle_search_input('o');

        assert_eq!(panel.get_search_query(), "hello");

        // Backspace
        panel.handle_search_backspace();
        assert_eq!(panel.get_search_query(), "hell");
    }

    #[test]
    fn test_execute_search() {
        let mut panel = ContentPanel::new();
        panel.enter_search_mode();

        panel.handle_search_input('t');
        panel.handle_search_input('e');
        panel.handle_search_input('s');
        panel.handle_search_input('t');

        // Execute search
        let result = panel.execute_search();
        assert_eq!(result, Some("test".to_string()));
        assert_eq!(panel.input_mode(), InputMode::Normal);
        assert_eq!(panel.get_active_search(), Some("test"));
    }

    #[test]
    fn test_execute_empty_search_returns_none() {
        let mut panel = ContentPanel::new();
        panel.enter_search_mode();

        // Execute without typing anything
        let result = panel.execute_search();
        assert_eq!(result, None);
    }

    #[test]
    fn test_clear_search() {
        let mut panel = ContentPanel::new();
        panel.enter_search_mode();
        panel.handle_search_input('t');
        panel.handle_search_input('e');
        panel.handle_search_input('s');
        panel.handle_search_input('t');
        panel.execute_search();

        assert_eq!(panel.get_active_search(), Some("test"));

        panel.clear_search();
        assert_eq!(panel.get_active_search(), None);
        assert_eq!(panel.get_search_query(), "");
    }

    // === Modal State Tests ===

    #[test]
    fn test_sort_mode_toggle() {
        let mut panel = ContentPanel::new();
        assert!(!panel.is_sort_mode_active());

        // Sort modal requires query_result with columns
        let data = QueryResultData {
            question_id: 1,
            question_name: "Test".to_string(),
            columns: vec!["Col1".to_string(), "Col2".to_string()],
            rows: vec![vec!["a".to_string(), "b".to_string()]],
        };
        panel.set_query_result(data);

        panel.open_sort_modal();
        assert!(panel.is_sort_mode_active());

        panel.close_sort_modal();
        assert!(!panel.is_sort_mode_active());
    }

    #[test]
    fn test_filter_mode_toggle() {
        let mut panel = ContentPanel::new();
        assert!(!panel.is_filter_mode_active());

        // Filter modal requires query_result with columns
        let data = QueryResultData {
            question_id: 1,
            question_name: "Test".to_string(),
            columns: vec!["Col1".to_string(), "Col2".to_string()],
            rows: vec![vec!["a".to_string(), "b".to_string()]],
        };
        panel.set_query_result(data);

        panel.open_filter_modal();
        assert!(panel.is_filter_mode_active());

        panel.close_filter_modal();
        assert!(!panel.is_filter_mode_active());
    }

    #[test]
    fn test_result_search_toggle() {
        let mut panel = ContentPanel::new();
        assert!(!panel.is_result_search_active());

        panel.open_result_search();
        assert!(panel.is_result_search_active());

        panel.close_result_search();
        assert!(!panel.is_result_search_active());
    }

    // === View Transition Tests ===

    #[test]
    fn test_content_view_default() {
        let panel = ContentPanel::new();
        assert_eq!(panel.current_view(), ContentView::Welcome);
    }

    #[test]
    fn test_set_view_changes_current_view() {
        let mut panel = ContentPanel::new();

        panel.set_view(ContentView::Questions);
        assert_eq!(panel.current_view(), ContentView::Questions);

        panel.set_view(ContentView::Collections);
        assert_eq!(panel.current_view(), ContentView::Collections);

        panel.set_view(ContentView::Databases);
        assert_eq!(panel.current_view(), ContentView::Databases);
    }

    // === Selection Navigation Tests ===

    #[test]
    fn test_select_navigation_with_loaded_questions() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Questions);

        // Load some questions
        let questions = vec![
            Question {
                id: 1,
                name: "Q1".to_string(),
                description: None,
                collection_id: None,
                collection: None,
            },
            Question {
                id: 2,
                name: "Q2".to_string(),
                description: None,
                collection_id: None,
                collection: None,
            },
            Question {
                id: 3,
                name: "Q3".to_string(),
                description: None,
                collection_id: None,
                collection: None,
            },
        ];
        panel.update_questions(&LoadState::Loaded(questions));

        // First item should be auto-selected
        assert_eq!(panel.table_state.selected(), Some(0));

        // Navigate down
        panel.select_next();
        assert_eq!(panel.table_state.selected(), Some(1));

        panel.select_next();
        assert_eq!(panel.table_state.selected(), Some(2));

        // Should not go beyond last item
        panel.select_next();
        assert_eq!(panel.table_state.selected(), Some(2));

        // Navigate up
        panel.select_previous();
        assert_eq!(panel.table_state.selected(), Some(1));

        // Jump to first
        panel.select_first();
        assert_eq!(panel.table_state.selected(), Some(0));

        // Jump to last
        panel.select_last();
        assert_eq!(panel.table_state.selected(), Some(2));
    }

    #[test]
    fn test_select_navigation_with_empty_list() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Questions);

        // Load empty list
        panel.update_questions(&LoadState::Loaded(vec![]));

        // Navigation should not panic
        panel.select_next();
        panel.select_previous();
        panel.select_first();
        panel.select_last();
    }

    // === Collection Questions Context Tests ===

    #[test]
    fn test_enter_collection_questions_sets_context() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Collections);

        panel.enter_collection_questions(42, "Test Collection".to_string());

        assert!(panel.is_collection_questions_view());
        assert_eq!(
            panel.get_collection_context(),
            Some((42_u32, "Test Collection".to_string()))
        );
    }

    #[test]
    fn test_exit_collection_questions_clears_context() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Collections);
        panel.enter_collection_questions(42, "Test Collection".to_string());

        panel.exit_collection_questions();

        assert_eq!(panel.current_view(), ContentView::Collections);
        assert_eq!(panel.get_collection_context(), None);
    }

    // === Database Drill-down Context Tests ===

    #[test]
    fn test_database_schema_drill_down() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Databases);

        // Enter database schemas
        panel.enter_database_schemas(1, "TestDB".to_string());
        assert!(panel.is_database_schemas_view());
        assert_eq!(
            panel.get_database_context(),
            Some((1_u32, "TestDB".to_string()))
        );

        // Exit back to databases
        panel.exit_database_schemas();
        assert_eq!(panel.current_view(), ContentView::Databases);
        assert_eq!(panel.get_database_context(), None);
    }

    #[test]
    fn test_schema_tables_drill_down() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Databases);
        panel.enter_database_schemas(1, "TestDB".to_string());

        // Enter schema tables
        panel.enter_schema_tables(1, "public".to_string());
        assert!(panel.is_schema_tables_view());
        assert_eq!(
            panel.get_schema_context(),
            Some((1_u32, "public".to_string()))
        );

        // Exit back to schemas
        panel.exit_schema_tables();
        assert!(panel.is_database_schemas_view());
        assert_eq!(panel.get_schema_context(), None);
    }

    // === Query Result Tests ===

    #[test]
    fn test_set_query_result() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Questions);

        let result_data = QueryResultData {
            question_id: 1,
            question_name: "Test Query".to_string(),
            columns: vec!["ID".to_string(), "Name".to_string()],
            rows: vec![
                vec!["1".to_string(), "Alice".to_string()],
                vec!["2".to_string(), "Bob".to_string()],
            ],
        };

        panel.set_query_result(result_data.clone());

        assert_eq!(panel.current_view(), ContentView::QueryResult);
        assert!(panel.query_result.is_some());
    }

    #[test]
    fn test_back_to_questions_clears_result() {
        let mut panel = ContentPanel::new();
        panel.set_view(ContentView::Questions);

        let result_data = QueryResultData {
            question_id: 1,
            question_name: "Test Query".to_string(),
            columns: vec!["ID".to_string()],
            rows: vec![vec!["1".to_string()]],
        };

        panel.set_query_result(result_data);
        assert!(panel.query_result.is_some());

        panel.back_to_questions();
        assert!(panel.query_result.is_none());
        assert_eq!(panel.current_view(), ContentView::Questions);
    }

    // === Load State Tests ===

    #[test]
    fn test_update_questions_auto_selects_first() {
        let mut panel = ContentPanel::new();

        let questions = vec![Question {
            id: 1,
            name: "Q1".to_string(),
            description: None,
            collection_id: None,
            collection: None,
        }];

        panel.update_questions(&LoadState::Loaded(questions));
        assert_eq!(panel.table_state.selected(), Some(0));
    }

    #[test]
    fn test_update_collections_auto_selects_first() {
        let mut panel = ContentPanel::new();

        let collections = vec![CollectionItem {
            id: Some(1),
            name: "C1".to_string(),
            description: None,
            location: None,
            personal_owner_id: None,
            archived: false,
        }];

        panel.update_collections(&LoadState::Loaded(collections));
        assert_eq!(panel.collections_table_state.selected(), Some(0));
    }

    #[test]
    fn test_update_databases_auto_selects_first() {
        let mut panel = ContentPanel::new();

        let databases = vec![Database {
            id: 1,
            name: "DB1".to_string(),
            engine: Some("postgres".to_string()),
            description: None,
            is_sample: false,
            is_saved_questions: false,
        }];

        panel.update_databases(&LoadState::Loaded(databases));
        assert_eq!(panel.databases_table_state.selected(), Some(0));
    }
}
