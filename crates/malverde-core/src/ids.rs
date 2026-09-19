//! Strongly-typed identifiers for the Malverde Framework

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;
use crate::MAX_ID_LENGTH;

/// Trait for all Malverde identifiers
pub trait MalverdeId: Sized + Clone + PartialEq + Eq + std::hash::Hash + fmt::Display + fmt::Debug + Serialize + for<'de> Deserialize<'de> {
    fn new() -> Self;
    fn from_string(s: &str) -> Result<Self, IdError>;
    fn as_str(&self) -> &str;
    fn validate(s: &str) -> bool;
}

fn validate_alphanumeric(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn validate_actor(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
}

fn validate_plugin(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

macro_rules! impl_id_type {
    ($name:ident, $validator:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl MalverdeId for $name {
            fn new() -> Self {
                $name(Uuid::new_v4().to_string())
            }
            fn from_string(s: &str) -> Result<Self, IdError> {
                if s.is_empty() { return Err(IdError::Empty); }
                if s.len() > MAX_ID_LENGTH { return Err(IdError::TooLong); }
                if !$validator(s) { return Err(IdError::InvalidCharacters); }
                Ok($name(s.to_string()))
            }
            fn as_str(&self) -> &str { &self.0 }
            fn validate(s: &str) -> bool { $validator(s) }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0) }
        }
        impl FromStr for $name {
            type Err = IdError;
            fn from_str(s: &str) -> Result<Self, Self::Err> { Self::from_string(s) }
        }
        impl From<String> for $name {
            fn from(s: String) -> Self { $name(s) }
        }
        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str { &self.0 }
        }
    };
}

/// Error type for identifier operations
#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum IdError {
    #[error("Identifier is empty")]
    Empty,
    #[error("Identifier is too long (max {MAX_ID_LENGTH} characters)")]
    TooLong,
    #[error("Identifier contains invalid characters")]
    InvalidCharacters,
    #[error("Invalid identifier format")]
    InvalidFormat,
}

impl_id_type!(ProjectId, validate_alphanumeric);
impl_id_type!(OperationId, validate_alphanumeric);
impl_id_type!(EventId, validate_alphanumeric);
impl_id_type!(JobId, validate_alphanumeric);
impl_id_type!(KnowledgeId, validate_alphanumeric);
impl_id_type!(MemoryId, validate_alphanumeric);
impl_id_type!(ActorId, validate_actor);
impl_id_type!(CheckpointId, validate_alphanumeric);
impl_id_type!(AuditId, validate_alphanumeric);
impl_id_type!(PluginId, validate_plugin);
impl_id_type!(EvidenceId, validate_alphanumeric);
impl_id_type!(PatternId, validate_alphanumeric);
