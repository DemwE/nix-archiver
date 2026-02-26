//! Tests for database functionality

use archiver_core::PackageEntry;
use archiver_db::ArchiverDb;
use anyhow::Result;
use tempfile::TempDir;

// ── fixtures ─────────────────────────────────────────────────────────────────
//
// Binary storage encodes commit_sha as [u8;20] and nar_hash as [u8;32], so
// both must be properly encoded values (40-char hex and valid SRI base64).

const SHA1: &str = "abc1234567890abcdef01234567890abcdef0123";
const SHA2: &str = "def1234567890abcdef01234567890abcdef0456";
const SHA_OLD: &str = "0000000000000000000000000000000000000001";
const SHA_NEW: &str = "0000000000000000000000000000000000000002";
/// BASE64([0u8; 32])  =  43 'A's + '='
const NAR: &str = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

fn node(ver: &str, sha: &str, ts: u64) -> PackageEntry {
    PackageEntry::new("nodejs".to_string(), ver.to_string(), sha.to_string(), NAR.to_string(), ts)
}

// ── insert / get ─────────────────────────────────────────────────────────────

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
    assert_eq!(db.get("nodejs", "14.17.0")?, Some(entry));
    Ok(())
}

#[test]
fn test_get_nonexistent_returns_none() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;
    assert_eq!(db.get("nonexistent", "0.0.0")?, None);
    Ok(())
}

// ── deduplication ────────────────────────────────────────────────────────────

#[test]
fn test_deduplication_newer_wins() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;

    db.insert_if_better(&node("14.17.0", SHA_OLD, 1000))?;
    db.insert_if_better(&node("14.17.0", SHA_NEW, 2000))?;

    assert_eq!(db.get("nodejs", "14.17.0")?.unwrap().commit_sha, SHA_NEW);
    Ok(())
}

#[test]
fn test_deduplication_older_does_not_overwrite() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;

    db.insert_if_better(&node("14.17.0", SHA_NEW, 2000))?;
    // Attempt to insert older entry — should be ignored
    db.insert_if_better(&node("14.17.0", SHA_OLD, 1000))?;

    assert_eq!(db.get("nodejs", "14.17.0")?.unwrap().commit_sha, SHA_NEW);
    Ok(())
}

// ── get_all_versions ─────────────────────────────────────────────────────────

#[test]
fn test_get_all_versions_sorted_newest_first() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;

    db.insert_if_better(&node("14.17.0", SHA1, 1000))?;
    db.insert_if_better(&node("16.0.0",  SHA2, 2000))?;
    db.insert_if_better(&node("18.0.0",  SHA_NEW, 3000))?;

    let versions = db.get_all_versions("nodejs")?;
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0].version, "18.0.0"); // newest first
    assert_eq!(versions[2].version, "14.17.0");
    Ok(())
}

// ── search_packages (prefix scan) ────────────────────────────────────────────

#[test]
fn test_search_packages_prefix() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;

    let py311 = PackageEntry::new("python311".to_string(), "3.11.14".to_string(), SHA1.to_string(), NAR.to_string(), 1000);
    let py312 = PackageEntry::new("python312".to_string(), "3.12.12".to_string(), SHA2.to_string(), NAR.to_string(), 2000);
    let node  = PackageEntry::new("nodejs".to_string(),    "20.0.0".to_string(),  SHA_NEW.to_string(), NAR.to_string(), 3000);

    db.insert_if_better(&py311)?;
    db.insert_if_better(&py312)?;
    db.insert_if_better(&node)?;

    let results = db.search_packages("python")?;
    assert_eq!(results.len(), 2, "prefix 'python' should match python311 and python312");
    assert!(results.contains_key("python311"));
    assert!(results.contains_key("python312"));
    assert!(!results.contains_key("nodejs"));
    Ok(())
}

#[test]
fn test_search_packages_exact_name() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;

    let e = PackageEntry::new("nodejs".to_string(), "20.0.0".to_string(), SHA1.to_string(), NAR.to_string(), 1000);
    db.insert_if_better(&e)?;

    let results = db.search_packages("nodejs")?;
    assert_eq!(results.len(), 1);
    assert!(results.contains_key("nodejs"));
    Ok(())
}

// ── commit tracking ──────────────────────────────────────────────────────────

#[test]
fn test_commit_tracking() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;

    assert!(!db.is_commit_processed(SHA1)?);
    db.mark_commit_processed(SHA1, 1234567890)?;
    assert!(db.is_commit_processed(SHA1)?);
    // Different SHA not affected
    assert!(!db.is_commit_processed(SHA2)?);
    Ok(())
}
