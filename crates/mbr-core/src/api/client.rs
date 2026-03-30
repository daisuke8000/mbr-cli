use crate::error::{ApiError, AppError};
use crate::utils::error_helpers::*;
use reqwest::{Client, Method, RequestBuilder, Response};
use serde::Serialize;
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const USER_AGENT: &str = concat!("mbr-cli/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct MetabaseClient {
    client: Client,
    pub base_url: String,
    pub session_token: Option<String>,
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
        })
    }

    pub fn is_authenticated(&self) -> bool {
        self.session_token.is_some()
    }

    pub fn with_session_token(base_url: String, session_token: String) -> Result<Self, ApiError> {
        let mut client = MetabaseClient::new(base_url)?;
        client.session_token = Some(session_token);
        Ok(client)
    }

    /// Build a request with the given HTTP method and path.
    ///
    /// Automatically adds session token authentication header if configured.
    /// For query parameters, either:
    /// - Chain `.query()` on the returned `RequestBuilder`
    /// - Use `build_request_with_query()` for typed parameters
    pub fn build_request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, url);

        if let Some(token) = &self.session_token {
            request = request.header("X-Metabase-Session", token);
        }

        request
    }

    /// Build a request with typed query parameters.
    ///
    /// Provides type-safe query parameter handling with automatic URL encoding.
    /// Use this when query parameters are known at compile time.
    ///
    /// # Example
    /// ```ignore
    /// #[derive(Serialize)]
    /// struct SearchParams {
    ///     q: String,
    ///     models: String,
    /// }
    ///
    /// let params = SearchParams { q: "term".into(), models: "card".into() };
    /// client.build_request_with_query(Method::GET, "/api/search", Some(&params))
    ///     .send()
    ///     .await?;
    /// ```
    pub fn build_request_with_query<T: Serialize>(
        &self,
        method: Method,
        path: &str,
        query: Option<&T>,
    ) -> RequestBuilder {
        let mut request = self.build_request(method, path);

        if let Some(q) = query {
            request = request.query(q);
        }

        request
    }

    /// Get current user information from Metabase
    /// Used for validating session authentication
    pub async fn get_current_user(&self) -> Result<crate::api::models::CurrentUser, AppError> {
        let endpoint = "/api/user/current";

        let response = self
            .build_request(Method::GET, endpoint)
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, endpoint)))?;

        Self::handle_response(response, endpoint).await
    }

    /// Login to Metabase and return a session token.
    pub async fn login(base_url: &str, username: &str, password: &str) -> Result<String, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .user_agent(USER_AGENT)
            .build()
            .map_err(|e| AppError::Api(convert_request_error(e, "client_init")))?;

        let url = format!("{}/api/session", base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "username": username,
            "password": password
        });

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, "/api/session")))?;

        if response.status().is_success() {
            #[derive(serde::Deserialize)]
            struct SessionResponse {
                id: String,
            }
            let session: SessionResponse = response
                .json()
                .await
                .map_err(|e| AppError::Api(convert_json_error(e, "/api/session")))?;
            Ok(session.id)
        } else {
            let status = response.status().as_u16();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(AppError::Auth(crate::error::AuthError::LoginFailed {
                message: if status == 401 {
                    "Invalid username or password".to_string()
                } else {
                    format!("Server returned {}: {}", status, error_text)
                },
            }))
        }
    }

    /// Logout from Metabase by invalidating the server-side session.
    pub async fn logout(&self) -> Result<(), AppError> {
        if self.session_token.is_none() {
            return Ok(());
        }
        let _ = self
            .build_request(Method::DELETE, "/api/session")
            .send()
            .await;
        Ok(())
    }

    /// List questions from Metabase with optional search, limit, and collection filters
    pub async fn list_questions(
        &self,
        search: Option<&str>,
        limit: Option<u32>,
        collection: Option<&str>,
    ) -> Result<Vec<crate::api::models::Question>, AppError> {
        // If search term is provided, use /api/search endpoint
        if let Some(search_term) = search
            && !search_term.is_empty()
        {
            return self.search_questions(search_term, limit).await;
        }

        // Otherwise, use /api/card endpoint for listing all questions
        // TODO: Metabase's /api/card endpoint does not support server-side limit/offset.
        // Pagination is applied client-side via truncation below.
        // Consider migrating to /api/search with empty query for server-side pagination.
        let mut params = vec!["f=all".to_string()];

        if let Some(collection_id) = collection
            && !collection_id.is_empty()
        {
            params.push(format!("collection={}", collection_id));
        }

        let endpoint = format!("/api/card?{}", params.join("&"));

        let response = self
            .build_request(Method::GET, &endpoint)
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, &endpoint)))?;

        let mut questions: Vec<crate::api::models::Question> =
            Self::handle_response(response, &endpoint).await?;

        // Apply client-side limit if specified
        if let Some(limit_value) = limit
            && limit_value > 0
        {
            questions.truncate(limit_value as usize);
        }

        Ok(questions)
    }

    /// Search questions using /api/search endpoint.
    /// Uses reqwest's .query() for proper URL encoding of search terms,
    /// including multibyte characters (Japanese, etc.).
    /// Supports server-side pagination via limit and offset parameters.
    async fn search_questions(
        &self,
        search_term: &str,
        limit: Option<u32>,
    ) -> Result<Vec<crate::api::models::Question>, AppError> {
        use crate::api::models::{Collection, Question, SearchResponse};

        /// Query parameters for Metabase search API with pagination support
        #[derive(Serialize)]
        struct SearchQuery<'a> {
            q: &'a str,
            models: &'static str,
            #[serde(skip_serializing_if = "Option::is_none")]
            limit: Option<u32>,
        }

        let endpoint = "/api/search";
        let query = SearchQuery {
            q: search_term,
            models: "card",
            limit,
        };

        // Use build_request_with_query for type-safe URL encoding
        let response = self
            .build_request_with_query(Method::GET, endpoint, Some(&query))
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, endpoint)))?;

        let search_response: SearchResponse = Self::handle_response(response, endpoint).await?;

        // Convert SearchResultItem to Question
        let questions: Vec<Question> = search_response
            .data
            .into_iter()
            .filter(|item| item.model == "card")
            .map(|item| Question {
                id: item.id,
                name: item.name,
                description: item.description,
                collection_id: item.collection_id,
                collection: item.collection.map(|c| Collection {
                    id: c.id,
                    name: c.name,
                }),
            })
            .collect();

        Ok(questions)
    }

    /// Execute a question and return query results
    pub async fn execute_question(
        &self,
        question_id: u32,
        parameters: Option<HashMap<String, String>>,
    ) -> Result<crate::api::models::QueryResult, AppError> {
        let endpoint = format!("/api/card/{}/query", question_id);

        let mut request = self.build_request(Method::POST, &endpoint);

        if let Some(params) = parameters
            && !params.is_empty()
        {
            request = request.json(&params);
        }

        // Extended timeout for query execution
        let response = request
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|_e| {
                AppError::Api(ApiError::Timeout {
                    timeout_secs: 60,
                    endpoint: endpoint.clone(),
                })
            })?;

        // Handle 404 with custom message before generic handling
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::Api(ApiError::Http {
                status: 404,
                endpoint,
                message: format!("Question with ID {} not found", question_id),
            }));
        }

        Self::handle_response(response, &endpoint).await
    }

    /// List all collections from Metabase
    pub async fn list_collections(
        &self,
    ) -> Result<Vec<crate::api::models::CollectionItem>, AppError> {
        let endpoint = "/api/collection";

        let response = self
            .build_request(Method::GET, endpoint)
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, endpoint)))?;

        let collections: Vec<crate::api::models::CollectionItem> =
            Self::handle_response(response, endpoint).await?;

        // Filter out archived collections
        Ok(collections.into_iter().filter(|c| !c.archived).collect())
    }

    /// List all databases from Metabase
    pub async fn list_databases(&self) -> Result<Vec<crate::api::models::Database>, AppError> {
        let endpoint = "/api/database";

        let response = self
            .build_request(Method::GET, endpoint)
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, endpoint)))?;

        // Metabase returns { "data": [...] } for databases
        #[derive(serde::Deserialize)]
        struct DatabaseResponse {
            data: Vec<crate::api::models::Database>,
        }

        let response_body: DatabaseResponse = Self::handle_response(response, endpoint).await?;

        Ok(response_body.data)
    }

    /// List schemas for a specific database.
    /// Returns a list of schema names.
    pub async fn list_schemas(&self, database_id: u32) -> Result<Vec<String>, AppError> {
        let endpoint = format!("/api/database/{}/schemas", database_id);

        let response = self
            .build_request(Method::GET, &endpoint)
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, &endpoint)))?;

        // Handle 404 with custom message
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::Api(ApiError::Http {
                status: 404,
                endpoint,
                message: format!("Database with ID {} not found", database_id),
            }));
        }

        Self::handle_response(response, &endpoint).await
    }

    /// List tables in a specific schema of a database.
    pub async fn list_tables(
        &self,
        database_id: u32,
        schema: &str,
    ) -> Result<Vec<crate::api::models::TableInfo>, AppError> {
        let endpoint = format!("/api/database/{}/schema/{}", database_id, schema);

        let response = self
            .build_request(Method::GET, &endpoint)
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, &endpoint)))?;

        // Handle 404 with custom message
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::Api(ApiError::Http {
                status: 404,
                endpoint,
                message: format!("Schema '{}' not found in database {}", schema, database_id),
            }));
        }

        Self::handle_response(response, &endpoint).await
    }

    /// Preview table data (fetch sample rows).
    /// Uses POST /api/dataset to query the table directly.
    pub async fn preview_table(
        &self,
        database_id: u32,
        table_id: u32,
        limit: u32,
    ) -> Result<crate::api::models::QueryResult, AppError> {
        let endpoint = "/api/dataset";

        // Build the query payload for table preview
        let query_payload = serde_json::json!({
            "database": database_id,
            "type": "query",
            "query": {
                "source-table": table_id,
                "limit": limit
            }
        });

        let response = self
            .build_request(Method::POST, endpoint)
            .json(&query_payload)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|_e| {
                AppError::Api(ApiError::Timeout {
                    timeout_secs: 60,
                    endpoint: endpoint.to_string(),
                })
            })?;

        Self::handle_response(response, endpoint).await
    }

    /// Common HTTP response handler for all API methods.
    /// Handles success/error responses with consistent error mapping.
    async fn handle_response<T>(response: Response, endpoint: &str) -> Result<T, AppError>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            response
                .json::<T>()
                .await
                .map_err(|e| AppError::Api(convert_json_error(e, endpoint)))
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(AppError::Api(match status.as_u16() {
                401 => ApiError::Unauthorized {
                    status: status.as_u16(),
                    endpoint: endpoint.to_string(),
                    server_message: error_text,
                },
                403 => ApiError::Forbidden {
                    status: status.as_u16(),
                    endpoint: endpoint.to_string(),
                    server_message: error_text,
                },
                408 | 504 => ApiError::Timeout {
                    timeout_secs: DEFAULT_TIMEOUT_SECS,
                    endpoint: endpoint.to_string(),
                },
                _ => ApiError::Http {
                    status: status.as_u16(),
                    endpoint: endpoint.to_string(),
                    message: error_text,
                },
            }))
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
    fn test_not_authenticated_without_session_token() {
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");
        assert!(!client.is_authenticated());
    }

    #[test]
    fn test_with_session_token() {
        let client = MetabaseClient::with_session_token(
            "http://example.test".to_string(),
            "token".to_string(),
        );
        assert!(client.is_ok());
        if let Ok(client) = client {
            assert!(client.is_authenticated());
            assert_eq!(Some("token".to_string()), client.session_token);
        }
    }

    #[test]
    fn test_build_request_without_auth() {
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");
        let request = client.build_request(Method::GET, "/api/card");

        let built_request = request.build().expect("Failed to build request");

        assert_eq!(built_request.url().as_str(), "http://example.test/api/card");
        assert_eq!(built_request.method(), Method::GET);
        assert!(built_request.headers().get("X-Metabase-Session").is_none());
    }

    #[test]
    fn test_build_request_with_session_token() {
        let client = MetabaseClient::with_session_token(
            "http://example.test".to_string(),
            "test_session_token_123".to_string(),
        )
        .expect("client creation failed");

        let request = client.build_request(Method::GET, "/api/card");
        let built_request = request.build().expect("Failed to build request");

        assert_eq!(
            built_request
                .headers()
                .get("X-Metabase-Session")
                .unwrap()
                .to_str()
                .unwrap(),
            "test_session_token_123"
        );
    }

    #[test]
    fn test_base_url_trailing_slash_removed() {
        let client = MetabaseClient::new("http://example.test/".to_string())
            .expect("client creation failed");
        assert_eq!(client.base_url, "http://example.test");
    }

    #[test]
    fn test_build_request_with_query_params() {
        #[derive(Serialize)]
        struct TestQuery {
            q: String,
            limit: u32,
        }

        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");

        let query = TestQuery {
            q: "search term".to_string(),
            limit: 10,
        };

        let request = client.build_request_with_query(Method::GET, "/api/search", Some(&query));
        let built_request = request.build().expect("Failed to build request");

        // URL should contain query parameters (URL-encoded)
        let url = built_request.url().as_str();
        assert!(url.starts_with("http://example.test/api/search?"));
        assert!(url.contains("q=search"));
        assert!(url.contains("limit=10"));
    }

    #[test]
    fn test_build_request_with_query_none() {
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");

        let request = client.build_request_with_query::<()>(Method::GET, "/api/search", None);
        let built_request = request.build().expect("Failed to build request");

        // URL should not contain query string when None is passed
        assert_eq!(
            built_request.url().as_str(),
            "http://example.test/api/search"
        );
    }

    #[test]
    fn test_build_request_with_query_unicode() {
        #[derive(Serialize)]
        struct SearchQuery<'a> {
            q: &'a str,
        }

        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");

        // Test with Japanese characters (multibyte)
        let query = SearchQuery {
            q: "売上データ"
        };

        let request = client.build_request_with_query(Method::GET, "/api/search", Some(&query));
        let built_request = request.build().expect("Failed to build request");

        // URL should be properly URL-encoded
        let url = built_request.url().as_str();
        assert!(url.starts_with("http://example.test/api/search?q="));
        // URL-encoded Japanese characters
        assert!(url.contains("%"));
    }
}
