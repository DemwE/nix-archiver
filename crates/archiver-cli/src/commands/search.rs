//! Search command implementation

use std::collections::HashMap;
use anyhow::Result;
use archiver_db::ArchiverDb;
use colored::Colorize;
use tabled::{Table, settings::{Style, Color, Modify, object::Rows}};
use crate::helpers::{sort_versions_semver, filter_versions, format_relative_time, format_timestamp};
use crate::output::{PackageSummaryRow, PackageSetRow, VersionRow};

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
        // Phase 1: fast prefix scan ("python" → python311, python314, …)
        let mut matches = db.search_packages(&attr_name)?;
        let mut used_substring = false;

        // Phase 2: substring fallback ("biomejs" → vscode-extensions.biomejs.biome, etc.)
        if matches.is_empty() {
            matches = db.search_packages_contains(&attr_name)?;
            used_substring = true;
        }

        if matches.is_empty() {
            println!("{} No packages found matching '{}'", "❌".red(), attr_name.bold());
            println!("  {} Try a different spelling or a broader term", "💡".yellow());
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
        return display_multiple_packages(&attr_name, matches, limit, used_substring);
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

/// Extracts the top-level namespace (package set) from an attr_name.
///
/// Examples:
///   "vscode-extensions.biomejs.biome" → "vscode-extensions"
///   "python313Packages.numpy"          → "python313Packages"
///   "python314"                        → "(top-level)"
fn attr_namespace(attr_name: &str) -> &str {
    match attr_name.find('.') {
        Some(pos) => &attr_name[..pos],
        None => "(top-level)",
    }
}

/// Displays a grouped summary table when multiple packages match.
/// Shows a package-set breakdown (like NixOS search sidebar) followed by
/// a paginated alphabetical package list.
fn display_multiple_packages(
    query: &str,
    matches: HashMap<String, Vec<archiver_core::PackageEntry>>,
    limit: usize,
    used_substring: bool,
) -> Result<()> {
    let mut names: Vec<String> = matches.keys().cloned().collect();
    names.sort();

    let total = names.len();
    let display_limit = limit.min(total);

    let mode_tag = if used_substring {
        "substring".bright_yellow()
    } else {
        "prefix".bright_cyan()
    };

    println!("\n{} {}  {} {}",
        "🔍".bright_cyan(),
        format!("Showing 1-{} of {} packages matching '{}'", display_limit, total, query)
            .bold().bright_white(),
        "mode:".dimmed(),
        mode_tag,
    );
    println!("{}", "━".repeat(70).bright_black());

    // ── Package sets breakdown (mirrors NixOS search sidebar) ──────────────
    let mut set_counts: HashMap<&str, usize> = HashMap::new();
    for name in &names {
        *set_counts.entry(attr_namespace(name)).or_insert(0) += 1;
    }
    let mut set_names: Vec<&str> = set_counts.keys().cloned().collect();
    set_names.sort_by(|a, b| {
        let ca = set_counts[a];
        let cb = set_counts[b];
        if *a == "(top-level)" { return std::cmp::Ordering::Less; }
        if *b == "(top-level)" { return std::cmp::Ordering::Greater; }
        cb.cmp(&ca).then(a.cmp(b))
    });

    if set_names.len() > 1 || set_names.first().is_some_and(|s| *s != "(top-level)") {
        println!("\n{}", "📦 Package sets:".bright_cyan());
        let set_rows: Vec<PackageSetRow> = set_names.iter().map(|s| PackageSetRow {
            set: s.to_string(),
            packages: set_counts[s].to_string(),
        }).collect();
        let mut set_table = Table::new(set_rows);
        set_table.with(Style::rounded())
            .with(Modify::new(Rows::first()).with(Color::FG_BRIGHT_CYAN));
        println!("{}", set_table);
        println!();
    }

    // ── Package list ────────────────────────────────────────────────────────
    let rows: Vec<PackageSummaryRow> = names.iter().take(display_limit).map(|name| {
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

    if display_limit < total {
        println!("\n  {} {} more packages not shown (use {} to increase the limit)",
            "...".dimmed(),
            (total - display_limit).to_string().bold(),
            "--limit N".bright_cyan(),
        );
    }

    println!(
        "\n  {} Run {} for details on a specific package",
        "💡".yellow(),
        "search <name>".bright_cyan()
    );
    Ok(())
}
