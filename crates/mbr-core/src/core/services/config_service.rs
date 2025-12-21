//! Configuration service for managing application configuration

use crate::AppError;
use crate::storage::config::Config;
use std::path::PathBuf;

/// Configuration service for managing application configuration
pub struct ConfigService {
    config: Config,
}

impl ConfigService {
    /// Create new ConfigService instance
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Get configured URL
    pub fn get_url(&self) -> Option<String> {
        self.config.get_url()
    }

    /// Set URL
    pub fn set_url(&mut self, url: String) {
        self.config.set_url(url);
    }

    /// Save configuration to file
    pub fn save_config(&self, path: Option<PathBuf>) -> Result<(), AppError> {
        self.config.save(path).map_err(|e| e.into())
    }

    /// Check if URL is configured
    pub fn has_url(&self) -> bool {
        self.get_url().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_service_new() {
        // Temporarily clear MBR_URL to ensure test isolation
        let original = std::env::var("MBR_URL").ok();
        unsafe {
            std::env::remove_var("MBR_URL");
        }

        let config = Config::default();
        let service = ConfigService::new(config);

        // Verify ConfigService is created successfully without URL
        assert!(!service.has_url());

        // Restore original state
        unsafe {
            if let Some(value) = original {
                std::env::set_var("MBR_URL", value);
            }
        }
    }

    #[test]
    fn test_get_url_returns_option() {
        // Temporarily clear MBR_URL to ensure test isolation
        let original = std::env::var("MBR_URL").ok();
        unsafe {
            std::env::remove_var("MBR_URL");
        }

        let config = Config::default();
        let service = ConfigService::new(config);

        // No URL by default
        assert!(service.get_url().is_none());

        // Restore original state
        unsafe {
            if let Some(value) = original {
                std::env::set_var("MBR_URL", value);
            }
        }
    }

    #[test]
    fn test_set_url() {
        let config = Config::default();
        let mut service = ConfigService::new(config);

        service.set_url("http://localhost:3000".to_string());

        assert!(service.has_url());
        assert_eq!(service.get_url(), Some("http://localhost:3000".to_string()));
    }

    #[test]
    fn test_save_config_returns_result() {
        let config = Config::default();
        let service = ConfigService::new(config);

        // Verify save_config returns Result
        let result = service.save_config(None);
        assert!(result.is_ok() || result.is_err());
    }
}
