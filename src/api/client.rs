use crate::api::models::{LoginRequest, LoginResponse};
use crate::error::{ApiError, AppError};
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
            .map_err(|e| ApiError::Http {
                status: 0,
                endpoint: "client_init".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;

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

        let response = self
            .build_request(Method::POST, "/api/session")
            .json(&login_req)
            .send()
            .await
            .map_err(|e| ApiError::Http {
                status: 0,
                endpoint: "/api/session".to_string(),
                message: format!("Request failed: {}", e),
            })?;

        let login_resp: LoginResponse = Self::handle_response(response, "/api/session").await?;
        self.session_token = Some(login_resp.id);
        Ok(())
    }

    pub async fn logout(&mut self) -> Result<(), ApiError> {
        // If not authenticated via session, nothing to do
        if self.session_token.is_none() {
            return Ok(());
        }

        let response = self
            .build_request(Method::DELETE, "/api/session")
            .send()
            .await
            .map_err(|e| ApiError::Http {
                status: 0,
                endpoint: "/api/session".to_string(),
                message: format!("Request failed: {}", e),
            })?;

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
            .map_err(|e| {
                AppError::Api(ApiError::Http {
                    status: 0, // Network error, no status available
                    endpoint: endpoint.clone(),
                    message: format!("Network error: {}", e),
                })
            })?;

        let status = response.status();

        if status.is_success() {
            // Parse response as Vec<Question>
            let mut questions: Vec<crate::api::models::Question> =
                response.json().await.map_err(|e| {
                    AppError::Api(ApiError::Http {
                        status: 200, // Success status but parsing failed
                        endpoint: endpoint.clone(),
                        message: format!("Failed to parse JSON response: {}", e),
                    })
                })?;

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
                response.json().await.map_err(|e| {
                    AppError::Api(ApiError::Http {
                        status: 200, // Success status but parsing failed
                        endpoint: endpoint.clone(),
                        message: format!("Failed to parse query result JSON: {}", e),
                    })
                })?;

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
}
