# Architektura — nix-archiver

## Schemat crate'ów

```
archiver-cli          ← punkt wejścia, CLI (clap)
    ├── archiver-db   ← baza danych (sled), serializacja (bincode)
    ├── archiver-index← indeksowanie nixpkgs (git2, rnix)
    └── archiver-core ← wspólne modele danych, generowanie kodu Nix
```

---

## `archiver-core`

Wspólne typy danych używane przez wszystkie crate'y.

### `PackageEntry`

```rust
pub struct PackageEntry {
    pub attr_name:  String,   // np. "nodejs", "vscode-extensions.biomejs.biome"
    pub version:    String,   // np. "20.11.0"
    pub commit_sha: String,   // 40-znakowy SHA1 commitu nixpkgs
    pub nar_hash:   String,   // SHA256 w formacie SRI ("sha256-...")
    pub timestamp:  u64,      // Unix timestamp commitu
    pub is_primary: bool,     // true jeśli to "kanoniczny" commit dla tej wersji
}
```

Klucz w bazie: `attr_name:version` (np. `nodejs:20.11.0`).

### Generowane wyrażenia Nix

`to_nix_import()` → wyrażenie `import (fetchTarball {...}) {}`  
`to_nix_fetchtarball()` → sam blok `fetchTarball`

---

## `archiver-db`

Warstwa persistencji oparta na [sled](https://github.com/spacejam/sled) (embedded KV store).

### Format przechowywania

Dane serializowane binarnie przez **bincode**. Wewnętrzna struktura `StoredEntry` używa:
- `commit_sha: [u8; 20]` — zamiast 40-znakowego hex-stringa (-20 bajtów)
- `nar_hash: [u8; 32]` — zamiast 59-znakowego SRI stringa (-27 bajtów)

Oszczędność: ~50% mniej miejsca per wpis vs. poprzedni format JSON.

### Logika deduplikacji

`insert_if_better(entry)` — wstawia nowy wpis **tylko jeśli** jego timestamp jest nowszy niż istniejący dla tego samego `attr_name:version`. Zapewnia, że w bazie jest zawsze najnowszy commit dla danej wersji.

### Wyszukiwanie

| Metoda | Mechanizm | Użycie |
|---|---|---|
| `search_packages(q)` | `scan_prefix(q)` — szybki, O(log n) | `python` → `python311`, `python314`... |
| `search_packages_contains(q)` | pełny skan, case-insensitive | `biomejs` → `vscode-extensions.biomejs.biome` |

CLI używa dwufazowego wyszukiwania: najpierw prefix, w razie braku wyników — substring.

---

## `archiver-index`

Silnik ETL przetwarzający historię Git repozytorium nixpkgs.

### Przepływ przetwarzania

```
git log --name-only      →  lista zmienionych .nix plików na commit
    ↓
parser AST (rnix)        →  ekstrakcja attr_name + version
    ↓
NAR hash (sha256 blob)   →  hash z zawartości pliku
    ↓
archiver-db              →  insert_if_better()
```

### Parser AST

`ast_parser.rs` używa biblioteki **rnix** do parsowania plików `.nix`.  
Trzy strategie ekstrakcji wersji:

| Strategia | Co szuka | Przykład |
|---|---|---|
| 1. `pname` + `version` | atrybuty w definicji pakietu | większość pakietów |
| 2. `mktplcRef` | rozszerzenia VSCode | `vscode-extensions.biomejs.biome` |
| 3. Ścieżka pliku | fallback z nazwy pliku | `pkgs/by-name/no/nodejs/package.nix` |

### Dlaczego system `git`, nie libgit2?

Eksperyment pokazał, że `git2::diff_tree_to_tree` ładuje zawartość blobów do obliczenia diffu → **8.6 commitów/s**.  
`git log --name-only` (subprocess) robi porównanie OID na poziomie drzewa → **400–500 commitów/s**.

Dlatego `commit.rs` używa `process::Command("git")`.

---

## `archiver-cli`

Interfejs CLI zbudowany na **clap** v4.

### Komendy

| Komenda | Opis |
|---|---|
| `index` | Indeksuje nixpkgs, zapisuje do bazy |
| `search` | Szuka pakietów (prefix + substring fallback) |
| `generate` | Czyta `packages.nix`, pisze `frozen.nix` |
| `stats` | Statystyki bazy |

### Wyświetlanie wyników

- Tabele przez **tabled** z kolorowaniem **colored**
- Multi-wynik: podział na "Package sets" (jak NixOS search sidebar)
- Wersje sortowane semver-aware przez `sort_versions_semver()`
