# ✅ Konfiguracja GitHub - Ukończona

Repozytorium jest już skonfigurowane do użycia z GitHub!

**URL GitHub**: https://github.com/DemwE/nix-archiver

Wszystkie pliki zostały zaktualizowane z właściwym URL repozytorium.

## 🚀 Gotowe do użycia

Możesz teraz zainstalować nix-archiver bezpośrednio z GitHub:

```bash
# Zainstaluj system-wide
nix profile install github:DemwE/nix-archiver

# Lub użyj bezpośrednio
nix run github:DemwE/nix-archiver -- --help

# Lub sklonuj i zainstaluj lokalnie
git clone https://github.com/DemwE/nix-archiver.git
cd nix-archiver
nix profile install .
```

## 📦 Zaktualizowane pliki

Następujące pliki zawierają właściwy URL GitHub:

- ✅ `default.nix` - homepage w metadata
- ✅ `README.md` - wszystkie przykłady i instrukcje
- ✅ `INSTALL.md` - wszystkie metody instalacji
- ✅ `modules/README.md` - dokumentacja modułu NixOS

## 🎯 Instalacja w NixOS

Możesz użyć pełnego modułu NixOS:

```nix
# /etc/nixos/configuration.nix
{ config, pkgs, ... }:

{
  # Import modułu z lokalnego repo
  imports = [ /path/to/nix-archiver/modules/nix-archiver.nix ];
  
  services.nix-archiver = {
    enable = true;
    
    # Automatyczne indeksowanie
    indexer = {
      enable = true;
      updateInterval = "daily";
    };
    
    # Pinowane pakiety
    pinnedPackages = {
      nodejs = "20.11.0";
      python3 = "3.11.7";
    };
  };
}
```

Rebuild systemu:
```bash
sudo nixos-rebuild switch
```

## 📋 Dla kontrybutorów (fork)

Jeśli robisz fork tego repozytorium, możesz zamienić URL na swoje:

```bash
# Ustaw swoją nazwę użytkownika GitHub
export GITHUB_USER="twoja-nazwa"

# Automatyczna zamiana we wszystkich plikach
sed -i "s/DemwE/$GITHUB_USER/g" default.nix README.md INSTALL.md modules/README.md

# Na macOS użyj:
# sed -i '' "s/DemwE/$GITHUB_USER/g" default.nix README.md INSTALL.md modules/README.md

# Sprawdź zmiany
git diff
```

## 🎯 Następne kroki

1. **Zainstaluj**: Wybierz metodę z [INSTALL.md](INSTALL.md)
2. **Użyj CLI**: Zobacz [README.md](README.md) dla podstawowego użycia
3. **Konfiguruj NixOS**: Sprawdź [modules/README.md](modules/README.md) dla integracji systemowej

## 🔗 Zobacz także

- [README.md](README.md) - Główna dokumentacja
- [INSTALL.md](INSTALL.md) - 7 metod instalacji
- [modules/README.md](modules/README.md) - Moduł NixOS
- [examples/nixos/](examples/nixos/) - Przykładowe konfiguracje
