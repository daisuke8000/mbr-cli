//! File system operations and path handling utilities
//!
//! This module provides utilities for file system operations, path manipulation,
//! and directory management across the application.

use crate::error::ConfigError;
use std::path::Path;

/// Ensure a directory exists, creating it if necessary
pub fn ensure_directory_exists<P: AsRef<Path>>(path: P) -> crate::Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| ConfigError::InvalidValue {
            field: "directory_path".to_string(),
            value: path.to_string_lossy().to_string(),
            reason: format!("Failed to create directory: {}", e),
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_directory_exists_creates_new_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let test_path = temp_dir.path().join("new_directory");

        assert!(!test_path.exists());
        ensure_directory_exists(&test_path).expect("Failed to create new directory");
        assert!(test_path.exists());
        assert!(test_path.is_dir());
    }

    #[test]
    fn test_ensure_directory_exists_handles_existing_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let test_path = temp_dir.path();

        // Should not error on existing directory
        ensure_directory_exists(test_path).expect("Should handle existing directory without error");
    }
}
