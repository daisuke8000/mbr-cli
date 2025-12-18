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
