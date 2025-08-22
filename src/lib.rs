pub use error::AppError;

/// Main architecture layers (dependency flow: CLI → Core → Storage)
pub mod cli; // Command-line interface
pub mod core; // Business logic
pub mod storage; // Configuration and data persistence

/// Support modules (used across layers)
pub mod api; // Metabase API client
pub mod display; // Output formatting
pub mod error; // Error handling
pub mod utils; // Shared utilities and helpers

pub type Result<T> = std::result::Result<T, AppError>;
