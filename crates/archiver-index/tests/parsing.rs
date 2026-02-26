//! Tests for the AST-based Nix expression parser
//!
//! Covers the three extraction strategies implemented in `ast_parser.rs`:
//!   1. Multi-package callPackage + sourceVersion  (e.g. python/default.nix)
//!   2. mktplcRef  (VSCode extensions – biome-style and ruff/let-in-style)
//!   3. Single-package pname + version  (literal and interpolated)
//!
//! Also covers version validation and path-to-attr-name helpers.

use archiver_index::parsers::{extract_packages_from_file, is_valid_version};
use regex::Regex;

fn ver_regex() -> Regex {
    Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap()
}

/// Extract exactly one package from a single-package .nix file.
fn extract_one(path: &str, content: &str) -> Option<archiver_index::PackageInfo> {
    extract_packages_from_file(path, content, &ver_regex()).into_iter().next()
}

// ── Strategy 3: simple pname + version ───────────────────────────────────────

#[test]
fn test_ast_simple_pname_version() {
    let content = r#"
        { lib, stdenv }:
        stdenv.mkDerivation rec {
            pname = "ripgrep";
            version = "14.1.1";
        }
    "#;
    let info = extract_one("pkgs/tools/text/ripgrep/default.nix", content).unwrap();
    assert_eq!(info.attr_name, "ripgrep");
    assert_eq!(info.version, "14.1.1");
}

#[test]
fn test_ast_fallback_to_path_when_no_pname() {
    let content = r#"
        { lib, stdenv }:
        stdenv.mkDerivation {
            version = "1.0.0";
        }
    "#;
    let info = extract_one("pkgs/development/libraries/mylib/default.nix", content).unwrap();
    assert_eq!(info.attr_name, "mylib");
    assert_eq!(info.version, "1.0.0");
}

// ── Strategy 3: interpolated version ─────────────────────────────────────────

#[test]
fn test_ast_interpolated_version() {
    let content = r#"
        { lib, stdenv }:
        let
            major = "3";
            minor = "12";
            patch = "5";
        in
        stdenv.mkDerivation {
            pname = "cpython";
            version = "${major}.${minor}.${patch}";
        }
    "#;
    let info = extract_one(
        "pkgs/development/interpreters/cpython/default.nix", content,
    ).unwrap();
    assert_eq!(info.attr_name, "cpython");
    assert_eq!(info.version, "3.12.5");
}

// ── Strategy 2: mktplcRef – biome-style (simple attrset) ─────────────────────

#[test]
fn test_ast_mktplcref_biome_style() {
    let content = r#"
        { lib, vscode-utils }:
        vscode-utils.buildVscodeMarketplaceExtension {
            mktplcRef = {
                name = "biome";
                publisher = "biomejs";
                version = "2025.10.241456";
                hash = "sha256-tihEFcDDYr/khLIcJbR5VSC/RujEvp/gcnWlokAqNBc=";
            };
        }
    "#;
    let info = extract_one(
        "pkgs/applications/editors/vscode/extensions/biomejs.biome/default.nix",
        content,
    ).unwrap();
    assert_eq!(info.attr_name, "vscode-extensions.biomejs.biome");
    assert_eq!(info.version, "2025.10.241456");
}

// ── Strategy 2: mktplcRef – ruff-style (let + // merge) ──────────────────────

#[test]
fn test_ast_mktplcref_ruff_let_style() {
    let content = r#"
        { stdenvNoCC, lib, vscode-utils }:
        vscode-utils.buildVscodeMarketplaceExtension {
            mktplcRef =
                let
                    sources = {
                        "x86_64-linux" = { arch = "linux-x64"; hash = "sha256-abc="; };
                    };
                in
                {
                    name = "ruff";
                    publisher = "charliermarsh";
                    version = "2026.36.0";
                }
                // sources."x86_64-linux";
        }
    "#;
    let info = extract_one(
        "pkgs/applications/editors/vscode/extensions/charliermarsh.ruff/default.nix",
        content,
    ).unwrap();
    assert_eq!(info.attr_name, "vscode-extensions.charliermarsh.ruff");
    assert_eq!(info.version, "2026.36.0");
}

// ── Strategy 1: multi-package callPackage + sourceVersion ────────────────────

#[test]
fn test_ast_multi_package_sourceversion() {
    let content = r#"
        {
            python311 = callPackage ./cpython {
                sourceVersion = { major = "3"; minor = "11"; patch = "14"; };
            };
            python312 = callPackage ./cpython {
                sourceVersion = { major = "3"; minor = "12"; patch = "12"; };
            };
        }
    "#;
    let pkgs = extract_packages_from_file(
        "pkgs/development/interpreters/python/default.nix",
        content, &ver_regex(),
    );
    assert_eq!(pkgs.len(), 2);
    let names: Vec<&str> = pkgs.iter().map(|p| p.attr_name.as_str()).collect();
    assert!(names.contains(&"python311"));
    assert!(names.contains(&"python312"));
    let v311 = pkgs.iter().find(|p| p.attr_name == "python311").unwrap();
    assert_eq!(v311.version, "3.11.14");
    let v312 = pkgs.iter().find(|p| p.attr_name == "python312").unwrap();
    assert_eq!(v312.version, "3.12.12");
}

// ── version validation ────────────────────────────────────────────────────────

#[test]
fn test_version_validation() {
    // Valid
    assert!(is_valid_version("14.17.0"));
    assert!(is_valid_version("1.2.3"));
    assert!(is_valid_version("2.0.0-alpha"));
    assert!(is_valid_version("3.1.4+build.123"));
    assert!(is_valid_version("20.20.0"));
    assert!(is_valid_version("v1.0.0"));
    assert!(is_valid_version("2026.36.0"));

    // Invalid – Nix code patterns
    assert!(!is_valid_version("v${lib.head (lib.strings.splitString"));
    assert!(!is_valid_version("${version}"));
    assert!(!is_valid_version("lib.version"));
    assert!(!is_valid_version("(someFunction)"));
    assert!(!is_valid_version("{interpolation}"));

    // Invalid – no digits or empty
    assert!(!is_valid_version("invalid"));
    assert!(!is_valid_version(""));
}

// ── path-to-attr-name helper ──────────────────────────────────────────────────

#[test]
fn test_path_to_attr_name() {
    use archiver_index::parsers::path_to_attr_name;
    assert_eq!(
        path_to_attr_name("pkgs/development/libraries/nodejs/default.nix"),
        Some("nodejs".to_string())
    );
    assert_eq!(
        path_to_attr_name("pkgs/applications/editors/vscode/extensions/biomejs.biome/default.nix"),
        Some("biomejs.biome".to_string())
    );
    // Too short – no valid parent dir
    assert_eq!(path_to_attr_name("default.nix"), None);
}
