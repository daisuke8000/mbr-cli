use super::Result;
use crate::error::StorageError;
use dirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub default_profile: Option<String>,
    pub profiles: HashMap<String, Profile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    pub metabase_url: String,
    pub timeout_seconds: Option<u64>,
    pub cache_enabled: Option<bool>,
}

impl Config {
    pub fn default() -> Self {
        Self {
            default_profile: None,
            profiles: HashMap::new(),
        }
    }

    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let config_path = match path {
            Some(p) => p,
            None => Self::config_file_path()?,
        };

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path).map_err(|source| StorageError::FileIo {
            path: config_path.to_string_lossy().to_string(),
            source,
        })?;

        let config: Config =
            toml::from_str(&content).map_err(|_| StorageError::ConfigSaveFailed)?; // TODO: better error handling

        Ok(config)
    }

    pub fn save(&self, path: Option<PathBuf>) -> Result<()> {
        let config_path = match path {
            Some(p) => p,
            None => Self::config_file_path()?,
        };

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|source| StorageError::FileIo {
                path: parent.to_string_lossy().to_string(),
                source,
            })?;
        }

        let toml_content = toml::to_string(self).map_err(|_| StorageError::ConfigSaveFailed)?; // TODO: better error handling

        fs::write(&config_path, toml_content).map_err(|source| StorageError::FileIo {
            path: config_path.to_string_lossy().to_string(),
            source,
        })?;

        Ok(())
    }

    fn config_file_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().ok_or(StorageError::ConfigSaveFailed)?; // TODO: better error handling

        let app_config_dir = config_dir.join("mbr-cli");
        let config_file = app_config_dir.join("config.toml");

        Ok(config_file)
    }

    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    pub fn set_profile(&mut self, name: String, profile: Profile) {
        self.profiles.insert(name, profile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.default_profile, None);
        assert_eq!(config.profiles.len(), 0);
    }

    #[test]
    fn test_profile_management() {
        let mut config = Config::default();
        // Create a profile
        let profile = Profile {
            metabase_url: "http://example.test".to_string(),
            timeout_seconds: Some(30),
            cache_enabled: Some(true),
        };
        // Set the profile in the config
        config.set_profile("test".to_string(), profile.clone());
        // Get the profile
        let retrieved = config.get_profile("test");
        assert!(retrieved.is_some());
        if let Some(retrieved) = retrieved {
            assert_eq!(retrieved.metabase_url, profile.metabase_url);
            assert_eq!(retrieved.timeout_seconds, profile.timeout_seconds);
            assert_eq!(retrieved.cache_enabled, profile.cache_enabled);
        }
        // Nonexistent profile should return None
        assert!(config.get_profile("nonexistent").is_none());
    }

    #[test]
    fn test_config_load_save() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");

        // Create a sample config
        let mut config = Config::default();
        config.default_profile = Some("test".to_string());
        config.profiles.insert(
            "test".to_string(),
            Profile {
                metabase_url: "http://example.test".to_string(),
                timeout_seconds: Some(30),
                cache_enabled: Some(true),
            },
        );

        // Save the config to the temp directory
        config
            .save(Some(config_path.clone()))
            .expect("Failed to save config");

        // Load the config from the temp directory
        let loaded_config = Config::load(Some(config_path)).expect("Failed to load config");

        // Check if loaded config matches saved config
        assert_eq!(loaded_config.default_profile, config.default_profile);
        assert_eq!(loaded_config.profiles.len(), 1);
        assert!(loaded_config.get_profile("test").is_some());
    }

    #[test]
    fn test_load_nonexistent_file() {
        let config = Config::load(None);
        assert!(config.is_ok());

        let config = config.expect("Failed to load default config");
        assert_eq!(config.default_profile, None);
        assert_eq!(config.profiles.len(), 0);
    }
}
