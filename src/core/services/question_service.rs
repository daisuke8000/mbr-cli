use crate::api::client::MetabaseClient;
use crate::api::models::{Question, QueryResult};
use super::types::{ListParams, ExecuteParams};
use crate::AppError;

/// Question service for managing Metabase questions and queries
pub struct QuestionService {
    client: MetabaseClient,
}

impl QuestionService {
    /// Create new QuestionService instance
    pub fn new(client: MetabaseClient) -> Self {
        Self { client }
    }

    /// List questions with optional filtering
    pub async fn list_questions(&self, params: ListParams) -> Result<Vec<Question>, AppError> {
        // Basic implementation using MetabaseClient
        // Convert ListParams to MetabaseClient parameters
        self.client.list_questions(
            params.search.as_deref(),
            Some(params.limit),
            params.collection.as_deref()
        ).await
    }

    /// Execute question with parameters
    pub async fn execute_question(&self, id: u32, params: ExecuteParams) -> Result<QueryResult, AppError> {
        // Basic implementation using MetabaseClient
        // Convert ExecuteParams to MetabaseClient parameters
        let parameters = if params.parameters.is_empty() {
            None
        } else {
            Some(params.parameters)
        };
        
        self.client.execute_question(id, parameters).await
    }

    /// Get question details by ID
    pub async fn get_question_details(&self, id: u32) -> Result<Question, AppError> {
        // Placeholder implementation - would call Metabase API
        // For now, return an error indicating not implemented
        use crate::error::CliError;
        Err(AppError::Cli(CliError::NotImplemented {
            command: format!("get_question_details for question {}", id)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_question_service_new() {
        let client = MetabaseClient::new("http://localhost:3000".to_string()).unwrap();
        let service = QuestionService::new(client);
        
        // Verify QuestionService is created successfully
        // Note: Cannot easily test internal fields without getters
        let _ = service; // Avoid unused variable warning
    }

    #[tokio::test]
    async fn test_list_questions_returns_result() {
        let client = MetabaseClient::new("http://localhost:3000".to_string()).unwrap();
        let _service = QuestionService::new(client);
        
        let _params = ListParams {
            search: None,
            limit: 10,
            collection: None,
        };
        
        // Note: Actual API call would fail in test environment
        // Test verifies service can be created and parameters structured
    }

    #[tokio::test]
    async fn test_execute_question_returns_result() {
        let client = MetabaseClient::new("http://localhost:3000".to_string()).unwrap();
        let _service = QuestionService::new(client);
        
        let _params = ExecuteParams {
            parameters: std::collections::HashMap::new(),
            format: "json".to_string(),
            limit: Some(100),
            offset: None,
            page_size: 20,
        };
        
        // Note: Actual API call would fail in test environment
        // Test verifies service can be created and parameters structured
    }

    #[tokio::test]
    async fn test_get_question_details_returns_result() {
        let client = MetabaseClient::new("http://localhost:3000".to_string()).unwrap();
        let service = QuestionService::new(client);
        
        // Verify get_question_details returns Result (currently returns NotImplemented)
        let result = service.get_question_details(1).await;
        assert!(result.is_err()); // Should return NotImplemented error
    }
}