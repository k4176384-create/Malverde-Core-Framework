//! Configuration error types

use malverde_core::MalverdeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for configuration operations
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),
    #[error("Invalid configuration format: {0}")]
    InvalidFormat(String),
    #[error("Configuration key not found: {0}")]
    KeyNotFound(String),
    #[error("Invalid configuration value: {0}")]
    InvalidValue(String),
    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),
    #[error("Configuration save failed: {0}")]
    SaveFailed(String),
    #[error("Configuration reload failed: {0}")]
    ReloadFailed(String),
    #[error("Configuration merge conflict: {0}")]
    MergeConflict(String),
}

impl ConfigError {
    pub fn file_not_found(path: impl Into<String>) -> Self {
        ConfigError::FileNotFound(path.into())
    }
    pub fn invalid_format(message: impl Into<String>) -> Self {
        ConfigError::InvalidFormat(message.into())
    }
    pub fn key_not_found(key: impl Into<String>) -> Self {
        ConfigError::KeyNotFound(key.into())
    }
    pub fn invalid_value(message: impl Into<String>) -> Self {
        ConfigError::InvalidValue(message.into())
    }
    pub fn validation_failed(message: impl Into<String>) -> Self {
        ConfigError::ValidationFailed(message.into())
    }
}

impl From<ConfigError> for MalverdeError {
    fn from(err: ConfigError) -> Self {
        MalverdeError::Configuration {
            message: err.to_string(),
            key: None,
            context: None,
        }
    }
}

pub type ConfigResult<T> = Result<T, ConfigError>;
