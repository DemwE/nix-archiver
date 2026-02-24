//! Error types for archiver-core

/// Errors specific to archiver-core
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Invalid package entry: {0}")]
    InvalidEntry(String),
    
    #[error("Invalid NAR hash format: {0}")]
    InvalidNarHash(String),
    
    #[error("Version parsing error: {0}")]
    VersionParsing(String),
}
