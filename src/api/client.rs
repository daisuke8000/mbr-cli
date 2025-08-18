use crate::error::ApiError;
use reqwest::{Client, Method, RequestBuilder, Response};
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const USER_AGENT: &str = concat!("mbr-cli/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone)]
pub struct MetabaseClient {
    client: Client,
    pub base_url: String,
    pub session_token: Option<String>,
    // pub api_key: Option<String>,
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
        })
    }

    pub fn set_session_token(&mut self, token: String) {
        self.session_token = Some(token);
    }

    pub fn get_session_token(&self) -> Option<String> {
        self.session_token.clone()
    }

    pub fn is_authenticated(&self) -> bool {
        self.session_token.is_some()
    }

    pub fn build_request(&self, method: Method, path: &str) -> RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, url);

        if let Some(token) = &self.session_token {
            request = request.header("X-Metabase-Session", token);
        }

        request
    }

    pub async fn handle_response<T>(
        &self,
        response: Response,
        endpoint: &str,
    ) -> Result<T, ApiError>
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
    fn test_build_request() {
        let client =
            MetabaseClient::new("http://example.test".to_string()).expect("client creation failed");
        let request = client.build_request(Method::GET, "/api/session");

        let built_request = request.build().expect("Failed to build request");

        assert_eq!(
            built_request.url().as_str(),
            "http://example.test/api/session"
        );
        assert_eq!(built_request.method(), Method::GET);
        assert!(built_request.headers().get("X-Metabase-Session").is_none());
    }
}
