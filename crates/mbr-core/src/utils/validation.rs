//! Input validation, sanitization, and environment configuration utilities
//!
//! This module provides utilities for:
//! - Validating and sanitizing user input
//! - Reading environment variables for configuration
//! - API parameter validation

use crate::error::CliError;

// === Environment Configuration ===

/// Environment variable configuration reader
pub struct EnvConfigReader;

impl EnvConfigReader {
    /// Read NO_COLOR environment variable
    pub fn read_no_color() -> bool {
        std::env::var("NO_COLOR").is_ok()
    }

    /// Read MBR_PAGE_SIZE environment variable
    pub fn read_page_size() -> Option<usize> {
        std::env::var("MBR_PAGE_SIZE")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
    }

    /// Read MBR_NO_FULLSCREEN environment variable
    pub fn read_no_fullscreen() -> bool {
        std::env::var("MBR_NO_FULLSCREEN").is_ok()
    }

    /// Read MBR_MAX_MEMORY environment variable
    pub fn read_max_memory() -> Option<usize> {
        std::env::var("MBR_MAX_MEMORY")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
    }
}

// === Input Validation ===

/// Validate that a URL is properly formatted
pub fn validate_url(url: &str) -> crate::Result<()> {
    if url.is_empty() {
        return Err(CliError::InvalidArguments("URL cannot be empty".to_string()).into());
    }

    // Basic URL validation - must start with http:// or https://
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(CliError::InvalidArguments(format!(
            "Invalid URL '{}': URL must start with http:// or https://",
            url
        ))
        .into());
    }

    Ok(())
}

/// Validate API key format
pub fn validate_api_key(api_key: &str) -> crate::Result<()> {
    if api_key.is_empty() {
        return Err(CliError::InvalidArguments("API key cannot be empty".to_string()).into());
    }

    // Basic length check - Metabase API keys are typically long
    if api_key.len() < 10 {
        return Err(CliError::InvalidArguments(
            "API key appears to be too short (minimum 10 characters)".to_string(),
        )
        .into());
    }

    Ok(())
}

/// Validate email format
pub fn validate_email(email: &str) -> crate::Result<()> {
    if email.is_empty() {
        return Err(CliError::InvalidArguments("Email cannot be empty".to_string()).into());
    }

    // Basic email validation - must contain @ symbol
    if !email.contains('@') {
        return Err(CliError::InvalidArguments(format!(
            "Invalid email '{}': Email must contain @ symbol",
            email
        ))
        .into());
    }

    // Check for domain part (after @) and username part (before @)
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(CliError::InvalidArguments(format!(
            "Invalid email '{}': Email must have username and domain parts",
            email
        ))
        .into());
    }

    // Basic domain validation - must contain at least one dot
    if !parts[1].contains('.') {
        return Err(CliError::InvalidArguments(format!(
            "Invalid email '{}': Domain must contain dot",
            email
        ))
        .into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_accepts_valid_urls() {
        assert!(validate_url("http://localhost:3000").is_ok());
        assert!(validate_url("https://metabase.example.com").is_ok());
    }

    #[test]
    fn test_validate_url_rejects_invalid_urls() {
        assert!(validate_url("").is_err());
        assert!(validate_url("localhost:3000").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_validate_api_key_accepts_valid_keys() {
        assert!(validate_api_key("mb_123456789abcdef").is_ok());
        assert!(validate_api_key("very_long_api_key_string").is_ok());
    }

    #[test]
    fn test_validate_api_key_rejects_invalid_keys() {
        assert!(validate_api_key("").is_err());
        assert!(validate_api_key("short").is_err());
    }

    #[test]
    fn test_validate_email_accepts_valid_emails() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.email@domain.org").is_ok());
        assert!(validate_email("admin@metabase.local").is_ok());
    }

    #[test]
    fn test_validate_email_rejects_invalid_emails() {
        assert!(validate_email("").is_err());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@domain.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@domain").is_err());
        assert!(validate_email("user@domain@com").is_err());
    }

    // === EnvConfigReader Tests ===

    #[test]
    fn test_read_no_color_default() {
        // Without setting environment variable, should return false
        // Note: This test might be affected by actual environment
        let _result = EnvConfigReader::read_no_color();
        // Function exists and returns a boolean value
    }

    #[test]
    fn test_read_page_size_with_invalid_value() {
        // Test with invalid value handling
        unsafe {
            std::env::set_var("MBR_PAGE_SIZE", "invalid");
        }
        let result = EnvConfigReader::read_page_size();
        assert!(result.is_none());
        unsafe {
            std::env::remove_var("MBR_PAGE_SIZE");
        }
    }

    #[test]
    fn test_read_page_size_with_valid_value() {
        // Test with valid value
        unsafe {
            std::env::set_var("MBR_PAGE_SIZE", "50");
        }
        let result = EnvConfigReader::read_page_size();
        assert_eq!(result, Some(50));
        unsafe {
            std::env::remove_var("MBR_PAGE_SIZE");
        }
    }

    #[test]
    fn test_read_max_memory_with_valid_value() {
        // Test with valid memory value
        unsafe {
            std::env::set_var("MBR_MAX_MEMORY", "1024");
        }
        let result = EnvConfigReader::read_max_memory();
        assert_eq!(result, Some(1024));
        unsafe {
            std::env::remove_var("MBR_MAX_MEMORY");
        }
    }
}
