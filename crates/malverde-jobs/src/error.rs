//! Job-specific error types

use malverde_core::MalverdeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for job operations
#[derive(Debug, Clone, PartialEq, Error, Serialize, Deserialize)]
pub enum JobError {
    #[error("Job not found: {0}")]
    NotFound(String),
    #[error("Job already exists: {0}")]
    AlreadyExists(String),
    #[error("Job is not in a valid state for this operation: {0}")]
    InvalidState(String),
    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Job timeout: {0}")]
    Timeout(String),
    #[error("Job was cancelled")]
    Cancelled,
    #[error("Checkpoint failed: {0}")]
    CheckpointFailed(String),
    #[error("No valid checkpoint found for recovery")]
    NoValidCheckpoint,
    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),
    #[error("Progress cannot decrease")]
    InvalidProgress,
    #[error("Job configuration is invalid: {0}")]
    InvalidConfiguration(String),
}

impl JobError {
    pub fn not_found(id: impl Into<String>) -> Self {
        JobError::NotFound(id.into())
    }
    pub fn already_exists(id: impl Into<String>) -> Self {
        JobError::AlreadyExists(id.into())
    }
    pub fn invalid_state(message: impl Into<String>) -> Self {
        JobError::InvalidState(message.into())
    }
    pub fn execution_failed(message: impl Into<String>) -> Self {
        JobError::ExecutionFailed(message.into())
    }
    pub fn timeout(message: impl Into<String>) -> Self {
        JobError::Timeout(message.into())
    }
    pub fn checkpoint_failed(message: impl Into<String>) -> Self {
        JobError::CheckpointFailed(message.into())
    }
    pub fn no_valid_checkpoint() -> Self {
        JobError::NoValidCheckpoint
    }
    pub fn recovery_failed(message: impl Into<String>) -> Self {
        JobError::RecoveryFailed(message.into())
    }
}

impl From<JobError> for MalverdeError {
    fn from(err: JobError) -> Self {
        MalverdeError::JobExecution {
            message: err.to_string(),
            job_id: None,
            state: None,
            context: None,
            source: Some("malverde-jobs".to_string()),
        }
    }
}

pub type JobResult<T> = Result<T, JobError>;
