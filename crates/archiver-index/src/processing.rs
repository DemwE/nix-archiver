//! Commit processing logic

use anyhow::{Context, Result};
use archiver_core::PackageEntry;
use git2::{Commit, Oid, Repository, TreeWalkMode, TreeWalkResult};
use rayon::prelude::*;
use regex::Regex;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::formatting::{format_duration, format_number, format_unix_timestamp};
use crate::indexer::Indexer;
use crate::nar::compute_nar_hash_for_blob;
use crate::parsers::extract_package_info_static;
use crate::stats::{CommitStats, IndexStats};

impl Indexer {
    /// Indexes all commits from the specified commit backwards
    /// Uses parallel processing to utilize multiple CPU cores
    pub fn index_from_commit(&self, commit_sha: &str, max_commits: Option<usize>, batch_size: usize) -> Result<IndexStats> {
        let start_time = Instant::now();
        let repo = Repository::open(&self.repo_path)
            .context("Failed to open repository")?;
        
        let oid = Oid::from_str(commit_sha)
            .context("Invalid commit SHA")?;
        
        let commit = repo.find_commit(oid)
            .context("Failed to find commit")?;
        
        // Log commit info
        let commit_time = commit.time().seconds();
        let commit_date = format_unix_timestamp(commit_time as u64);
        log::info!("From commit: {} ({})", &commit_sha[..12], commit_date);

        let stats = Arc::new(Mutex::new(IndexStats::default()));
        let mut revwalk = repo.revwalk()?;
        revwalk.push(commit.id())?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        // Collect commits in batches for parallel processing
        // Larger batch size = better CPU utilization
        // Default: 100 commits, configurable via CLI
        const FLUSH_INTERVAL: usize = 5; // Flush every N batches
        let mut batch = Vec::with_capacity(batch_size);
        let mut total_processed = 0;
        let mut batches_processed = 0;
        
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        for oid_result in revwalk {
            if let Some(max) = max_commits {
                if total_processed >= max {
                    log::info!("Reached max commit limit: {}", max);
                    break;
                }
            }

            let oid = oid_result.context("Failed to get commit OID")?;
            
            // Skip if already processed
            if self.db.is_commit_processed(&oid.to_string())? {
                let mut stats_lock = stats.lock().unwrap();
                stats_lock.skipped += 1;
                continue;
            }

            batch.push(oid);
            total_processed += 1;

            // Process batch when full or reached end
            if batch.len() >= batch_size {
                let commits_to_mark = self.process_batch(&batch, &stats)?;
                batches_processed += 1;
                
                let stats_lock = stats.lock().unwrap();
                let elapsed = start_time.elapsed();
                let commits_done = stats_lock.processed;
                let packages_inserted = stats_lock.packages_inserted;
                let packages_found = stats_lock.packages_found;
                
                // Calculate speed and ETA
                let speed = if elapsed.as_secs() > 0 {
                    commits_done as f64 / elapsed.as_secs_f64()
                } else {
                    0.0
                };
                
                let progress_pct = if let Some(max) = max_commits {
                    (commits_done as f64 / max as f64 * 100.0) as u32
                } else {
                    0
                };
                
                let eta_str = if let Some(max) = max_commits {
                    if speed > 0.0 {
                        let remaining = max.saturating_sub(commits_done);
                        let eta_secs = remaining as f64 / speed;
                        format_duration(Duration::from_secs_f64(eta_secs))
                    } else {
                        "calculating...".to_string()
                    }
                } else {
                    "unknown".to_string()
                };
                
                // Log progress
                if let Some(max) = max_commits {
                    log::info!(
                        "⚡ Batch #{} | Commits: {}/{} ({}%) | Packages: {} inserted ({} found) | Speed: {:.1} commits/s | ETA: {}",
                        batches_processed,
                        format_number(commits_done),
                        format_number(max),
                        progress_pct,
                        format_number(packages_inserted),
                        format_number(packages_found),
                        speed,
                        eta_str
                    );
                } else {
                    log::info!(
                        "⚡ Batch #{} | Commits: {} | Packages: {} inserted ({} found) | Speed: {:.1} commits/s | Elapsed: {}",
                        batches_processed,
                        format_number(commits_done),
                        format_number(packages_inserted),
                        format_number(packages_found),
                        speed,
                        format_duration(elapsed)
                    );
                }
                
                drop(stats_lock);
                
                // Flush less frequently to reduce I/O overhead
                if batches_processed % FLUSH_INTERVAL == 0 {
                    let flush_start = Instant::now();
                    self.db.flush()?;
                    let flush_time = flush_start.elapsed();
                    log::debug!("Database flushed after {} batches ({:.2}s flush time)", 
                        batches_processed, flush_time.as_secs_f64());
                    
                    // NOW mark commits as processed - only after successful flush
                    for (commit_sha, timestamp) in commits_to_mark.iter() {
                        self.db.mark_commit_processed(commit_sha, *timestamp)?;
                    }
                    log::debug!("Marked {} commits as processed", commits_to_mark.len());
                } else {
                    // Mark commits immediately if not flushing (will be flushed later)
                    for (commit_sha, timestamp) in commits_to_mark.iter() {
                        self.db.mark_commit_processed(commit_sha, *timestamp)?;
                    }
                }
                
                batch.clear();
            }
        }

        // Process remaining commits
        if !batch.is_empty() {
            let commits_to_mark = self.process_batch(&batch, &stats)?;
            
            // Always flush at the end
            self.db.flush()?;
            
            // Mark remaining commits as processed after final flush
            for (commit_sha, timestamp) in commits_to_mark.iter() {
                self.db.mark_commit_processed(commit_sha, *timestamp)?;
            }
            log::debug!("Marked {} final commits as processed", commits_to_mark.len());
        } else {
            // Flush even if no remaining commits (to ensure all data is persisted)
            self.db.flush()?;
        }
        
        let mut final_stats = match Arc::try_unwrap(stats) {
            Ok(mutex) => mutex.into_inner().unwrap(),
            Err(arc) => arc.lock().unwrap().clone(),
        };
        
        // Add timing information
        let total_time = start_time.elapsed();
        final_stats.elapsed_time = total_time;
        
        // Log final statistics
        log::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        log::info!("✅ Indexing completed!");
        log::info!("📊 Final Statistics:");
        log::info!("   • Total time:        {}", format_duration(total_time));
        log::info!("   • Commits processed: {} ({} new, {} skipped)",
            format_number(final_stats.processed),
            format_number(final_stats.processed),
            format_number(final_stats.skipped)
        );
        log::info!("   • Packages found:    {}", format_number(final_stats.packages_found));
        log::info!("   • Packages inserted: {} ({} duplicates filtered)",
            format_number(final_stats.packages_inserted),
            format_number(final_stats.packages_found.saturating_sub(final_stats.packages_inserted))
        );
        
        let avg_commit_speed = if total_time.as_secs() > 0 {
            final_stats.processed as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };
        let avg_package_speed = if total_time.as_secs() > 0 {
            final_stats.packages_inserted as f64 / total_time.as_secs_f64()
        } else {
            0.0
        };
        
        log::info!("   • Average speed:     {:.1} commits/s, {:.1} packages/s",
            avg_commit_speed, avg_package_speed
        );
        
        if final_stats.errors > 0 {
            log::warn!("   • Errors:            {} ({:.1}%)",
                final_stats.errors,
                (final_stats.errors as f64 / final_stats.processed as f64 * 100.0)
            );
        } else {
            log::info!("   • Errors:            0");
        }
        
        Ok(final_stats)
    }

