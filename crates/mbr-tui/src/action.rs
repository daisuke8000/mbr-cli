//! Application actions for event-driven architecture.
//!
//! Implements Flux-like unidirectional data flow:
//! Component → AppAction → App State → Component Re-render

// Allow unused variants as they are designed for Phase 4+ implementation
#![allow(dead_code)]

use mbr_core::api::models::{CollectionItem, CurrentUser, Database, Question, TableInfo};

use crate::components::QueryResultData;

/// Application-level actions for component-to-app communication.
///
/// This enum enables clean separation between UI components and application logic.
/// Components emit actions, the App processes them and updates state.
#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    /// Quit the application
    Quit,

    /// Change focus to the next panel
    NextPanel,

    /// Change focus to the previous panel
    PreviousPanel,

    /// Navigate to a specific content view
    Navigate(ContentTarget),

    /// Load data from API
    LoadData(DataRequest),

    /// Show an error message
    ShowError(String),

    /// Clear current error message
    ClearError,

    /// Update status message
    SetStatus(String),

    /// Clear status message
    ClearStatus,

    // === Completion Notifications (Phase 4) ===
    /// Questions loaded successfully from API
    QuestionsLoaded(Vec<Question>),

    /// Collections loaded successfully from API
    CollectionsLoaded(Vec<CollectionItem>),

    /// Databases loaded successfully from API
    DatabasesLoaded(Vec<Database>),

    /// Authentication validated successfully
    AuthValidated(CurrentUser),

    /// Data loading failed with context and error message
    LoadFailed(DataRequest, String),

    // === Query Execution (Phase 6) ===
    /// Execute a question query
    ExecuteQuestion(u32),

    /// Query execution completed successfully (request_id, data)
    QueryResultLoaded(u64, QueryResultData),

    /// Query execution failed (request_id, error)
    QueryFailed(u64, String),

    /// Return to Questions list from query result view
    BackToQuestions,

    // === Collection Drill-down (Phase 3) ===
    /// Drill down into a collection to view its questions
    DrillDownCollection(u32, String), // (collection_id, collection_name)

    /// Return to Collections list from collection questions view
    BackToCollections,

    // === Database Drill-down (Phase 3) ===
    /// Drill down into a database to view its schemas
    DrillDownDatabase(u32, String), // (database_id, database_name)

    /// Drill down into a schema to view its tables
    DrillDownSchema(String), // schema_name (database_id from context)

    /// Drill down into a table to preview its data
    DrillDownTable(u32, String), // (table_id, table_name)

    /// Return to Databases list from schemas view
    BackToDatabases,

    /// Return to schemas list from tables view
    BackToSchemas,

    /// Return to tables list from table preview view
    BackToTables,

    /// Schemas loaded successfully from API
    SchemasLoaded(Vec<String>),

    /// Tables loaded successfully from API
    TablesLoaded(Vec<TableInfo>),

    /// Table preview data loaded successfully
    TablePreviewLoaded(QueryResultData),
}

/// Target content views for navigation
#[derive(Debug, Clone, PartialEq)]
pub enum ContentTarget {
    /// Welcome/home screen
    Welcome,
    /// Questions list view
    Questions,
    /// Collections list view
    Collections,
    /// Databases list view
    Databases,
}

/// Data loading requests
#[derive(Debug, Clone, PartialEq)]
pub enum DataRequest {
    /// Load questions list
    Questions,
    /// Search questions by query string
    SearchQuestions(String),
    /// Load questions filtered by collection ID
    FilterQuestionsByCollection(u32),
    /// Load collections list
    Collections,
    /// Load databases list
    Databases,
    /// Load a specific question's details
    QuestionDetails(u32),
    /// Execute a specific question's query
    Execute(u32),
    /// Refresh current data
    Refresh,
    /// Load schemas for a database
    Schemas(u32), // database_id
    /// Load tables for a schema
    Tables(u32, String), // (database_id, schema_name)
    /// Preview table data
    TablePreview(u32, u32), // (database_id, table_id)
}

impl From<usize> for ContentTarget {
    fn from(index: usize) -> Self {
        match index {
            0 => ContentTarget::Questions,
            1 => ContentTarget::Collections,
            2 => ContentTarget::Databases,
            _ => ContentTarget::Welcome,
        }
    }
}
