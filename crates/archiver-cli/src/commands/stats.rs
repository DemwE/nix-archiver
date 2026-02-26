//! Stats command implementation

use anyhow::Result;
use archiver_db::ArchiverDb;
use colored::Colorize;

fn format_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;

    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Displays database statistics
pub fn cmd_stats(db: ArchiverDb) -> Result<()> {
    let size = db.db_size_bytes();
    println!("{}", "Database Statistics:".bright_cyan().bold());
    println!("  {}: {}", "Packages".bright_yellow(),          db.package_count().to_string().bold());
    println!("  {}: {}", "Processed commits".bright_yellow(), db.processed_commit_count().to_string().bold());
    println!("  {}: {}", "Database size".bright_yellow(),     format_size(size).bold());
    Ok(())
}
