# Quick NixOS Installation - nix-archiver

## 🚀 Najszybsza instalacja (kopiuj-wklej)

### Metoda 1: Prosty pakiet z GitHub

Dodaj do `/etc/nixos/configuration.nix`:

```nix
{ config, pkgs, ... }:

let
  nix-archiver = (pkgs.callPackage (pkgs.fetchFromGitHub {
    owner = "DemwE";
    repo = "nix-archiver";
    rev = "master";
    sha256 = "sha256-CWwxZjkqI50VVKuP0umG4W6O6WRldg3jxbFCRElDGKo=";
  }) {}).overrideAttrs (oldAttrs: {
    buildInputs = (oldAttrs.buildInputs or []) ++ [ pkgs.openssl ];
    nativeBuildInputs = (oldAttrs.nativeBuildInputs or []) ++ [ pkgs.pkg-config pkgs.perl ];
    OPENSSL_NO_VENDOR = "1";
  });
in
{
  environment.systemPackages = [ nix-archiver ];
}
```

Następnie:
```bash
sudo nixos-rebuild switch
nix-archiver --version
```

---

### Metoda 2: Pełny moduł NixOS (zalecane)

1. **Sklonuj repo**:
```bash
git clone https://github.com/DemwE/nix-archiver.git /etc/nixos/nix-archiver
```

2. **Dodaj do `/etc/nixos/configuration.nix`**:
```nix
{ config, pkgs, ... }:

{
  imports = [ 
    ./hardware-configuration.nix
    ./nix-archiver/modules/nix-archiver.nix  # ← Dodaj tę linię
  ];

  services.nix-archiver = {
    enable = true;
    
    indexer = {
      enable = true;
      updateInterval = "daily";
      maxCommits = 5000;
    };
    
    pinnedPackages = {
      nodejs = "20.11.0";
      python3 = "3.11.7";
    };
    
    generateOverlay = true;
  };

  # Reszta twojej konfiguracji...
}
```

3. **Rebuild**:
```bash
sudo nixos-rebuild switch
```

4. **Sprawdź**:
```bash
systemctl status nix-archiver-indexer.service
nix-archiver stats
```

---

## 📋 Po instalacji

### Podstawowe użycie:

```bash
# Wyszukaj pakiet
nix-archiver search nodejs

# Filtruj po wersji
nix-archiver search --version "20.*" nodejs

# Statystyki bazy
nix-archiver stats

# Wygeneruj packages.nix
nix-archiver generate -o packages.nix
```

### Sprawdź czy pinowane pakiety działają:

```bash
# Sprawdź wersję nodejs (powinno być 20.11.0)
nix-instantiate --eval -E 'with import <nixpkgs> {}; nodejs.version'

# Użyj w nix-shell
nix-shell -p nodejs
node --version
```

---

## ⚙️ Konfiguracja modułu

Wszystkie opcje w sekcji `services.nix-archiver`:

```nix
services.nix-archiver = {
  # Podstawowe
  enable = true;                    # Włącz moduł
  package = pkgs.nix-archiver;      # Pakiet (domyślnie)
  stateDirectory = "/var/lib/nix-archiver";  # Katalog danych
  logLevel = "info";                # error, warn, info, debug, trace
  
  # Indekser
  indexer = {
    enable = true;                  # Włącz automatyczne indeksowanie
    repository = "${stateDirectory}/nixpkgs";  # Ścieżka do Nixpkgs
    database = "${stateDirectory}/db";         # Ścieżka do bazy
    updateInterval = "daily";       # daily, weekly, hourly lub "0/4:00:00"
    maxCommits = 5000;              # null = wszystkie commity
    batchSize = 100;                # Rozmiar partii (50-200)
    threads = null;                 # null = auto-detect
  };
  
  # Pinowane pakiety
  pinnedPackages = {
    # Dodaj pakiety które chcesz przypić do konkretnych wersji
    nodejs = "20.11.0";
    python3 = "3.11.7";
    go = "1.21.5";
  };
  
  # Overlay
  generateOverlay = true;           # Auto-generuj overlay dla pinowanych pakietów
};
```

---

## 🔧 Troubleshooting

### Problem: Build fails z błędem OpenSSL

**Rozwiązanie**: Już zastosowane w Metodzie 1 (`OPENSSL_NO_VENDOR = "1"`).

### Problem: Hash mismatch

**Rozwiązanie**: Pobierz nowy hash:
```bash
nix-prefetch-url --unpack https://github.com/DemwE/nix-archiver/archive/master.tar.gz
```

### Problem: Service nie startuje

**Rozwiązanie**: Sprawdź logi:
```bash
journalctl -u nix-archiver-indexer.service -n 50
sudo systemctl start nix-archiver-indexer.service
```

Więcej: [TROUBLESHOOTING.md](TROUBLESHOOTING.md)

---

## 📚 Dokumentacja

- [README.md](README.md) - Główna dokumentacja
- [INSTALL.md](INSTALL.md) - Wszystkie metody instalacji
- [modules/README.md](modules/README.md) - Pełna dokumentacja modułu
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Rozwiązywanie problemów
- [examples/nixos/](examples/nixos/) - Przykładowe konfiguracje
