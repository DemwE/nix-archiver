//! Search command implementation

use anyhow::Result;
use archiver_db::ArchiverDb;
use colored::Colorize;
use tabled::{Table, settings::{Style, Color, Modify, object::Rows}};
use crate::helpers::{sort_versions_semver, filter_versions, format_relative_time, format_timestamp};
use crate::output::VersionRow;

/// Searches for package in database
pub fn cmd_search(
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
