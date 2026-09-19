//! Event bus error types

use malverde_core::MalverdeError;
use malverde_events::EventError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for event bus operations
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum BusError {
    #[error("Subscriber already registered for topic: {0}")]
    SubscriberAlreadyRegistered(String),
    #[error("No subscribers for topic: {0}")]
    NoSubscribers(String),
    #[error("Bus is closed")]
    BusClosed,
    #[error("Publish failed: {0}")]
    PublishFailed(String),
    #[error("Subscribe failed: {0}")]
    SubscribeFailed(String),
    #[error("Unsubscribe failed: {0}")]
    UnsubscribeFailed(String),
    #[error("Channel error: {0}")]
    ChannelError(String),
    #[error("Timeout waiting for event")]
    Timeout,
    #[error("Event processing error: {0}")]
    ProcessingError(String),
}

impl BusError {
    pub fn subscriber_already_registered(topic: impl Into<String>) -> Self {
        BusError::SubscriberAlreadyRegistered(topic.into())
    }
    pub fn no_subscribers(topic: impl Into<String>) -> Self {
        BusError::NoSubscribers(topic.into())
    }
    pub fn publish_failed(message: impl Into<String>) -> Self {
        BusError::PublishFailed(message.into())
    }
    pub fn channel_error(message: impl Into<String>) -> Self {
        BusError::ChannelError(message.into())
    }
}

impl From<BusError> for MalverdeError {
    fn from(err: BusError) -> Self {
        MalverdeError::Generic {
            message: err.to_string(),
            severity: malverde_core::ErrorSeverity::Error,
            context: None,
            source: Some("malverde-bus".to_string()),
        }
    }
}

impl From<EventError> for BusError {
    fn from(err: EventError) -> Self {
        BusError::ProcessingError(err.to_string())
    }
}

pub type BusResult<T> = Result<T, BusError>;
