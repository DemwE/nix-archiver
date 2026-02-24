//! Archiver Core - Wspólne modele danych i logika generowania kodu Nix
//!
//! Ten crate definiuje podstawowe struktury danych używane w całym projekcie,
//! w tym `PackageEntry` oraz funkcje do generowania wyrażeń Nix.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Wpis pakietu w bazie danych
///
/// Reprezentuje konkretną wersję pakietu w konkretnym commicie Nixpkgs.
/// Dla każdej unikalnej wersji przechowywany jest tylko najnowszy commit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageEntry {
    /// Nazwa atrybutu w Nixpkgs (np. "nodejs", "python3")
    pub attr_name: String,
    
    /// Wersja pakietu (np. "14.17.0")
    pub version: String,
    
    /// SHA commita w Nixpkgs
    pub commit_sha: String,
    
    /// Hash NAR w formacie SRI (np. "sha256-...")
    pub nar_hash: String,
    
    /// Timestamp commita (Unix epoch)
    pub timestamp: u64,
    
    /// Czy to jest główna/aktywna wersja
    pub is_primary: bool,
}

impl PackageEntry {
    /// Tworzy nowy wpis pakietu
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

    /// Generuje klucz do przechowywania w bazie danych
    /// Format: "attr_name:version"
    pub fn key(&self) -> String {
        format!("{}:{}", self.attr_name, self.version)
    }

    /// Generuje blok `fetchTarball` w formacie Nix
    ///
    /// Przykład wyjścia:
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

    /// Generuje pełne wyrażenie Nix dla importu pakietu
    ///
    /// Przykład wyjścia:
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

/// Błędy specyficzne dla archiver-core
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
