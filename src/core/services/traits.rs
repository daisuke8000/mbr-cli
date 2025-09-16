use crate::core::services::types::ServiceError;
use async_trait::async_trait;

/// Common parameters for listing resources
#[derive(Debug, Clone, Default)]
pub struct ListParams {
    /// Search term to filter results
    pub search: Option<String>,
    /// Maximum number of results to return
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Trait for services that can list resources
#[async_trait]
pub trait ListService<T> {
    /// List all resources matching the given parameters
    async fn list(&self, params: ListParams) -> Result<Vec<T>, ServiceError>;
}

/// Trait for services that can retrieve individual resources
#[async_trait]
pub trait GetService<T> {
    /// Get a single resource by ID
    async fn get(&self, id: u32) -> Result<T, ServiceError>;
}

/// Trait for services that can create resources
#[async_trait]
pub trait CreateService<T, CreateInput> {
    /// Create a new resource
    async fn create(&self, input: CreateInput) -> Result<T, ServiceError>;
}

/// Trait for services that can update resources
#[async_trait]
pub trait UpdateService<T, UpdateInput> {
    /// Update an existing resource
    async fn update(&self, id: u32, input: UpdateInput) -> Result<T, ServiceError>;
}

/// Trait for services that can delete resources
#[async_trait]
pub trait DeleteService {
    /// Delete a resource by ID
    async fn delete(&self, id: u32) -> Result<(), ServiceError>;
}

/// Combined CRUD trait for full resource management
#[async_trait]
pub trait CrudService<T, CreateInput, UpdateInput>:
    ListService<T>
    + GetService<T>
    + CreateService<T, CreateInput>
    + UpdateService<T, UpdateInput>
    + DeleteService
{
}

/// Trait for services that support statistics
#[async_trait]
pub trait StatsService<T> {
    /// Get statistics for a resource
    async fn get_stats(&self, id: u32) -> Result<T, ServiceError>;
}

/// Trait for services that support hierarchical data
#[async_trait]
pub trait TreeService<T> {
    /// List resources in tree format
    async fn list_tree(&self) -> Result<Vec<T>, ServiceError>;
}

/// Helper macro to implement ListService for services with existing list methods
#[macro_export]
macro_rules! impl_list_service {
    ($service:ty, $item_type:ty) => {
        #[async_trait]
        impl ListService<$item_type> for $service {
            async fn list(&self, params: ListParams) -> Result<Vec<$item_type>, ServiceError> {
                // Use the service's existing list method with parameter mapping
                self.list_items(params.search.as_deref(), params.limit, params.offset)
                    .await
            }
        }
    };
}

/// Helper macro to implement GetService for services with existing get methods
#[macro_export]
macro_rules! impl_get_service {
    ($service:ty, $item_type:ty) => {
        #[async_trait]
        impl GetService<$item_type> for $service {
            async fn get(&self, id: u32) -> Result<$item_type, ServiceError> {
                // Use the service's existing get method
                self.get_item(id).await
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::services::types::ServiceError;

    // Mock service for testing traits
    struct MockService;

    #[async_trait]
    impl ListService<String> for MockService {
        async fn list(&self, _params: ListParams) -> Result<Vec<String>, ServiceError> {
            Ok(vec!["item1".to_string(), "item2".to_string()])
        }
    }

    #[async_trait]
    impl GetService<String> for MockService {
        async fn get(&self, _id: u32) -> Result<String, ServiceError> {
            Ok("test_item".to_string())
        }
    }

    #[tokio::test]
    async fn test_list_service() {
        let service = MockService;
        let params = ListParams {
            search: Some("test".to_string()),
            limit: Some(10),
            offset: Some(0),
        };

        let result = service.list(params).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_service() {
        let service = MockService;
        let result = service.get(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_item");
    }

    #[test]
    fn test_list_params_default() {
        let params = ListParams::default();
        assert!(params.search.is_none());
        assert!(params.limit.is_none());
        assert!(params.offset.is_none());
    }
}
