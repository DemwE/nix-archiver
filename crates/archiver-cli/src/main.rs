//! Nix-Archiver CLI - User interface for Nixpkgs archiving system
//!
//! Provides:
//! - Indexing of Nixpkgs repository
//! - Searching for specific package versions
//! - Generating frozen.nix files with pinned versions

use anyhow::{Context, Result};
use archiver_db::ArchiverDb;
use archiver_index::Indexer;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tabled::{Table, Tabled, settings::Style};
use chrono::{DateTime, Utc};

#[derive(Parser)]
#[command(name = "nix-archiver")]
#[command(about = "Declarative pinning of packages to historical versions in Nixpkgs", long_about = None)]
#[command(version)]
struct Cli {
    /// Path to the database
    #[arg(short, long, default_value = "./nix-archiver.db")]
    database: PathBuf,

    /// Logging level (error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
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

        /// Maximum number of commits to process
        #[arg(short, long)]
        max_commits: Option<usize>,

        /// Number of threads for parallel processing (default: number of CPU cores)
        #[arg(short = 'j', long)]
        threads: Option<usize>,

        /// Batch size for parallel processing (default: 100)
        #[arg(short = 'b', long, default_value = "100")]
        batch_size: usize,
    },

    /// Searches for a specific package version
    Search {
        /// Package attribute name (e.g., "nodejs")
        attr_name: String,

        /// Version to search for (optional - displays all versions)
        version: Option<String>,
    },

    /// Generates frozen.nix file based on specification
    Generate {
        /// Input file with version specification
        #[arg(short, long)]
        input: PathBuf,

        /// Output frozen.nix file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Displays database statistics
    Stats,
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
        Commands::Index { repo, from, max_commits, threads, batch_size } => {
            cmd_index(repo, from, max_commits, threads, batch_size, db)?;
        }
        Commands::Search { attr_name, version } => {
            cmd_search(attr_name, version, db)?;
        }
        Commands::Generate { input, output } => {
            cmd_generate(input, output, db)?;
        }
        Commands::Stats => {
            cmd_stats(db)?;
        }
    }

    Ok(())
}

/// Indexes Nixpkgs repository
fn cmd_index(repo_path: PathBuf, from_commit: String, max_commits: Option<usize>, threads: Option<usize>, batch_size: usize, db: ArchiverDb) -> Result<()> {
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

/// Table row for displaying package versions
#[derive(Tabled)]
struct VersionRow {
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Commit")]
    commit: String,
    #[tabled(rename = "Date")]
    date: String,
    #[tabled(rename = "NAR Hash")]
    nar_hash: String,
}

/// Searches for package in database
fn cmd_search(attr_name: String, version: Option<String>, db: ArchiverDb) -> Result<()> {
    if let Some(ver) = version {
        // Search for specific version
        match db.get(&attr_name, &ver)? {
            Some(entry) => {
                println!("\n📦 Package: {} v{}", attr_name, ver);
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("  Commit:    {}", entry.commit_sha);
                println!("  Date:      {}", format_timestamp(entry.timestamp));
                println!("  NAR Hash:  {}", entry.nar_hash);
                println!("\n📝 Nix expression:");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("{}", entry.to_nix_import());
            }
            None => {
                eprintln!("❌ Package {}:{} not found in database", attr_name, ver);
                
                // Suggest available versions
                let all_versions = db.get_all_versions(&attr_name)?;
                if !all_versions.is_empty() {
                    eprintln!("\n💡 Available versions for {}:", attr_name);
                    let rows: Vec<VersionRow> = all_versions.iter()
                        .take(10)
                        .map(|entry| VersionRow {
                            version: entry.version.clone(),
                            commit: entry.commit_sha[..12].to_string(),
                            date: format_timestamp(entry.timestamp),
                            nar_hash: if entry.nar_hash == "unknown" { 
                                "-".to_string() 
                            } else { 
                                entry.nar_hash[..16].to_string() + "..." 
                            },
                        })
                        .collect();
                    
                    let mut table = Table::new(rows);
                    table.with(Style::rounded());
                    eprintln!("{}", table);
                    
                    if all_versions.len() > 10 {
                        eprintln!("\n  ... and {} more versions", all_versions.len() - 10);
                    }
                } else {
                    eprintln!("\n❌ No versions found for package '{}'", attr_name);
                }
                
                std::process::exit(1);
            }
        }
    } else {
        // Display all versions
        let all_versions = db.get_all_versions(&attr_name)?;
        
        if all_versions.is_empty() {
            println!("❌ No versions found for package '{}'", attr_name);
        } else {
            println!("\n📦 Found {} versions of '{}':\n", all_versions.len(), attr_name);
            
            let rows: Vec<VersionRow> = all_versions.iter()
                .map(|entry| VersionRow {
                    version: entry.version.clone(),
                    commit: entry.commit_sha[..12].to_string(),
                    date: format_timestamp(entry.timestamp),
                    nar_hash: if entry.nar_hash == "unknown" { 
                        "-".to_string() 
                    } else { 
                        entry.nar_hash[..16].to_string() + "..." 
                    },
                })
                .collect();
            
            let mut table = Table::new(rows);
            table.with(Style::rounded());
            println!("{}", table);
        }
    }

    Ok(())
}

/// Generates frozen.nix file
fn cmd_generate(_input: PathBuf, _output: PathBuf, _db: ArchiverDb) -> Result<()> {
    // TODO: Implementation of input file parsing and frozen.nix generation
    eprintln!("Generate command not yet implemented");
    eprintln!("This will be implemented in Phase 4");
    std::process::exit(1);
}

/// Displays database statistics
fn cmd_stats(db: ArchiverDb) -> Result<()> {
    println!("Database Statistics:");
    println!("  Packages: {}", db.package_count());
    println!("  Processed commits: {}", db.processed_commit_count());
    Ok(())
}

/// Formats Unix timestamp to readable date
fn format_timestamp(timestamp: u64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
        .unwrap_or_else(|| DateTime::<Utc>::MIN_UTC);
    dt.format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test that CLI parses correctly
        let cli = Cli::try_parse_from(&[
            "nix-archiver",
            "--database", "./test.db",
            "stats"
        ]);
        assert!(cli.is_ok());
    }
}
