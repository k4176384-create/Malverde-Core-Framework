//! Storage error types

use malverde_core::MalverdeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for storage operations
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum StorageError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Database query failed: {0}")]
    QueryFailed(String),
    #[error("Table not found: {0}")]
    TableNotFound(String),
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Already exists: {0}")]
    AlreadyExists(String),
    #[error("Migration failed: {0}")]
    MigrationFailed(String),
    #[error("Integrity check failed: {0}")]
    IntegrityCheckFailed(String),
    #[error("Database is locked")]
    DatabaseLocked,
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("IO error: {0}")]
    IoError(String),
}

impl StorageError {
    pub fn connection_failed(message: impl Into<String>) -> Self {
        StorageError::ConnectionFailed(message.into())
    }
    pub fn query_failed(message: impl Into<String>) -> Self {
        StorageError::QueryFailed(message.into())
    }
    pub fn not_found(message: impl Into<String>) -> Self {
        StorageError::NotFound(message.into())
    }
    pub fn already_exists(message: impl Into<String>) -> Self {
        StorageError::AlreadyExists(message.into())
    }
    pub fn migration_failed(message: impl Into<String>) -> Self {
        StorageError::MigrationFailed(message.into())
    }
    pub fn integrity_check_failed(message: impl Into<String>) -> Self {
        StorageError::IntegrityCheckFailed(message.into())
    }
}

impl From<StorageError> for MalverdeError {
    fn from(err: StorageError) -> Self {
        MalverdeError::Database {
            message: err.to_string(),
            query: None,
            context: None,
            source: Some("malverde-storage".to_string()),
        }
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::DatabaseCorrupt(_) => StorageError::IntegrityCheckFailed(err.to_string()),
            rusqlite::Error::DatabaseLocked => StorageError::DatabaseLocked,
            rusqlite::Error::ForeignKeyConstraintViolation(_) => {
                StorageError::ConstraintViolation(err.to_string())
            }
            rusqlite::Error::UniqueConstraintViolation(_) => StorageError::AlreadyExists(err.to_string()),
            _ => StorageError::QueryFailed(err.to_string()),
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

pub type StorageResult<T> = Result<T, StorageError>;
