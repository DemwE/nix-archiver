//! Regex-based fallback parser for .nix files that cannot be parsed by rnix.

use regex::Regex;
use crate::stats::PackageInfo;
use super::ast_parser::{is_valid_version, path_to_attr_name};

/// Extracts package info using regex heuristics.
/// Used when AST parsing fails or yields no results.
pub fn extract_packages_regex(
    path: &str,
    content: &str,
    version_regex: &Regex,
    nar_hash: Option<String>,
) -> Option<PackageInfo> {
    let attr_name = extract_pname(content)
        .or_else(|| extract_callpackage_attr(content))
        .or_else(|| path_to_attr_name(path))?;

    // 1. Simple literal: version = "1.2.3";
    let version = version_regex.captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .filter(|v| is_valid_version(v));

    // 2. sourceVersion block
    let version = version.or_else(|| extract_sourceversion(content));

    // 3. mktplcRef
    let version = version.or_else(|| extract_mktplcref(content));

    // 4. major/minor/patch interpolation
    let version = version.or_else(|| extract_interpolation(content));

    let version = version?;

    Some(PackageInfo { attr_name, version, nar_hash })
}

fn extract_pname(content: &str) -> Option<String> {
    Regex::new(r#"pname\s*=\s*"([^"]+)""#).ok()?
        .captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn extract_callpackage_attr(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains('=') && line.contains("callPackage") {
            if let Some(eq) = line.find('=') {
                let attr = line[..eq].trim().split_whitespace().last()?.to_string();
                let end = std::cmp::min(i + 20, lines.len());
                for j in (i + 1)..end {
                    if lines[j].contains("sourceVersion") {
                        return Some(attr);
                    }
                    if lines[j].trim().starts_with('}') && !lines[j].contains('{') {
                        break;
                    }
                }
            }
        }
    }
    None
}

fn extract_sourceversion(content: &str) -> Option<String> {
    let re = Regex::new(
        r#"sourceVersion\s*=\s*\{[^}]*major\s*=\s*"(\d+)"[^}]*minor\s*=\s*"(\d+)"[^}]*patch\s*=\s*"(\d+)"[^}]*\}"#
    ).ok()?;

    let caps = re.captures(content)?;
    let major = caps.get(1)?.as_str();
    let minor = caps.get(2)?.as_str();
    let patch = caps.get(3)?.as_str();
    let suffix = Regex::new(r#"suffix\s*=\s*"([^"]*)""#).ok()?
        .captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    Some(format!("{}.{}.{}{}", major, minor, patch, suffix))
}

fn extract_mktplcref(content: &str) -> Option<String> {
    Regex::new(r#"mktplcRef\s*=\s*\{[^}]*version\s*=\s*"([^"]+)"[^}]*\}"#).ok()?
        .captures(content)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .filter(|v| is_valid_version(v))
}

fn extract_interpolation(content: &str) -> Option<String> {
    let major = Regex::new(r#"\bmajor\s*=\s*"(\d+)""#).ok()?
        .captures(content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())?;
    let minor = Regex::new(r#"\bminor\s*=\s*"(\d+)""#).ok()?
        .captures(content).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())?;
    let patch = Regex::new(r#"\bpatch\s*=\s*"(\d+)""#).ok()?
        .captures(content).and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "0".to_string());

    Some(format!("{}.{}.{}", major, minor, patch))
}
