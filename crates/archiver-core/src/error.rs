//! Error types for archiver-core

/// Errors specific to archiver-core
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Invalid package entry: {0}")]
    InvalidEntry(String),
    
    #[error("Version parsing error: {0}")]
    VersionParsing(String),
}
