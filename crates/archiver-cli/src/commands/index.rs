//! Index command implementation

use anyhow::{Context, Result};
use archiver_db::ArchiverDb;
use archiver_index::Indexer;
use std::path::PathBuf;

/// Indexes Nixpkgs repository
pub fn cmd_index(
    repo_path: PathBuf,
    from_commit: String,
    to_commit: Option<String>,
    to_date: Option<String>,
    max_commits: Option<usize>,
    full_repo: bool,
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

    let indexer = Indexer::new(&repo_path, db)
        .context("Failed to create indexer")?;

    // If from_commit is "HEAD", resolve to concrete SHA
    let from_sha = if from_commit == "HEAD" {
        resolve_head(&repo_path)?
    } else {
        from_commit
    };

    // Calculate max_commits based on to_commit, to_date, or full_repo
    let computed_max_commits = if full_repo {
        log::info!("Indexing entire repository history (no limit)");
        None
    } else if let Some(to_date_str) = to_date {
        log::info!("Indexing until date: {}", to_date_str);
        let to_sha = resolve_commit_by_date(&repo_path, &to_date_str)?;
        let count = count_commits_between(&repo_path, &from_sha, &to_sha)?;
        log::info!("Found {} commits between {} and date {}", count, &from_sha[..8], to_date_str);
        Some(count)
    } else if let Some(to_sha) = to_commit {
        log::info!("Indexing until commit: {}", &to_sha[..12]);
        let count = count_commits_between(&repo_path, &from_sha, &to_sha)?;
        log::info!("Found {} commits between {} and {}", count, &from_sha[..8], &to_sha[..8]);
        Some(count)
    } else {
        max_commits
    };

    if let Some(max) = computed_max_commits {
        log::info!("Max commits: {}", max);
    }

    let _stats = indexer.index_from_commit(&from_sha, computed_max_commits, batch_size)
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

/// Resolves commit SHA by date using git log --until
fn resolve_commit_by_date(repo_path: &PathBuf, date: &str) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("log")
        .arg(&format!("--until={}", date))
        .arg("--format=%H")
        .arg("-1")
        .output()
        .context("Failed to run git log")?;

    if !output.status.success() {
        anyhow::bail!("git log failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let commit_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if commit_sha.is_empty() {
        anyhow::bail!("No commits found until date {}", date);
    }

    Ok(commit_sha)
}

/// Counts commits between two commits (from..to)
fn count_commits_between(repo_path: &PathBuf, from_sha: &str, to_sha: &str) -> Result<usize> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("rev-list")
        .arg("--count")
        .arg(&format!("{}..{}", to_sha, from_sha))  // Reverse: to..from to count forward
        .output()
        .context("Failed to run git rev-list")?;

    if !output.status.success() {
        anyhow::bail!("git rev-list failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let count = count_str.parse::<usize>()
        .with_context(|| format!("Failed to parse commit count: {}", count_str))?;

    Ok(count)
}
