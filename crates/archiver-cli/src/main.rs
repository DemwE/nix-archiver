//! Nix-Archiver CLI - User interface for Nixpkgs archiving system
//!
//! Provides:
//! - Indexing of Nixpkgs repository
//! - Searching for specific package versions
//! - Generating frozen.nix files with pinned versions

mod commands;
mod helpers;
mod output;

use anyhow::{Context, Result};
use archiver_db::ArchiverDb;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use commands::{cmd_index, cmd_search, cmd_generate, cmd_stats, cmd_prefetch_hashes};

#[derive(Parser)]
#[command(name = "nix-archiver")]
#[command(about = "Declarative pinning of packages to historical versions in Nixpkgs", long_about = None)]
#[command(version)]
struct Cli {
    /// Path to the database
    #[arg(short, long, default_value = "./nix-archiver.db")]
    database: PathBuf,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Indexes Nixpkgs repository
    Index {
        /// Path to local Nixpkgs repository
        #[arg(short, long)]
        repo: PathBuf,

        /// Commit to start indexing from (default: HEAD)
        #[arg(short, long, default_value = "HEAD")]
        from: String,

        /// Stop indexing at this commit SHA (optional)
        #[arg(long, conflicts_with = "to_date", conflicts_with = "max_commits", conflicts_with = "full_repo")]
        to_commit: Option<String>,

        /// Stop indexing at this date (YYYY-MM-DD) (optional)
        #[arg(long, conflicts_with = "to_commit", conflicts_with = "max_commits", conflicts_with = "full_repo")]
        to_date: Option<String>,

        /// Maximum number of commits to process
        #[arg(short, long, conflicts_with = "full_repo")]
        max_commits: Option<usize>,

        /// Index entire repository history (no commit limit)
        #[arg(long, conflicts_with = "max_commits", conflicts_with = "to_commit", conflicts_with = "to_date")]
        full_repo: bool,

        /// Number of threads for parallel processing (default: number of CPU cores)
        #[arg(short = 'j', long)]
        threads: Option<usize>,

        /// Batch size for parallel processing (default: 500)
        #[arg(short = 'b', long, default_value = "500")]
        batch_size: usize,
    },

    /// Searches for a specific package version
    Search {
        /// Package attribute name (e.g., "nodejs")
        attr_name: String,

        /// Version to search for (optional - displays all versions)
        version: Option<String>,
        
        /// Maximum number of versions to display (default: 50)
        #[arg(short = 'n', long, default_value = "50")]
        limit: usize,
        
        /// Search by major version (e.g., "20" matches "20.x.x")
        #[arg(short, long)]
        major: Option<u64>,

        /// Use fuzzy pattern matching
        #[arg(short, long)]
        pattern: Option<String>,

        /// Show versions since date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Show all versions (no limit)
        #[arg(short, long)]
        all: bool,
    },

    /// Generates frozen.nix from requirements file
    Generate {
        /// Input requirements file
        #[arg(short, long)]
        input: PathBuf,

        /// Output frozen.nix file
        #[arg(short, long)]5555
        output: PathBuf,

        /// Path to a local nixpkgs bare git repo (e.g. /data/nixpkgs.git).
        /// When provided, frozen.nix will use builtins.fetchGit with a local
        /// file:// URL instead of fetching from GitHub — fully offline.
        #[arg(long)]
        nixpkgs: Option<PathBuf>,
    },

    /// Show database statistics
    Stats,

    /// Fetch and cache nixpkgs tarball sha256 for each indexed commit.
    /// After running this, `generate` will produce fully pinned fetchTarball
    /// expressions with sha256 — no local nixpkgs needed at evaluation time.
    PrefetchHashes {
        /// Maximum number of commits to fetch (default: all pending)
        #[arg(short, long)]
        limit: Option<usize>,

        /// Re-fetch even if a hash is already stored
        #[arg(long)]
        force: bool,

        /// Number of parallel nix-prefetch-url invocations (default: 4)
        #[arg(short, long, default_value = "4")]
        jobs: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Configure logger
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&cli.log_level)
    ).init();

    // Open database
    let db = ArchiverDb::open(&cli.database)
        .with_context(|| format!("Failed to open database at {:?}", cli.database))?;

    match cli.command {
        Commands::Index { repo, from, to_commit, to_date, max_commits, full_repo, threads, batch_size } => {
            cmd_index(repo, from, to_commit, to_date, max_commits, full_repo, threads, batch_size, db)?;
        }
        Commands::Search { attr_name, version, limit, major, pattern, since, all } => {
            cmd_search(attr_name, version, limit, major, pattern, since, all, db)?;
        }
        Commands::Generate { input, output, nixpkgs } => {
            cmd_generate(input, output, nixpkgs, db)?;
        }
        Commands::Stats => {
            cmd_stats(db)?;
        }
        Commands::PrefetchHashes { limit, force, jobs } => {
            cmd_prefetch_hashes(limit, force, jobs, db)?;
        }
    }

    Ok(())
}
