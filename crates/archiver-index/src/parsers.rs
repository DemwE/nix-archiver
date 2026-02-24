//! Parsing utilities for extracting package information

use regex::Regex;
use crate::stats::PackageInfo;

/// Extracts package information from a .nix file (static version for use in closures)
pub(crate) fn extract_package_info_static(
    path: &str,
    content: &str,
    version_regex: &Regex,
    nar_hash: Option<String>,
) -> Option<PackageInfo> {
    // Extract attribute name from path
    // e.g., "pkgs/development/libraries/nodejs/default.nix" -> "nodejs"
    let attr_name = extract_attr_name(path)?;

    // Extract version using regex
    let version = version_regex
        .captures(content)?
        .get(1)?
        .as_str()
        .to_string();

    // Filter out invalid versions (Nix code, interpolations, etc.)
    if !is_valid_version(&version) {
        return None;
    }
    
    Some(PackageInfo {
        attr_name,
        version,
        nar_hash,
    })
}

/// Validates that a version string looks like a real version, not Nix code
pub(crate) fn is_valid_version(version: &str) -> bool {
    // Reject versions containing Nix interpolation or code patterns
    if version.contains("${") || version.contains("lib.") || 
       version.contains('(') || version.contains(')') ||
       version.contains('{') || version.contains('}') ||
       version.contains("splitString") {
        return false;
    }
    
    // Version should contain at least one digit
    if !version.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    
    // Version should only contain allowed characters:
    // alphanumeric, dots, hyphens, underscores, plus signs
    version.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == '+'
    })
}

/// Extracts attribute name from file path
pub(crate) fn extract_attr_name(path: &str) -> Option<String> {
    // Path format: pkgs/<category>/<subcategory>/<name>/...
    let parts: Vec<&str> = path.split('/').collect();
    
    if parts.len() >= 4 && parts[0] == "pkgs" {
        // Try to extract name from the third level
        Some(parts[3].to_string())
    } else {
        None
    }
}
