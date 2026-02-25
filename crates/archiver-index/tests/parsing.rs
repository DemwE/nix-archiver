//! Tests for parsing and version validation

use archiver_index::parsers::{extract_package_info_static, is_valid_version};
use regex::Regex;

#[test]
fn test_version_regex() {
    let indexer_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();
    
    let content = r#"
        pname = "nodejs";
        version = "14.17.0";
    "#;
    
    let caps = indexer_regex.captures(content).unwrap();
    assert_eq!(caps.get(1).unwrap().as_str(), "14.17.0");
}

#[test]
fn test_pname_extraction() {
    let version_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();
    
    // Test pname extraction from content
    let content = r#"
        pname = "gitlab.vim";
        version = "0.1.1";
    "#;
    
    let info = extract_package_info_static(
        "pkgs/applications/editors/vim/plugins/gitlab-vim/default.nix",
        content,
        &version_regex,
        None,
    ).unwrap();
    
    assert_eq!(info.attr_name, "gitlab.vim");
    assert_eq!(info.version, "0.1.1");
}

#[test]
fn test_fallback_to_path() {
    let version_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();
    
    // Test fallback to path when no pname
    let content = r#"
        version = "1.0.0";
    "#;
    
    let info = extract_package_info_static(
        "pkgs/development/libraries/mylib/default.nix",
        content,
        &version_regex,
        None,
    ).unwrap();
    
    assert_eq!(info.attr_name, "mylib");
    assert_eq!(info.version, "1.0.0");
}

#[test]
fn test_attr_name_extraction() {
    // Test attribute name extraction from path
    let path = "pkgs/development/libraries/nodejs/default.nix";
    let parts: Vec<&str> = path.split('/').collect();
    assert_eq!(parts[3], "nodejs");
}

#[test]
fn test_version_validation() {
    // Valid versions
    assert!(is_valid_version("14.17.0"));
    assert!(is_valid_version("1.2.3"));
    assert!(is_valid_version("2.0.0-alpha"));
    assert!(is_valid_version("3.1.4+build.123"));
    assert!(is_valid_version("20.20.0"));
    assert!(is_valid_version("v1.0.0"));
    
    // Invalid versions (Nix code)
    assert!(!is_valid_version("v${lib.head (lib.strings.splitString"));
    assert!(!is_valid_version("${version}"));
    assert!(!is_valid_version("lib.version"));
    assert!(!is_valid_version("(someFunction)"));
    assert!(!is_valid_version("{interpolation}"));
    
    // Invalid versions (no digits)
    assert!(!is_valid_version("invalid"));
    assert!(!is_valid_version(""));
}
