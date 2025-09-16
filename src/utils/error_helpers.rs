use crate::error::{ApiError, DisplayError};
use std::io;

/// Helper functions for standardizing error conversions across the codebase
/// This module reduces boilerplate and provides consistent error handling patterns.
/// Convert reqwest errors to ApiError with endpoint context
pub fn convert_request_error(error: reqwest::Error, endpoint: &str) -> ApiError {
    ApiError::Http {
        status: error.status().map(|s| s.as_u16()).unwrap_or(0),
        endpoint: endpoint.to_string(),
        message: error.to_string(),
    }
}

/// Convert timeout errors to ApiError with endpoint context
pub fn convert_timeout_error(endpoint: &str, timeout_secs: u64) -> ApiError {
    ApiError::Timeout {
        timeout_secs,
        endpoint: endpoint.to_string(),
    }
}

/// Convert JSON deserialization errors to ApiError with endpoint context
pub fn convert_json_error(error: reqwest::Error, endpoint: &str) -> ApiError {
    ApiError::Http {
        status: 0,
        endpoint: endpoint.to_string(),
        message: format!("JSON parse error: {}", error),
    }
}

/// Convert IO errors to DisplayError for terminal operations
pub fn convert_io_to_display_error(error: io::Error, operation: &str) -> DisplayError {
    DisplayError::TerminalOutput(format!("{}: {}", operation, error))
}

/// Convert crossterm errors to DisplayError
pub fn convert_crossterm_error(error: io::Error, operation: &str) -> DisplayError {
    DisplayError::TerminalOutput(format!("Terminal {}: {}", operation, error))
}

/// Helper macro for standardizing map_err patterns
#[macro_export]
macro_rules! map_api_error {
    ($result:expr, $endpoint:expr) => {
        $result.map_err(|e| $crate::utils::error_helpers::convert_request_error(e, $endpoint))
    };
}

/// Helper macro for timeout errors
#[macro_export]
macro_rules! map_timeout_error {
    ($endpoint:expr, $timeout_secs:expr) => {
        $crate::utils::error_helpers::convert_timeout_error($endpoint, $timeout_secs)
    };
}

/// Helper macro for JSON parsing errors
#[macro_export]
macro_rules! map_json_error {
    ($result:expr, $endpoint:expr) => {
        $result.map_err(|e| $crate::utils::error_helpers::convert_json_error(e, $endpoint))
    };
}

/// Helper macro for display errors
#[macro_export]
macro_rules! map_display_error {
    ($result:expr, $operation:expr) => {
        $result
            .map_err(|e| $crate::utils::error_helpers::convert_io_to_display_error(e, $operation))
    };
}

/// Helper macro for crossterm errors
#[macro_export]
macro_rules! map_crossterm_error {
    ($result:expr, $operation:expr) => {
        $result.map_err(|e| $crate::utils::error_helpers::convert_crossterm_error(e, $operation))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_timeout_error_simple() {
        let api_error = convert_timeout_error("/test", 30);
        match api_error {
            ApiError::Timeout {
                endpoint,
                timeout_secs,
            } => {
                assert_eq!(endpoint, "/test");
                assert_eq!(timeout_secs, 30);
            }
            _ => panic!("Expected Timeout error"),
        }
    }

    #[test]
    fn test_convert_timeout_error() {
        let api_error = convert_timeout_error("/test", 30);

        match api_error {
            ApiError::Timeout {
                endpoint,
                timeout_secs,
            } => {
                assert_eq!(endpoint, "/test");
                assert_eq!(timeout_secs, 30);
            }
            _ => panic!("Expected Timeout error"),
        }
    }

    #[test]
    fn test_convert_io_to_display_error() {
        let io_error = io::Error::new(io::ErrorKind::BrokenPipe, "test");
        let display_error = convert_io_to_display_error(io_error, "write");

        match display_error {
            DisplayError::TerminalOutput(msg) => assert!(msg.contains("write")),
            _ => panic!("Expected TerminalOutput error"),
        }
    }
}
