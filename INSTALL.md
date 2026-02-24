# Instalacja nix-archiver

Ten dokument opisuje wszystkie metody instalacji nix-archiver - od szybkiego startu po pełną integrację systemową w NixOS.

## TL;DR - Najszybsza instalacja

```bash
# Dla użytkowników Nix z Flakes
nix profile install github:DemwE/nix-archiver

# Dla użytkowników bez Nix
git clone https://github.com/DemwE/nix-archiver.git
cd nix-archiver
cargo install --path crates/archiver-cli
```

---

## Metoda 1: Nix Profile (recommended dla single-user)

### Z GitHub (wymaga flakes)

```bash
# Zainstaluj bezpośrednio z GitHub
nix profile install github:DemwE/nix-archiver

# Użyj bez instalacji
nix run github:DemwE/nix-archiver -- search nodejs

# Weryfikuj
nix-archiver --version
```

### Z lokalnego repozytorium

```bash
# Sklonuj repo
git clone https://github.com/DemwE/nix-archiver.git
cd nix-archiver

# Zainstaluj
nix profile install .

# Lub użyj shell bez instalacji
nix shell . -c nix-archiver --help
```

---

## Metoda 2: nix-env (bez flakes)

```bash
# Sklonuj repozytorium
git clone https://github.com/DemwE/nix-archiver.git
cd nix-archiver

# Zainstaluj przez nix-env
nix-env -f default.nix -i nix-archiver

# Odinstaluj (jeśli potrzeba)
nix-env -e nix-archiver
```

---

## Metoda 3: NixOS System-wide Installation

### Opcja A: Prosty pakiet (bez modułu)

Edytuj `/etc/nixos/configuration.nix`:

```nix
{ config, pkgs, ... }:

let
  # Import pakietu z lokalnego źródła
  nix-archiver = pkgs.callPackage /path/to/nix-archiver/default.nix {};
  
  # LUB z GitHub (wymaga fetchFromGitHub)
  nix-archiver = pkgs.callPackage (pkgs.fetchFromGitHub {
    owner = "DemwE";
    repo = "nix-archiver";
    rev = "main";  # lub konkretny commit/tag
    sha256 = "0000000000000000000000000000000000000000000000000000";  # użyj nix-prefetch-url
  }) {};
in
{
  environment.systemPackages = [ nix-archiver ];
}
```

Rebuild:
```bash
sudo nixos-rebuild switch
```

### Opcja B: Pełny moduł NixOS (zalecane)

Dla pełnej integracji z automatycznym indeksowaniem i pinowaniem pakietów:

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
      maxCommits = 5000;
    };
    
    # Pinowane pakiety
    pinnedPackages = {
      nodejs = "20.11.0";
      python3 = "3.11.7";
    };
    
    generateOverlay = true;
  };
}
```

Zobacz [modules/README.md](../modules/README.md) dla pełnej dokumentacji.

---

## Metoda 4: Flake w NixOS Configuration

Dla systemów używających flakes (`/etc/nixos/flake.nix`):

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nix-archiver.url = "github:DemwE/nix-archiver";
  };

  outputs = { self, nixpkgs, nix-archiver }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        
        # Dodaj pakiet
        ({ pkgs, ... }: {
          environment.systemPackages = [
            nix-archiver.packages.x86_64-linux.default
          ];
        })
        
        # LUB użyj modułu (jeśli jest wyeksportowany w flake)
        nix-archiver.nixosModules.default
      ];
    };
  };
}
```

Rebuild:
```bash
sudo nixos-rebuild switch --flake /etc/nixos#myhostname
```

---

## Metoda 5: Overlay w configuration.nix

Dodaj jako overlay do swojego nixpkgs:

```nix
{ config, pkgs, ... }:

{
  nixpkgs.overlays = [
    (self: super: {
      nix-archiver = super.callPackage /path/to/nix-archiver/default.nix {};
    })
  ];

  environment.systemPackages = with pkgs; [
    nix-archiver
  ];
}
```

---

## Metoda 6: Cargo Install (bez Nix)

Dla użytkowników spoza ekosystemu Nix.

### Zainstaluj zależności systemowe

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev git
```

**Fedora/RHEL/CentOS:**
```bash
sudo dnf install gcc pkg-config openssl-devel git
```

**Arch Linux:**
```bash
sudo pacman -S base-devel pkg-config openssl git
```

**macOS:**
```bash
brew install pkg-config openssl git
```

### Zainstaluj nix-archiver

```bash
# Sklonuj repozytorium
git clone https://github.com/DemwE/nix-archiver.git
cd nix-archiver

# Instaluj przez cargo (do ~/.cargo/bin)
cargo install --path crates/archiver-cli

