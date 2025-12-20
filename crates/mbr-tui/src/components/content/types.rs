//! Type definitions for the content panel.
//!
//! Contains view types, input modes, sort orders, and query result data structures.

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
pub const DEFAULT_ROWS_PER_PAGE: usize = 100;
