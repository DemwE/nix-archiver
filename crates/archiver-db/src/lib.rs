//! Archiver DB - Warstwa persistency z deduplikacją
//!
//! Ten crate zarządza lokalną bazą danych Sled, implementując logikę deduplikacji:
//! dla każdej unikalnej wersji pakietu przechowywany jest tylko najnowszy commit.

use archiver_core::PackageEntry;
use anyhow::{Context, Result};
use sled::Db;
use std::path::Path;

/// Główna struktura zarządzająca bazą danych
pub struct ArchiverDb {
    /// Drzewo przechowujące wpisy pakietów (klucz: "attr_name:version")
    packages: sled::Tree,
    
    /// Drzewo śledzące przetworzone commity
    processed_commits: sled::Tree,
    
    /// Instancja bazy Sled
    db: Db,
}

impl ArchiverDb {
    /// Otwiera lub tworzy nową bazę danych w podanej lokalizacji
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path.as_ref())
            .with_context(|| format!("Failed to open database at {:?}", path.as_ref()))?;
        
        let packages = db
            .open_tree("packages")
            .context("Failed to open packages tree")?;
        
        let processed_commits = db
            .open_tree("processed_commits")
            .context("Failed to open processed_commits tree")?;
        
        Ok(Self {
            packages,
            processed_commits,
            db,
        })
    }

    /// Wstawia wpis pakietu tylko jeśli jest nowszy niż istniejący
    ///
    /// Logika deduplikacji: jeśli wpis dla danej wersji już istnieje,
    /// zastępowany jest tylko wtedy, gdy nowy wpis ma nowszy timestamp.
    pub fn insert_if_better(&self, entry: &PackageEntry) -> Result<bool> {
        let key = entry.key();
        let new_value = serde_json::to_vec(entry)
            .context("Failed to serialize PackageEntry")?;

        let was_inserted = self.packages.update_and_fetch(key.as_bytes(), |old_value| {
            match old_value {
                None => {
                    // Brak istniejącej wartości - wstawiamy
                    Some(new_value.clone())
                }
                Some(old_bytes) => {
                    // Sprawdzamy timestamp istniejącej wartości
                    match serde_json::from_slice::<PackageEntry>(old_bytes) {
                        Ok(old_entry) => {
                            if entry.timestamp > old_entry.timestamp {
                                // Nowy wpis jest nowszy - nadpisujemy
                                log::info!(
                                    "Updating {} from commit {} -> {} (newer timestamp)",
                                    key,
                                    &old_entry.commit_sha[..8],
                                    &entry.commit_sha[..8]
                                );
                                Some(new_value.clone())
                            } else {
                                // Stary wpis jest nowszy - zostawiamy bez zmian
                                Some(old_bytes.to_vec())
                            }
                        }
                        Err(_) => {
                            // Błąd deserializacji - nadpisujemy z ostrzeżeniem
                            log::warn!("Corrupted entry for {}, overwriting", key);
                            Some(new_value.clone())
                        }
                    }
                }
            }
        })
        .context("Failed to update package entry")?;

        // Sprawdzamy czy faktycznie wstawiliśmy nowy wpis
        if let Some(final_value) = was_inserted {
            let final_entry: PackageEntry = serde_json::from_slice(&final_value)
                .context("Failed to deserialize final entry")?;
            Ok(final_entry.commit_sha == entry.commit_sha)
        } else {
            Ok(false)
        }
    }

    /// Pobiera wpis pakietu według nazwy atrybutu i wersji
    pub fn get(&self, attr_name: &str, version: &str) -> Result<Option<PackageEntry>> {
        let key = format!("{}:{}", attr_name, version);
        
        match self.packages.get(key.as_bytes())? {
            Some(bytes) => {
                let entry = serde_json::from_slice(&bytes)
                    .context("Failed to deserialize PackageEntry")?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Pobiera wszystkie wersje danego pakietu
    pub fn get_all_versions(&self, attr_name: &str) -> Result<Vec<PackageEntry>> {
        let prefix = format!("{}:", attr_name);
        let mut results = Vec::new();

        for item in self.packages.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.context("Failed to read from database")?;
            let entry: PackageEntry = serde_json::from_slice(&value)
                .context("Failed to deserialize PackageEntry")?;
            results.push(entry);
        }

        // Sortujemy po timestampie (najnowsze najpierw)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(results)
    }

    /// Zaznacza commit jako przetworzony
    pub fn mark_commit_processed(&self, commit_sha: &str, timestamp: u64) -> Result<()> {
        self.processed_commits
            .insert(commit_sha.as_bytes(), &timestamp.to_le_bytes())
            .context("Failed to mark commit as processed")?;
        Ok(())
    }

    /// Sprawdza czy commit został już przetworzony
    pub fn is_commit_processed(&self, commit_sha: &str) -> Result<bool> {
        Ok(self.processed_commits.contains_key(commit_sha.as_bytes())?)
    }

    /// Zwraca liczbę przechowywanych pakietów
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Zwraca liczbę przetworzonych commitów
    pub fn processed_commit_count(&self) -> usize {
        self.processed_commits.len()
    }

    /// Flush'uje wszystkie oczekujące operacje na dysk
    pub fn flush(&self) -> Result<()> {
        self.db.flush().context("Failed to flush database")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_insert_and_get() -> Result<()> {
        let tmp = TempDir::new()?;
        let db = ArchiverDb::open(tmp.path())?;

        let entry = PackageEntry::new(
            "nodejs".to_string(),
            "14.17.0".to_string(),
            "abc123".to_string(),
            "sha256-test".to_string(),
            1234567890,
        );

        db.insert_if_better(&entry)?;
        let retrieved = db.get("nodejs", "14.17.0")?;

        assert_eq!(retrieved, Some(entry));
        Ok(())
    }

    #[test]
    fn test_deduplication_newer_wins() -> Result<()> {
        let tmp = TempDir::new()?;
        let db = ArchiverDb::open(tmp.path())?;

        let old_entry = PackageEntry::new(
            "nodejs".to_string(),
            "14.17.0".to_string(),
            "old123".to_string(),
            "sha256-old".to_string(),
            1000,
        );

        let new_entry = PackageEntry::new(
            "nodejs".to_string(),
            "14.17.0".to_string(),
            "new456".to_string(),
            "sha256-new".to_string(),
            2000,
        );

        db.insert_if_better(&old_entry)?;
        db.insert_if_better(&new_entry)?;

        let retrieved = db.get("nodejs", "14.17.0")?;
        assert_eq!(retrieved.unwrap().commit_sha, "new456");
        Ok(())
    }
}
