//! Archiver Core - Shared data models and Nix code generation logic
//!
//! This crate defines the core data structures used throughout the project,
//! including `PackageEntry` and functions for generating Nix expressions.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Package entry in the database
///
/// Represents a specific package version in a specific Nixpkgs commit.
/// For each unique version, only the latest commit is stored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageEntry {
    /// Attribute name in Nixpkgs (e.g., "nodejs", "python3")
    pub attr_name: String,
    
    /// Package version (e.g., "14.17.0")
    pub version: String,
    
    /// Commit SHA in Nixpkgs
    pub commit_sha: String,
    
    /// NAR hash in SRI format (e.g., "sha256-...")
    pub nar_hash: String,
    
    /// Commit timestamp (Unix epoch)
    pub timestamp: u64,
    
    /// Whether this is the primary/active version
    pub is_primary: bool,
}

impl PackageEntry {
    /// Creates a new package entry
    pub fn new(
        attr_name: String,
        version: String,
        commit_sha: String,
        nar_hash: String,
        timestamp: u64,
    ) -> Self {
        Self {
            attr_name,
            version,
            commit_sha,
            nar_hash,
            timestamp,
            is_primary: true,
        }
    }

    /// Generates a key for database storage
    /// Format: "attr_name:version"
    pub fn key(&self) -> String {
        format!("{}:{}", self.attr_name, self.version)
    }

    /// Generates a `fetchTarball` block in Nix format
    ///
    /// Example output:
    /// ```nix
    /// fetchTarball {
    ///   url = "https://github.com/NixOS/nixpkgs/archive/abc123.tar.gz";
    ///   sha256 = "sha256-...";
    /// }
    /// ```
    pub fn to_nix_fetchtarball(&self) -> String {
        format!(
            r#"fetchTarball {{
  url = "https://github.com/NixOS/nixpkgs/archive/{}.tar.gz";
  sha256 = "{}";
}}"#,
            self.commit_sha, self.nar_hash
        )
    }

    /// Generates a complete Nix expression for package import
    ///
    /// Example output:
    /// ```nix
    /// let
    ///   pkgs = import (fetchTarball { ... }) {};
    /// in
    ///   pkgs.nodejs
    /// ```
    pub fn to_nix_import(&self) -> String {
        format!(
            r#"let
  pkgs = import ({}) {{}};
in
  pkgs.{}"#,
            self.to_nix_fetchtarball(),
            self.attr_name
        )
    }
}

impl fmt::Display for PackageEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} @ {} ({})",
            self.attr_name,
            self.version,
            &self.commit_sha[..8],
            self.nar_hash
        )
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_entry_key() {
        let entry = PackageEntry::new(
            "nodejs".to_string(),
            "14.17.0".to_string(),
            "abc123".to_string(),
            "sha256-test".to_string(),
            1234567890,
        );
        assert_eq!(entry.key(), "nodejs:14.17.0");
    }

    #[test]
    fn test_nix_fetchtarball_generation() {
        let entry = PackageEntry::new(
            "nodejs".to_string(),
            "14.17.0".to_string(),
            "abc123".to_string(),
            "sha256-test".to_string(),
            1234567890,
        );
        let nix = entry.to_nix_fetchtarball();
        assert!(nix.contains("abc123.tar.gz"));
        assert!(nix.contains("sha256-test"));
    }
}
