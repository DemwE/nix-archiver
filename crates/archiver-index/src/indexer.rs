//! Main indexer structure

use anyhow::{Context, Result};
use archiver_db::ArchiverDb;
use git2::Repository;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Main indexer structure
pub struct Indexer {
    /// Path to Nixpkgs Git repository
    pub(crate) repo_path: PathBuf,
    
    /// Database for storing results (thread-safe)
    pub(crate) db: Arc<ArchiverDb>,
    
    /// Regex for extracting versions from Nix files
    pub(crate) version_regex: Arc<Regex>,
}

impl Indexer {
    /// Creates a new indexer for the given repository and database
    pub fn new<P: AsRef<Path>>(repo_path: P, db: ArchiverDb) -> Result<Self> {
        // Verify repository exists
        let repo = Repository::open(repo_path.as_ref())
            .with_context(|| format!("Failed to open repository at {:?}", repo_path.as_ref()))?;
        drop(repo); // We'll open it per-thread
        
        // Regex for extracting versions in format: version = "x.y.z"
        // Also supports: pname = "name"; version = "1.2.3";
        let version_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#)
            .context("Failed to compile version regex")?;

        Ok(Self {
            repo_path: repo_path.as_ref().to_path_buf(),
            db: Arc::new(db),
            version_regex: Arc::new(version_regex),
        })
    }
}
