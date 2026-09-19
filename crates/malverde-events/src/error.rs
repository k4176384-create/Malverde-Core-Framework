//! Event-specific error types

use malverde_core::MalverdeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for event operations
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum EventError {
    #[error("Invalid event: {0}")]
    InvalidEvent(String),
    #[error("Event validation failed: {0}")]
    ValidationFailed(String),
    #[error("Event timestamp is in the future")]
    FutureTimestamp,
    #[error("Event ID collision: {0}")]
    IdCollision(String),
    #[error("Invalid parent event reference: {0}")]
    InvalidParent(String),
    #[error("Event serialization failed: {0}")]
    SerializationFailed(String),
    #[error("Event deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("Event hash mismatch")]
    HashMismatch,
}

impl EventError {
    pub fn invalid_event(message: impl Into<String>) -> Self {
        EventError::InvalidEvent(message.into())
    }
    pub fn validation_failed(message: impl Into<String>) -> Self {
        EventError::ValidationFailed(message.into())
    }
}

impl From<EventError> for MalverdeError {
    fn from(err: EventError) -> Self {
        MalverdeError::Generic {
            message: err.to_string(),
            severity: malverde_core::ErrorSeverity::Error,
            context: None,
            source: Some("malverde-events".to_string()),
        }
    }
}

pub type EventResult<T> = Result<T, EventError>;
