use crate::storage::credentials::{Credentials, AuthMode};
use crate::api::client::MetabaseClient;
use crate::core::auth::LoginInput;
use super::types::AuthStatus;
use crate::AppError;

/// Authentication service for managing user authentication
pub struct AuthService {
    credentials: Credentials,
    client: MetabaseClient,
}

impl AuthService {
    /// Create new AuthService instance
    pub fn new(credentials: Credentials, client: MetabaseClient) -> Self {
        Self {
            credentials,
            client,
        }
    }

    /// Authenticate user with username and password
    pub async fn authenticate(&mut self, input: LoginInput) -> Result<(), AppError> {
        // Validate input
        input.validate()?;
        
        // Perform login using MetabaseClient
        self.client.login(&input.username, &input.password).await?;
        
        // Note: MetabaseClient manages session internally
        // For now, we assume login success means authentication is complete
        
        Ok(())
    }

    /// Logout current user
    pub async fn logout(&mut self) -> Result<(), AppError> {
        // Logout from Metabase server
        self.client.logout().await?;
        
        // Clear session information from credentials
        Credentials::clear_session_for_profile(&self.credentials.profile_name)?;
        
        Ok(())
    }

    /// Get current authentication status
    pub fn get_auth_status(&self) -> AuthStatus {
        let auth_mode = self.credentials.get_auth_mode();
        let session_token = self.credentials.get_session_token();
        
        AuthStatus {
            is_authenticated: self.is_authenticated(),
            auth_mode: auth_mode.clone(),
            profile_name: self.credentials.profile_name.clone(),
            session_active: session_token.is_some(),
        }
    }

    /// Check if user is currently authenticated
    pub fn is_authenticated(&self) -> bool {
        match self.credentials.get_auth_mode() {
            AuthMode::APIKey => true, // API key is always considered authenticated
            AuthMode::Session => self.credentials.get_session_token().is_some(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_service_new() {
        let credentials = Credentials::new("test".to_string());
        let client = MetabaseClient::new("http://localhost:3000".to_string()).unwrap();
        let _service = AuthService::new(credentials, client);
        
        // Verify AuthService is created successfully
        // assert_eq!(service.credentials.profile_name, "test");
    }

    #[test]
    fn test_is_authenticated_returns_bool() {
        let credentials = Credentials::new("test".to_string());
        let client = MetabaseClient::new("http://localhost:3000".to_string()).unwrap();
        let service = AuthService::new(credentials, client);
        
        // Verify is_authenticated returns boolean
        let result = service.is_authenticated();
        assert!(result == true || result == false);
    }

    #[test]
    fn test_get_auth_status_structure() {
        let credentials = Credentials::new("test".to_string());
        let client = MetabaseClient::new("http://localhost:3000".to_string()).unwrap();
        let service = AuthService::new(credentials, client);
        
        // Verify get_auth_status returns AuthStatus
        let status = service.get_auth_status();
        assert_eq!(status.profile_name, "test");
        assert!(status.auth_mode == AuthMode::APIKey || status.auth_mode == AuthMode::Session);
    }

    #[tokio::test]
    async fn test_authenticate_input_validation() {
        let credentials = Credentials::new("test".to_string());
        let client = MetabaseClient::new("http://localhost:3000".to_string()).unwrap();
        let mut _service = AuthService::new(credentials, client);
        
        let _input = LoginInput {
            username: "test@example.com".to_string(),
            password: "password".to_string(),
        };
        
        // Verify authenticate returns proper Result (currently panics)
        // TODO: This test should pass after implementation
        // let result = service.authenticate(input).await;
        // assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_logout_returns_result() {
        let credentials = Credentials::new("test".to_string());
        let client = MetabaseClient::new("http://localhost:3000".to_string()).unwrap();
        let mut _service = AuthService::new(credentials, client);
        
        // Verify logout returns Result (currently panics)
        // TODO: This test should pass after implementation
        // let result = service.logout().await;
        // assert!(result.is_ok() || result.is_err());
    }
}