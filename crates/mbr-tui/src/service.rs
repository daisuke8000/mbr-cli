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
use mbr_core::storage::credentials::{Session, get_credentials, load_session};

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
    pub fn new(base_url: String, session_token: Option<String>) -> Result<Self, String> {
        let client = if let Some(token) = session_token {
            MetabaseClient::with_session_token(base_url.clone(), token)
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

    /// Validate session by fetching current user
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

/// Initialize service client from stored session or environment credentials.
pub async fn init_service() -> Result<Arc<ServiceClient>, String> {
    let config = Config::load(None).ok();
    let base_url = config
        .as_ref()
        .and_then(|c| c.get_url())
        .unwrap_or_else(|| "http://localhost:3000".to_string());

    // Try stored session first
    if let Some(session) = load_session()
        && session.url == base_url
    {
        return ServiceClient::new(base_url, Some(session.session_token)).map(Arc::new);
    }

    // Try auto-login via environment variables
    if let Some((username, password)) = get_credentials() {
        match MetabaseClient::login(&base_url, &username, &password).await {
            Ok(token) => {
                let session = Session {
                    session_token: token.clone(),
                    url: base_url.clone(),
                    username,
                    created_at: mbr_core::storage::credentials::now_iso8601(),
                };
                let _ = mbr_core::storage::credentials::save_session(&session);
                return ServiceClient::new(base_url, Some(token)).map(Arc::new);
            }
            Err(e) => {
                return Err(format!(
                    "Auto-login failed: {}\n\nPlease run 'mbr login' first, or check MBR_USERNAME and MBR_PASSWORD.",
                    e
                ));
            }
        }
    }

    Err("No active session.\n\nPlease run 'mbr login' first, or set MBR_USERNAME and MBR_PASSWORD environment variables.".to_string())
}

/// Connection status for display
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ConnectionStatus {
    /// Not connected (no session)
    #[default]
    Disconnected,
    /// Connecting/validating
    Connecting,
    /// Connected and authenticated
    Connected(String), // Username or email
    /// Connection failed
    Error(String),
}
