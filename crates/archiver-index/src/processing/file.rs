//! File processing logic

use archiver_core::PackageEntry;
use git2::{Oid, Repository};
use regex::Regex;

use crate::nar::compute_nar_hash_for_blob;
use crate::parsers::extract_package_info_static;
use crate::stats::CommitStats;

/// Helper function to process a single file (shared between diff and tree walk)
pub(super) fn process_file(
    repo: &Repository,
    full_path: &str,
    oid: Oid,
    commit_sha: &str,
    timestamp: u64,
    db: &archiver_db::ArchiverDb,
    version_regex: &Regex,
    stats: &mut CommitStats,
) {
    // Get the blob from the tree
    if let Ok(object) = repo.find_object(oid, None) {
        if let Some(blob) = object.as_blob() {
            // Calculate NAR hash from blob content
            let nar_hash = match compute_nar_hash_for_blob(blob.content()) {
                Ok(hash) => Some(hash),
                Err(e) => {
                    log::debug!("Failed to compute NAR hash for {}: {}", full_path, e);
                    None
                }
            };
            
            if let Ok(content) = std::str::from_utf8(blob.content()) {
                // Try to extract package information
                if let Some(package_info) = extract_package_info_static(full_path, content, version_regex, nar_hash) {
                    stats.packages_found += 1;

                    let entry = PackageEntry::new(
                        package_info.attr_name,
                        package_info.version,
                        commit_sha.to_string(),
                        package_info.nar_hash.unwrap_or_else(|| "unknown".to_string()),
                        timestamp,
                    );

                    // Insert into database (with deduplication)
                    match db.insert_if_better(&entry) {
                        Ok(true) => stats.packages_inserted += 1,
                        Ok(false) => {},  // Not inserted - older version
                        Err(e) => {
                            log::warn!("Failed to insert package {}: {:?}", entry.key(), e);
                        }
                    }
                }
            }
        }
    }
}
