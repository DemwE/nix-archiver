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
use colored::Colorize;
use std::path::PathBuf;
use tabled::{Table, Tabled, settings::{Style, Color, Modify, object::Rows}};
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
        
        /// Maximum number of versions to display (default: 50)
        #[arg(short = 'n', long, default_value = "50")]
        limit: usize,
        
        /// Filter by major version (e.g., 20 for 20.x.x)
        #[arg(long)]
        major: Option<u64>,
        
        /// Filter by regex pattern
        #[arg(short = 'p', long)]
        pattern: Option<String>,
        
        /// Show versions since date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,
        
        /// Show all versions (ignore limit)
        #[arg(short = 'a', long)]
        all: bool,
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
        Commands::Search { attr_name, version, limit, major, pattern, since, all } => {
            cmd_search(attr_name, version, limit, major, pattern, since, all, db)?;
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
fn cmd_search(
    attr_name: String,
    version: Option<String>,
    limit: usize,
    major: Option<u64>,
    pattern: Option<String>,
    since: Option<String>,
    show_all: bool,
    db: ArchiverDb,
) -> Result<()> {
    if let Some(ver) = version {
        // Search for specific version
        match db.get(&attr_name, &ver)? {
            Some(entry) => {
                println!("\n{} {}", "📦 Package:".bright_cyan(), format!("{} v{}", attr_name, ver).bold());
                println!("{}", "━".repeat(60).bright_black());
                println!("  {}    {}", "Commit:".bright_yellow(), entry.commit_sha);
                println!("  {}      {}", "Date:".bright_yellow(), format_timestamp(entry.timestamp));
                println!("  {}  {}", "NAR Hash:".bright_yellow(), entry.nar_hash);
                println!("\n{}", "📝 Nix expression:".bright_cyan());
                println!("{}", "━".repeat(60).bright_black());
                println!("{}", entry.to_nix_import().bright_white());
            }
            None => {
                eprintln!("{} Package {}:{} not found in database", "❌".red(), attr_name.bold(), ver.bold());
                
                // Suggest available versions
                let all_versions = db.get_all_versions(&attr_name)?;
                if !all_versions.is_empty() {
                    eprintln!("\n{} Available versions for {}:", "💡".yellow(), attr_name.bold());
                    let sorted = sort_versions_semver(all_versions);
                    let rows: Vec<VersionRow> = sorted.iter()
                        .take(10)
                        .map(|entry| VersionRow {
                            version: entry.version.clone(),
                            commit: entry.commit_sha.clone(),
                            date: format_relative_time(entry.timestamp),
                            nar_hash: if entry.nar_hash == "unknown" { 
                                "-".to_string()
                            } else { 
                                entry.nar_hash.clone()
                            },
                        })
                        .collect();
                    
                    let mut table = Table::new(rows);
                    table.with(Style::rounded())
                        .with(Modify::new(Rows::first()).with(Color::FG_BRIGHT_CYAN));
                    eprintln!("{}", table);
                    
                    if sorted.len() > 10 {
                        eprintln!("\n  {} and {} more versions", "...".dimmed(), (sorted.len() - 10).to_string().bold());
                    }
                } else {
                    eprintln!("\n{} No versions found for package '{}'", "❌".red(), attr_name.bold());
                }
                
                std::process::exit(1);
            }
        }
    } else {
        // Display all versions with filtering
        let mut all_versions = db.get_all_versions(&attr_name)?;
        
        if all_versions.is_empty() {
            println!("{} No versions found for package '{}'", "❌".red(), attr_name.bold());
            return Ok(());
        }
        
        // Apply filters
        all_versions = filter_versions(all_versions, major, pattern.as_deref(), since.as_deref())?;
        
        if all_versions.is_empty() {
            println!("{} No versions match the specified filters", "❌".red());
            return Ok(());
        }
        
        // Sort by semver
        let sorted = sort_versions_semver(all_versions);
        
        // Calculate statistics
        let total_count = sorted.len();
        let newest = &sorted[0];
        let oldest = &sorted[sorted.len() - 1];
        
        // Print summary
        println!("\n{} {}", "📦".bright_cyan(), attr_name.bold().bright_white());
        println!("{}", "━".repeat(60).bright_black());
        println!("  {} {}  {} {}  {} {}", 
            "Total:".bright_yellow(), 
            total_count.to_string().bold(),
            "Newest:".bright_green(),
            newest.version.clone().green().bold(),
            "Oldest:".bright_blue(),
            oldest.version.clone().blue()
        );
        
        // Determine display limit
        let display_limit = if show_all { total_count } else { limit.min(total_count) };
        
        println!();
        
        let rows: Vec<VersionRow> = sorted.iter()
            .take(display_limit)
            .map(|entry| VersionRow {
                version: entry.version.clone(),
                commit: entry.commit_sha.clone(),
                date: format_relative_time(entry.timestamp),
                nar_hash: if entry.nar_hash == "unknown" { 
                    "-".to_string()
                } else { 
                    entry.nar_hash.clone()
                },
            })
            .collect();
        
        let mut table = Table::new(rows);
        table.with(Style::rounded())
            .with(Modify::new(Rows::first()).with(Color::FG_BRIGHT_CYAN));
        println!("{}", table);
        
        if display_limit < total_count {
            println!("\n  {} and {} more versions (use {} to see all)", 
                "...".dimmed(), 
                (total_count - display_limit).to_string().bold(),
                "-a".bright_cyan()
            );
        }
    }

    Ok(())
}

/// Generates frozen.nix file from package specification
fn cmd_generate(input: PathBuf, output: PathBuf, db: ArchiverDb) -> Result<()> {
    use std::fs;
    use std::io::Write;
    use regex::Regex;
    
    println!("{} Reading package specification from {}...", "📖".bright_cyan(), input.display());
    
    // Read input file
    let content = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;
    
    // Parse Nix attribute set format: { package = "version"; }
    // Match patterns like: nodejs = "20.11.0";
    let re = Regex::new(r#"^\s*([a-zA-Z0-9_-]+)\s*=\s*"([^"]+)"\s*;?\s*$"#)
        .context("Failed to compile regex")?;
    
    let mut packages = Vec::new();
    let mut errors = Vec::new();
    
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        
        // Skip empty lines, comments, and structural characters
        if line.is_empty() || line.starts_with('#') || line == "{" || line == "}" {
            continue;
        }
        
        // Try to match Nix attribute pattern
        if let Some(caps) = re.captures(line) {
            let attr_name = caps.get(1).unwrap().as_str();
            let version = caps.get(2).unwrap().as_str();
            
            // Look up in database
            match db.get(attr_name, version)? {
                Some(entry) => {
                    println!("  {} Found: {} v{} @ commit {}", 
                        "✓".green(), 
                        attr_name.bold(), 
                        version, 
                        &entry.commit_sha[..12].dimmed());
                    packages.push(entry);
                }
                None => {
                    errors.push(format!("Line {}: Package {}:{} not found in database", 
                        line_num + 1, attr_name, version));
                    
                    // Try to suggest available versions
                    let available = db.get_all_versions(attr_name)?;
                    if !available.is_empty() {
                        let sorted = sort_versions_semver(available);
                        let suggestions: Vec<String> = sorted.iter()
                            .take(5)
                            .map(|e| e.version.clone())
                            .collect();
                        errors.push(format!("         Available versions: {}", suggestions.join(", ")));
                    } else {
                        errors.push(format!("         No versions available for package '{}'", attr_name));
                    }
                }
            }
        } else if !line.is_empty() {
            errors.push(format!("Line {}: Invalid syntax '{}' (expected: package = \"version\";)", 
                line_num + 1, line));
        }
    }
    
    // Report errors if any
    if !errors.is_empty() {
        eprintln!("\n{} Errors found:\n", "❌".red().bold());
        for error in &errors {
            eprintln!("  {}", error.red());
        }
        eprintln!("\n{} Expected input format:", "💡".yellow());
        eprintln!("  {{\n    nodejs = \"20.11.0\";\n    python = \"3.11.7\";\n  }}");
        anyhow::bail!("Failed to resolve all packages. Fix the errors above and try again.");
    }
    
    if packages.is_empty() {
        eprintln!("{} No packages found in input file.", "❌".red());
        eprintln!("\n{} Expected input format:", "💡".yellow());
        eprintln!("  {{\n    nodejs = \"20.11.0\";\n    python = \"3.11.7\";\n  }}");
        anyhow::bail!("Input file is empty or invalid");
    }
    
    // Generate frozen.nix content
    println!("\n{} Generating frozen.nix with {} package{}...", 
        "🔨".bright_cyan(), 
        packages.len(), 
        if packages.len() == 1 { "" } else { "s" });
    
    let mut nix_content = String::from("# Generated by nix-archiver\n");
    nix_content.push_str("# This file pins packages to specific historical versions from Nixpkgs\n\n");
    nix_content.push_str("{\n");
    
    for entry in &packages {
        nix_content.push_str(&format!("  # {} v{} (commit: {})\n", 
            entry.attr_name, entry.version, &entry.commit_sha));
        nix_content.push_str(&format!("  {} = import ({}) {{}};\n\n", 
            entry.attr_name, 
            entry.to_nix_fetchtarball()));
    }
    
    nix_content.push_str("}\n");
    
    // Write to output file
    let mut file = fs::File::create(&output)
        .with_context(|| format!("Failed to create output file: {}", output.display()))?;
    
    file.write_all(nix_content.as_bytes())
        .with_context(|| format!("Failed to write to output file: {}", output.display()))?;
    
    println!("{} Successfully generated: {}", "✓".green().bold(), output.display().to_string().bold());
    println!("\n{} Usage:\n  nix-shell {}", "💡".yellow(), output.display());
    
    Ok(())
}

/// Sorts versions using semantic versioning (newest first)
fn sort_versions_semver(mut versions: Vec<archiver_core::PackageEntry>) -> Vec<archiver_core::PackageEntry> {
    versions.sort_by(|a, b| {
        use semver::Version;
        
        // Try to parse as semver
        let a_semver = Version::parse(&a.version);
        let b_semver = Version::parse(&b.version);
        
        match (a_semver, b_semver) {
            (Ok(av), Ok(bv)) => {
                // Both are valid semver - compare them (reversed for newest first)
                bv.cmp(&av)
            }
            (Ok(_), Err(_)) => {
                // a is valid semver, b is not - a comes first
                std::cmp::Ordering::Less
            }
            (Err(_), Ok(_)) => {
                // b is valid semver, a is not - b comes first
                std::cmp::Ordering::Greater
            }
            (Err(_), Err(_)) => {
                // Neither is valid semver - compare by timestamp (newer first)
                b.timestamp.cmp(&a.timestamp)
            }
        }
    });
    
    versions
}

/// Filters versions based on criteria
fn filter_versions(
    versions: Vec<archiver_core::PackageEntry>,
    major: Option<u64>,
    pattern: Option<&str>,
    since: Option<&str>,
) -> Result<Vec<archiver_core::PackageEntry>> {
    use semver::Version;
    use regex::Regex;
    
    let mut filtered = versions;
    
    // Filter by major version
    if let Some(major_ver) = major {
        filtered = filtered.into_iter()
            .filter(|entry| {
                if let Ok(v) = Version::parse(&entry.version) {
                    v.major == major_ver
                } else {
                    // For non-semver, check if version starts with major number
                    entry.version.starts_with(&format!("{}.", major_ver))
                }
            })
            .collect();
    }
    
    // Filter by regex pattern
    if let Some(pat) = pattern {
        let re = Regex::new(pat)
            .with_context(|| format!("Invalid regex pattern: {}", pat))?;
        filtered = filtered.into_iter()
            .filter(|entry| re.is_match(&entry.version))
            .collect();
    }
    
    // Filter by date
    if let Some(since_str) = since {
        use chrono::NaiveDate;
        let since_date = NaiveDate::parse_from_str(since_str, "%Y-%m-%d")
            .with_context(|| format!("Invalid date format: {}. Expected YYYY-MM-DD", since_str))?;
        let since_timestamp = since_date.and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp() as u64;
        
        filtered = filtered.into_iter()
            .filter(|entry| entry.timestamp >= since_timestamp)
            .collect();
    }
    
    Ok(filtered)
}

/// Formats timestamp as relative time (e.g., "2 days ago")
fn format_relative_time(timestamp: u64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
        .unwrap_or_else(|| DateTime::<Utc>::MIN_UTC);
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);
    
    if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        format!("{} min{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if duration.num_days() < 30 {
        let days = duration.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if duration.num_days() < 365 {
        let months = duration.num_days() / 30;
        format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
    } else {
        let years = duration.num_days() / 365;
        format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
    }
}

/// Displays database statistics
fn cmd_stats(db: ArchiverDb) -> Result<()> {
    println!("{}", "Database Statistics:".bright_cyan().bold());
    println!("  {}: {}", "Packages".bright_yellow(), db.package_count().to_string().bold());
    println!("  {}: {}", "Processed commits".bright_yellow(), db.processed_commit_count().to_string().bold());
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
