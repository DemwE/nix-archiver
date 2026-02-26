//! Tests for database functionality

use archiver_core::PackageEntry;
use archiver_db::ArchiverDb;
use anyhow::Result;
use tempfile::TempDir;

// Valid 40-char hex commit SHAs and SRI nar hashes required by the binary
// storage format (commit_sha → [u8;20], nar_hash → "sha256-" + base64([u8;32])).
const SHA1: &str = "abc1234567890abcdef01234567890abcdef0123";
const SHA_OLD: &str = "0000000000000000000000000000000000000001";
const SHA_NEW: &str = "0000000000000000000000000000000000000002";
// BASE64([0u8; 32]) = 43 'A's + '=' = 44 chars
const NAR: &str = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

#[test]
fn test_insert_and_get() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;

    let entry = PackageEntry::new(
        "nodejs".to_string(),
        "14.17.0".to_string(),
        SHA1.to_string(),
        NAR.to_string(),
        1234567890,
    );

    db.insert_if_better(&entry)?;
    let retrieved = db.get("nodejs", "14.17.0")?;

    assert_eq!(retrieved, Some(entry));
    Ok(())
}

#[test]
fn test_deduplication_newer_wins() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;

    let old_entry = PackageEntry::new(
        "nodejs".to_string(),
        "14.17.0".to_string(),
        SHA_OLD.to_string(),
        NAR.to_string(),
        1000,
    );

    let new_entry = PackageEntry::new(
        "nodejs".to_string(),
        "14.17.0".to_string(),
        SHA_NEW.to_string(),
        NAR.to_string(),
        2000,
    );

    db.insert_if_better(&old_entry)?;
    db.insert_if_better(&new_entry)?;

    let retrieved = db.get("nodejs", "14.17.0")?;
    assert_eq!(retrieved.unwrap().commit_sha, SHA_NEW);
    Ok(())
}
