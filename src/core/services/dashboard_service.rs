use crate::api::{client::MetabaseClient, models::Dashboard};
use crate::core::services::types::{ListParams, ServiceError};
use std::sync::Arc;

pub struct DashboardService {
    client: Arc<MetabaseClient>,
}

impl DashboardService {
    pub fn new(client: Arc<MetabaseClient>) -> Self {
        Self { client }
    }

    pub async fn list(&self, params: ListParams) -> Result<Vec<Dashboard>, ServiceError> {
        // Validate parameters
        if let Some(limit) = params.limit
            && limit == 0
        {
            return Err(ServiceError::Validation {
                field: "limit".to_string(),
                message: "Limit must be greater than 0".to_string(),
            });
        }

        // Call API client
        let search = params.search.as_deref();
        let dashboards = self.client.get_dashboards(search, params.limit).await?;

        Ok(dashboards)
    }

    pub async fn show(&self, id: u32) -> Result<Dashboard, ServiceError> {
        // Validate ID
        if id == 0 {
            return Err(ServiceError::Validation {
                field: "id".to_string(),
                message: "Dashboard ID must be greater than 0".to_string(),
            });
        }

        // Call API client
        let dashboard = self.client.get_dashboard(id).await.map_err(|e| {
            // Convert specific API errors to service errors
            match e {
                crate::error::ApiError::Http { status: 404, .. } => ServiceError::NotFound {
                    resource_type: "Dashboard".to_string(),
                    id,
                },
                _ => ServiceError::Api(e),
            }
        })?;

        Ok(dashboard)
    }

    pub async fn get_cards(
        &self,
        id: u32,
    ) -> Result<Vec<crate::api::models::DashboardCard>, ServiceError> {
        // Validate ID
        if id == 0 {
            return Err(ServiceError::Validation {
                field: "id".to_string(),
                message: "Dashboard ID must be greater than 0".to_string(),
            });
        }

        // Call API client
        let cards = self.client.get_dashboard_cards(id).await.map_err(|e| {
            // Convert specific API errors to service errors
            match e {
                crate::error::ApiError::Http { status: 404, .. } => ServiceError::NotFound {
                    resource_type: "Dashboard".to_string(),
                    id,
                },
                _ => ServiceError::Api(e),
            }
        })?;

        Ok(cards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Tests are focused on validation and service creation
    use std::sync::Arc;

    #[tokio::test]
    async fn test_dashboard_service_validation() {
        // Test validation without needing API client
        let client = MetabaseClient::new("http://test.example".to_string()).unwrap();
        let service = DashboardService::new(Arc::new(client));

        // Test list validation - zero limit should fail
        let params = ListParams {
            search: None,
            limit: Some(0),
            offset: None,
            collection: None,
        };

        let result = service.list(params).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::Validation { field, .. } => {
                assert_eq!(field, "limit");
            }
            _ => panic!("Expected validation error"),
        }

        // Test show validation - zero ID should fail
        let result = service.show(0).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::Validation { field, .. } => {
                assert_eq!(field, "id");
            }
            _ => panic!("Expected validation error"),
        }

        // Test get_cards validation - zero ID should fail
        let result = service.get_cards(0).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ServiceError::Validation { field, .. } => {
                assert_eq!(field, "id");
            }
            _ => panic!("Expected validation error"),
        }
    }

    #[test]
    fn test_dashboard_service_new() {
        // Test service creation
        let client = MetabaseClient::new("http://test.example".to_string()).unwrap();
        let service = DashboardService::new(Arc::new(client));

        // Verify service was created (basic smoke test)
        assert!(!service.client.is_authenticated());
    }
}
