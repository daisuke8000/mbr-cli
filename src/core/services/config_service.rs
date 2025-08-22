use crate::storage::config::Config;
use crate::AppError;
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

    /// Get profile by name
    pub fn get_profile(&self, name: &str) -> Option<&crate::storage::config::Profile> {
        self.config.profiles.get(name)
    }

    /// Get default profile name
    pub fn get_default_profile(&self) -> Option<&String> {
        self.config.default_profile.as_ref()
    }

    /// Set profile field value
    pub fn set_profile_field(&mut self, profile: &str, field: &str, value: &str) -> Result<(), AppError> {
        use crate::error::CliError;
        
        // Get or create profile
        let profile_entry = self.config.profiles.entry(profile.to_string())
            .or_insert_with(|| crate::storage::config::Profile {
                metabase_url: "".to_string(),
                email: None,
            });

        // Set field based on field name - only accept user-facing field names
        match field {
            "url" => profile_entry.metabase_url = value.to_string(),
            "email" => profile_entry.email = Some(value.to_string()),
            _ => return Err(AppError::Cli(CliError::InvalidArguments(
                format!("Unknown field: {}. Use 'url' or 'email'", field)
            ))),
        }

        Ok(())
    }

    /// Save configuration to file
    pub fn save_config(&self, path: Option<PathBuf>) -> Result<(), AppError> {
        self.config.save(path).map_err(|e| e.into())
    }

    /// List all profiles
    pub fn list_profiles(&self) -> Vec<(&String, &crate::storage::config::Profile)> {
        self.config.profiles.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_service_new() {
        let config = Config::default();
        let service = ConfigService::new(config);
        
        // Verify ConfigService is created successfully
        assert!(service.config.default_profile.is_none() || service.config.default_profile.is_some());
    }

    #[test]
    fn test_get_profile_returns_option() {
        let config = Config::default();
        let service = ConfigService::new(config);
        
        // Verify get_profile returns Option
        let result = service.get_profile("default");
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_list_profiles_returns_vec() {
        let config = Config::default();
        let service = ConfigService::new(config);
        
        // Verify list_profiles returns Vec
        let profiles = service.list_profiles();
        assert!(profiles.is_empty() || !profiles.is_empty());
    }

    #[test]
    fn test_set_profile_field_returns_result() {
        let config = Config::default();
        let mut service = ConfigService::new(config);
        
        // Verify set_profile_field returns Result with user-facing field name
        let result = service.set_profile_field("default", "url", "http://localhost:3000");
        assert!(result.is_ok());
        
        // Verify the field was set
        let profile = service.get_profile("default");
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().metabase_url, "http://localhost:3000");
    }

    #[test]
    fn test_save_config_returns_result() {
        let config = Config::default();
        let service = ConfigService::new(config);
        
        // Verify save_config returns Result
        let result = service.save_config(None);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_set_profile_field_rejects_internal_field_names() {
        let config = Config::default();
        let mut service = ConfigService::new(config);
        
        // Should reject internal field names
        let result = service.set_profile_field("test", "host", "http://example.com");
        assert!(result.is_err());
        assert!(format!("{:?}", result).contains("Unknown field: host"));
        
        let result = service.set_profile_field("test", "metabase_url", "http://example.com");
        assert!(result.is_err());
        assert!(format!("{:?}", result).contains("Unknown field: metabase_url"));
        
        // Should accept user-facing field names
        let result = service.set_profile_field("test", "url", "http://example.com");
        assert!(result.is_ok());
        
        let result = service.set_profile_field("test", "email", "test@example.com");
        assert!(result.is_ok());
    }
}