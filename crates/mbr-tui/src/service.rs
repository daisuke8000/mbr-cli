//! Service integration layer for mbr-core.
//!
//! Provides async data fetching using mbr-core's QuestionService and MetabaseClient.

// Allow unused fields/methods as they are designed for Phase 4+ implementation
#![allow(dead_code)]

use mbr_core::api::client::MetabaseClient;
use mbr_core::api::models::{CurrentUser, Question};
use mbr_core::core::services::question_service::QuestionService;
use mbr_core::core::services::types::ListParams;
use mbr_core::storage::config::Config;
use mbr_core::storage::credentials::get_api_key;

/// Application state loaded from mbr-core services
#[derive(Debug, Clone, Default)]
pub struct AppData {
    /// Loaded questions from Metabase
    pub questions: Vec<Question>,
    /// Current user information (if authenticated)
    pub current_user: Option<CurrentUser>,
    /// Error message from last operation
    pub error: Option<String>,
    /// Whether data is currently loading
    pub loading: bool,
}

/// Service client wrapper for async operations
pub struct ServiceClient {
    client: MetabaseClient,
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
}

/// Initialize service client from environment and config
pub fn init_service() -> Result<ServiceClient, String> {
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

    ServiceClient::new(base_url, api_key)
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
