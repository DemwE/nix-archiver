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

### Opcja 1: NixOS Module (zalecane dla użytkowników NixOS)

Dodaj moduł do swojej konfiguracji `/etc/nixos/configuration.nix`:

```nix
{ config, pkgs, ... }:

{
  imports = [
    /path/to/nix-archiver/modules/nix-archiver.nix
  ];

  services.nix-archiver = {
    enable = true;
    
    # Automatyczne indeksowanie
    indexer = {
      enable = true;
      updateInterval = "daily";
    };
    
    # Pinuj konkretne wersje pakietów
    pinnedPackages = {
      nodejs = "20.11.0";
      python3 = "3.11.7";
      postgresql = "15.5";
    };
  };

  # Użyj przypięte wersje
  environment.systemPackages = with pkgs; [
    nodejs      # wersja 20.11.0
    python3     # wersja 3.11.7
    postgresql  # wersja 15.5
  ];
}
```

Pełna dokumentacja modułu: [modules/README.md](modules/README.md)

### Opcja 2: Tradycyjne CLI

#### Indeksowanie repozytorium Nixpkgs

```bash
# Sklonuj Nixpkgs (jeśli jeszcze nie masz)
git clone https://github.com/NixOS/nixpkgs.git

# Zindeksuj ostatnie 1000 commitów
nix-archiver index \
  --repo ./nixpkgs \
  --from HEAD \
  --max-commits 1000
```

#### Wyszukiwanie wersji pakietu

```bash
# Pokaż wszystkie wersje nodejs
nix-archiver search nodejs

# Znajdź konkretną wersję
nix-archiver search nodejs 14.17.0
```

#### Generowanie packages.nix

```bash
# Wygeneruj plik z pinowanymi pakietami
nix-archiver generate -o packages.nix
```

#### Wyświetlanie statystyk

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

**Ukończone Fazy** (1-8b): ✅
- Models, database, Git indexer, CLI, NAR hashing, table formatting, parallel processing, logging, resumability

**Phase 10-11** (W realizacji): 🔄
- ✅ **Level 1**: NixOS Module - deklaratywne pinowanie pakietów, automatyczne indeksowanie przez systemd
- [ ] **Phase 10**: Lock files, apply/sync commands, format converters
- [ ] **Phase 11**: Pełna integracja NixOS module (testy, dokumentacja)
- [ ] **Phase 12**: Flake library & outputs
- [ ] **Phase 13**: Home Manager integration
- [ ] **Phase 14+**: Cloud API, web dashboard, advanced features

📋 **Szczegółowa roadmapa**: Zobacz [ROADMAP.md](ROADMAP.md) dla pełnego planu integracji systemowej, NixOS modules, flake support, i długoterminowej wizji projektu.

## 🤝 Wkład

Pull requesty są mile widziane! Przy większych zmianach, proszę najpierw otworzyć issue.

## 📄 Licencja

MIT

## 🔗 Linki

- [Specyfikacja techniczna](spec.md)
- [Roadmap i plany rozwoju](ROADMAP.md)
- [NixOS](https://nixos.org)
- [Nixpkgs](https://github.com/NixOS/nixpkgs)
