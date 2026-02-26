//! Database operations and management

use archiver_core::PackageEntry;
use anyhow::{Context, Result};
use data_encoding::{BASE64, HEXLOWER};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// Compact binary storage format
// ---------------------------------------------------------------------------

/// Internal representation stored in sled. Uses raw bytes for SHA fields,
/// eliminating hex/base64 strings and JSON overhead.
///
/// Byte savings per entry (typical):
///   commit_sha: 40-char hex string → [u8; 20]  (-20 bytes)
///   nar_hash:   ~59-char SRI string → [u8; 32]  (-27 bytes)
///   JSON overhead (field names, punctuation) → 0 with bincode (-~50 bytes)
///   Total saving: ~97 bytes per entry
#[derive(Serialize, Deserialize)]
struct StoredEntry {
    attr_name: String,
    version: String,
    commit_sha: [u8; 20],
    nar_hash: [u8; 32],
    timestamp: u64,
    is_primary: bool,
}

/// Serialize a `PackageEntry` into compact binary bytes.
fn pack(entry: &PackageEntry) -> Result<Vec<u8>> {
    let sha_vec = HEXLOWER
        .decode(entry.commit_sha.to_ascii_lowercase().as_bytes())
        .context("Invalid commit SHA hex encoding")?;
    let mut commit_bytes = [0u8; 20];
    commit_bytes.copy_from_slice(&sha_vec);

    let b64 = entry
        .nar_hash
        .strip_prefix("sha256-")
        .context("NAR hash missing 'sha256-' prefix")?;
    let hash_vec = BASE64
        .decode(b64.as_bytes())
        .context("Invalid NAR hash base64 encoding")?;
    let mut nar_bytes = [0u8; 32];
    nar_bytes.copy_from_slice(&hash_vec);

    let stored = StoredEntry {
        attr_name: entry.attr_name.clone(),
        version: entry.version.clone(),
        commit_sha: commit_bytes,
        nar_hash: nar_bytes,
        timestamp: entry.timestamp,
        is_primary: entry.is_primary,
    };
    bincode::serialize(&stored).context("Failed to serialize PackageEntry")
}

/// Deserialize a `PackageEntry` from compact binary bytes.
fn unpack(bytes: &[u8]) -> Result<PackageEntry> {
    let stored: StoredEntry =
        bincode::deserialize(bytes).context("Failed to deserialize PackageEntry")?;
    Ok(PackageEntry {
        attr_name: stored.attr_name,
        version: stored.version,
        commit_sha: HEXLOWER.encode(&stored.commit_sha),
        nar_hash: format!("sha256-{}", BASE64.encode(&stored.nar_hash)),
        timestamp: stored.timestamp,
        is_primary: stored.is_primary,
    })
}

/// Main structure managing the database
pub struct ArchiverDb {
    /// Tree storing package entries (key: "attr_name:version")
    packages: sled::Tree,

    /// Tree tracking processed commits
    processed_commits: sled::Tree,

    /// Sled database instance
    db: Db,

    /// Path to the database directory (for size calculation)
    path: std::path::PathBuf,
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
            path: path.as_ref().to_path_buf(),
        })
    }

    /// Inserts package entry only if it's newer than existing one
    ///
    /// Deduplication logic: if an entry for the given version already exists,
    /// it is replaced only when the new entry has a newer timestamp.
    pub fn insert_if_better(&self, entry: &PackageEntry) -> Result<bool> {
        let key = entry.key();
        let new_value = pack(entry)
            .context("Failed to serialize PackageEntry")?;

        let was_inserted = self.packages.update_and_fetch(key.as_bytes(), |old_value| {
            match old_value {
                None => {
                    // No existing value - insert
                    Some(new_value.clone())
                }
                Some(old_bytes) => {
                    // Check timestamp of existing value
                    match unpack(old_bytes) {
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
            let final_entry = unpack(&final_value)
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
                let entry = unpack(&bytes)
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
            let entry = unpack(&value)
                .context("Failed to deserialize PackageEntry")?;
            results.push(entry);
        }

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(results)
    }

    /// Searches packages by prefix across all attr_names.
    /// e.g. query "python" matches python27, python311, python312, python313, ...
    /// Returns a map of attr_name → list of versions (sorted newest first).
    pub fn search_packages(&self, query: &str) -> Result<HashMap<String, Vec<PackageEntry>>> {
        let mut results: HashMap<String, Vec<PackageEntry>> = HashMap::new();

        for item in self.packages.scan_prefix(query.as_bytes()) {
            let (_, value) = item.context("Failed to read from database")?;
            let entry = unpack(&value)
                .context("Failed to deserialize PackageEntry")?;
            results.entry(entry.attr_name.clone()).or_default().push(entry);
        }

        // Sort each group by timestamp (newest first)
        for entries in results.values_mut() {
            entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        }

        Ok(results)
    }

    /// Searches packages by case-insensitive substring match anywhere in attr_name.
    ///
    /// Full-table scan used as fallback when prefix search returns no results.
    /// e.g. "biomejs" finds "vscode-extensions.biomejs.biome",
    /// "numpy" finds "python313Packages.numpy".
    pub fn search_packages_contains(&self, query: &str) -> Result<HashMap<String, Vec<PackageEntry>>> {
        let query_lower = query.to_ascii_lowercase();
        let mut results: HashMap<String, Vec<PackageEntry>> = HashMap::new();

        for item in self.packages.iter() {
            let (_, value) = item.context("Failed to read from database")?;
            let entry = unpack(&value).context("Failed to deserialize PackageEntry")?;
            if entry.attr_name.to_ascii_lowercase().contains(&query_lower) {
                results.entry(entry.attr_name.clone()).or_default().push(entry);
            }
        }

        for entries in results.values_mut() {
            entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        }

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

    /// Returns the total number of stored (attr_name, version) entries.
    pub fn version_count(&self) -> usize {
        self.packages.len()
    }

    /// Returns the number of distinct package attr_names.
    /// Scans only keys (no value deserialization) for performance.
    pub fn unique_package_count(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        for item in self.packages.iter().keys() {
            if let Ok(key) = item {
                // key format: "attr_name:version" — take bytes before first ':'
                let pos = key.iter().position(|&b| b == b':').unwrap_or(key.len());
                seen.insert(key[..pos].to_vec());
            }
        }
        seen.len()
    }

    /// Checks if database is empty (no packages indexed yet)
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.packages.is_empty())
    }

    /// Returns the number of processed commits
    pub fn processed_commit_count(&self) -> usize {
        self.processed_commits.len()
    }

    /// Returns total on-disk size of the database directory in bytes.
    /// Sums sizes of all files inside the sled directory recursively.
    pub fn db_size_bytes(&self) -> u64 {
        fn dir_size(path: &std::path::Path) -> u64 {
            let Ok(entries) = std::fs::read_dir(path) else { return 0; };
            entries.flatten().map(|e| {
                let p = e.path();
                if p.is_dir() {
                    dir_size(&p)
                } else {
                    e.metadata().map(|m| m.len()).unwrap_or(0)
                }
            }).sum()
        }
        dir_size(&self.path)
    }

    /// Flushes all pending operations to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush().context("Failed to flush database")?;
        Ok(())
    }
}
