//! # Malverde Learning
//!
//! Learning system for the Malverde Framework.
//! Provides pattern recognition, knowledge inference, and learning from data.

pub mod error;
pub mod learning;
pub mod inference;
pub mod patterns;

pub use error::*;
pub use learning::*;
pub use inference::*;
pub use patterns::*;
