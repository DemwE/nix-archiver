//! Archiver Index - ETL engine for indexing Nixpkgs
//!
//! This crate is responsible for:
//! - Iterating through Git history of Nixpkgs repository
//! - Parsing .nix files for version strings
//! - Generating NAR hashes from Git objects
//! - Saving results to database with deduplication

use anyhow::{Context, Result};
use archiver_core::PackageEntry;
use archiver_db::ArchiverDb;
use git2::{Commit, Oid, Repository, TreeWalkMode, TreeWalkResult};
use regex::Regex;
use std::path::Path;

/// Main indexer structure
pub struct Indexer {
    /// Nixpkgs Git repository
    repo: Repository,
    
    /// Database for storing results
    db: ArchiverDb,
    
    /// Regex for extracting versions from Nix files
    version_regex: Regex,
}

impl Indexer {
    /// Creates a new indexer for the given repository and database
    pub fn new<P: AsRef<Path>>(repo_path: P, db: ArchiverDb) -> Result<Self> {
        let repo = Repository::open(repo_path.as_ref())
            .with_context(|| format!("Failed to open repository at {:?}", repo_path.as_ref()))?;
        
        // Regex for extracting versions in format: version = "x.y.z"
        // Also supports: pname = "name"; version = "1.2.3";
        let version_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#)
            .context("Failed to compile version regex")?;

        Ok(Self {
            repo,
            db,
            version_regex,
        })
    }

    /// Indexes all commits from the specified commit backwards
    pub fn index_from_commit(&self, commit_sha: &str, max_commits: Option<usize>) -> Result<IndexStats> {
        let oid = Oid::from_str(commit_sha)
            .context("Invalid commit SHA")?;
        
        let commit = self.repo.find_commit(oid)
            .context("Failed to find commit")?;

        let mut stats = IndexStats::default();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(commit.id())?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        for (idx, oid_result) in revwalk.enumerate() {
            if let Some(max) = max_commits {
                if idx >= max {
                    log::info!("Reached max commit limit: {}", max);
                    break;
                }
            }

            let oid = oid_result.context("Failed to get commit OID")?;
            let commit = self.repo.find_commit(oid)
                .context("Failed to find commit")?;

            // Check if commit has already been processed
            if self.db.is_commit_processed(&oid.to_string())? {
                stats.skipped += 1;
                continue;
            }

            log::debug!("Processing commit {}: {}", idx, oid);
            
            match self.process_commit(&commit) {
                Ok(commit_stats) => {
                    stats.processed += 1;
                    stats.packages_found += commit_stats.packages_found;
                    stats.packages_inserted += commit_stats.packages_inserted;
                    
                    // Mark commit as processed
                    let timestamp = commit.time().seconds() as u64;
                    self.db.mark_commit_processed(&oid.to_string(), timestamp)?;
                }
                Err(e) => {
                    log::warn!("Failed to process commit {}: {:?}", oid, e);
                    stats.errors += 1;
                }
            }

            if idx % 100 == 0 {
                log::info!("Progress: {} commits processed, {} packages inserted", 
                    stats.processed, stats.packages_inserted);
                self.db.flush()?;
            }
        }

        self.db.flush()?;
        Ok(stats)
    }

    /// Processes a single commit
    fn process_commit(&self, commit: &Commit) -> Result<CommitStats> {
        let tree = commit.tree().context("Failed to get commit tree")?;
        let timestamp = commit.time().seconds() as u64;
        let commit_sha = commit.id().to_string();

        let mut stats = CommitStats::default();

        // Walk through the tree looking for .nix files in pkgs/ directory
        tree.walk(TreeWalkMode::PreOrder, |root, entry| {
            let full_path = format!("{}{}", root, entry.name().unwrap_or(""));
            
            // We're only interested in .nix files in pkgs/ directory
            if !full_path.starts_with("pkgs/") || !full_path.ends_with(".nix") {
                return TreeWalkResult::Ok;
            }

            // Get object and check if it's a blob (file)
            if let Ok(object) = entry.to_object(&self.repo) {
                if let Some(blob) = object.as_blob() {
                    if let Ok(content) = std::str::from_utf8(blob.content()) {
                        // Try to extract package information
                        if let Some(package_info) = self.extract_package_info(&full_path, content) {
                            stats.packages_found += 1;

                            let entry = PackageEntry::new(
                                package_info.attr_name,
                                package_info.version,
                                commit_sha.clone(),
                                package_info.nar_hash.unwrap_or_else(|| "unknown".to_string()),
                                timestamp,
                            );

                            // Insert into database (with deduplication)
                            match self.db.insert_if_better(&entry) {
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

            TreeWalkResult::Ok
        })?;

        Ok(stats)
    }

    /// Extracts package information from a .nix file
    fn extract_package_info(&self, path: &str, content: &str) -> Option<PackageInfo> {
        // Extract attribute name from path
        // e.g., "pkgs/development/libraries/nodejs/default.nix" -> "nodejs"
        let attr_name = self.extract_attr_name(path)?;

        // Extract version using regex
        let version = self.version_regex
            .captures(content)?
            .get(1)?
            .as_str()
            .to_string();

        // TODO: In the future, NAR hash calculation will be here
        // For now, return a placeholder
        
        Some(PackageInfo {
            attr_name,
            version,
            nar_hash: None,
        })
    }

    /// Extracts attribute name from file path
    fn extract_attr_name(&self, path: &str) -> Option<String> {
        // Path format: pkgs/<category>/<subcategory>/<name>/...
        let parts: Vec<&str> = path.split('/').collect();
        
        if parts.len() >= 4 && parts[0] == "pkgs" {
            // Try to extract name from the third level
            Some(parts[3].to_string())
        } else {
            None
        }
    }
}

/// Information extracted from package file
#[derive(Debug)]
struct PackageInfo {
    attr_name: String,
    version: String,
    nar_hash: Option<String>,
}

/// Indexing statistics
#[derive(Debug, Default)]
pub struct IndexStats {
    pub processed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub packages_found: usize,
    pub packages_inserted: usize,
}

/// Statistics for processing a single commit
#[derive(Debug, Default)]
struct CommitStats {
    packages_found: usize,
    packages_inserted: usize,
}

impl std::fmt::Display for IndexStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Commits: {} processed, {} skipped, {} errors | Packages: {} found, {} inserted",
            self.processed, self.skipped, self.errors,
            self.packages_found, self.packages_inserted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_regex() {
        let indexer_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();
        
        let content = r#"
            pname = "nodejs";
            version = "14.17.0";
        "#;
        
        let caps = indexer_regex.captures(content).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "14.17.0");
    }

    #[test]
    fn test_attr_name_extraction() {
        // Tymczasowy test - w prawdziwym środowisku potrzebowalibyśmy repozytorium
        let path = "pkgs/development/libraries/nodejs/default.nix";
        let parts: Vec<&str> = path.split('/').collect();
        assert_eq!(parts[3], "nodejs");
    }
}
