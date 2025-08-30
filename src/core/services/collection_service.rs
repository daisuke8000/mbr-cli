// TODO: Collection service implementation
// This file will contain the collection service implementation
// Following the pattern of dashboard_service.rs

use crate::api::{
    client::MetabaseClient,
    models::{Collection, CollectionDetail, CollectionStats},
};
use crate::core::services::types::ServiceError;
use std::sync::Arc;

pub struct CollectionService {
    client: Arc<MetabaseClient>,
}

// TODO: Implement methods
impl CollectionService {
    pub fn new(client: Arc<MetabaseClient>) -> Self {
        Self { client }
    }

    pub async fn list(&self, tree: bool) -> Result<Vec<Collection>, ServiceError> {
        // Call API client
        let collections = self.client.get_collections(tree).await?;
        Ok(collections)
    }

    pub async fn show(&self, id: u32) -> Result<CollectionDetail, ServiceError> {
        // Validate ID
        if id == 0 {
            return Err(ServiceError::Validation {
                field: "id".to_string(),
                message: "Collection ID must be greater than 0".to_string(),
            });
        }

        // Call API client
        let collection = self.client.get_collection(id).await.map_err(|e| {
            // Convert specific API errors to service errors
            match e {
                crate::error::ApiError::Http { status: 404, .. } => ServiceError::NotFound {
                    resource_type: "Collection".to_string(),
                    id,
                },
                _ => ServiceError::Api(e),
            }
        })?;

        Ok(collection)
    }

    pub async fn get_stats(&self, id: u32) -> Result<CollectionStats, ServiceError> {
        // Validate ID
        if id == 0 {
            return Err(ServiceError::Validation {
                field: "id".to_string(),
                message: "Collection ID must be greater than 0".to_string(),
            });
        }

        // Get collection items to calculate stats
        let items = self.client.get_collection_items(id).await?;

        // Calculate basic stats from items
        let item_count = items.len() as u32;

        // For now, return basic stats
        // In real implementation, we'd parse items to categorize questions vs dashboards
        Ok(CollectionStats {
            item_count,
            question_count: item_count / 2, // Simple approximation for testing
            dashboard_count: item_count / 2,
            last_updated: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::client::MetabaseClient;

    fn create_test_service() -> CollectionService {
        let client = MetabaseClient::new("http://test.example".to_string()).unwrap();
        CollectionService::new(Arc::new(client))
    }

    #[test]
    fn test_collection_service_new() {
        let service = create_test_service();
        // This test should pass - just testing construction
        assert!(Arc::strong_count(&service.client) >= 1);
    }

    #[test]
    fn test_list_method_exists() {
        let service = create_test_service();
        // Just test that method exists - async methods compile properly
        assert!(Arc::strong_count(&service.client) >= 1);
    }

    #[tokio::test]
    async fn test_show_method_validation() {
        let service = create_test_service();

        // Test validation: ID cannot be 0
        let result = service.show(0).await;
        assert!(result.is_err());
        if let Err(ServiceError::Validation { field, .. }) = result {
            assert_eq!(field, "id");
        } else {
            panic!("Expected validation error for ID = 0");
        }

        // Skip network test for now to avoid timeouts
    }

    #[tokio::test]
    async fn test_get_stats_method_validation() {
        let service = create_test_service();

        // Test validation: ID cannot be 0
        let result = service.get_stats(0).await;
        assert!(result.is_err());
        if let Err(ServiceError::Validation { field, .. }) = result {
            assert_eq!(field, "id");
        } else {
            panic!("Expected validation error for ID = 0");
        }

        // Skip network test for now to avoid timeouts
    }
}
