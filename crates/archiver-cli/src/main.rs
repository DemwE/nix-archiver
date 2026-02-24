//! Nix-Archiver CLI - Interfejs użytkownika dla systemu archiwizacji Nixpkgs
//!
//! Umożliwia:
//! - Indeksowanie repozytorium Nixpkgs
//! - Wyszukiwanie konkretnych wersji pakietów
//! - Generowanie plików frozen.nix z pinned wersjami

use anyhow::{Context, Result};
use archiver_db::ArchiverDb;
use archiver_index::Indexer;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nix-archiver")]
#[command(about = "Deklaratywne przypisywanie pakietów do wersji historycznych w Nixpkgs", long_about = None)]
#[command(version)]
struct Cli {
    /// Ścieżka do bazy danych
    #[arg(short, long, default_value = "./nix-archiver.db")]
    database: PathBuf,

    /// Poziom logowania (error, warn, info, debug, trace)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Indeksuje repozytorium Nixpkgs
    Index {
        /// Ścieżka do lokalnego repozytorium Nixpkgs
        #[arg(short, long)]
        repo: PathBuf,

        /// Commit od którego zacząć indeksowanie (domyślnie HEAD)
        #[arg(short, long, default_value = "HEAD")]
        from: String,

        /// Maksymalna liczba commitów do przetworzenia
        #[arg(short, long)]
        max_commits: Option<usize>,
    },

    /// Wyszukuje konkretną wersję pakietu
    Search {
        /// Nazwa atrybutu pakietu (np. "nodejs")
        attr_name: String,

        /// Wersja do wyszukania (opcjonalna - wyświetli wszystkie wersje)
        version: Option<String>,
    },

    /// Generuje plik frozen.nix na podstawie specyfikacji
    Generate {
        /// Plik wejściowy ze specyfikacją wersji
        #[arg(short, long)]
        input: PathBuf,

        /// Plik wyjściowy frozen.nix
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Wyświetla statystyki bazy danych
    Stats,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Konfiguracja loggera
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(&cli.log_level)
    ).init();

    // Otwórz bazę danych
    let db = ArchiverDb::open(&cli.database)
        .with_context(|| format!("Failed to open database at {:?}", cli.database))?;

    match cli.command {
        Commands::Index { repo, from, max_commits } => {
            cmd_index(repo, from, max_commits, db)?;
        }
        Commands::Search { attr_name, version } => {
            cmd_search(attr_name, version, db)?;
        }
        Commands::Generate { input, output } => {
            cmd_generate(input, output, db)?;
        }
        Commands::Stats => {
            cmd_stats(db)?;
        }
    }

    Ok(())
}

/// Indeksuje repozytorium Nixpkgs
fn cmd_index(repo_path: PathBuf, from_commit: String, max_commits: Option<usize>, db: ArchiverDb) -> Result<()> {
    log::info!("Starting indexing of repository at {:?}", repo_path);
    log::info!("From commit: {}", from_commit);
    if let Some(max) = max_commits {
        log::info!("Max commits: {}", max);
    }

    let indexer = Indexer::new(&repo_path, db)
        .context("Failed to create indexer")?;

    // Jeśli from_commit to "HEAD", rozwiąż do konkretnego SHA
    let commit_sha = if from_commit == "HEAD" {
        resolve_head(&repo_path)?
    } else {
        from_commit
    };

    let stats = indexer.index_from_commit(&commit_sha, max_commits)
        .context("Failed to index repository")?;

    log::info!("Indexing completed!");
    log::info!("{}", stats);

    Ok(())
}

/// Rozwiązuje HEAD do konkretnego commit SHA
fn resolve_head(repo_path: &PathBuf) -> Result<String> {
    use git2::Repository;
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    Ok(commit.id().to_string())
}

/// Wyszukuje pakiet w bazie danych
fn cmd_search(attr_name: String, version: Option<String>, db: ArchiverDb) -> Result<()> {
    if let Some(ver) = version {
        // Wyszukaj konkretną wersję
        match db.get(&attr_name, &ver)? {
            Some(entry) => {
                println!("Found: {}", entry);
                println!("\nNix expression:");
                println!("{}", entry.to_nix_import());
            }
            None => {
                eprintln!("Package {}:{} not found in database", attr_name, ver);
                
                // Zasugeruj dostępne wersje
                let all_versions = db.get_all_versions(&attr_name)?;
                if !all_versions.is_empty() {
                    eprintln!("\nAvailable versions for {}:", attr_name);
                    for entry in all_versions.iter().take(10) {
                        eprintln!("  - {} (commit {})", entry.version, &entry.commit_sha[..8]);
                    }
                    if all_versions.len() > 10 {
                        eprintln!("  ... and {} more", all_versions.len() - 10);
                    }
                } else {
                    eprintln!("\nNo versions found for package '{}'", attr_name);
                }
                
                std::process::exit(1);
            }
        }
    } else {
        // Wyświetl wszystkie wersje
        let all_versions = db.get_all_versions(&attr_name)?;
        
        if all_versions.is_empty() {
            println!("No versions found for package '{}'", attr_name);
        } else {
            println!("Found {} versions of {}:", all_versions.len(), attr_name);
            for entry in all_versions {
                println!("  - {} @ {} ({})", 
                    entry.version, 
                    &entry.commit_sha[..8],
                    format_timestamp(entry.timestamp)
                );
            }
        }
    }

    Ok(())
}

/// Generuje plik frozen.nix
fn cmd_generate(_input: PathBuf, _output: PathBuf, _db: ArchiverDb) -> Result<()> {
    // TODO: Implementacja parsowania pliku wejściowego i generowania frozen.nix
    eprintln!("Generate command not yet implemented");
    eprintln!("This will be implemented in Phase 4");
    std::process::exit(1);
}

/// Wyświetla statystyki bazy danych
fn cmd_stats(db: ArchiverDb) -> Result<()> {
    println!("Database Statistics:");
    println!("  Packages: {}", db.package_count());
    println!("  Processed commits: {}", db.processed_commit_count());
    Ok(())
}

/// Formatuje timestamp Unix na czytelną datę
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    let datetime = UNIX_EPOCH + Duration::from_secs(timestamp);
    // Proste formatowanie - w produkcji użyć biblioteki chrono
    format!("{:?}", datetime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        // Test czy CLI się poprawnie parsuje
        let cli = Cli::try_parse_from(&[
            "nix-archiver",
            "--database", "./test.db",
            "stats"
        ]);
        assert!(cli.is_ok());
    }
}
