# Troubleshooting - nix-archiver

Ten dokument zawiera rozwiązania najczęstszych problemów z instalacją i użyciem nix-archiver.

## Problemy z instalacją

### ❌ Błąd: "error building OpenSSL" lub "vendored OpenSSL compilation failed"

**Problem**: Podczas budowania pakietu w NixOS występuje błąd kompilacji związany z vendored OpenSSL.

**Rozwiązanie**: Użyj `overrideAttrs` aby wymusić użycie systemowego OpenSSL:

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
    OPENSSL_NO_VENDOR = "1";  # ← Kluczowa linia
  });
in
{
  environment.systemPackages = [ nix-archiver ];
}
```

**Wyjaśnienie**:
- `OPENSSL_NO_VENDOR = "1"` wyłącza kompilację OpenSSL ze źródeł
- Dodanie `pkgs.openssl` do `buildInputs` zapewnia dostęp do systemowej biblioteki
- `pkg-config` i `perl` są wymagane do linkowania z systemowym OpenSSL

---

### ❌ Błąd: "hash mismatch" podczas instalacji z GitHub

**Problem**: 
```
error: hash mismatch in fixed-output derivation
  expected: sha256-AAAAAAA...
```

**Rozwiązanie**: Zaktualizuj hash SHA256 po zmianach w repozytorium:

```bash
# Pobierz aktualny hash
nix-prefetch-url --unpack https://github.com/DemwE/nix-archiver/archive/master.tar.gz

# Skopiuj wynik (np. sha256-CWwxZjkqI50VVKuP0umG4W6O6WRldg3jxbFCRElDGKo=)
# i zamień w configuration.nix
```

**Alternatywnie** - użyj konkretnego commita zamiast brancha:

```nix
{
  rev = "abc123...";  # konkretny commit SHA
  sha256 = "sha256-...";  # hash dla tego commita
}
```

---

### ❌ Błąd: "error: cannot find attribute 'nix-archiver'"

**Problem**: NixOS nie może znaleźć pakietu podczas `nixos-rebuild switch`.

**Rozwiązanie**: Sprawdź czy:

1. **Definicja `let` jest kompletna**:
   ```nix
   let
     nix-archiver = ...;  # ← Definicja
   in
   {
     environment.systemPackages = [ nix-archiver ];  # ← Użycie
   }
   ```

2. **Import modułu jest poprawny** (jeśli używasz modułu):
   ```nix
   imports = [ /ścieżka/do/nix-archiver/modules/nix-archiver.nix ];
   ```

3. **Ścieżka do lokalnego repo jest prawidłowa**:
   ```nix
   imports = [ /home/user/nix-archiver/modules/nix-archiver.nix ];  # ← Pełna ścieżka
   ```

---

### ❌ Błąd: "experimental Nix feature 'flakes' is disabled"

**Problem**: Próba użycia `nix run` lub `nix profile install` bez włączonych flakes.

**Rozwiązanie A** - Włącz flakes globalnie w `/etc/nix/nix.conf`:
```
experimental-features = nix-command flakes
```

Następnie restart demona Nix:
```bash
sudo systemctl restart nix-daemon
```

**Rozwiązanie B** - Użyj flag przy każdej komendzie:
```bash
nix --extra-experimental-features 'nix-command flakes' run github:DemwE/nix-archiver
```

---

### ❌ Błąd: "error: getting status of '/nix/store/...': No such file or directory"

**Problem**: Brakujące pliki w nix store po aktualizacji.

**Rozwiązanie**: Wyczyść cache i przebuduj:
```bash
# Usuń stare buildy
sudo nix-collect-garbage -d

# Przebuduj system
sudo nixos-rebuild switch
```

---

## Problemy z użyciem

### ❌ "command not found: nix-archiver"

**Problem**: Komenda `nix-archiver` nie jest dostępna w PATH.

**Rozwiązanie A** - Dla `nix profile install`:
```bash
# Sprawdź czy jest zainstalowane
nix profile list | grep nix-archiver

# Jeśli nie ma, zainstaluj:
nix profile install github:DemwE/nix-archiver
```

**Rozwiązanie B** - Dla `cargo install`:
```bash
# Sprawdź PATH
echo $PATH | grep .cargo/bin

# Jeśli nie ma, dodaj do ~/.bashrc lub ~/.zshrc:
export PATH="$HOME/.cargo/bin:$PATH"
source ~/.bashrc  # lub ~/.zshrc
```

**Rozwiązanie C** - W NixOS sprawdź czy pakiet jest w systemPackages:
```nix
environment.systemPackages = [ nix-archiver ];  # ← To musi być
```

---

### ❌ Błąd: "Failed to open database" lub "Database is locked"

**Problem**: Nie można otworzyć bazy danych Sled.

**Rozwiązanie**:

1. **Sprawdź uprawnienia**:
   ```bash
   ls -la /var/lib/nix-archiver/db
   # Powinno być: drwxr-xr-x nix-archiver nix-archiver
   ```

2. **Jeśli używasz modułu NixOS**, baza powinna być zarządzana automatycznie.

3. **Dla ręcznej bazy**, upewnij się że użytkownik ma uprawnienia:
   ```bash
   sudo chown -R $USER:$USER ~/nix-archiver.db
   ```

4. **Jeśli baza jest zablokowana**, sprawdź czy nie ma innej instancji:
   ```bash
   ps aux | grep nix-archiver
   # Zabij proces jeśli jest nieaktywny
   ```

---

### ❌ Bardzo wolne indeksowanie repozytorium

**Problem**: Indeksowanie trwa bardzo długo (>30 minut).

**Rozwiązanie**: Optymalizacja konfiguracji:

```bash
# Ogranicz liczbę commitów
nix-archiver index --max-commits 1000 ...

