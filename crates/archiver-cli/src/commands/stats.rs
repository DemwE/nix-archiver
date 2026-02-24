//! Stats command implementation

use anyhow::Result;
use archiver_db::ArchiverDb;
use colored::Colorize;

/// Displays database statistics
pub fn cmd_stats(db: ArchiverDb) -> Result<()> {
    println!("{}", "Database Statistics:".bright_cyan().bold());
    println!("  {}: {}", "Packages".bright_yellow(), db.package_count().to_string().bold());
    println!("  {}: {}", "Processed commits".bright_yellow(), db.processed_commit_count().to_string().bold());
    Ok(())
}
