//! Statistics and data structures for indexing

use std::time::Duration;
use crate::formatting::{format_number, format_duration};

/// Information extracted from package file
#[derive(Debug)]
pub struct PackageInfo {
    pub attr_name: String,
    pub version: String,
}

/// Indexing statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub processed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub packages_found: usize,
    pub packages_inserted: usize,
    pub elapsed_time: Duration,
}

impl Default for IndexStats {
    fn default() -> Self {
        Self {
            processed: 0,
            skipped: 0,
            errors: 0,
            packages_found: 0,
            packages_inserted: 0,
            elapsed_time: Duration::from_secs(0),
        }
    }
}

/// Statistics for processing a single commit
#[derive(Debug, Default)]
pub(crate) struct CommitStats {
    pub packages_found: usize,
    pub packages_inserted: usize,
}

impl std::fmt::Display for IndexStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Commits: {} processed, {} skipped, {} errors | Packages: {} found, {} inserted | Time: {}",
            format_number(self.processed), 
            format_number(self.skipped), 
            self.errors,
            format_number(self.packages_found), 
            format_number(self.packages_inserted),
            format_duration(self.elapsed_time)
        )
    }
}
