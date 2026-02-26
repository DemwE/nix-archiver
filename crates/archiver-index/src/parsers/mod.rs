//! Nix expression parser - AST-first, regex fallback.
//!
//! Primary entry point: [`extract_packages_from_file`]
//!
//! Flow:
//!   1. Try AST parser (rnix) - precise, handles multi-package files
//!   2. If AST returns nothing, fall back to regex heuristics

mod ast_parser;
mod regex_fallback;

use regex::Regex;
use crate::stats::PackageInfo;

// Re-export for tests / external callers
pub use ast_parser::{is_valid_version, path_to_attr_name};

/// Extracts all packages from a `.nix` file.
///
/// Tries AST parsing first; falls back to regex on parse failure.
/// One file can yield multiple packages (e.g. `python/default.nix`).
pub fn extract_packages_from_file(
    path: &str,
    content: &str,
    version_regex: &Regex,
) -> Vec<PackageInfo> {
    let ast_result = ast_parser::extract_packages_ast(path, content);
    if !ast_result.is_empty() {
        return ast_result;
    }

    if let Some(pkg) = regex_fallback::extract_packages_regex(path, content, version_regex) {
        log::debug!("[regex-fallback] {} -> {} v{}", path, pkg.attr_name, pkg.version);
        return vec![pkg];
    }

    vec![]
}