# Lub zbuduj i skopiuj ręcznie
cargo build --release
sudo cp target/release/nix-archiver /usr/local/bin/

# Weryfikuj instalację
nix-archiver --version
which nix-archiver
```

**Uwaga**: Upewnij się że `~/.cargo/bin` jest w PATH:
```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

---

## Metoda 7: Development Setup

Dla kontrybutorów i developerów.

### Z Nix (zalecane)

```bash
git clone https://github.com/DemwE/nix-archiver.git
cd nix-archiver

# Wejdź do dev shell
nix develop

# Lub z direnv (automatyczne)
direnv allow

# Build i test
cargo build
cargo test --workspace
cargo run --bin nix-archiver -- --help
```

### Bez Nix

```bash
git clone https://github.com/DemwE/nix-archiver.git
cd nix-archiver

# Zainstaluj Rust (jeśli nie masz)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Zainstaluj zależności systemowe (zobacz Metoda 6)
# ...

# Build i test
cargo build
cargo test --workspace

# Formatowanie i linting
cargo fmt
cargo clippy
```

---

## Weryfikacja instalacji

Po instalacji sprawdź czy wszystko działa:

```bash
# Sprawdź wersję
nix-archiver --version

# Wyświetl pomoc
nix-archiver --help

# Sprawdź dostępne komendy
nix-archiver help index
nix-archiver help search
nix-archiver help generate
nix-archiver help stats
```

---

## Aktualizacja

### Nix Profile

```bash
# Usuń starą wersję
nix profile remove nix-archiver

# Zainstaluj nową
nix profile install github:DemwE/nix-archiver

# Lub update (jeśli zainstalowane z GitHub)
nix profile upgrade '.*nix-archiver.*'
```

### NixOS (configuration.nix)

```bash
# Usuń cachowane buildy
nix-collect-garbage -d

# Rebuild z fresh fetch
sudo nixos-rebuild switch
```

### Cargo

```bash
cd /path/to/nix-archiver
git pull origin main
cargo install --path crates/archiver-cli --force
```

---

## Deinstalacja

### Nix Profile

```bash
nix profile remove nix-archiver
```

### nix-env

```bash
nix-env -e nix-archiver
```

### NixOS

Usuń z `configuration.nix`:
```nix
# Usuń linię:
environment.systemPackages = [ nix-archiver ];

# LUB wyłącz moduł:
services.nix-archiver.enable = false;
```

Rebuild:
```bash
sudo nixos-rebuild switch
```

### Cargo

```bash
cargo uninstall nix-archiver

# Lub ręcznie
rm ~/.cargo/bin/nix-archiver
# Lub
sudo rm /usr/local/bin/nix-archiver
```

---

## Troubleshooting

### "command not found: nix-archiver"

**Nix Profile:**
```bash
# Sprawdź czy jest zainstalowane
nix profile list | grep nix-archiver

# Sprawdź PATH
echo $PATH | grep .nix-profile
```

**Cargo:**
```bash
# Sprawdź czy jest w ~/.cargo/bin
ls -la ~/.cargo/bin/nix-archiver

# Dodaj do PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

### "error: experimental Nix feature 'flakes' is disabled"

Włącz flakes w `/etc/nix/nix.conf`:
```
experimental-features = nix-command flakes
```

Lub użyj flag:
```bash
nix --extra-experimental-features 'nix-command flakes' profile install github:DemwE/nix-archiver
```

### "error: hash mismatch" podczas instalacji w NixOS

Użyj `nix-prefetch-url` aby uzyskać poprawny hash:

```bash
nix-prefetch-url --unpack https://github.com/DemwE/nix-archiver/archive/main.tar.gz
```

### Build errors z Cargo

Upewnij się że masz wszystkie zależności:
```bash
# Ubuntu/Debian
sudo apt install build-essential pkg-config libssl-dev

# Sprawdź wersję Rust (wymaga 1.70+)
rustc --version

# Aktualizuj Rust
rustup update stable
```

---

## Kolejne kroki

Po instalacji:

1. **Zindeksuj Nixpkgs**: Zobacz [README.md](../README.md#-użycie)
2. **Skonfiguruj moduł NixOS**: Zobacz [modules/README.md](../modules/README.md)
3. **Eksploruj przykłady**: Zobacz [examples/nixos/](../examples/nixos/)

---

## Zobacz także

- [README.md](../README.md) - Główna dokumentacja
- [modules/README.md](../modules/README.md) - Dokumentacja modułu NixOS
- [ROADMAP.md](../ROADMAP.md) - Plan rozwoju
- [TESTING.md](../TESTING.md) - Instrukcje testowania
