//! prefetch-hashes command — fetches nixpkgs tarball sha256 per commit

use anyhow::{Context, Result};
use archiver_db::ArchiverDb;
use colored::Colorize;
use std::process::Command;

const NIXPKGS_TARBALL: &str =
    "https://github.com/NixOS/nixpkgs/archive/{commit}.tar.gz";

/// Runs `nix-prefetch-url --unpack <url>` and returns the hash string on success.
fn prefetch(commit: &str) -> Result<String> {
    let url = NIXPKGS_TARBALL.replace("{commit}", commit);
    let out = Command::new("nix-prefetch-url")
        .args(["--unpack", "--type", "sha256", &url])
        .output()
        .context("Failed to run nix-prefetch-url — is nix installed?")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("nix-prefetch-url failed: {}", stderr.trim());
    }

    // stdout is the hash (Nix base32), trim whitespace
    let hash = String::from_utf8(out.stdout)
        .context("nix-prefetch-url output is not valid UTF-8")?
        .trim()
        .to_string();

    if hash.is_empty() {
        anyhow::bail!("nix-prefetch-url returned empty output");
    }

    Ok(hash)
}

/// Fetches and stores nixpkgs tarball sha256 hashes for all commits in the database.
///
/// Each hash is computed by `nix-prefetch-url --unpack` and stored so
/// `generate` can produce fully pinned `fetchTarball { sha256 = "..."; }` blocks.
pub fn cmd_prefetch_hashes(limit: Option<usize>, force: bool, jobs: usize, db: ArchiverDb) -> Result<()> {
    println!("{} Scanning database for unique commits...", "🔍".bright_cyan());
    let all_commits = db.all_unique_commits()?;

    let to_fetch: Vec<String> = if force {
        all_commits
    } else {
        all_commits
            .into_iter()
            .filter(|c| db.get_tarball_hash(c).ok().flatten().is_none())
            .collect()
    };

    let total = match limit {
        Some(n) => to_fetch.len().min(n),
        None => to_fetch.len(),
    };
    let to_fetch = &to_fetch[..total];

    let already = db.tarball_hash_count();
    println!(
        "  {} already cached,  {} to fetch{}",
        already.to_string().green(),
        total.to_string().yellow(),
        if force { " (--force, re-fetching all)" } else { "" }
    );

    if total == 0 {
        println!("{} Nothing to do.", "✓".green());
        return Ok(());
    }

    println!("  Using {} parallel job{}\n", jobs, if jobs == 1 { "" } else { "s" });

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let done = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));

    // Process in chunks of `jobs` using rayon thread pool scoped to limit concurrency
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .context("Failed to build thread pool")?;

    let results: Vec<(String, Result<String>)> = pool.install(|| {
        use rayon::prelude::*;
        to_fetch
            .par_iter()
            .map(|commit| {
                let result = prefetch(commit);
                (commit.clone(), result)
            })
            .collect()
    });

    for (commit, result) in results {
        match result {
            Ok(hash) => {
                db.store_tarball_hash(&commit, &hash)
                    .with_context(|| format!("Failed to store hash for {}", &commit[..12]))?;
                done.fetch_add(1, Ordering::Relaxed);
                println!("  {} {}  {}", "✓".green(), &commit[..12].dimmed(), hash.bright_yellow());
            }
            Err(e) => {
                errors.fetch_add(1, Ordering::Relaxed);
                eprintln!("  {} {}  {}", "✗".red(), &commit[..12].dimmed(), e.to_string().red());
            }
        }
    }

    db.flush()?;

    let n_done = done.load(Ordering::Relaxed);
    let n_err = errors.load(Ordering::Relaxed);
    println!(
        "\n{} Done: {} fetched, {} errors. Total cached: {}",
        if n_err == 0 { "✓".green() } else { "⚠".yellow() },
        n_done.to_string().green(),
        n_err.to_string().red(),
        db.tarball_hash_count().to_string().bold(),
    );

    Ok(())
}
