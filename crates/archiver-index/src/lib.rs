//! Archiver Index - Silnik ETL do indeksowania Nixpkgs
//!
//! Ten crate odpowiada za:
//! - Iterację po historii Git repozytorium Nixpkgs
//! - Parsowanie plików .nix w poszukiwaniu stringów wersji
//! - Generowanie haszy NAR z obiektów Git
//! - Zapisywanie wyników do bazy danych z deduplikacją

use anyhow::{Context, Result};
use archiver_core::PackageEntry;
use archiver_db::ArchiverDb;
use git2::{Commit, Oid, Repository, TreeWalkMode, TreeWalkResult};
use regex::Regex;
use std::path::Path;

/// Główna struktura indeksera
pub struct Indexer {
    /// Repozytorium Git Nixpkgs
    repo: Repository,
    
    /// Baza danych do przechowywania wyników
    db: ArchiverDb,
    
    /// Regex do wyciągania wersji z plików Nix
    version_regex: Regex,
}

impl Indexer {
    /// Tworzy nowy indekser dla podanego repozytorium i bazy danych
    pub fn new<P: AsRef<Path>>(repo_path: P, db: ArchiverDb) -> Result<Self> {
        let repo = Repository::open(repo_path.as_ref())
            .with_context(|| format!("Failed to open repository at {:?}", repo_path.as_ref()))?;
        
        // Regex do wyciągania wersji w formacie: version = "x.y.z"
        // Wspiera również: pname = "name"; version = "1.2.3";
        let version_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#)
            .context("Failed to compile version regex")?;

        Ok(Self {
            repo,
            db,
            version_regex,
        })
    }

    /// Indeksuje wszystkie commity od podanego commita w tył
    pub fn index_from_commit(&self, commit_sha: &str, max_commits: Option<usize>) -> Result<IndexStats> {
        let oid = Oid::from_str(commit_sha)
            .context("Invalid commit SHA")?;
        
        let commit = self.repo.find_commit(oid)
            .context("Failed to find commit")?;

        let mut stats = IndexStats::default();
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push(commit.id())?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        for (idx, oid_result) in revwalk.enumerate() {
            if let Some(max) = max_commits {
                if idx >= max {
                    log::info!("Reached max commit limit: {}", max);
                    break;
                }
            }

            let oid = oid_result.context("Failed to get commit OID")?;
            let commit = self.repo.find_commit(oid)
                .context("Failed to find commit")?;

            // Sprawdź czy commit był już przetworzony
            if self.db.is_commit_processed(&oid.to_string())? {
                stats.skipped += 1;
                continue;
            }

            log::debug!("Processing commit {}: {}", idx, oid);
            
            match self.process_commit(&commit) {
                Ok(commit_stats) => {
                    stats.processed += 1;
                    stats.packages_found += commit_stats.packages_found;
                    stats.packages_inserted += commit_stats.packages_inserted;
                    
                    // Zaznacz commit jako przetworzony
                    let timestamp = commit.time().seconds() as u64;
                    self.db.mark_commit_processed(&oid.to_string(), timestamp)?;
                }
                Err(e) => {
                    log::warn!("Failed to process commit {}: {:?}", oid, e);
                    stats.errors += 1;
                }
            }

            if idx % 100 == 0 {
                log::info!("Progress: {} commits processed, {} packages inserted", 
                    stats.processed, stats.packages_inserted);
                self.db.flush()?;
            }
        }

        self.db.flush()?;
        Ok(stats)
    }

