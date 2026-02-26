//! Tests for NAR hash computation

use archiver_index::nar::compute_nar_hash_for_blob;

// ── format ────────────────────────────────────────────────────────────────────

#[test]
fn test_nar_hash_sri_format() {
    let hash = compute_nar_hash_for_blob(b"Hello, Nix!").unwrap();
    // Must start with "sha256-"
    assert!(hash.starts_with("sha256-"), "unexpected prefix: {}", hash);
    // SHA256 base64 = 44 chars; total = 7 + 44 = 51
    assert_eq!(hash.len(), 51, "unexpected hash length: {}", hash);
}

// ── determinism ───────────────────────────────────────────────────────────────

#[test]
fn test_nar_hash_is_deterministic() {
    let content = b"Hello, Nix!";
    let h1 = compute_nar_hash_for_blob(content).unwrap();
    let h2 = compute_nar_hash_for_blob(content).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn test_nar_hash_differs_for_different_content() {
    let h1 = compute_nar_hash_for_blob(b"content_a").unwrap();
    let h2 = compute_nar_hash_for_blob(b"content_b").unwrap();
    assert_ne!(h1, h2);
}

// ── padding ───────────────────────────────────────────────────────────────────

#[test]
fn test_nar_hash_empty_file() {
    // Empty file is valid and produces a stable, non-empty hash.
    let hash = compute_nar_hash_for_blob(b"").unwrap();
    assert!(hash.starts_with("sha256-"));
    assert_eq!(hash.len(), 51);
}

#[test]
fn test_nar_hash_content_not_multiple_of_8() {
    // 5 bytes – requires padding, must still produce a valid hash
    let hash = compute_nar_hash_for_blob(b"12345").unwrap();
    assert!(hash.starts_with("sha256-"));
    assert_eq!(hash.len(), 51);
}

#[test]
fn test_nar_hash_content_exactly_8_bytes() {
    // 8 bytes – no padding needed
    let hash = compute_nar_hash_for_blob(b"12345678").unwrap();
    assert!(hash.starts_with("sha256-"));
    assert_eq!(hash.len(), 51);
}

#[test]
fn test_nar_hash_different_sizes_produce_different_hashes() {
    let h5 = compute_nar_hash_for_blob(b"12345").unwrap();
    let h8 = compute_nar_hash_for_blob(b"12345678").unwrap();
    assert_ne!(h5, h8);
}
