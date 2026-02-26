//! Generate command implementation

use anyhow::{Context, Result};
use archiver_db::ArchiverDb;
use colored::Colorize;
use rnix::ast::{self, AttrpathValue, Expr, InterpolPart};
use rowan::ast::AstNode;
use std::path::PathBuf;

use crate::helpers::sort_versions_semver;

// ─── Parser ───────────────────────────────────────────────────────────────────

/// Parses a packages.nix attrset and returns (attr_name, version) pairs.
///
/// Uses rnix AST so comments, multi-line strings, and all valid Nix syntax are
/// handled correctly — no manual comment stripping or regex needed.
fn parse_packages_spec(path: &std::path::Path, content: &str) -> Result<Vec<(String, String)>> {
    let parsed = rnix::Root::parse(content);

    if !parsed.errors().is_empty() {
        let errs: Vec<String> = parsed.errors().iter().map(|e| e.to_string()).collect();
        anyhow::bail!("Nix parse error in {}: {}", path.display(), errs.join("; "));
    }

    let mut result = Vec::new();

    for node in parsed.tree().syntax().descendants() {
        let Some(kv) = AttrpathValue::cast(node) else { continue };

        // Accept only simple (non-dotted) keys
        let Some(attrpath) = kv.attrpath() else { continue };
        let mut attrs = attrpath.attrs();
        let Some(first) = attrs.next() else { continue };
        if attrs.next().is_some() {
            // dotted path like foo.bar — not a package spec entry
            continue;
        }

        let attr_name = match first {
            ast::Attr::Ident(ident) => match ident.ident_token() {
                Some(t) => t.text().to_string(),
                None => continue,
            },
            _ => continue,
        };

        // Value must be a plain string literal (no interpolation)
        let Some(value) = kv.value() else { continue };
        let Expr::Str(s) = value else { continue };

        // normalized_parts() yields InterpolPart<String> — Literal is already a plain String,
        // Interpolation means ${...} is present and we skip those entries.
        let mut version = String::new();
        let mut has_interpolation = false;
        for part in s.normalized_parts() {
            match part {
                InterpolPart::Literal(text) => version.push_str(&text),
                InterpolPart::Interpolation(_) => {
                    has_interpolation = true;
                    break;
                }
            }
        }

        if has_interpolation {
            eprintln!(
                "{} Skipping '{}': interpolated strings are not supported",
                "⚠".yellow(),
                attr_name
            );
            continue;
        }

        result.push((attr_name, version));
    }

    Ok(result)
}

// ─── Command ──────────────────────────────────────────────────────────────────

/// Generates frozen.nix file from package specification
pub fn cmd_generate(input: PathBuf, output: PathBuf, db: ArchiverDb) -> Result<()> {
    use std::fs;
    use std::io::Write;

    println!(
        "{} Reading package specification from {}...",
        "📖".bright_cyan(),
        input.display()
    );

    let content = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read input file: {}", input.display()))?;

    let spec = parse_packages_spec(&input, &content)?;

    let mut packages = Vec::new();
    let mut errors = Vec::new();

    for (attr_name, version) in spec {
        let entry = if version == "latest" {
            let available = db.get_all_versions(&attr_name)?;
            if available.is_empty() {
                errors.push(format!("No versions found for package '{}'", attr_name));
                continue;
            }
            let mut sorted = sort_versions_semver(available);
            let newest = sorted.remove(0);
            println!(
                "  {} Resolved: {} latest → v{} @ commit {}",
                "✓".green(),
                attr_name.bold(),
                newest.version.bright_yellow(),
                &newest.commit_sha[..12].dimmed()
            );
            newest
        } else {
            match db.get(&attr_name, &version)? {
                Some(entry) => {
                    println!(
                        "  {} Found: {} v{} @ commit {}",
                        "✓".green(),
                        attr_name.bold(),
                        version.bright_yellow(),
                        &entry.commit_sha[..12].dimmed()
                    );
                    entry
                }
                None => {
                    errors.push(format!(
                        "Package {}:{} not found in database",
                        attr_name, version
                    ));
                    let available = db.get_all_versions(&attr_name)?;
                    if !available.is_empty() {
                        let sorted = sort_versions_semver(available);
                        let suggestions: Vec<String> = sorted
                            .iter()
                            .take(5)
                            .map(|e| e.version.clone())
                            .collect();
                        errors.push(format!(
                            "         Available versions: {}",
                            suggestions.join(", ")
                        ));
                    } else {
                        errors.push(format!(
                            "         No versions available for package '{}'",
                            attr_name
                        ));
                    }
                    continue;
                }
            }
        };
        packages.push(entry);
    }

    // Report errors if any
    if !errors.is_empty() {
        eprintln!("\n{} Errors found:\n", "❌".red().bold());
        for error in &errors {
            eprintln!("  {}", error.red());
        }
        eprintln!("\n{} Expected input format:", "💡".yellow());
        eprintln!(
            "  {{\n    nodejs = \"20.11.0\";  # specific version\n    python = \"latest\";   # newest version in database\n  }}"
        );
        anyhow::bail!("Failed to resolve all packages. Fix the errors above and try again.");
    }

    if packages.is_empty() {
        eprintln!("{} No packages found in input file.", "❌".red());
        eprintln!("\n{} Expected input format:", "💡".yellow());
        eprintln!(
            "  {{\n    nodejs = \"20.11.0\";  # specific version\n    python = \"latest\";   # newest version in database\n  }}"
        );
        anyhow::bail!("Input file is empty or invalid");
    }

    // Generate frozen.nix
    println!(
        "\n{} Generating frozen.nix with {} package{}...",
        "🔨".bright_cyan(),
        packages.len(),
        if packages.len() == 1 { "" } else { "s" }
    );

    let mut nix_content = String::from("# Generated by nix-archiver\n");
    nix_content
        .push_str("# This file pins packages to specific historical versions from Nixpkgs\n\n");
    nix_content.push_str("{\n");

    for entry in &packages {
        nix_content.push_str(&format!(
            "  # {} v{} (commit: {})\n",
            entry.attr_name, entry.version, &entry.commit_sha
        ));
        nix_content.push_str(&format!(
            "  {} = import ({}) {{}};\n\n",
            entry.attr_name,
            entry.to_nix_fetchtarball()
        ));
    }

    nix_content.push_str("}\n");

    let mut file = fs::File::create(&output)
        .with_context(|| format!("Failed to create output file: {}", output.display()))?;

    file.write_all(nix_content.as_bytes())
        .with_context(|| format!("Failed to write to output file: {}", output.display()))?;

    println!(
        "{} Successfully generated: {}",
        "✓".green().bold(),
        output.display().to_string().bold()
    );
    println!("\n{} Usage:\n  nix-shell {}", "💡".yellow(), output.display());

    Ok(())
}
