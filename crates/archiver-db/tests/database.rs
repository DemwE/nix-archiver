//! Tests for database functionality

use archiver_core::PackageEntry;
use archiver_db::ArchiverDb;
use anyhow::Result;
use tempfile::TempDir;

#[test]
fn test_insert_and_get() -> Result<()> {
    let tmp = TempDir::new()?;
    let db = ArchiverDb::open(tmp.path())?;

    let entry = PackageEntry::new(
        "nodejs".to_string(),
        "14.17.0".to_string(),
        "abc123".to_string(),
        "sha256-test".to_string(),
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
        "old123".to_string(),
        "sha256-old".to_string(),
        1000,
    );

    let new_entry = PackageEntry::new(
        "nodejs".to_string(),
        "14.17.0".to_string(),
        "new456".to_string(),
        "sha256-new".to_string(),
        2000,
    );

    db.insert_if_better(&old_entry)?;
    db.insert_if_better(&new_entry)?;

    let retrieved = db.get("nodejs", "14.17.0")?;
    assert_eq!(retrieved.unwrap().commit_sha, "new456");
    Ok(())
}
