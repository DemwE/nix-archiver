//! Output formatting structures for CLI display

use tabled::Tabled;

/// Table row for displaying package versions
#[derive(Tabled)]
pub struct VersionRow {
    #[tabled(rename = "Version")]
    pub version: String,
    #[tabled(rename = "Commit")]
    pub commit: String,
    #[tabled(rename = "Date")]
    pub date: String,
    #[tabled(rename = "NAR Hash")]
    pub nar_hash: String,
}
