//! Learning-specific error types

use malverde_core::MalverdeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for learning operations
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum LearningError {
    #[error("Learning data not found: {0}")]
    NotFound(String),
    #[error("Learning data already exists: {0}")]
    AlreadyExists(String),
    #[error("Invalid learning data: {0}")]
    InvalidData(String),
    #[error("Learning configuration error: {0}")]
    ConfigurationError(String),
    #[error("Learning storage error: {0}")]
    StorageError(String),
    #[error("Pattern recognition failed: {0}")]
    PatternRecognitionFailed(String),
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    #[error("Training failed: {0}")]
    TrainingFailed(String),
    #[error("Learning rate error: {0}")]
    LearningRateError(String),
    #[error("Convergence error: {0}")]
    ConvergenceError(String),
}

impl LearningError {
    pub fn not_found(id: impl Into<String>) -> Self {
        LearningError::NotFound(id.into())
    }

    pub fn already_exists(id: impl Into<String>) -> Self {
        LearningError::AlreadyExists(id.into())
    }

    pub fn invalid_data(message: impl Into<String>) -> Self {
        LearningError::InvalidData(message.into())
    }

    pub fn configuration_error(message: impl Into<String>) -> Self {
        LearningError::ConfigurationError(message.into())
    }

    pub fn storage_error(message: impl Into<String>) -> Self {
        LearningError::StorageError(message.into())
    }

    pub fn pattern_recognition_failed(message: impl Into<String>) -> Self {
        LearningError::PatternRecognitionFailed(message.into())
    }

    pub fn inference_failed(message: impl Into<String>) -> Self {
        LearningError::InferenceFailed(message.into())
    }

    pub fn training_failed(message: impl Into<String>) -> Self {
        LearningError::TrainingFailed(message.into())
    }
}

impl From<LearningError> for MalverdeError {
    fn from(err: LearningError) -> Self {
        MalverdeError::Generic {
            message: err.to_string(),
            severity: malverde_core::ErrorSeverity::Error,
            context: None,
            source: Some("malverde-learning".to_string()),
        }
    }
}

pub type LearningResult<T> = Result<T, LearningError>;
