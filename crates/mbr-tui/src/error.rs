//! Error types for mbr-tui.
//!
//! Provides domain-specific error handling for the TUI application.

// Allow unused for Phase 5 foundation - will be integrated in Phase 6
#![allow(dead_code)]

use std::io;
use thiserror::Error;

/// TUI-specific error type.
#[derive(Error, Debug)]
pub enum TuiError {
    /// Terminal I/O error.
    #[error("Terminal error: {0}")]
    Terminal(#[from] io::Error),

    /// Service/API error.
    #[error("Service error: {0}")]
    Service(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Event handling error.
    #[error("Event error: {0}")]
    Event(String),
}

/// Result type alias for TUI operations.
pub type TuiResult<T> = Result<T, TuiError>;

impl From<&str> for TuiError {
    fn from(msg: &str) -> Self {
        TuiError::Service(msg.to_string())
    }
}
