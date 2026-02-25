//! Tests for NAR hash computation

use archiver_index::nar::compute_nar_hash_for_blob;

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
