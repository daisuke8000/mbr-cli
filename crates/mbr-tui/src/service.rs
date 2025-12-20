//! Service integration layer for mbr-core.
//!
//! Provides async data fetching using mbr-core's QuestionService and MetabaseClient.

use std::sync::Arc;

use mbr_core::api::client::MetabaseClient;
use mbr_core::api::models::{
    CollectionItem, CurrentUser, Database, QueryResult, Question, TableInfo,
};
use mbr_core::core::services::question_service::QuestionService;
use mbr_core::core::services::types::ListParams;
use mbr_core::storage::config::Config;
use mbr_core::storage::credentials::get_api_key;

use crate::components::QueryResultData;

/// Generic loading state for async data.
///
/// This enum enforces proper handling of all loading states at compile time,
/// preventing bugs like displaying stale data while loading.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum LoadState<T> {
    /// Initial state, no data loaded yet
    #[default]
    Idle,
    /// Data is being fetched
    Loading,
    /// Data successfully loaded
    Loaded(T),
    /// Loading failed with error message
    Error(String),
}

// Methods designed for Phase 5+ implementation
#[allow(dead_code)]
impl<T> LoadState<T> {
    /// Check if currently loading
    pub fn is_loading(&self) -> bool {
        matches!(self, LoadState::Loading)
    }

    /// Check if data is loaded
    pub fn is_loaded(&self) -> bool {
        matches!(self, LoadState::Loaded(_))
    }

    /// Check if in error state
    pub fn is_error(&self) -> bool {
        matches!(self, LoadState::Error(_))
    }

    /// Get reference to loaded data if available
    pub fn data(&self) -> Option<&T> {
        match self {
            LoadState::Loaded(data) => Some(data),
            _ => None,
        }
    }

    /// Get error message if in error state
    pub fn error(&self) -> Option<&str> {
        match self {
            LoadState::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Application data state using LoadState for each resource.
#[derive(Debug, Clone, Default)]
pub struct AppData {
    /// Questions list with loading state
    pub questions: LoadState<Vec<Question>>,
    /// Collections list with loading state
    pub collections: LoadState<Vec<CollectionItem>>,
    /// Databases list with loading state
    pub databases: LoadState<Vec<Database>>,
    /// Schemas list with loading state (for database drill-down)
    pub schemas: LoadState<Vec<String>>,
    /// Tables list with loading state (for schema drill-down)
    pub tables: LoadState<Vec<TableInfo>>,
    /// Current user information (if authenticated)
    pub current_user: Option<CurrentUser>,
    /// Query result data (centralized storage)
    pub query_result: Option<QueryResultData>,
}

/// Service client wrapper for async operations.
///
/// Designed to be wrapped in Arc for sharing across tokio tasks.
#[derive(Clone)]
pub struct ServiceClient {
    client: MetabaseClient,
    #[allow(dead_code)] // Designed for future features
    base_url: String,
}

impl ServiceClient {
    /// Create a new service client from configuration
    pub fn new(base_url: String, api_key: Option<String>) -> Result<Self, String> {
        let client = if let Some(key) = api_key {
            MetabaseClient::with_api_key(base_url.clone(), key)
                .map_err(|e| format!("Failed to create client: {}", e))?
        } else {
            MetabaseClient::new(base_url.clone())
                .map_err(|e| format!("Failed to create client: {}", e))?
        };

        Ok(Self { client, base_url })
    }

    /// Check if the client is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.client.is_authenticated()
    }

    /// Get the base URL
    #[allow(dead_code)] // Designed for future features
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Validate API key by fetching current user
    pub async fn validate_auth(&self) -> Result<CurrentUser, String> {
        self.client
            .get_current_user()
            .await
            .map_err(|e| format!("Authentication failed: {}", e))
    }

    /// Fetch questions list
    pub async fn fetch_questions(
        &self,
        search: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Question>, String> {
        let service = QuestionService::new(self.client.clone());
        let params = ListParams {
            search: search.map(String::from),
            limit,
            collection: None,
            offset: None,
        };

        service
            .list_questions(params)
            .await
            .map_err(|e| format!("Failed to fetch questions: {}", e))
    }

    /// Execute a question query and return results
    pub async fn execute_question(&self, id: u32) -> Result<QueryResult, String> {
        self.client
            .execute_question(id, None)
            .await
            .map_err(|e| format!("Query execution failed: {}", e))
    }

    /// Fetch questions filtered by collection
    pub async fn fetch_questions_by_collection(
        &self,
        collection_id: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Question>, String> {
        let service = QuestionService::new(self.client.clone());
        let params = ListParams {
            search: None,
            limit,
            collection: Some(collection_id.to_string()),
            offset: None,
        };

        service
            .list_questions(params)
            .await
            .map_err(|e| format!("Failed to fetch questions: {}", e))
    }

    /// Fetch collections list
    pub async fn fetch_collections(&self) -> Result<Vec<CollectionItem>, String> {
        self.client
            .list_collections()
            .await
            .map_err(|e| format!("Failed to fetch collections: {}", e))
    }

    /// Fetch databases list
    pub async fn fetch_databases(&self) -> Result<Vec<Database>, String> {
        self.client
            .list_databases()
            .await
            .map_err(|e| format!("Failed to fetch databases: {}", e))
    }

    /// Fetch schemas for a specific database
    pub async fn fetch_schemas(&self, database_id: u32) -> Result<Vec<String>, String> {
        self.client
            .list_schemas(database_id)
            .await
            .map_err(|e| format!("Failed to fetch schemas: {}", e))
    }

    /// Fetch tables for a specific schema in a database
    pub async fn fetch_tables(
        &self,
        database_id: u32,
        schema: &str,
    ) -> Result<Vec<TableInfo>, String> {
        self.client
            .list_tables(database_id, schema)
            .await
            .map_err(|e| format!("Failed to fetch tables: {}", e))
    }

    /// Preview table data (fetch sample rows)
    pub async fn preview_table(
        &self,
        database_id: u32,
        table_id: u32,
        limit: u32,
    ) -> Result<QueryResult, String> {
        self.client
            .preview_table(database_id, table_id, limit)
            .await
            .map_err(|e| format!("Failed to preview table: {}", e))
    }
}

/// Initialize service client from environment and config, wrapped in Arc.
pub fn init_service() -> Result<Arc<ServiceClient>, String> {
    // Get API key from environment
    let api_key = get_api_key();

    // Try to load config for base URL
    let config = Config::load(None).ok();

    // Get base URL from config (default profile) or use default
    let base_url = config
        .as_ref()
        .and_then(|c| c.get_profile("default"))
        .map(|p| p.url.clone())
        .unwrap_or_else(|| "http://localhost:3000".to_string());

    ServiceClient::new(base_url, api_key).map(Arc::new)
}

/// Connection status for display
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ConnectionStatus {
    /// Not connected (no API key)
    #[default]
    Disconnected,
    /// Connecting/validating
    Connecting,
    /// Connected and authenticated
    Connected(String), // Username or email
    /// Connection failed
    Error(String),
}
