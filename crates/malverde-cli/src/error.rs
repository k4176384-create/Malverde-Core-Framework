//! CLI-specific error types

use malverde_core::MalverdeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for CLI operations
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum CliError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Missing required argument: {0}")]
    MissingArgument(String),
    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl CliError {
    pub fn command_not_found(name: impl Into<String>) -> Self {
        CliError::CommandNotFound(name.into())
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        CliError::InvalidArgument(message.into())
    }

    pub fn missing_argument(name: impl Into<String>) -> Self {
        CliError::MissingArgument(name.into())
    }

    pub fn execution_failed(message: impl Into<String>) -> Self {
        CliError::ExecutionFailed(message.into())
    }

    pub fn io_error(message: impl Into<String>) -> Self {
        CliError::IoError(message.into())
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        CliError::ParseError(message.into())
    }

    pub fn database_error(message: impl Into<String>) -> Self {
        CliError::DatabaseError(message.into())
    }

    pub fn config_error(message: impl Into<String>) -> Self {
        CliError::ConfigError(message.into())
    }
}

impl From<CliError> for MalverdeError {
    fn from(err: CliError) -> Self {
        MalverdeError::Generic {
            message: err.to_string(),
            severity: malverde_core::ErrorSeverity::Error,
            context: None,
            source: Some("malverde-cli".to_string()),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::io_error(err.to_string())
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError::parse_error(err.to_string())
    }
}

pub type CliResult<T> = Result<T, CliError>;
