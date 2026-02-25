//! Commit processing logic

use anyhow::{Context, Result};
use git2::{Commit, Oid, Repository, TreeWalkMode, TreeWalkResult};
use rayon::prelude::*;
use regex::Regex;
use std::sync::{Arc, Mutex};

use crate::indexer::Indexer;
use crate::stats::{CommitStats, IndexStats};
use super::file::process_file;

impl Indexer {
    /// Processes a batch of commits in parallel
    /// Returns list of (commit_sha, timestamp) pairs to mark as processed after flush
    pub(super) fn process_batch(&self, oids: &[Oid], stats: &Arc<Mutex<IndexStats>>) -> Result<Vec<(String, u64)>> {
        let repo_path = &self.repo_path;
        let version_regex = &self.version_regex;

        // OPTIMIZATION: Split batch into chunks - each thread processes multiple commits
        // with ONE repository open, instead of opening repo for EACH commit!
        let num_threads = rayon::current_num_threads();
        let chunk_size = (oids.len() + num_threads - 1) / num_threads; // Round up
        
        let results: Vec<_> = oids.par_chunks(chunk_size.max(1))
            .flat_map(|chunk| {
                // Open repository ONCE per chunk (not per commit!)
                let repo = match Repository::open(repo_path) {
                    Ok(r) => r,
                    Err(e) => return vec![Err(anyhow::Error::from(e))],
                };
                
                // Process all commits in this chunk with same repo instance
                chunk.iter().map(|oid| {
                    let commit = repo.find_commit(*oid)?;
                
                    log::debug!("Processing commit: {}", oid);
                    
                    let commit_stats = self.process_commit_with_repo(&repo, &commit, version_regex)?;
                    
                    // Return commit info to mark as processed later (after flush)
                    let timestamp = commit.time().seconds() as u64;
                    
                    Ok::<_, anyhow::Error>((oid.to_string(), timestamp, commit_stats))
                }).collect::<Vec<_>>()
            })
            .collect();

        // Aggregate results and collect commits to mark
        let mut stats_lock = stats.lock().unwrap();
        let mut commits_to_mark = Vec::new();
        
        for result in results {
            match result {
                Ok((commit_sha, timestamp, commit_stats)) => {
                    stats_lock.processed += 1;
                    stats_lock.packages_found += commit_stats.packages_found;
                    stats_lock.packages_inserted += commit_stats.packages_inserted;
                    commits_to_mark.push((commit_sha, timestamp));
                }
                Err(e) => {
                    log::warn!("Failed to process commit: {:?}", e);
                    stats_lock.errors += 1;
                }
            }
        }

        Ok(commits_to_mark)
    }

    /// Processes a single commit with FULL tree walk (for initial HEAD scan)
    /// This indexes ALL packages in the commit to build complete database
    pub(super) fn process_commit_full_scan(&self, repo: &Repository, commit: &Commit) -> Result<CommitStats> {
        let tree = commit.tree().context("Failed to get commit tree")?;
        let timestamp = commit.time().seconds() as u64;
        let commit_sha = commit.id().to_string();
        let version_regex = &self.version_regex;

        let mut stats = CommitStats::default();
        let db = &self.db;

        // Walk entire tree to index all packages
        tree.walk(TreeWalkMode::PreOrder, |root, entry| {
            let full_path = format!("{}{}", root, entry.name().unwrap_or(""));
            
            // We're only interested in .nix files in pkgs/ directory
            if !full_path.starts_with("pkgs/") || !full_path.ends_with(".nix") {
                return TreeWalkResult::Ok;
            }

            // Get object and check if it's a blob (file)
            if let Ok(object) = entry.to_object(repo) {
                if let Some(blob) = object.as_blob() {
                    let oid = blob.id();
                    process_file(repo, &full_path, oid, &commit_sha, timestamp, db, version_regex, &mut stats);
                }
            }

            TreeWalkResult::Ok
        })?;

        Ok(stats)
    }

    /// Processes a single commit with DIFF optimization (only changed files)
    /// This is much faster than full tree walk - used after initial HEAD scan
    pub(super) fn process_commit_with_repo(&self, repo: &Repository, commit: &Commit, version_regex: &Regex) -> Result<CommitStats> {
        let tree = commit.tree().context("Failed to get commit tree")?;
        let timestamp = commit.time().seconds() as u64;
        let commit_sha = commit.id().to_string();

        let mut stats = CommitStats::default();
        let db = &self.db;

        // OPTIMIZATION: Use external git log to get changed files (much faster!)
        // Git's internal diff machinery is highly optimized with packfile deltas
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.repo_path)
            .arg("log")
            .arg("--name-only")
            .arg("--diff-filter=AM")  // Added or Modified only
            .arg("--format=")  // No commit message, just filenames
            .arg("-1")  // Only this commit
            .arg(&commit_sha)
            .output()
            .context("Failed to run git log")?;

        if !output.status.success() {
            anyhow::bail!("git log failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        let changed_files = String::from_utf8_lossy(&output.stdout);
        
        // Process each changed file
        for line in changed_files.lines() {
            let full_path = line.trim();
            if full_path.is_empty() {
                continue;
            }
            
            // We're only interested in .nix files in pkgs/ directory
            if !full_path.starts_with("pkgs/") || !full_path.ends_with(".nix") {
                continue;
            }

            // Get the file's OID from the tree
            if let Ok(entry) = tree.get_path(std::path::Path::new(full_path)) {
                process_file(repo, full_path, entry.id(), &commit_sha, timestamp, db, version_regex, &mut stats);
            }
        }

        Ok(stats)
    }
}
