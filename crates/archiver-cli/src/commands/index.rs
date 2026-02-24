//! Index command implementation

use anyhow::{Context, Result};
use archiver_db::ArchiverDb;
use archiver_index::Indexer;
use std::path::PathBuf;

/// Indexes Nixpkgs repository
pub fn cmd_index(
    repo_path: PathBuf,
    from_commit: String,
    max_commits: Option<usize>,
    threads: Option<usize>,
    batch_size: usize,
    db: ArchiverDb,
) -> Result<()> {
    // Configure Rayon thread pool if specified
    let num_threads = if let Some(num_threads) = threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .context("Failed to configure thread pool")?;
        num_threads
    } else {
        rayon::current_num_threads()
    };
    
    // Log startup information
    log::info!("Starting indexing of repository at {:?}", repo_path);
    log::info!("Using {} threads for parallel processing", num_threads);
    log::info!("Batch size: {} commits", batch_size);
    if let Some(max) = max_commits {
        log::info!("Max commits: {}", max);
    }

    let indexer = Indexer::new(&repo_path, db)
        .context("Failed to create indexer")?;

    // If from_commit is "HEAD", resolve to concrete SHA
    let commit_sha = if from_commit == "HEAD" {
        resolve_head(&repo_path)?
    } else {
        from_commit
    };

    let _stats = indexer.index_from_commit(&commit_sha, max_commits, batch_size)
        .context("Failed to index repository")?;

    // Final stats are already logged by the indexer
    Ok(())
}

/// Resolves HEAD to concrete commit SHA
fn resolve_head(repo_path: &PathBuf) -> Result<String> {
    use git2::Repository;
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    Ok(commit.id().to_string())
}
