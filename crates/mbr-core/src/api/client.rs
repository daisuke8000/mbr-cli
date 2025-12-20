use crate::error::{ApiError, AppError};
use crate::utils::error_helpers::*;
use reqwest::{Client, Method, RequestBuilder, Response};
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const USER_AGENT: &str = concat!("mbr-cli/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct MetabaseClient {
    client: Client,
    pub base_url: String,
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
            api_key: None,
        })
    }

    pub fn is_authenticated(&self) -> bool {
        self.api_key.is_some()
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
        }

        request
    }

    /// Get current user information from Metabase
    /// Used for validating API key authentication
    pub async fn get_current_user(&self) -> Result<crate::api::models::CurrentUser, AppError> {
        let endpoint = "/api/user/current";

        let response = self
            .build_request(Method::GET, endpoint)
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, endpoint)))?;

        Self::handle_response(response, endpoint).await
    }

    /// List questions from Metabase with optional search, limit, and collection filters
    pub async fn list_questions(
        &self,
        search: Option<&str>,
        limit: Option<u32>,
        collection: Option<&str>,
    ) -> Result<Vec<crate::api::models::Question>, AppError> {
        // If search term is provided, use /api/search endpoint
        if let Some(search_term) = search {
            if !search_term.is_empty() {
                return self.search_questions(search_term, limit).await;
            }
        }

        // Otherwise, use /api/card endpoint for listing all questions
        let mut params = vec!["f=all".to_string()];

        if let Some(collection_id) = collection {
            if !collection_id.is_empty() {
                params.push(format!("collection={}", collection_id));
            }
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
        if let Some(limit_value) = limit {
            if limit_value > 0 {
                questions.truncate(limit_value as usize);
            }
        }

        Ok(questions)
    }

    /// Search questions using /api/search endpoint.
    /// Uses reqwest's .query() for proper URL encoding of search terms,
    /// including multibyte characters (Japanese, etc.).
    async fn search_questions(
        &self,
        search_term: &str,
        limit: Option<u32>,
    ) -> Result<Vec<crate::api::models::Question>, AppError> {
        use crate::api::models::{Collection, Question, SearchResponse};

        let endpoint = "/api/search";

        // Use reqwest's .query() for safe URL encoding (handles UTF-8 correctly)
        let response = self
            .build_request(Method::GET, endpoint)
            .query(&[("q", search_term), ("models", "card")])
            .send()
            .await
            .map_err(|e| AppError::Api(convert_request_error(e, endpoint)))?;

        let search_response: SearchResponse = Self::handle_response(response, endpoint).await?;

        // Convert SearchResultItem to Question
        let mut questions: Vec<Question> = search_response
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

        // Apply client-side limit if specified
        if let Some(limit_value) = limit {
            if limit_value > 0 {
                questions.truncate(limit_value as usize);
            }
        }

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

        if let Some(params) = parameters {
            if !params.is_empty() {
                request = request.json(&params);
            }
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
                401 | 403 => ApiError::Unauthorized {
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
    fn test_not_authenticated_without_api_key() {
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");
        assert!(!client.is_authenticated());
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
    fn test_build_request_without_auth() {
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");
        let request = client.build_request(Method::GET, "/api/card");

        let built_request = request.build().expect("Failed to build request");

        assert_eq!(built_request.url().as_str(), "http://example.test/api/card");
        assert_eq!(built_request.method(), Method::GET);
        assert!(built_request.headers().get("x-api-key").is_none());
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
    }

    #[test]
    fn test_base_url_trailing_slash_removed() {
        let client = MetabaseClient::new("http://example.test/".to_string())
            .expect("client creation failed");
        assert_eq!(client.base_url, "http://example.test");
    }
}
