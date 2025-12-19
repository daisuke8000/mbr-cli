//! API Key credential management
//!
//! This module handles API key authentication via the MBR_API_KEY environment variable.
//! All authentication is stateless - no session tokens or keyring storage.

use std::env;

/// Get the API key from environment variable
///
/// Returns the value of MBR_API_KEY if set and non-empty, otherwise None.
pub fn get_api_key() -> Option<String> {
    env::var("MBR_API_KEY").ok().filter(|k| !k.is_empty())
}

/// Check if an API key is configured
pub fn has_api_key() -> bool {
    get_api_key().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_api_key_when_set() {
        // Save original state
        let original = env::var("MBR_API_KEY").ok();

        // Test with API key set
        unsafe {
            env::set_var("MBR_API_KEY", "test_api_key_123");
        }
        assert_eq!(get_api_key(), Some("test_api_key_123".to_string()));
        assert!(has_api_key());

        // Restore original state
        unsafe {
            match original {
                Some(value) => env::set_var("MBR_API_KEY", value),
                None => env::remove_var("MBR_API_KEY"),
            }
        }
    }

    #[test]
    fn test_get_api_key_when_empty() {
        // Save original state
        let original = env::var("MBR_API_KEY").ok();

        // Test with empty API key
        unsafe {
            env::set_var("MBR_API_KEY", "");
        }
        assert_eq!(get_api_key(), None);
        assert!(!has_api_key());

        // Restore original state
        unsafe {
            match original {
                Some(value) => env::set_var("MBR_API_KEY", value),
                None => env::remove_var("MBR_API_KEY"),
            }
        }
    }

    #[test]
    fn test_get_api_key_when_not_set() {
        // Save original state
        let original = env::var("MBR_API_KEY").ok();

        // Test with API key not set
        unsafe {
            env::remove_var("MBR_API_KEY");
        }
        assert_eq!(get_api_key(), None);
        assert!(!has_api_key());

        // Restore original state
        unsafe {
            match original {
                Some(value) => env::set_var("MBR_API_KEY", value),
                None => env::remove_var("MBR_API_KEY"),
            }
        }
    }
}
