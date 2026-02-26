//! AST-based parser using rnix for accurate Nix expression parsing

use std::collections::HashMap;
use rowan::ast::AstNode;
use rnix::ast::{self, AttrpathValue, Expr, Attr, HasEntry, AstToken};
use rnix::ast::InterpolPart;
use crate::stats::PackageInfo;

/// Keys that are NOT package names in top-level attribute sets
const NON_PACKAGE_KEYS: &[&str] = &[
    "self", "hash", "passthruFun", "callPackage", "sourceVersion",
    "patches", "meta", "src", "lib", "stdenv", "buildInputs",
    "nativeBuildInputs", "propagatedBuildInputs", "config", "pkgs",
    "inherit", "version", "pname", "description", "homepage", "license",
];

// ─── Public entry point ───────────────────────────────────────────────────────

/// Parses a .nix file using rnix AST and returns all packages found.
/// Returns empty Vec on parse failure (caller should use regex fallback).
pub fn extract_packages_ast(path: &str, content: &str) -> Vec<PackageInfo> {
    let parsed = rnix::Root::parse(content);

    if !parsed.errors().is_empty() {
        log::debug!(
            "[AST] {} parse error(s) in '{}', using regex fallback",
            parsed.errors().len(),
            path
        );
        return vec![];
    }

    let root = parsed.tree();

    // Strategy 1: multi-package files (e.g. python/default.nix)
    //   python311 = callPackage ./cpython { sourceVersion = { major="3"; … }; };
    let multi = extract_multi_callpackage(root.syntax());
    if !multi.is_empty() {
        log::debug!("[AST] multi-package '{}': {} package(s)", path, multi.len());
        return multi;
    }

    if let Some(pkg) = extract_mktplcref(root.syntax(), path) {
        log::debug!("[AST] mktplcRef '{}': {}", path, pkg.attr_name);
        return vec![pkg];
    }

    if let Some(pkg) = extract_single_package(root.syntax(), path) {
        log::debug!("[AST] single-package '{}': {} v{}", path, pkg.attr_name, pkg.version);
        return vec![pkg];
    }

    vec![]
}

// ─── Strategy 1 – multi-package (callPackage + sourceVersion) ────────────────

fn extract_multi_callpackage(root: &rnix::SyntaxNode) -> Vec<PackageInfo> {
    let mut result = Vec::new();

    for node in root.descendants() {
        let Some(kv) = AttrpathValue::cast(node) else { continue };

        let Some(key) = get_simple_key(&kv) else { continue };

        // Skip known non-package keys
        if NON_PACKAGE_KEYS.contains(&key.as_str()) {
            continue;
        }

        // The key must look like a package name (alphanumeric + common chars)
        if !looks_like_package_name(&key) {
            continue;
        }

        let Some(value) = kv.value() else { continue };

        // Only process Apply expressions (callPackage ./path { ... })
        if !matches!(value, Expr::Apply(_)) {
            continue;
        }

        // Search inside the value for a sourceVersion AttrSet
        if let Some(version) = find_sourceversion_in_expr(&value) {
            result.push(PackageInfo {
                attr_name: key,
                version,
            });
        }
    }

    result
}

/// Searches within an expression for `sourceVersion = { major=…; minor=…; patch=…; }`
fn find_sourceversion_in_expr(expr: &Expr) -> Option<String> {
    // Walk the syntax tree of this expression
    for node in expr.syntax().descendants() {
        let Some(kv) = AttrpathValue::cast(node) else { continue };

        if get_simple_key(&kv).as_deref() != Some("sourceVersion") {
            continue;
        }

        let Some(Expr::AttrSet(sv_set)) = kv.value() else { continue };

        if let Some(v) = extract_version_from_attrset_bindings(&sv_set) {
            return Some(v);
        }
    }
    None
}

/// Extracts version string from an AttrSet with major/minor/patch/suffix bindings
fn extract_version_from_attrset_bindings(set: &ast::AttrSet) -> Option<String> {
    let mut vars: HashMap<String, String> = HashMap::new();

    for kv in set.attrpath_values() {
        let Some(key) = get_simple_key(&kv) else { continue };
        let Some(Expr::Str(s)) = kv.value() else { continue };
        let Some(val) = get_string_literal(&s) else { continue };
        vars.insert(key, val);
    }

    let major = vars.get("major")?;
    let minor = vars.get("minor")?;
    let patch = vars.get("patch").map(|s| s.as_str()).unwrap_or("0");
    let suffix = vars.get("suffix").map(|s| s.as_str()).unwrap_or("");

    Some(format!("{}.{}.{}{}", major, minor, patch, suffix))
}

// ─── Strategy 2 – mktplcRef (VSCode extensions) ──────────────────────────────