# Zwiększ batch size
nix-archiver index --batch-size 200 ...

# Użyj więcej wątków (jeśli masz wiele rdzeni)
nix-archiver index --threads 8 ...
```

**W module NixOS**:
```nix
services.nix-archiver.indexer = {
  maxCommits = 1000;     # Mniej commitów
  batchSize = 200;       # Większe partie
  threads = 8;           # Więcej wątków
};
```

Zobacz [PERFORMANCE.md](PERFORMANCE.md) dla szczegółów.

---

### ❌ Pakiet nie znaleziony mimo że istnieje w Nixpkgs

**Problem**: `nix-archiver search nodejs` nie zwraca wyników mimo że nodejs jest w Nixpkgs.

**Możliwe przyczyny**:

1. **Baza nie jest zindeksowana**: Uruchom najpierw indeksowanie:
   ```bash
   nix-archiver index --repo /path/to/nixpkgs --max-commits 1000
   ```

2. **Nieprawidłowa nazwa pakietu**: Spróbuj:
   ```bash
   nix-archiver search node  # Bez "js"
   ```

3. **Za mało commitów**: Pakiet może być w starszych commitach:
   ```bash
   nix-archiver index --max-commits 5000 ...
   ```

---

## Problemy z NixOS Module

### ❌ Service nie startuje automatycznie

**Problem**: `nix-archiver-indexer.service` nie działa po `nixos-rebuild switch`.

**Rozwiązanie**:

1. **Sprawdź status serwisu**:
   ```bash
   systemctl status nix-archiver-indexer.service
   systemctl status nix-archiver-indexer.timer
   ```

2. **Zobacz logi**:
   ```bash
   journalctl -u nix-archiver-indexer.service -n 50
   ```

3. **Manualnie uruchom serwis**:
   ```bash
   sudo systemctl start nix-archiver-indexer.service
   ```

4. **Sprawdź konfigurację**:
   ```nix
   services.nix-archiver = {
     enable = true;          # ← Musi być true
     indexer.enable = true;  # ← I to też
   };
   ```

---

### ❌ Overlay nie jest aplikowany

**Problem**: Pinowane pakiety nie są dostępne mimo `generateOverlay = true`.

**Rozwiązanie**:

1. **Sprawdź czy overlay został wygenerowany**:
   ```bash
   ls -la /var/lib/nix-archiver/overlay.nix
   cat /var/lib/nix-archiver/packages.nix
   ```

2. **Sprawdź konfigurację**:
   ```nix
   services.nix-archiver = {
     pinnedPackages = {
       nodejs = "20.11.0";  # ← Nie może być puste!
     };
     generateOverlay = true;  # ← Musi być true
   };
   ```

3. **Przebuduj system**:
   ```bash
   sudo nixos-rebuild switch
   ```

4. **Sprawdź czy pakiet ma poprawną wersję**:
   ```bash
   nix-instantiate --eval -E 'with import <nixpkgs> {}; nodejs.version'
   ```

---

## Diagnostyka

### Sprawdź wersję i konfigurację

```bash
# Wersja nix-archiver
nix-archiver --version

# Statystyki bazy
nix-archiver stats

# Sprawdź czy moduł jest włączony (NixOS)
systemctl status nix-archiver-indexer.service

# Zobacz logi modułu
journalctl -u nix-archiver-indexer.service -f
```

### Debugowanie z większą ilością logów

```bash
# CLI z verbose logging
RUST_LOG=debug nix-archiver search nodejs

# Module z debug level (configuration.nix)
services.nix-archiver.logLevel = "debug";
```

Zobacz [LOGGING.md](LOGGING.md) dla szczegółów.

---

## Dalsze wsparcie

Jeśli problem nie został rozwiązany:

1. **Sprawdź dokumentację**:
   - [README.md](README.md) - Główna dokumentacja
   - [INSTALL.md](INSTALL.md) - Instalacja
   - [modules/README.md](modules/README.md) - Moduł NixOS
   - [TESTING.md](TESTING.md) - Testowanie

2. **Zgłoś issue na GitHub**: https://github.com/DemwE/nix-archiver/issues
   - Załącz wersję: `nix-archiver --version`
   - Załącz logi: `journalctl -u nix-archiver-indexer.service -n 100`
   - Opisz kroki reprodukcji problemu

3. **Sprawdź istniejące issues**: Twój problem może już być rozwiązany
