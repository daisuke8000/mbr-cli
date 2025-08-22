pub use error::AppError;

/// 4-Layer Architecture (dependency flow: CLI → Core → Storage → Utils)
/// Each layer only depends on layers below it, ensuring clean separation of concerns
pub mod cli; // Command-line interface - handles user input and command routing
pub mod core; // Business logic - contains services and domain models
pub mod storage; // Configuration and data persistence - manages files and credentials
pub mod utils; // Shared utilities and helpers - common functionality

/// Support modules (used across layers)
pub mod api; // Metabase API client - HTTP communication
pub mod display; // Output formatting - UI rendering and pagination
pub mod error; // Error handling - hierarchical error system

pub type Result<T> = std::result::Result<T, AppError>;
