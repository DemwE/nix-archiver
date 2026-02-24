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
mod nar;
mod parsers;
mod processing;
mod stats;

pub use indexer::Indexer;
pub use stats::IndexStats;

#[cfg(test)]
mod tests {
    use regex::Regex;
    use crate::nar::compute_nar_hash_for_blob;
    use crate::parsers::{is_valid_version, extract_package_info_static};

    #[test]
    fn test_version_regex() {
        let indexer_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();
        
        let content = r#"
            pname = "nodejs";
            version = "14.17.0";
        "#;
        
        let caps = indexer_regex.captures(content).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "14.17.0");
    }

    #[test]
    fn test_pname_extraction() {
        let version_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();
        
        // Test pname extraction from content
        let content = r#"
            pname = "gitlab.vim";
            version = "0.1.1";
        "#;
        
        let info = extract_package_info_static(
            "pkgs/applications/editors/vim/plugins/gitlab-vim/default.nix",
            content,
            &version_regex,
            None,
        ).unwrap();
        
        assert_eq!(info.attr_name, "gitlab.vim");
        assert_eq!(info.version, "0.1.1");
    }

    #[test]
    fn test_fallback_to_path() {
        let version_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();
        
        // Test fallback to path when no pname
        let content = r#"
            version = "1.0.0";
        "#;
        
        let info = extract_package_info_static(
            "pkgs/development/libraries/mylib/default.nix",
            content,
            &version_regex,
            None,
        ).unwrap();
        
        assert_eq!(info.attr_name, "mylib");
        assert_eq!(info.version, "1.0.0");
    }

    #[test]
    fn test_attr_name_extraction() {
        // Test attribute name extraction from path
        let path = "pkgs/development/libraries/nodejs/default.nix";
        let parts: Vec<&str> = path.split('/').collect();
        assert_eq!(parts[3], "nodejs");
    }

    #[test]
    fn test_version_validation() {
        // Valid versions
        assert!(is_valid_version("14.17.0"));
        assert!(is_valid_version("1.2.3"));
        assert!(is_valid_version("2.0.0-alpha"));
        assert!(is_valid_version("3.1.4+build.123"));
        assert!(is_valid_version("20.20.0"));
        assert!(is_valid_version("v1.0.0"));
        
        // Invalid versions (Nix code)
        assert!(!is_valid_version("v${lib.head (lib.strings.splitString"));
        assert!(!is_valid_version("${version}"));
        assert!(!is_valid_version("lib.version"));
        assert!(!is_valid_version("(someFunction)"));
        assert!(!is_valid_version("{interpolation}"));
        
        // Invalid versions (no digits)
        assert!(!is_valid_version("invalid"));
        assert!(!is_valid_version(""));
    }

    #[test]
    fn test_nar_hash_computation() {
        // Test with simple content
        let content = b"Hello, Nix!";
        let hash = compute_nar_hash_for_blob(content).unwrap();
        
        // Verify format: sha256-<base64>
        assert!(hash.starts_with("sha256-"));
        assert!(hash.len() > 10); // Base64 encoding of SHA256 should be 44 chars + prefix
        
        // Test with empty file
        let empty_hash = compute_nar_hash_for_blob(b"").unwrap();
        assert!(empty_hash.starts_with("sha256-"));
        
        // Same content should produce same hash
        let hash2 = compute_nar_hash_for_blob(content).unwrap();
        assert_eq!(hash, hash2);
        
        // Different content should produce different hash
        let different_hash = compute_nar_hash_for_blob(b"Different content").unwrap();
        assert_ne!(hash, different_hash);
    }
    
    #[test]
    fn test_nar_hash_with_various_sizes() {
        // Test with content that doesn't need padding
        let content_8_bytes = b"12345678";
        let hash1 = compute_nar_hash_for_blob(content_8_bytes).unwrap();
        assert!(hash1.starts_with("sha256-"));
        
        // Test with content that needs padding (not multiple of 8)
        let content_5_bytes = b"12345";
        let hash2 = compute_nar_hash_for_blob(content_5_bytes).unwrap();
        assert!(hash2.starts_with("sha256-"));
        assert_ne!(hash1, hash2);
    }
}

