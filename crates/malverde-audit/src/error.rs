//! Audit-specific error types

use malverde_core::MalverdeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for audit operations
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum AuditError {
    #[error("Audit record not found: {0}")]
    NotFound(String),
    #[error("Audit record already exists: {0}")]
    AlreadyExists(String),
    #[error("Audit log is disabled")]
    Disabled,
    #[error("Invalid audit record: {0}")]
    InvalidRecord(String),
    #[error("Audit storage error: {0}")]
    StorageError(String),
    #[error("Audit query error: {0}")]
    QueryError(String),
    #[error("Audit serialization error: {0}")]
    SerializationError(String),
    #[error("Audit timestamp error: {0}")]
    TimestampError(String),
}

impl AuditError {
    pub fn not_found(id: impl Into<String>) -> Self {
        AuditError::NotFound(id.into())
    }
    pub fn already_exists(id: impl Into<String>) -> Self {
        AuditError::AlreadyExists(id.into())
    }
    pub fn disabled() -> Self {
        AuditError::Disabled
    }
    pub fn invalid_record(message: impl Into<String>) -> Self {
        AuditError::InvalidRecord(message.into())
    }
    pub fn storage_error(message: impl Into<String>) -> Self {
        AuditError::StorageError(message.into())
    }
    pub fn query_error(message: impl Into<String>) -> Self {
        AuditError::QueryError(message.into())
    }
}

impl From<AuditError> for MalverdeError {
    fn from(err: AuditError) -> Self {
        MalverdeError::Generic {
            message: err.to_string(),
            severity: malverde_core::ErrorSeverity::Error,
            context: None,
            source: Some("malverde-audit".to_string()),
        }
    }
}

pub type AuditResult<T> = Result<T, AuditError>;
