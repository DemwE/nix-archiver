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

/// Table row for displaying a package summary across multiple packages
#[derive(Tabled)]
pub struct PackageSummaryRow {
    #[tabled(rename = "Package")]
    pub attr_name: String,
    #[tabled(rename = "Versions")]
    pub version_count: String,
    #[tabled(rename = "Latest")]
    pub latest_version: String,
    #[tabled(rename = "Date")]
    pub latest_date: String,
}

/// Table row for the package-set breakdown sidebar (mirrors NixOS search)
#[derive(Tabled)]
pub struct PackageSetRow {
    #[tabled(rename = "Package set")]
    pub set: String,
    #[tabled(rename = "Packages")]
    pub packages: String,
}
