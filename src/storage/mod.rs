//! Storage layer for MBR-CLI
//!
//! Handles configuration management, credential storage, and caching.
//! Uses OS keyring for secure credential storage and TOML for configuration files.

use crate::error::StorageError;

// TODO(human): Add the three main storage components as public modules:
// - config: Configuration file management (TOML)
// - credentials: Secure credential storage (OS keyring)
// - cache: Query result caching with TTL

type Result<T> = std::result::Result<T, StorageError>;
