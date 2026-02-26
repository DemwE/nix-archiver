//! CLI integration tests
//!
//! These tests run the compiled `nix-archiver` binary directly, so they work
//! even though the helper functions live in private modules of the bin crate.

use std::process::Command;
use tempfile::TempDir;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nix-archiver"))
}

// ── help / version ────────────────────────────────────────────────────────────

#[test]
fn test_help_exits_zero() {
    let status = bin().arg("--help").status().expect("failed to run binary");
    assert!(status.success(), "--help should exit 0");
}

#[test]
fn test_version_flag() {
    let output = bin().arg("--version").output().expect("failed to run binary");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // clap emits "nix-archiver X.Y.Z"
    assert!(
        stdout.contains("nix-archiver"),
        "version output should contain binary name, got: {}",
        stdout
    );
}

// ── stats on empty database ───────────────────────────────────────────────────

#[test]
fn test_stats_on_empty_db() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");

    let status = bin()
        .arg("--database").arg(&db_path)
        .arg("stats")
        .status()
        .expect("failed to run binary");

    assert!(status.success(), "stats on empty db should exit 0");
}

// ── search on empty database ──────────────────────────────────────────────────

#[test]
fn test_search_on_empty_db_prints_not_found() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");

    let output = bin()
        .arg("--database").arg(&db_path)
        .arg("search")
        .arg("nonexistentpackage_xyz")
        .output()
        .expect("failed to run binary");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("No versions found") || combined.contains("not found") || combined.contains("nonexistentpackage_xyz"),
        "expected a 'not found' message, got: {}",
        combined
    );
}