    /// Processes a batch of commits in parallel
    /// Returns list of (commit_sha, timestamp) pairs to mark as processed after flush
    fn process_batch(&self, oids: &[Oid], stats: &Arc<Mutex<IndexStats>>) -> Result<Vec<(String, u64)>> {
        let repo_path = &self.repo_path;
        let version_regex = &self.version_regex;

        // Process commits in parallel using rayon
        let results: Vec<_> = oids.par_iter()
            .map(|oid| {
                // Each thread opens its own repository connection
                let repo = Repository::open(repo_path)?;
                let commit = repo.find_commit(*oid)?;
                
                log::debug!("Processing commit: {}", oid);
                
                let commit_stats = self.process_commit_with_repo(&repo, &commit, version_regex)?;
                
                // Return commit info to mark as processed later (after flush)
                let timestamp = commit.time().seconds() as u64;
                
                Ok::<_, anyhow::Error>((oid.to_string(), timestamp, commit_stats))
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

    /// Processes a single commit (thread-safe version)
    fn process_commit_with_repo(&self, repo: &Repository, commit: &Commit, version_regex: &Regex) -> Result<CommitStats> {
        let tree = commit.tree().context("Failed to get commit tree")?;
        let timestamp = commit.time().seconds() as u64;
        let commit_sha = commit.id().to_string();

        let mut stats = CommitStats::default();
        let db = &self.db;

        // Walk through the tree looking for .nix files in pkgs/ directory
        tree.walk(TreeWalkMode::PreOrder, |root, entry| {
            let full_path = format!("{}{}", root, entry.name().unwrap_or(""));
            
            // We're only interested in .nix files in pkgs/ directory
            if !full_path.starts_with("pkgs/") || !full_path.ends_with(".nix") {
                return TreeWalkResult::Ok;
            }

            // Get object and check if it's a blob (file)
            if let Ok(object) = entry.to_object(repo) {
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
                        if let Some(package_info) = extract_package_info_static(&full_path, content, version_regex, nar_hash) {
                            stats.packages_found += 1;

                            let entry = PackageEntry::new(
                                package_info.attr_name,
                                package_info.version,
                                commit_sha.clone(),
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

            TreeWalkResult::Ok
        })?;

        Ok(stats)
    }
}
