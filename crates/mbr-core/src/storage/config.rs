//! Configuration management
//!
//! Simple configuration with URL stored in config file or environment variable.
//! Priority: CLI argument > MBR_URL environment variable > config.toml

use super::Result;
use crate::error::StorageError;
use dirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application configuration
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    /// Metabase server URL
    pub url: Option<String>,
}

impl Config {
    /// Load configuration from file
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let config_path = match path {
            Some(p) => p,
            None => Self::config_file_path()?,
        };

        if !config_path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&config_path).map_err(|source| StorageError::FileIo {
            path: config_path.to_string_lossy().to_string(),
            source,
        })?;

        let config: Config =
            toml::from_str(&content).map_err(|e| StorageError::ConfigParseError {
                message: format!("Failed to parse config file: {}", e),
            })?;

        Ok(config)
    }

    /// Save configuration to file
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

        let toml_content = toml::to_string(self).map_err(|e| StorageError::ConfigParseError {
            message: format!("Failed to serialize config: {}", e),
        })?;

        fs::write(&config_path, toml_content).map_err(|source| StorageError::FileIo {
            path: config_path.to_string_lossy().to_string(),
            source,
        })?;

        Ok(())
    }

    fn config_file_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or(StorageError::ConfigDirNotFound)?;

        let app_config_dir = home_dir.join(".config").join("mbr-cli");
        let config_file = app_config_dir.join("config.toml");

        Ok(config_file)
    }

    /// Get URL with fallback to environment variable
    pub fn get_url(&self) -> Option<String> {
        self.url
            .clone()
            .or_else(|| std::env::var("MBR_URL").ok().filter(|s| !s.is_empty()))
    }

    /// Set URL
    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.url.is_none());
    }

    #[test]
    fn test_url_management() {
        let mut config = Config::default();
        assert!(config.url.is_none());

        config.set_url("http://example.test".to_string());
        assert_eq!(config.url, Some("http://example.test".to_string()));
    }

    #[test]
    fn test_config_load_save() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");

        // Create a sample config
        let mut config = Config::default();
        config.set_url("http://example.test".to_string());

        // Save the config
        config
            .save(Some(config_path.clone()))
            .expect("Failed to save config");

        // Load the config
        let loaded_config = Config::load(Some(config_path)).expect("Failed to load config");

        // Check if loaded config matches saved config
        assert_eq!(loaded_config.url, Some("http://example.test".to_string()));
    }

    #[test]
    fn test_load_nonexistent_file() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let nonexistent_path = temp_dir.path().join("nonexistent.toml");

        // Load from a path that doesn't exist
        let config = Config::load(Some(nonexistent_path));
        assert!(config.is_ok());

        let config = config.expect("Failed to load default config");
        assert!(config.url.is_none());
    }
}