fn extract_mktplcref(root: &rnix::SyntaxNode, path: &str) -> Option<PackageInfo> {
    for node in root.descendants() {
        let Some(kv) = AttrpathValue::cast(node) else { continue };

        if get_simple_key(&kv).as_deref() != Some("mktplcRef") {
            continue;
        }

        // mktplcRef value can be:
        //   { name=…; publisher=…; version=…; } (biome-style)
        //   let sources = {…}; in { name=…; version=…; } // sources.${…} (ruff-style)
        let Some(value) = kv.value() else { continue };
        let Some(ref_set) = unwrap_to_attrset(value) else { continue };

        // Extract version from the mktplcRef attrset
        let version = match extract_string_binding(&ref_set, "version") {
            Some(v) => v,
            None => continue,
        };

        // Build "vscode-extensions.publisher.name" attr_name to match the
        // actual nixpkgs attribute path (pkgs.vscode-extensions.biomejs.biome).
        // Falls back to path-derived name if publisher/name bindings are absent.
        let publisher = extract_string_binding(&ref_set, "publisher");
        let name = extract_string_binding(&ref_set, "name");
        let attr_name = match (publisher, name) {
            (Some(p), Some(n)) => format!("vscode-extensions.{}.{}", p, n),
            (None, Some(n)) => format!("vscode-extensions.{}", n),
            _ => path_to_attr_name(path)
                .or_else(|| find_pname_in_tree(root))?,
        };

        return Some(PackageInfo {
            attr_name,
            version,
        });
    }

    None
}

/// Extracts the innermost AttrSet from an expression, handling:
/// - Direct: `{ … }`
/// - Let-in: `let … in { … }`
/// - BinOp `//` merge: `{ … } // extra` (returns left-hand attrset)
fn unwrap_to_attrset(expr: Expr) -> Option<ast::AttrSet> {
    match expr {
        Expr::AttrSet(s) => Some(s),
        Expr::LetIn(let_in) => {
            let body = let_in.body()?;
            unwrap_to_attrset(body)
        }
        Expr::BinOp(binop) => {
            // `lhs // rhs` — the version/name/publisher live in the lhs attrset
            let lhs = binop.lhs()?;
            unwrap_to_attrset(lhs)
        }
        _ => None,
    }
}

// ─── Strategy 3 – single package (pname + version) ───────────────────────────

fn extract_single_package(root: &rnix::SyntaxNode, path: &str) -> Option<PackageInfo> {
    // Collect a flat map of all simple string bindings in the file.
    // This gives us major/minor/patch/suffix and similar vars for interpolation.
    let vars = collect_string_vars(root);

    // Determine attr_name: pname binding OR path-based
    let attr_name = vars.get("pname").cloned()
        .or_else(|| path_to_attr_name(path))?;

    // Determine version
    let version = resolve_version(root, &vars)?;

    Some(PackageInfo {
        attr_name,
        version,
    })
}

/// Collects every `identifier = "literal string"` binding in the file.
fn collect_string_vars(root: &rnix::SyntaxNode) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for node in root.descendants() {
        let Some(kv) = AttrpathValue::cast(node) else { continue };
        let Some(key) = get_simple_key(&kv) else { continue };
        let Some(Expr::Str(s)) = kv.value() else { continue };
        let Some(val) = get_string_literal(&s) else { continue };

        // Only insert the first occurrence (outer scope wins)
        map.entry(key).or_insert(val);
    }

    map
}

/// Finds and resolves a `version = …` binding in the file.
fn resolve_version(root: &rnix::SyntaxNode, vars: &HashMap<String, String>) -> Option<String> {
    for node in root.descendants() {
        let Some(kv) = AttrpathValue::cast(node) else { continue };

        if get_simple_key(&kv).as_deref() != Some("version") {
            continue;
        }

        let Some(value) = kv.value() else { continue };

        match value {
            // Simple literal: version = "1.2.3";
            Expr::Str(ref s) => {
                if let Some(v) = get_string_literal(s) {
                    if is_valid_version(&v) {
                        return Some(v);
                    }
                }
                // Might be interpolated: "${major}.${minor}.${patch}"
                if let Some(v) = resolve_string_interpolation(s, vars) {
                    if is_valid_version(&v) {
                        return Some(v);
                    }
                }
            }
            // version = with sourceVersion; "${major}.${minor}.${patch}"
            Expr::With(ref with_expr) => {
                if let Some(v) = resolve_with_expr(with_expr, vars) {
                    if is_valid_version(&v) {
                        return Some(v);
                    }
                }
            }
            _ => {}
        }
    }

    // Fallback: assemble version from major/minor/patch vars
    if let (Some(major), Some(minor), Some(patch)) = (
        vars.get("major"), vars.get("minor"), vars.get("patch")
    ) {
        let suffix = vars.get("suffix").map(|s| s.as_str()).unwrap_or("");
        let v = format!("{}.{}.{}{}", major, minor, patch, suffix);
        if is_valid_version(&v) {
            return Some(v);
        }
    }

    None
}

