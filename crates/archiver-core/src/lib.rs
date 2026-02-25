//! Archiver Core - Shared data models and Nix code generation logic
//!
//! This crate defines the core data structures used throughout the project,
//! including `PackageEntry` and functions for generating Nix expressions.

mod models;
mod error;

pub use models::PackageEntry;
pub use error::CoreError;

