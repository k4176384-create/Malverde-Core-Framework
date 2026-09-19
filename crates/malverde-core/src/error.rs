//! Error types for the Malverde Framework

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Debug, Info, Warning, Error, Critical, Fatal,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Default for ErrorSeverity { fn default() -> Self { ErrorSeverity::Error } }

/// Error context for providing additional information about an error
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ErrorContext {
    pub operation: String,
    pub component: String,
    pub actor: Option<String>,
    pub project: Option<String>,
    pub metadata: serde_json::Value,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>, component: impl Into<String>) -> Self {
        ErrorContext { operation: operation.into(), component: component.into(), actor: None, project: None, metadata: serde_json::Value::Null }
    }
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self { self.actor = Some(actor.into()); self }
    pub fn with_project(mut self, project: impl Into<String>) -> Self { self.project = Some(project.into()); self }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self { self.metadata = metadata; self }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.operation, self.component)
    }
}

/// Main error type for the Malverde Framework
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum MalverdeError {
    #[error("Validation error: {message}")]
    Validation { message: String, field: Option<String>, context: Option<ErrorContext> },
    #[error("Not found: {resource_type} with id '{resource_id}'")]
    NotFound { resource_type: String, resource_id: String, context: Option<ErrorContext> },
    #[error("Already exists: {resource_type} with id '{resource_id}'")]
    AlreadyExists { resource_type: String, resource_id: String, context: Option<ErrorContext> },
    #[error("Storage error: {message}")]
    Storage { message: String, path: Option<String>, context: Option<ErrorContext>, source: Option<String> },
    #[error("Serialization error: {message}")]
    Serialization { message: String, context: Option<ErrorContext>, source: Option<String> },
    #[error("Deserialization error: {message}")]
    Deserialization { message: String, context: Option<ErrorContext>, source: Option<String> },
    #[error("Database error: {message}")]
    Database { message: String, query: Option<String>, context: Option<ErrorContext>, source: Option<String> },
    #[error("Migration error: {message}")]
    Migration { message: String, from_version: Option<String>, to_version: Option<String>, context: Option<ErrorContext>, source: Option<String> },
    #[error("Job error: {message}")]
    JobExecution { message: String, job_id: Option<String>, state: Option<String>, context: Option<ErrorContext>, source: Option<String> },
    #[error("Checkpoint error: {message}")]
    Checkpoint { message: String, checkpoint_id: Option<String>, context: Option<ErrorContext>, source: Option<String> },
    #[error("Plugin error: {message}")]
    Plugin { message: String, plugin_id: Option<String>, context: Option<ErrorContext>, source: Option<String> },
    #[error("Configuration error: {message}")]
    Configuration { message: String, key: Option<String>, context: Option<ErrorContext> },
    #[error("Permission denied: {message}")]
    Permission { message: String, actor: Option<String>, operation: Option<String>, context: Option<ErrorContext> },
    #[error("State error: {message}")]
    State { message: String, current_state: Option<String>, desired_state: Option<String>, context: Option<ErrorContext> },
    #[error("Timeout error: {message}")]
    Timeout { message: String, duration: Option<String>, context: Option<ErrorContext> },
    #[error("Network error: {message}")]
    Network { message: String, url: Option<String>, context: Option<ErrorContext>, source: Option<String> },
    #[error("Error: {message}")]
    Generic { message: String, severity: ErrorSeverity, context: Option<ErrorContext>, source: Option<String> },
}