/// Resolves `with <ns>; "${var1}.${var2}"` expressions.
fn resolve_with_expr(with_expr: &ast::With, vars: &HashMap<String, String>) -> Option<String> {
    // Get namespace: if it's an Ident or AttrSet, collect its vars
    let ns_vars: HashMap<String, String> = match with_expr.namespace()? {
        Expr::Ident(ident) => {
            // Look up "sourceVersion" (or whatever) in our flat vars
            // We already have major/minor/patch in the flat vars map from inside sourceVersion
            let _ns_name = ident.ident_token()?.text().to_string();
            HashMap::new() // flat vars already contains what we need
        }
        _ => HashMap::new(),
    };

    // Merge flat vars with namespace-specific vars
    let mut merged = vars.clone();
    merged.extend(ns_vars);

    // Resolve the body expression
    match with_expr.body()? {
        Expr::Str(ref s) => resolve_string_interpolation(s, &merged),
        _ => None,
    }
}

// ─── String helpers ─────────────────────────────────────────────────────────

/// Returns the string value if the Str has no interpolations.
fn get_string_literal(s: &ast::Str) -> Option<String> {
    let mut result = String::new();
    for part in s.parts() {
        match part {
            InterpolPart::Literal(lit) => {
                result.push_str(lit.syntax().text());
            }
            InterpolPart::Interpolation(_) => return None,
        }
    }
    Some(result)
}

/// Resolves a string with `${ident}` interpolations using the provided vars map.
fn resolve_string_interpolation(s: &ast::Str, vars: &HashMap<String, String>) -> Option<String> {
    let mut result = String::new();
    for part in s.parts() {
        match part {
            InterpolPart::Literal(lit) => {
                result.push_str(lit.syntax().text());
            }
            InterpolPart::Interpolation(dyn_part) => {
                match dyn_part.expr()? {
                    Expr::Ident(ident) => {
                        let name = ident.ident_token()?.text().to_string();
                        let val = vars.get(&name)?;
                        result.push_str(val);
                    }
                    // e.g. ${versions.major} – too complex, skip
                    _ => return None,
                }
            }
        }
    }
    Some(result)
}

// ─── AttrSet helpers ─────────────────────────────────────────────────────────

/// Extracts a named literal string binding from an AttrSet.
fn extract_string_binding(set: &ast::AttrSet, key: &str) -> Option<String> {
    for kv in set.attrpath_values() {
        if get_simple_key(&kv).as_deref() == Some(key) {
            if let Some(Expr::Str(s)) = kv.value() {
                return get_string_literal(&s);
            }
        }
    }
    None
}

/// Walks the root tree to find the first `pname = "…"` binding.
fn find_pname_in_tree(root: &rnix::SyntaxNode) -> Option<String> {
    for node in root.descendants() {
        let Some(kv) = AttrpathValue::cast(node) else { continue };
        if get_simple_key(&kv).as_deref() != Some("pname") { continue; }
        if let Some(Expr::Str(s)) = kv.value() {
            return get_string_literal(&s);
        }
    }
    None
}

// ─── Key / name helpers ──────────────────────────────────────────────────────

/// Returns the simple (non-nested) key of an `AttrpathValue`, if any.
/// e.g. `foo = …` → `Some("foo")`, `foo.bar = …` → `None`
pub fn get_simple_key(kv: &AttrpathValue) -> Option<String> {
    let attrpath = kv.attrpath()?;
    let mut attrs = attrpath.attrs();
    let first = attrs.next()?;
    // Reject dotted paths like `foo.bar`
    if attrs.next().is_some() {
        return None;
    }
    match first {
        Attr::Ident(ident) => Some(ident.ident_token()?.text().to_string()),
        Attr::Str(s) => get_string_literal(&s),
        _ => None,
    }
}

/// Heuristic: does the string look like a Nix package attribute name?
fn looks_like_package_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 80 {
        return false;
    }
    // Must start with a letter or underscore
    name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Extracts a package attribute name from a file path.
/// e.g. `pkgs/development/interpreters/python/default.nix` → `python`
pub fn path_to_attr_name(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 4 && parts[0] == "pkgs" {
        // Remove "default.nix" if present from the last component
        let candidate = parts[parts.len() - 2];
        if candidate != "pkgs" {
            return Some(candidate.to_string());
        }
    }
    None
}

// ─── Version validation ──────────────────────────────────────────────────────

/// Returns true if the string looks like a real version (not Nix code).
pub fn is_valid_version(version: &str) -> bool {
    if version.is_empty() {
        return false;
    }
    // Reject Nix code patterns
    if version.contains("${") || version.contains("lib.")
        || version.contains('(') || version.contains(')')
        || version.contains('{') || version.contains('}')
        || version.contains("splitString")
    {
        return false;
    }
    // Must contain at least one digit
    if !version.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }
    // Only allowed chars: alphanumeric, . - _ +
    version.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
}
