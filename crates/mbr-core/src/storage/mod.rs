use crate::error::StorageError;

pub mod config;
pub mod credentials;
// pub mod cache;

type Result<T> = std::result::Result<T, StorageError>;
