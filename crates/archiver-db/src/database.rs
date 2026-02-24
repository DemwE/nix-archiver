//! Database operations and management

use archiver_core::PackageEntry;
use anyhow::{Context, Result};
use sled::Db;
use std::path::Path;

/// Main structure managing the database
pub struct ArchiverDb {
    /// Tree storing package entries (key: "attr_name:version")
    packages: sled::Tree,
    
    /// Tree tracking processed commits
    processed_commits: sled::Tree,
    
    /// Sled database instance
    db: Db,
}

impl ArchiverDb {
    /// Opens or creates a new database at the specified location
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path.as_ref())
            .with_context(|| format!("Failed to open database at {:?}", path.as_ref()))?;
        
        let packages = db
            .open_tree("packages")
            .context("Failed to open packages tree")?;
        
        let processed_commits = db
            .open_tree("processed_commits")
            .context("Failed to open processed_commits tree")?;
        
        Ok(Self {
            packages,
            processed_commits,
            db,
        })
    }

    /// Inserts package entry only if it's newer than existing one
    ///
    /// Deduplication logic: if an entry for the given version already exists,
    /// it is replaced only when the new entry has a newer timestamp.
    pub fn insert_if_better(&self, entry: &PackageEntry) -> Result<bool> {
        let key = entry.key();
        let new_value = serde_json::to_vec(entry)
            .context("Failed to serialize PackageEntry")?;

        let was_inserted = self.packages.update_and_fetch(key.as_bytes(), |old_value| {
            match old_value {
                None => {
                    // No existing value - insert
                    Some(new_value.clone())
                }
                Some(old_bytes) => {
                    // Check timestamp of existing value
                    match serde_json::from_slice::<PackageEntry>(old_bytes) {
                        Ok(old_entry) => {
                            if entry.timestamp > old_entry.timestamp {
                                // New entry is newer - overwrite
                                log::info!(
                                    "Updating {} from commit {} -> {} (newer timestamp)",
                                    key,
                                    &old_entry.commit_sha[..8],
                                    &entry.commit_sha[..8]
                                );
                                Some(new_value.clone())
                            } else {
                                // Old entry is newer - keep unchanged
                                Some(old_bytes.to_vec())
                            }
                        }
                        Err(_) => {
                            // Deserialization error - overwrite with warning
                            log::warn!("Corrupted entry for {}, overwriting", key);
                            Some(new_value.clone())
                        }
                    }
                }
            }
        })
        .context("Failed to update package entry")?;

        // Check if we actually inserted a new entry
        if let Some(final_value) = was_inserted {
            let final_entry: PackageEntry = serde_json::from_slice(&final_value)
                .context("Failed to deserialize final entry")?;
            Ok(final_entry.commit_sha == entry.commit_sha)
        } else {
            Ok(false)
        }
    }

    /// Retrieves a package entry by attribute name and version
    pub fn get(&self, attr_name: &str, version: &str) -> Result<Option<PackageEntry>> {
        let key = format!("{}:{}", attr_name, version);
        
        match self.packages.get(key.as_bytes())? {
            Some(bytes) => {
                let entry = serde_json::from_slice(&bytes)
                    .context("Failed to deserialize PackageEntry")?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Retrieves all versions of a given package
    pub fn get_all_versions(&self, attr_name: &str) -> Result<Vec<PackageEntry>> {
        let prefix = format!("{}:", attr_name);
        let mut results = Vec::new();

        for item in self.packages.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.context("Failed to read from database")?;
            let entry: PackageEntry = serde_json::from_slice(&value)
                .context("Failed to deserialize PackageEntry")?;
            results.push(entry);
        }

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(results)
    }

    /// Marks a commit as processed
    pub fn mark_commit_processed(&self, commit_sha: &str, timestamp: u64) -> Result<()> {
        self.processed_commits
            .insert(commit_sha.as_bytes(), &timestamp.to_le_bytes())
            .context("Failed to mark commit as processed")?;
        Ok(())
    }

    /// Checks if a commit has already been processed
    pub fn is_commit_processed(&self, commit_sha: &str) -> Result<bool> {
        Ok(self.processed_commits.contains_key(commit_sha.as_bytes())?)
    }

    /// Returns the number of stored packages
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Returns the number of processed commits
    pub fn processed_commit_count(&self) -> usize {
        self.processed_commits.len()
    }

    /// Flushes all pending operations to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush().context("Failed to flush database")?;
        Ok(())
    }
}
