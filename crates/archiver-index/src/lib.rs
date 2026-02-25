//! Archiver Index - ETL engine for indexing Nixpkgs
//!
//! This crate is responsible for:
//! - Iterating through Git history of Nixpkgs repository
//! - Parsing .nix files for version strings
//! - Generating NAR hashes from Git objects
//! - Saving results to database with deduplication
//! - Parallel processing of commits for better performance

mod formatting;
mod indexer;
pub mod nar;
pub mod parsers;
mod processing;
mod stats;

pub use indexer::Indexer;
pub use stats::{IndexStats, PackageInfo};

