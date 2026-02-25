//! Archiver DB - Persistence layer with deduplication
//!
//! This crate manages the local Sled database, implementing deduplication logic:
//! for each unique package version, only the latest commit is stored.

mod database;

pub use database::ArchiverDb;

