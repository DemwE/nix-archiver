# Nix-Archiver 🚀

Deklaratywne przypisywanie pakietów do konkretnych wersji historycznych w ekosystemie NixOS.

## 📋 Opis projektu

Nix-Archiver to modularne narzędzie w Rust, które rozwiązuje problem "brakujących wersji" w NixOS poprzez:
- Indeksowanie historii Git repozytorium Nixpkgs
- Automatyczne wykrywanie i katalogowanie wersji pakietów
- Generowanie odtwarzalnych wyrażeń Nix dla konkretnych wersji
- Deduplikację danych (tylko najnowszy commit dla każdej wersji)

## 🏗️ Architektura

Projekt składa się z czterech crate'ów:

### `archiver-core`
Wspólne modele danych i logika generowania kodu Nix.
- Struktura `PackageEntry` - reprezentacja pakietu w bazie
- Generowanie bloków `fetchTarball` i wyrażeń Nix
- Konwersja hashów NAR do formatu SRI

### `archiver-db`
Warstwa persistencji z deduplikacją.
- Embedded KV store (Sled)
- Logika `insert_if_better` - tylko najnowsze commity
- Śledzenie przetworzonych commitów

### `archiver-index`
Silnik ETL do przetwarzania repozytorium Nixpkgs.
- Walker Git używający `git2-rs`
- Parsowanie plików `.nix` w poszukiwaniu wersji
- (TODO) Obliczanie hashów NAR bezpośrednio z obiektów Git

### `archiver-cli`
Interfejs CLI.
- Komendy: `index`, `search`, `generate`, `stats`
- Fuzzy matching do sugestii wersji
- (TODO) Generowanie pliku `frozen.nix`

## 🚀 Quickstart

### Środowisko Nix (zalecane)

```bash
# Wejdź do środowiska deweloperskiego
nix develop --extra-experimental-features 'nix-command flakes'

# Zbuduj projekt
cargo build --release

# Uruchom testy
cargo test --workspace

# Wyświetl pomoc
cargo run --bin nix-archiver -- --help
```

### Tradycyjne środowisko Rust

```bash
# Wymagane zależności systemowe (Ubuntu/Debian)
sudo apt install pkg-config libssl-dev

# Build
cargo build --release

# Testy
cargo test
```

## 📖 Użycie

### Indeksowanie repozytorium Nixpkgs

```bash
# Sklonuj Nixpkgs (jeśli jeszcze nie masz)
git clone https://github.com/NixOS/nixpkgs.git

# Zindeksuj ostatnie 1000 commitów
nix-archiver index \
  --repo ./nixpkgs \
  --from HEAD \
  --max-commits 1000
```

### Wyszukiwanie wersji pakietu

```bash
# Pokaż wszystkie wersje nodejs
nix-archiver search nodejs

# Znajdź konkretną wersję
nix-archiver search nodejs 14.17.0
```

### Wyświetlanie statystyk

```bash
nix-archiver stats
```

## 🛠️ Development

### Struktura workspace

```
nix-archiver/
├── Cargo.toml              # Root workspace
├── flake.nix               # Nix flake definition
├── shell.nix               # Nix shell (legacy)
├── spec.md                 # Szczegółowa specyfikacja techniczna
└── crates/
    ├── archiver-core/      # Modele danych
    ├── archiver-db/        # Warstwa bazy danych
    ├── archiver-index/     # Silnik indeksowania
    └── archiver-cli/       # CLI interface
```

### Uruchamianie testów

```bash
# Wszystkie testy
cargo test --workspace

# Testy dla konkretnego crate
cargo test -p archiver-core

# Testy z logami
RUST_LOG=debug cargo test
```

### Formatowanie i linting

```bash
# Format
cargo fmt --all

# Clippy
cargo clippy --workspace -- -D warnings
```

## 📝 Roadmap

- [x] **Phase 1**: Modele w `archiver-core` i walidacja hashowania NAR
- [x] **Phase 2**: Integracja `archiver-db` ze Sledem i logiką deduplikacji
- [x] **Phase 3**: Walker Git w `archiver-index`
- [x] **Phase 4**: CLI z obsługą błędów
- [ ] **Phase 5**: Generowanie plików `frozen.nix`
- [ ] **Phase 6**: Hashowanie NAR bez zależności binarnej Nix
- [ ] **Phase 7**: Cloud API (Axum), PostgreSQL backend
- [ ] **Phase 8**: Frontend w Next.js

## 🤝 Wkład

Pull requesty są mile widziane! Przy większych zmianach, proszę najpierw otworzyć issue.

## 📄 Licencja

MIT

## 🔗 Linki

- [Specyfikacja techniczna](spec.md)
- [NixOS](https://nixos.org)
- [Nixpkgs](https://github.com/NixOS/nixpkgs)
