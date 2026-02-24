//! Helper functions for CLI operations

use anyhow::{Context, Result};
use archiver_core::PackageEntry;
use chrono::{DateTime, Utc};

/// Sorts versions using semantic versioning (newest first)
pub fn sort_versions_semver(mut versions: Vec<PackageEntry>) -> Vec<PackageEntry> {
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
pub fn filter_versions(
    versions: Vec<PackageEntry>,
    major: Option<u64>,
    pattern: Option<&str>,
    since: Option<&str>,
) -> Result<Vec<PackageEntry>> {
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
pub fn format_relative_time(timestamp: u64) -> String {
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

/// Formats Unix timestamp to readable date
pub fn format_timestamp(timestamp: u64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
        .unwrap_or_else(|| DateTime::<Utc>::MIN_UTC);
    dt.format("%Y-%m-%d %H:%M").to_string()
}
