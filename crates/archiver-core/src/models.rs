//! Core data models for the archiver

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