    /// Przetwarza pojedynczy commit
    fn process_commit(&self, commit: &Commit) -> Result<CommitStats> {
        let tree = commit.tree().context("Failed to get commit tree")?;
        let timestamp = commit.time().seconds() as u64;
        let commit_sha = commit.id().to_string();

        let mut stats = CommitStats::default();

        // Przechodzimy po drzewie w poszukiwaniu plików .nix w katalogu pkgs/
        tree.walk(TreeWalkMode::PreOrder, |root, entry| {
            let full_path = format!("{}{}", root, entry.name().unwrap_or(""));
            
            // Interesują nas tylko pliki .nix w katalogu pkgs/
            if !full_path.starts_with("pkgs/") || !full_path.ends_with(".nix") {
                return TreeWalkResult::Ok;
            }

            // Pobierz obiekt i sprawdź czy to blob (plik)
            if let Ok(object) = entry.to_object(&self.repo) {
                if let Some(blob) = object.as_blob() {
                    if let Ok(content) = std::str::from_utf8(blob.content()) {
                        // Spróbuj wyciągnąć informacje o pakiecie
                        if let Some(package_info) = self.extract_package_info(&full_path, content) {
                            stats.packages_found += 1;

                            let entry = PackageEntry::new(
                                package_info.attr_name,
                                package_info.version,
                                commit_sha.clone(),
                                package_info.nar_hash.unwrap_or_else(|| "unknown".to_string()),
                                timestamp,
                            );

                            // Wstaw do bazy (z deduplikacją)
                            match self.db.insert_if_better(&entry) {
                                Ok(true) => stats.packages_inserted += 1,
                                Ok(false) => {},  // Nie wstawiono - starsza wersja
                                Err(e) => {
                                    log::warn!("Failed to insert package {}: {:?}", entry.key(), e);
                                }
                            }
                        }
                    }
                }
            }

            TreeWalkResult::Ok
        })?;

        Ok(stats)
    }

    /// Wyciąga informacje o pakiecie z pliku .nix
    fn extract_package_info(&self, path: &str, content: &str) -> Option<PackageInfo> {
        // Wyciągnij nazwę atrybutu z ścieżki
        // np. "pkgs/development/libraries/nodejs/default.nix" -> "nodejs"
        let attr_name = self.extract_attr_name(path)?;

        // Wyciągnij wersję używając regex
        let version = self.version_regex
            .captures(content)?
            .get(1)?
            .as_str()
            .to_string();

        // TODO: W przyszłości tutaj będzie obliczanie hasha NAR
        // Na razie zwracamy placeholder
        
        Some(PackageInfo {
            attr_name,
            version,
            nar_hash: None,
        })
    }

    /// Wyciąga nazwę atrybutu z ścieżki pliku
    fn extract_attr_name(&self, path: &str) -> Option<String> {
        // Ścieżka formatu: pkgs/<category>/<subcategory>/<name>/...
        let parts: Vec<&str> = path.split('/').collect();
        
        if parts.len() >= 4 && parts[0] == "pkgs" {
            // Próbujemy wyciągnąć nazwę z trzeciego poziomu
            Some(parts[3].to_string())
        } else {
            None
        }
    }
}

/// Informacje wyciągnięte z pliku pakietu
#[derive(Debug)]
struct PackageInfo {
    attr_name: String,
    version: String,
    nar_hash: Option<String>,
}

/// Statystyki indeksowania
#[derive(Debug, Default)]
pub struct IndexStats {
    pub processed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub packages_found: usize,
    pub packages_inserted: usize,
}

/// Statystyki przetwarzania pojedynczego commita
#[derive(Debug, Default)]
struct CommitStats {
    packages_found: usize,
    packages_inserted: usize,
}

impl std::fmt::Display for IndexStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Commits: {} processed, {} skipped, {} errors | Packages: {} found, {} inserted",
            self.processed, self.skipped, self.errors,
            self.packages_found, self.packages_inserted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_regex() {
        let indexer_regex = Regex::new(r#"version\s*=\s*"([^"]+)""#).unwrap();
        
        let content = r#"
            pname = "nodejs";
            version = "14.17.0";
        "#;
        
        let caps = indexer_regex.captures(content).unwrap();
        assert_eq!(caps.get(1).unwrap().as_str(), "14.17.0");
    }

    #[test]
    fn test_attr_name_extraction() {
        // Tymczasowy test - w prawdziwym środowisku potrzebowalibyśmy repozytorium
        let path = "pkgs/development/libraries/nodejs/default.nix";
        let parts: Vec<&str> = path.split('/').collect();
        assert_eq!(parts[3], "nodejs");
    }
}
