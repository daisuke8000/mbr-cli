//! Selection navigation methods for ContentPanel.
//!
//! Provides select_next, select_previous, select_first, select_last methods
//! for each view type (Questions, Collections, Databases, Schemas, Tables, Results).

use mbr_core::api::models::{CollectionItem, Database, Question, TableInfo};

use super::ContentPanel;
use super::types::ContentView;
use crate::service::LoadState;

impl ContentPanel {
    // === Questions view navigation ===

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
    pub(super) fn select_collections_next(&mut self) {
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
    pub(super) fn select_collections_previous(&mut self) {
        let current = self.collections_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.collections_table_state.select(Some(prev));
    }

    /// Select first collection in list.
    pub(super) fn select_collections_first(&mut self) {
        self.collections_table_state.select(Some(0));
    }

    /// Select last collection in list.
    pub(super) fn select_collections_last(&mut self) {
        if let LoadState::Loaded(collections) = &self.collections {
            if !collections.is_empty() {
                self.collections_table_state
                    .select(Some(collections.len() - 1));
            }
        }
    }

    // === Databases view navigation ===

    /// Select next database in list.
    pub(super) fn select_databases_next(&mut self) {
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
    pub(super) fn select_databases_previous(&mut self) {
        let current = self.databases_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.databases_table_state.select(Some(prev));
    }

    /// Select first database in list.
    pub(super) fn select_databases_first(&mut self) {
        self.databases_table_state.select(Some(0));
    }

    /// Select last database in list.
    pub(super) fn select_databases_last(&mut self) {
        if let LoadState::Loaded(databases) = &self.databases {
            if !databases.is_empty() {
                self.databases_table_state.select(Some(databases.len() - 1));
            }
        }
    }

    // === Schemas view navigation ===

    /// Select next schema in list.
    pub(super) fn select_schemas_next(&mut self) {
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
    pub(super) fn select_schemas_previous(&mut self) {
        let current = self.schemas_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.schemas_table_state.select(Some(prev));
    }

    /// Select first schema in list.
    pub(super) fn select_schemas_first(&mut self) {
        self.schemas_table_state.select(Some(0));
    }

    /// Select last schema in list.
    pub(super) fn select_schemas_last(&mut self) {
        if let LoadState::Loaded(schemas) = &self.schemas {
            if !schemas.is_empty() {
                self.schemas_table_state.select(Some(schemas.len() - 1));
            }
        }
    }

    // === Tables view navigation ===

    /// Select next table in list.
    pub(super) fn select_tables_next(&mut self) {
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
    pub(super) fn select_tables_previous(&mut self) {
        let current = self.tables_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.tables_table_state.select(Some(prev));
    }

    /// Select first table in list.
    pub(super) fn select_tables_first(&mut self) {
        self.tables_table_state.select(Some(0));
    }

    /// Select last table in list.
    pub(super) fn select_tables_last(&mut self) {
        if let LoadState::Loaded(tables) = &self.tables {
            if !tables.is_empty() {
                self.tables_table_state.select(Some(tables.len() - 1));
            }
        }
    }

    // === Result table navigation ===

    /// Navigate result table - select next row.
    pub(super) fn select_result_next(&mut self) {
        if let Some(ref result) = self.query_result {
            if result.rows.is_empty() {
                return;
            }
            let current = self.result_table_state.selected().unwrap_or(0);
            let next = (current + 1).min(result.rows.len() - 1);
            self.result_table_state.select(Some(next));
        }
    }

    /// Navigate result table - select previous row.
    pub(super) fn select_result_previous(&mut self) {
        let current = self.result_table_state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.result_table_state.select(Some(prev));
    }

    // === Data update methods ===

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

    // === Selected item getters ===

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
}
