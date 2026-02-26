//! Search command implementation

use anyhow::Result;
use archiver_db::ArchiverDb;
use colored::Colorize;
use tabled::{Table, settings::{Style, Color, Modify, object::Rows}};
use crate::helpers::{sort_versions_semver, filter_versions, format_relative_time, format_timestamp};
use crate::output::{PackageSummaryRow, VersionRow};

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
        // Use prefix search - finds exact match AND packages with the same prefix
        let matches = db.search_packages(&attr_name)?;

        if matches.is_empty() {
            println!("{} No versions found for package '{}'", "❌".red(), attr_name.bold());
            return Ok(());
        }

        if matches.len() == 1 {
            // Only one package matched - show detailed version list
            let (name, entries) = matches.into_iter().next().unwrap();
            return display_single_package(name, entries, major, pattern.as_deref(), since.as_deref(), limit, show_all);
        }

        // Multiple packages matched:
        // - exact name match → show detail with hint about others
        // - no exact match → show grouped summary table
        if matches.contains_key(&attr_name) && filter_is_specific(major, &pattern, &since) {
            // User is filtering, so they probably want the exact package
            let entries = matches[&attr_name].clone();
            let other_count = matches.len() - 1;
            if other_count > 0 {
                let mut other_names: Vec<&str> = matches.keys()
                    .map(|k| k.as_str())
                    .filter(|k| *k != attr_name.as_str())
                    .collect();
                other_names.sort();
                println!("{} Also matches {} more package(s): {}",
                    "💡".yellow(),
                    other_count,
                    other_names.join(", ").bright_cyan()
                );
                println!();
            }
            return display_single_package(attr_name, entries, major, pattern.as_deref(), since.as_deref(), limit, show_all);
        }

        // Show grouped summary for all matching packages
        return display_multiple_packages(&attr_name, matches);
    }

    Ok(())
}

fn filter_is_specific(major: Option<u64>, pattern: &Option<String>, since: &Option<String>) -> bool {
    major.is_some() || pattern.is_some() || since.is_some()
}

/// Displays detailed version list for a single package
fn display_single_package(
    attr_name: String,
    all_versions: Vec<archiver_core::PackageEntry>,
    major: Option<u64>,
    pattern: Option<&str>,
    since: Option<&str>,
    limit: usize,
    show_all: bool,
) -> Result<()> {
    let all_versions = filter_versions(all_versions, major, pattern, since)?;

    if all_versions.is_empty() {
        println!("{} No versions match the specified filters", "❌".red());
        return Ok(());
    }

    let sorted = sort_versions_semver(all_versions);
    let total_count = sorted.len();
    let newest = &sorted[0];
    let oldest = &sorted[sorted.len() - 1];

    println!("\n{} {}", "📦".bright_cyan(), attr_name.bold().bright_white());
    println!("{}", "━".repeat(60).bright_black());
    println!("  {} {}  {} {}  {} {}",
        "Total:".bright_yellow(), total_count.to_string().bold(),
        "Newest:".bright_green(), newest.version.clone().green().bold(),
        "Oldest:".bright_blue(), oldest.version.clone().blue()
    );
    println!();

    let display_limit = if show_all { total_count } else { limit.min(total_count) };
    let rows: Vec<VersionRow> = sorted.iter().take(display_limit).map(|entry| VersionRow {
        version: entry.version.clone(),
        commit: entry.commit_sha.clone(),
        date: format_relative_time(entry.timestamp),
        nar_hash: if entry.nar_hash == "unknown" { "-".to_string() } else { entry.nar_hash.clone() },
    }).collect();

    let mut table = Table::new(rows);
    table.with(Style::rounded()).with(Modify::new(Rows::first()).with(Color::FG_BRIGHT_CYAN));
    println!("{}", table);

    if display_limit < total_count {
        println!("\n  {} and {} more versions (use {} to see all)",
            "...".dimmed(), (total_count - display_limit).to_string().bold(), "-a".bright_cyan()
        );
    }
    Ok(())
}

/// Displays a grouped summary table when multiple packages match
fn display_multiple_packages(
    query: &str,
    matches: std::collections::HashMap<String, Vec<archiver_core::PackageEntry>>,
) -> Result<()> {
    // Sort package names alphabetically
    let mut names: Vec<String> = matches.keys().cloned().collect();
    names.sort();

    println!("\n{} {}",
        "🔍".bright_cyan(),
        format!("Packages matching '{}'", query).bold().bright_white()
    );
    println!("{}", "━".repeat(60).bright_black());
    println!("  {} {}",
        "Found:".bright_yellow(),
        format!("{} packages", names.len()).bold()
    );
    println!();

    let rows: Vec<PackageSummaryRow> = names.iter().map(|name| {
        let entries = &matches[name];
        let sorted = sort_versions_semver(entries.clone());
        let newest = sorted.first().unwrap();
        PackageSummaryRow {
            attr_name: name.clone(),
            version_count: sorted.len().to_string(),
            latest_version: newest.version.clone(),
            latest_date: format_relative_time(newest.timestamp),
        }
    }).collect();

    let mut table = Table::new(rows);
    table.with(Style::rounded()).with(Modify::new(Rows::first()).with(Color::FG_BRIGHT_CYAN));
    println!("{}", table);
    println!(
        "\n  {} Run {} for details on a specific package",
        "💡".yellow(),
        format!("search <name>", ).bright_cyan()
    );
    Ok(())
}
