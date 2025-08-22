//! Input validation and sanitization utilities
//!
//! This module provides utilities for validating and sanitizing user input,
//! configuration values, and API parameters.

use crate::error::CliError;

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
}
