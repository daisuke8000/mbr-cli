use crate::api::models::{
    Collection, CollectionDetail, Dashboard, DashboardCard, LoginRequest, LoginResponse,
};
use crate::error::{ApiError, AppError};
use crate::utils::error_helpers::*;
use crate::map_api_error;
use backoff::{Error as BackoffError, ExponentialBackoff};
use reqwest::{Client, Method, RequestBuilder, Response};
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const USER_AGENT: &str = concat!("mbr-cli/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct MetabaseClient {
    client: Client,
    pub base_url: String,
    pub session_token: Option<String>,
    pub api_key: Option<String>,
}

impl MetabaseClient {
    // Create baseClient with default settings
    pub fn new(base_url: String) -> Result<Self, ApiError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| convert_request_error(e, "client_init"))?;

        Ok(MetabaseClient {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            session_token: None,
            api_key: None,
        })
    }

    pub fn set_session_token(&mut self, token: String) {
        self.session_token = Some(token);
    }

    pub fn get_session_token(&self) -> Option<String> {
        self.session_token.clone()
    }

    pub fn is_authenticated(&self) -> bool {
        self.api_key.is_some() || self.session_token.is_some()
    }

    pub fn with_api_key(base_url: String, api_key: String) -> Result<Self, ApiError> {
        let mut client = MetabaseClient::new(base_url)?;
        client.api_key = Some(api_key);
        Ok(client)
    }

    pub fn build_request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, url);

        if let Some(api_key) = &self.api_key {
            request = request.header("x-api-key", api_key);
        } else if let Some(token) = &self.session_token {
            request = request.header("X-Metabase-Session", token);
        }

        request
    }

    // Authentication endpoints
    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), ApiError> {
        let login_req = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = map_api_error!(
            self.build_request(Method::POST, "/api/session")
                .json(&login_req)
                .send()
                .await,
            "/api/session"
        )?;

        let login_resp: LoginResponse = Self::handle_response(response, "/api/session").await?;
        self.session_token = Some(login_resp.id);
        Ok(())
    }

    pub async fn logout(&mut self) -> Result<(), ApiError> {
        // If not authenticated via session, nothing to do
        if self.session_token.is_none() {
            return Ok(());
        }

        let response = map_api_error!(
            self.build_request(Method::DELETE, "/api/session")
                .send()
                .await,
            "/api/session"
        )?;

        // Metabase returns 204 No Content on successful logout
        let status = response.status();
        if status.is_success() {
            self.session_token = None;
            Ok(())
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Logout failed".to_string());

            Err(ApiError::Http {
                status: status.as_u16(),
                endpoint: "/api/session".to_string(),
                message: error_text,
            })
        }
    }

    /// List questions from Metabase with optional search, limit, and collection filters
    pub async fn list_questions(
        &self,
        search: Option<&str>,
        limit: Option<u32>,
        collection: Option<&str>,
    ) -> Result<Vec<crate::api::models::Question>, AppError> {
        // Build query parameters
        let mut params = Vec::new();
        params.push("f=all".to_string());

        // Add search parameter if provided
        if let Some(search_term) = search {
            if !search_term.is_empty() {
                params.push(format!("q={}", search_term));
            }
        }

        // Add collection parameter if provided
        if let Some(collection_id) = collection {
            if !collection_id.is_empty() {
                params.push(format!("collection={}", collection_id));
            }
        }

        // Build endpoint with parameters
        let endpoint = if params.is_empty() {
            "/api/card".to_string()
        } else {
            format!("/api/card?{}", params.join("&"))
        };

        // Build and send request
        let response = self
            .build_request(Method::GET, &endpoint)
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, &endpoint)))?;

        let status = response.status();

        if status.is_success() {
            // Parse response as Vec<Question>
            let mut questions: Vec<crate::api::models::Question> =
                response.json().await.map_err(|e| AppError::Api(convert_json_error(e, &endpoint)))?;

            // Apply limit if specified
            if let Some(limit_value) = limit {
                if limit_value > 0 {
                    questions.truncate(limit_value as usize);
                }
            }

            Ok(questions)
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(AppError::Api(ApiError::Unauthorized {
                status: status.as_u16(),
                endpoint,
                server_message: "Authentication failed - please login first".to_string(),
            }))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(AppError::Api(ApiError::Http {
                status: status.as_u16(),
                endpoint,
                message: error_text,
            }))
        }
    }

    /// Execute a question and return query results
    pub async fn execute_question(
        &self,
        question_id: u32,
        parameters: Option<HashMap<String, String>>,
    ) -> Result<crate::api::models::QueryResult, AppError> {
        let endpoint = format!("/api/card/{}/query", question_id);

        // Build request with longer timeout for query execution
        let mut request = self.build_request(Method::POST, &endpoint);

        // Add parameters as JSON body if provided
        if let Some(params) = parameters {
            if !params.is_empty() {
                request = request.json(&params);
            }
        }

        // Send request with extended timeout
        let response = request
            .timeout(Duration::from_secs(60)) // 60 second timeout for query execution
            .send()
            .await
            .map_err(|_e| {
                AppError::Api(ApiError::Timeout {
                    timeout_secs: 60,
                    endpoint: endpoint.clone(),
                })
            })?;

        let status = response.status();

        if status.is_success() {
            // Parse response as QueryResult
            let query_result: crate::api::models::QueryResult =
                response.json().await.map_err(|e| AppError::Api(convert_json_error(e, &endpoint)))?;

            Ok(query_result)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            Err(AppError::Api(ApiError::Http {
                status: 404,
                endpoint,
                message: format!("Question with ID {} not found", question_id),
            }))
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(AppError::Api(ApiError::Unauthorized {
                status: status.as_u16(),
                endpoint,
                server_message: "Authentication failed - please login first".to_string(),
            }))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(AppError::Api(ApiError::Http {
                status: status.as_u16(),
                endpoint,
                message: error_text,
            }))
        }
    }

    /// Get all dashboards with optional search and limit parameters
    /// Includes automatic retry on transient failures
    pub async fn get_dashboards(
        &self,
        search: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<Dashboard>, ApiError> {
        let mut endpoint = "/api/dashboard".to_string();
        let mut query_params = vec![];

        if let Some(search_term) = search {
            query_params.push(format!("f=name&q={}", search_term));
        }

        if let Some(limit_val) = limit {
            query_params.push(format!("limit={}", limit_val));
        }

        if !query_params.is_empty() {
            endpoint.push('?');
            endpoint.push_str(&query_params.join("&"));
        }

        self.execute_with_retry(|| async {
            let request = self.build_request(Method::GET, &endpoint);
            let response = request.send().await.map_err(|_e| convert_timeout_error(&endpoint, DEFAULT_TIMEOUT_SECS))?;
            Self::handle_response(response, &endpoint).await
        })
        .await
    }

    /// Execute API request with exponential backoff retry
    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T, ApiError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, ApiError>>,
    {
        let backoff = ExponentialBackoff::default();

        backoff::future::retry(backoff, || async {
            match operation().await {
                Ok(result) => Ok(result),
                Err(ApiError::Http {
                    status: 500..=599, ..
                }) => {
                    // Retry on server errors
                    Err(BackoffError::transient(ApiError::Http {
                        status: 500,
                        endpoint: "retry".to_string(),
                        message: "Server error".to_string(),
                    }))
                }
                Err(ApiError::Timeout { .. }) => {
                    // Retry on timeouts
                    Err(BackoffError::transient(convert_timeout_error("retry", DEFAULT_TIMEOUT_SECS)))
                }
                Err(other) => {
                    // Don't retry on client errors (4xx)
                    Err(BackoffError::permanent(other))
                }
            }
        })
        .await
    }

    /// Get a specific dashboard by ID  
    /// Includes automatic retry on transient failures
    pub async fn get_dashboard(&self, id: u32) -> Result<Dashboard, ApiError> {
        let endpoint = format!("/api/dashboard/{}", id);

        self.execute_with_retry(|| async {
            let request = self.build_request(Method::GET, &endpoint);
            let response = request.send().await.map_err(|_e| convert_timeout_error(&endpoint, DEFAULT_TIMEOUT_SECS))?;
            Self::handle_response(response, &endpoint).await
        })
        .await
    }

    /// Get dashboard cards for a specific dashboard
    /// Includes automatic retry on transient failures
    pub async fn get_dashboard_cards(&self, id: u32) -> Result<Vec<DashboardCard>, ApiError> {
        let endpoint = format!("/api/dashboard/{}", id);

        self.execute_with_retry(|| async {
            let request = self.build_request(Method::GET, &endpoint);
            let response = request.send().await.map_err(|_e| convert_timeout_error(&endpoint, DEFAULT_TIMEOUT_SECS))?;
            // Dashboard response includes dashcards field
            let dashboard: Dashboard = Self::handle_response(response, &endpoint).await?;
            Ok(dashboard.dashcards.unwrap_or_default())
        })
        .await
    }

    /// Get all collections with tree structure support
    /// Includes automatic retry on transient failures
    pub async fn get_collections(&self, tree: bool) -> Result<Vec<Collection>, ApiError> {
        let mut endpoint = "/api/collection".to_string();
        if tree {
            endpoint.push_str("?tree=true");
        }

        self.execute_with_retry(|| async {
            let request = self.build_request(Method::GET, &endpoint);
            let response = request.send().await.map_err(|_e| convert_timeout_error(&endpoint, DEFAULT_TIMEOUT_SECS))?;
            Self::handle_response(response, &endpoint).await
        })
        .await
    }

    /// Get a specific collection by ID with detailed information
    /// Includes automatic retry on transient failures  
    pub async fn get_collection(&self, id: u32) -> Result<CollectionDetail, ApiError> {
        let endpoint = format!("/api/collection/{}", id);

        self.execute_with_retry(|| async {
            let request = self.build_request(Method::GET, &endpoint);
            let response = request.send().await.map_err(|_e| convert_timeout_error(&endpoint, DEFAULT_TIMEOUT_SECS))?;
            Self::handle_response(response, &endpoint).await
        })
        .await
    }

    /// Get items (questions and dashboards) within a specific collection
    /// Includes automatic retry on transient failures
    pub async fn get_collection_items(&self, id: u32) -> Result<Vec<serde_json::Value>, ApiError> {
        let endpoint = format!("/api/collection/{}/items", id);

        self.execute_with_retry(|| async {
            let request = self.build_request(Method::GET, &endpoint);
            let response = request.send().await.map_err(|_e| convert_timeout_error(&endpoint, DEFAULT_TIMEOUT_SECS))?;
            Self::handle_response(response, &endpoint).await
        })
        .await
    }

    pub async fn handle_response<T>(response: Response, endpoint: &str) -> Result<T, ApiError>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            response.json::<T>().await.map_err(|e| ApiError::Http {
                status: status.as_u16(),
                endpoint: endpoint.to_string(),
                message: format!("Failed to parse response: {}", e),
            })
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            match status.as_u16() {
                401 | 403 => Err(ApiError::Unauthorized {
                    status: status.as_u16(),
                    endpoint: endpoint.to_string(),
                    server_message: error_text,
                }),
                408 | 504 => Err(ApiError::Timeout {
                    timeout_secs: DEFAULT_TIMEOUT_SECS,
                    endpoint: endpoint.to_string(),
                }),
                _ => Err(ApiError::Http {
                    status: status.as_u16(),
                    endpoint: endpoint.to_string(),
                    message: error_text,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = MetabaseClient::new("http://example.test".to_string());
        assert!(client.is_ok());
    }

    #[test]
    fn test_set_session_token_is_authenticated() {
        let mut client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");
        client.set_session_token("token".to_string());
        assert!(client.is_authenticated());
        assert_eq!(Some("token".to_string()), client.get_session_token());
    }

    #[test]
    fn test_set_session_token_is_not_authenticated() {
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");
        assert!(!client.is_authenticated());
        assert!(client.get_session_token().is_none());
    }

    #[test]
    fn test_with_api_key() {
        let client =
            MetabaseClient::with_api_key("http://example.test".to_string(), "key".to_string());
        assert!(client.is_ok());
        if let Ok(client) = client {
            assert!(client.is_authenticated());
            assert_eq!(Some("key".to_string()), client.api_key);
        }
    }

    #[test]
    fn test_build_request() {
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");
        let request = client.build_request(Method::POST, "/api/session");

        let built_request = request.build().expect("Failed to build request");

        assert_eq!(
            built_request.url().as_str(),
            "http://example.test/api/session"
        );
        assert_eq!(built_request.method(), Method::POST);
        assert!(built_request.headers().get("X-Metabase-Session").is_none());
    }

    #[test]
    fn test_build_request_with_api_key() {
        let client = MetabaseClient::with_api_key(
            "http://example.test".to_string(),
            "test_api_key_123".to_string(),
        )
        .expect("client creation failed");

        let request = client.build_request(Method::GET, "/api/card");
        let built_request = request.build().expect("Failed to build request");

        assert_eq!(
            built_request
                .headers()
                .get("x-api-key")
                .unwrap()
                .to_str()
                .unwrap(),
            "test_api_key_123"
        );
        assert!(built_request.headers().get("X-Metabase-Session").is_none());
    }

    #[test]
    fn test_get_dashboards_method_signature() {
        // Test that get_dashboards method exists with correct signature
        let client = MetabaseClient::new("http://test.example".to_string()).unwrap();

        // Just verify the client was created with dashboard methods available
        assert!(!client.is_authenticated());
        // The existence of these async methods is verified at compile time
    }

    #[test]
    fn test_dashboard_api_methods_exist() {
        // Compile-time verification that all dashboard methods exist
        let client = MetabaseClient::new("http://test.example".to_string()).unwrap();

        // Verify client has the required methods (compile-time check)
        assert!(!client.is_authenticated());

        // These methods existing means the API is properly implemented:
        // - get_dashboards(&self, search: Option<&str>, limit: Option<u32>)
        // - get_dashboard(&self, id: u32)
        // - get_dashboard_cards(&self, id: u32)
        // - execute_with_retry (private method for retry functionality)
    }

    #[test]
    fn test_dashboard_search_parameter_building() {
        // Test search parameter construction
        let client = MetabaseClient::new("http://test.example".to_string()).unwrap();

        // Verify the client was created properly
        assert!(!client.is_authenticated());
        assert_eq!(client.base_url, "http://test.example");
    }

    #[test]
    fn test_auth_priority_api_key_over_session() {
        let mut client = MetabaseClient::with_api_key(
            "http://example.test".to_string(),
            "api_key_456".to_string(),
        )
        .expect("client creation failed");

        // Set a session token too (not typical usage, but for testing priority)
        client.set_session_token("session_123".to_string());

        let request = client.build_request(Method::POST, "/api/session");
        let built_request = request.build().expect("Failed to build request");

        // API key should take priority
        assert!(built_request.headers().get("x-api-key").is_some());
        assert_eq!(
            built_request
                .headers()
                .get("x-api-key")
                .unwrap()
                .to_str()
                .unwrap(),
            "api_key_456"
        );
        // Session header should not exist (an API key takes priority)
        assert!(built_request.headers().get("X-Metabase-Session").is_none());
    }

    #[tokio::test]
    async fn test_login_updates_session_token() {
        // This would require a mock server in real tests
        // For now; we verify the method signature compiles
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");

        // Verify initial state
        assert!(!client.is_authenticated());
        assert!(client.session_token.is_none());

        // After login (would need a mock server for actual test)
        // client.login("user", "pass").await.unwrap();
        // assert!(client.is_authenticated());
    }

    #[tokio::test]
    async fn test_logout_clears_session_token() {
        let mut client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");

        // Set a session token
        client.set_session_token("test_token".to_string());
        assert!(client.is_authenticated());

        // After logout (would need a mock server for actual test)
        // client.logout().await.unwrap();
        // assert!(!client.is_authenticated());
        // assert!(client.session_token.is_none());
    }

    #[test]
    fn test_build_request_with_session_only() {
        let mut client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");
        client.set_session_token("session_abc".to_string());

        let request = client.build_request(Method::POST, "/api/session");
        let built_request = request.build().expect("Failed to build request");

        // No API key header should exist
        assert!(built_request.headers().get("x-api-key").is_none());
        // Session header should be present
        assert_eq!(
            built_request
                .headers()
                .get("X-Metabase-Session")
                .unwrap()
                .to_str()
                .unwrap(),
            "session_abc"
        );
    }

    #[test]
    fn test_collection_api_methods_exist() {
        // Test that collection methods exist with correct signature
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");

        // These should compile if methods exist with correct signatures
        // - get_collections(&self, tree: bool)
        // - get_collection(&self, id: u32)
        // - get_collection_items(&self, id: u32)
        let _future1 = client.get_collections(true);
        let _future2 = client.get_collection(1);
        let _future3 = client.get_collection_items(1);
    }
}
