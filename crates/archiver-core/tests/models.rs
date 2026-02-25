//! Tests for core models

use archiver_core::PackageEntry;

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
