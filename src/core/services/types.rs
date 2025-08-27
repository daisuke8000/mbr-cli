use crate::error::ApiError;
use crate::storage::credentials::AuthMode;
use std::collections::HashMap;

/// Service layer error types
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },

    #[error("Not found: {resource_type} with ID {id}")]
    NotFound { resource_type: String, id: u32 },

    #[error("Configuration error: {0}")]
    Config(String),
}

/// 認証状態情報
#[derive(Debug, Clone)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub auth_mode: AuthMode,
    pub profile_name: String,
    pub session_active: bool,
}

/// 質問リスト取得パラメータ
#[derive(Debug, Clone)]
pub struct ListParams {
    pub search: Option<String>,
    pub limit: Option<u32>,
    pub collection: Option<String>,
    pub offset: Option<u32>,
}

/// 質問実行パラメータ
#[derive(Debug, Clone)]
pub struct ExecuteParams {
    pub parameters: HashMap<String, String>,
    pub format: String,
    pub limit: Option<u32>,
    pub offset: Option<usize>,
    pub page_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_status_creation() {
        let status = AuthStatus {
            is_authenticated: true,
            auth_mode: AuthMode::Session,
            profile_name: "test".to_string(),
            session_active: true,
        };
        assert!(status.is_authenticated);
        assert_eq!(status.profile_name, "test");
    }

    #[test]
    fn test_list_params_creation() {
        let params = ListParams {
            search: Some("test".to_string()),
            limit: Some(10),
            collection: None,
            offset: None,
        };
        assert_eq!(params.search.unwrap(), "test");
        assert_eq!(params.limit, Some(10));
        assert!(params.collection.is_none());
    }

    #[test]
    fn test_execute_params_creation() {
        let mut parameters = HashMap::new();
        parameters.insert("param1".to_string(), "value1".to_string());

        let params = ExecuteParams {
            parameters,
            format: "json".to_string(),
            limit: Some(100),
            offset: None,
            page_size: 20,
        };
        assert_eq!(params.format, "json");
        assert_eq!(params.limit, Some(100));
        assert_eq!(params.page_size, 20);
    }
}
