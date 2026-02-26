//! Helper functions for CLI operations

use anyhow::{Context, Result};
use archiver_core::PackageEntry;
use chrono::{DateTime, Utc};

/// Parsed version key for comparison.
/// Represents versions like: 1.20.2, 1.26rc3, 1.18beta1, 1.18.0-alpha.1
struct VersionKey {
    /// Numeric components, e.g. [1, 20, 2] for "1.20.2"
    nums: Vec<u64>,
    /// Pre-release tier: 3=stable, 2=rc, 1=beta, 0=alpha (higher = newer)
    pre_tier: u8,
    /// Pre-release index, e.g. 3 for "rc3"
    pre_num: u64,
}

fn parse_version_key(v: &str) -> VersionKey {
    // Match: numeric parts, optional pre-release tag, optional trailing number
    // Handles: "1.20.2", "1.26rc3", "1.18beta1", "1.18rc1", "1.18.0-beta.1"
    let v_lower = v.to_ascii_lowercase();
    // Normalise semver pre-release separator: "1.18.0-rc.2" → "1.18.0rc2"
    let v_norm = v_lower.replace("-rc.", "rc").replace("-beta.", "beta").replace("-alpha.", "alpha");

    // Split at the first non-numeric, non-dot character
    let tag_start = v_norm.find(|c: char| !c.is_ascii_digit() && c != '.');
    let (num_part, rest) = match tag_start {
        Some(i) => (&v_norm[..i], &v_norm[i..]),
        None    => (v_norm.as_str(), ""),
    };

    let nums: Vec<u64> = num_part
        .split('.')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap_or(0))
        .collect();

    let (pre_tier, pre_num) = if rest.is_empty() {
        (3u8, 0u64)
    } else if rest.starts_with("rc") {
        let n = rest[2..].parse().unwrap_or(0);
        (2, n)
    } else if rest.starts_with("beta") {
        let n = rest[4..].parse().unwrap_or(0);
        (1, n)
    } else if rest.starts_with("alpha") {
        let n = rest[5..].parse().unwrap_or(0);
        (0, n)
    } else {
        // Unknown suffix — treat as stable but preserve trailing digits for ordering
        let n: u64 = rest.chars().filter(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0);
        (3, n)
    };

    VersionKey { nums, pre_tier, pre_num }
}

fn cmp_num_vecs(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let len = a.len().max(b.len());
    for i in 0..len {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// Sorts versions newest-first using a natural version comparator.
///
/// Correctly handles: stable releases, rc, beta, alpha suffixes.
/// Examples (newest first): 1.21 > 1.21rc3 > 1.21rc2 > 1.21beta1 > 1.20.2 > 1.20.1
pub fn sort_versions_semver(mut versions: Vec<PackageEntry>) -> Vec<PackageEntry> {
    versions.sort_by(|a, b| {
        let ka = parse_version_key(&a.version);
        let kb = parse_version_key(&b.version);

        // 1. Compare numeric parts (newest first → reverse)
        match cmp_num_vecs(&ka.nums, &kb.nums).reverse() {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
        // 2. Same numeric version → stable > rc > beta > alpha (reverse for newest first)
        match ka.pre_tier.cmp(&kb.pre_tier).reverse() {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
        // 3. Same tag → higher index is newer (rc3 > rc2, reverse for newest first)
        ka.pre_num.cmp(&kb.pre_num).reverse()
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
    use regex::Regex;

    let mut filtered = versions;

    // Filter by major version — use our own parser instead of semver crate
    if let Some(major_ver) = major {
        filtered = filtered.into_iter()
            .filter(|entry| {
                let key = parse_version_key(&entry.version);
                key.nums.first().copied().unwrap_or(u64::MAX) == major_ver
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
