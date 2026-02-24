//! NAR hash computation for Git blobs

use anyhow::Result;
use data_encoding::BASE64;
use sha2::{Digest, Sha256};
use std::io::Write;

/// Computes NAR hash for a single file (Git blob)
/// Returns hash in SRI format: sha256-<base64>
pub(crate) fn compute_nar_hash_for_blob(content: &[u8]) -> Result<String> {
    // NAR format for a regular file:
    // - "nix-archive-1\0\0\0\0" (16 bytes, magic + padding)
    // - "(\0\0\0\0\0\0\0" (8 bytes, opening paren + padding)
    // - "type\0\0\0\0" (8 bytes)
    // - "regular\0" (8 bytes)
    // - "contents\0\0\0\0" (12 bytes, then padding to 8-byte boundary)
    // - file size as 8-byte little-endian
    // - file content
    // - padding to 8-byte boundary
    // - ")\0\0\0\0\0\0\0" (8 bytes, closing paren + padding)
    
    let mut nar_data = Vec::new();
    
    // Magic header + padding
    nar_data.extend_from_slice(b"nix-archive-1\0\0\0");
    
    // Opening paren + padding  
    nar_data.extend_from_slice(b"(\0\0\0\0\0\0\0");
    
    // type marker + padding
    nar_data.extend_from_slice(b"type\0\0\0\0");
    
    // "regular" + padding
    nar_data.extend_from_slice(b"regular\0");
    
    // "contents" marker
    nar_data.extend_from_slice(b"contents\0\0\0\0");
    
    // File size (8 bytes, little-endian)
    let size = content.len() as u64;
    nar_data.write_all(&size.to_le_bytes())?;
    
    // File content
    nar_data.write_all(content)?;
    
    // Padding to 8-byte boundary
    let padding_needed = (8 - (content.len() % 8)) % 8;
    for _ in 0..padding_needed {
        nar_data.write_all(&[0])?;
    }
    
    // Closing paren + padding
    nar_data.extend_from_slice(b")\0\0\0\0\0\0\0");
    
    // Calculate SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(&nar_data);
    let hash_bytes = hasher.finalize();
    
    // Encode in SRI format: sha256-<base64>
    let base64_hash = BASE64.encode(&hash_bytes);
    Ok(format!("sha256-{}", base64_hash))
}
