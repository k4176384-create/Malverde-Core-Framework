//! # Malverde Events
//!
//! Structured event system for the Malverde Framework.
//! Provides event creation, serialization, validation, and timeline reconstruction.

pub mod error;
pub mod event;
pub mod timeline;

pub use error::*;
pub use event::*;
pub use timeline::*;
