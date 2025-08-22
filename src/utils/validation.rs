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
}
