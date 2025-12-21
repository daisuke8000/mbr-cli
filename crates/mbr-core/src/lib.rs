//! # mbr-core
//!
//! Core library for Metabase API interaction.
//!
//! This crate provides the shared functionality used by both `mbr-cli` and `mbr-tui`.
//! It implements a 4-layer clean architecture for maintainability and testability.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use mbr_core::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> mbr_core::Result<()> {
//!     // Load configuration
//!     let config = Config::load(None)?;
//!
//!     // Create API client
//!     let client = MetabaseClient::new("http://localhost:3000".to_string())?;
//!
//!     // List questions
//!     let questions = client.list_questions(None, Some(10), None).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture (4-Layer)
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │           API Layer                 │  HTTP client, request/response models
//! ├─────────────────────────────────────┤
//! │          Core Layer                 │  Business logic, services, domain models
//! ├─────────────────────────────────────┤
//! │        Storage Layer                │  Configuration, credentials persistence
//! ├─────────────────────────────────────┤
//! │         Utils Layer                 │  Validation, helpers, common utilities
//! └─────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`api`]: Metabase HTTP client and data models
//! - [`core`]: Business logic and service layer
//! - [`storage`]: Configuration and credential management
//! - [`utils`]: Shared utilities (validation, text formatting, etc.)
//! - [`display`]: Output formatting, tables, pagination
//! - [`error`]: Hierarchical error system with troubleshooting hints
//!
//! ## Feature Highlights
//!
//! - **API Key Authentication**: Simple authentication via `MBR_API_KEY` environment variable
//! - **Multi-Profile**: Manage multiple Metabase server configurations
//! - **Rich Error Handling**: Contextual errors with severity levels

pub use error::AppError;

/// Prelude module for convenient imports.
///
/// This module re-exports the most commonly used types, allowing users to
/// quickly get started with a single import:
///
/// ```rust,ignore
/// use mbr_core::prelude::*;
/// ```
pub mod prelude {
    // Error handling
    pub use crate::Result;
    pub use crate::error::AppError;

    // API client and models
    pub use crate::api::client::MetabaseClient;
    pub use crate::api::models::{Collection, QueryResult, Question};

    // Services
    pub use crate::core::services::config_service::ConfigService;
    pub use crate::core::services::question_service::QuestionService;

    // Storage
    pub use crate::storage::config::Config;
    pub use crate::storage::credentials::{get_api_key, has_api_key};

    // Display utilities
    pub use crate::display::TableDisplay;
}

/// Business logic layer - services and domain models.
///
/// Contains the service layer that orchestrates API calls and business rules:
/// - [`core::services::config_service`]: Configuration management
/// - [`core::services::question_service`]: Question operations
pub mod core;

/// Storage layer - configuration and credential persistence.
///
/// Manages persistent data:
/// - [`storage::config`]: TOML configuration with multi-profile support
/// - [`storage::credentials`]: API key retrieval from environment
pub mod storage;

/// Utilities layer - shared helpers and common functionality.
///
/// Provides reusable utilities:
/// - [`utils::validation`]: URL and API key validation
/// - [`utils::text`]: Text formatting and truncation
/// - [`utils::data`]: Data manipulation helpers
pub mod utils;

/// API layer - Metabase HTTP client and data models.
///
/// Handles communication with Metabase server:
/// - [`api::client`]: HTTP client with authentication support
/// - [`api::models`]: Request/response data structures
pub mod api;

/// Display layer - output formatting and UI components.
///
/// Provides formatting utilities for terminal output:
/// - [`display::table`]: Table rendering with Unicode support
/// - [`display::pagination`]: Paginated data handling
/// - [`display::progress`]: Progress indicators
pub mod display;

/// Error handling - hierarchical error system.
///
/// Provides structured error handling:
/// - Domain-specific error variants (API, Auth, Config, etc.)
/// - Severity levels (Critical, High, Medium, Low)
/// - Troubleshooting hints for common issues
pub mod error;

/// Convenient Result type alias using [`AppError`].
///
/// This allows functions to return `mbr_core::Result<T>` instead of
/// `std::result::Result<T, AppError>`.
pub type Result<T> = std::result::Result<T, AppError>;

// Re-export commonly used types at crate root for backward compatibility.
// These are hidden from documentation to encourage using the prelude module.
#[doc(hidden)]
pub use api::client::MetabaseClient;
#[doc(hidden)]
pub use api::models::{Collection, CurrentUser, QueryResult, Question};
#[doc(hidden)]
pub use core::services::config_service::ConfigService;
#[doc(hidden)]
pub use core::services::question_service::QuestionService;
#[doc(hidden)]
pub use storage::config::Config;
#[doc(hidden)]
pub use storage::credentials::{get_api_key, has_api_key};
