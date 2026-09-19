//! # Malverde Core
//!
//! Core types and domain models for the Malverde Framework.

pub mod error;
pub mod ids;
pub mod models;
pub mod states;
pub mod timestamps;
pub mod trust;

pub use error::*;
pub use ids::*;
pub use models::*;
pub use states::*;
pub use timestamps::*;
pub use trust::*;

/// Magic bytes for Malverde data identification
pub const MAGIC_BYTES: &[u8; 4] = b"MLVD";
/// Current framework version
pub const FRAMEWORK_VERSION: &str = "0.1.0";
/// Maximum length for string identifiers
pub const MAX_ID_LENGTH: usize = 64;
/// Maximum length for descriptions
pub const MAX_DESCRIPTION_LENGTH: usize = 1024;
/// Maximum length for payload content
pub const MAX_PAYLOAD_LENGTH: usize = 10 * 1024 * 1024;
