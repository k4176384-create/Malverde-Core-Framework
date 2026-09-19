//! # Malverde Storage
//!
//! SQLite-based persistence layer for the Malverde Framework.
//! Provides database initialization, migrations, and CRUD operations.

pub mod error;
pub mod db;
pub mod migration;
pub mod repository;

pub use error::*;
pub use db::*;
pub use migration::*;
pub use repository::*;