impl MalverdeError {
    pub fn validation(message: impl Into<String>) -> Self {
        MalverdeError::Validation { message: message.into(), field: None, context: None }
    }
    pub fn not_found(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        MalverdeError::NotFound { resource_type: resource_type.into(), resource_id: resource_id.into(), context: None }
    }
    pub fn already_exists(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        MalverdeError::AlreadyExists { resource_type: resource_type.into(), resource_id: resource_id.into(), context: None }
    }
    pub fn storage(message: impl Into<String>) -> Self {
        MalverdeError::Storage { message: message.into(), path: None, context: None, source: None }
    }
    pub fn database(message: impl Into<String>) -> Self {
        MalverdeError::Database { message: message.into(), query: None, context: None, source: None }
    }
    pub fn generic(message: impl Into<String>) -> Self {
        MalverdeError::Generic { message: message.into(), severity: ErrorSeverity::Error, context: None, source: None }
    }
    pub fn with_severity(message: impl Into<String>, severity: ErrorSeverity) -> Self {
        MalverdeError::Generic { message: message.into(), severity, context: None, source: None }
    }
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            MalverdeError::Validation { .. } => ErrorSeverity::Warning,
            MalverdeError::NotFound { .. } => ErrorSeverity::Warning,
            MalverdeError::AlreadyExists { .. } => ErrorSeverity::Warning,
            MalverdeError::Storage { .. } => ErrorSeverity::Error,
            MalverdeError::Serialization { .. } => ErrorSeverity::Error,
            MalverdeError::Deserialization { .. } => ErrorSeverity::Error,
            MalverdeError::Database { .. } => ErrorSeverity::Error,
            MalverdeError::Migration { .. } => ErrorSeverity::Critical,
            MalverdeError::JobExecution { .. } => ErrorSeverity::Error,
            MalverdeError::Checkpoint { .. } => ErrorSeverity::Error,
            MalverdeError::Plugin { .. } => ErrorSeverity::Error,
            MalverdeError::Configuration { .. } => ErrorSeverity::Error,
            MalverdeError::Permission { .. } => ErrorSeverity::Error,
            MalverdeError::State { .. } => ErrorSeverity::Error,
            MalverdeError::Timeout { .. } => ErrorSeverity::Warning,
            MalverdeError::Network { .. } => ErrorSeverity::Warning,
            MalverdeError::Generic { severity, .. } => *severity,
        }
    }
    pub fn message(&self) -> &str {
        match self {
            MalverdeError::Validation { message, .. } => message,
            MalverdeError::NotFound { message, .. } => message,
            MalverdeError::AlreadyExists { message, .. } => message,
            MalverdeError::Storage { message, .. } => message,
            MalverdeError::Serialization { message, .. } => message,
            MalverdeError::Deserialization { message, .. } => message,
            MalverdeError::Database { message, .. } => message,
            MalverdeError::Migration { message, .. } => message,
            MalverdeError::JobExecution { message, .. } => message,
            MalverdeError::Checkpoint { message, .. } => message,
            MalverdeError::Plugin { message, .. } => message,
            MalverdeError::Configuration { message, .. } => message,
            MalverdeError::Permission { message, .. } => message,
            MalverdeError::State { message, .. } => message,
            MalverdeError::Timeout { message, .. } => message,
            MalverdeError::Network { message, .. } => message,
            MalverdeError::Generic { message, .. } => message,
        }
    }
    pub fn is_retryable(&self) -> bool {
        matches!(self, MalverdeError::Timeout { .. } | MalverdeError::Network { .. } | MalverdeError::Database { .. })
    }
    pub fn is_fatal(&self) -> bool { self.severity() == ErrorSeverity::Fatal }
}

pub type MalverdeResult<T> = Result<T, MalverdeError>;

impl From<std::io::Error> for MalverdeError {
    fn from(err: std::io::Error) -> Self {
        MalverdeError::Storage { message: err.to_string(), path: None, context: None, source: None }
    }
}

impl From<serde_json::Error> for MalverdeError {
    fn from(err: serde_json::Error) -> Self {
        MalverdeError::Serialization { message: err.to_string(), context: None, source: None }
    }
}

impl From<uuid::Error> for MalverdeError {
    fn from(err: uuid::Error) -> Self { MalverdeError::generic(err.to_string()) }
}

impl From<chrono::ParseError> for MalverdeError {
    fn from(err: chrono::ParseError) -> Self { MalverdeError::generic(err.to_string()) }
}
