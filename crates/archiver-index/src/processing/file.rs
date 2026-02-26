//! File processing logic

use archiver_core::PackageEntry;
use git2::{Oid, Repository};
use regex::Regex;

use crate::parsers::extract_packages_from_file;
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
    if let Ok(object) = repo.find_object(oid, None) {
        if let Some(blob) = object.as_blob() {
            if let Ok(content) = std::str::from_utf8(blob.content()) {
                let packages = extract_packages_from_file(full_path, content, version_regex);

                for package_info in packages {
                    stats.packages_found += 1;

                    let entry = PackageEntry::new(
                        package_info.attr_name,
                        package_info.version,
                        commit_sha.to_string(),
                        timestamp,
                    );

                    match db.insert_if_better(&entry) {
                        Ok(true) => stats.packages_inserted += 1,
                        Ok(false) => {},
                        Err(e) => {
                            log::warn!("Failed to insert package {}: {:?}", entry.key(), e);
                        }
                    }
                }
            }
        }
    }
}
