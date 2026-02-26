//! Tests for core data models

use archiver_core::PackageEntry;

// ── fixtures ─────────────────────────────────────────────────────────────────

fn make_entry() -> PackageEntry {
    PackageEntry::new(
        "nodejs".to_string(),
        "14.17.0".to_string(),
        "abc1234567890abcdef01234567890abcdef0123".to_string(),
        1234567890,
    )
}

// ── key ──────────────────────────────────────────────────────────────────────

#[test]
fn test_package_entry_key() {
    assert_eq!(make_entry().key(), "nodejs:14.17.0");
}

#[test]
fn test_key_uses_attr_name_and_version() {
    let e = PackageEntry::new(
        "charliermarsh.ruff".to_string(),
        "2026.36.0".to_string(),
        "abc1234567890abcdef01234567890abcdef0123".to_string(),
        0,
    );
    assert_eq!(e.key(), "charliermarsh.ruff:2026.36.0");
}

// ── nix generation ───────────────────────────────────────────────────────────

#[test]
fn test_nix_fetchtarball_contains_sha() {
    let nix = make_entry().to_nix_fetchtarball();
    assert!(nix.contains("abc1234567890abcdef01234567890abcdef0123.tar.gz"));
}

#[test]
fn test_nix_import_contains_pkgs_and_attr() {
    let nix = make_entry().to_nix_import();
    assert!(nix.contains("import"));
    assert!(nix.contains("pkgs.nodejs"));
}

// ── display ──────────────────────────────────────────────────────────────────

#[test]
fn test_display_format_contains_name_version_and_short_sha() {
    let s = make_entry().to_string();
    assert!(s.contains("nodejs"));
    assert!(s.contains("14.17.0"));
    // Display uses first 8 chars of commit SHA
    assert!(s.contains("abc12345"));
}
