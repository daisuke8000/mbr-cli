//! Navigation stack and view transition methods for ContentPanel.
//!
//! Handles multi-level drill-down navigation (Databases → Schemas → Tables → Preview)
//! and view transitions between different content types.

use ratatui::widgets::TableState;

use super::ContentPanel;
use super::types::{ContentView, QueryResultData};
use crate::service::LoadState;

impl ContentPanel {
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

    // === Query Result Navigation ===

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
}
