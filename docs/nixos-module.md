# Integracja z NixOS — bez flaków

Ten dokument opisuje jak dodać `nix-archiver` do zwykłej konfiguracji NixOS
(`/etc/nixos/configuration.nix`) bez użycia flaków.

---

## Minimalna konfiguracja

### 1. Sklonuj nixpkgs (jednorazowo)

Zanim uruchomisz moduł, potrzebujesz lokalnej kopii nixpkgs do indeksowania:

```bash
git clone --bare https://github.com/NixOS/nixpkgs.git /var/lib/nix-archiver/nixpkgs
```

> Opcja `--bare` zmniejsza rozmiar (~3 GB vs ~6 GB) — brak katalogu roboczego,  
> tylko obiekty Git. nix-archiver działa z bare repo.

### 2. Zbuduj i ustal hash modułu

```bash
# Pobierz tarball i sprawdź hash
nix-prefetch-url --unpack https://github.com/DemwE/nix-archiver/archive/main.tar.gz
# wypisze hash np.: 1abc2def...
```

### 3. Dodaj import do `configuration.nix`

```nix
{ config, pkgs, ... }:

{
  imports = [
    (builtins.fetchTarball {
      url    = "https://github.com/DemwE/nix-archiver/archive/main.tar.gz";
      sha256 = "1abc2def...";  # hash z kroku 2
    } + "/module.nix")
  ];

  services.nix-archiver = {
    enable     = true;
    repository = "/var/lib/nix-archiver/nixpkgs";  # ścieżka do bare clone

    # Automatyczne indeksowanie raz w tygodniu
    indexer = {
      enable   = true;
      schedule = "weekly";
      fromDate = "2023-01-01";  # nie indeksuj całej historii — tylko od tej daty
    };
  };
}
```

### 4. Przebuduj system

```bash
sudo nixos-rebuild switch
```

Po przebudowie `nix-archiver` jest dostępny w `$PATH` dla wszystkich użytkowników.  
Indeksowanie uruchomi się automatycznie zgodnie z harmonogramem.

### 5. Uruchom indeksowanie ręcznie (pierwsza instalacja)

```bash
sudo systemctl start nix-archiver-index
journalctl -u nix-archiver-index -f   # obserwuj postęp
```

---

## Następne kroki: użycie frozen.nix

Po zaindeksowaniu możesz generować przypięte wersje pakietów.

### Utwórz `packages.nix`

```nix
# /etc/nixos/packages.nix
{
  nodejs = "20.11.0";
  go     = "1.21.5";
}
```

### Wygeneruj `frozen.nix`

```bash
nix-archiver --database /var/lib/nix-archiver/db \
  generate --input /etc/nixos/packages.nix --output /etc/nixos/frozen.nix
```

### Użyj `frozen.nix` w `configuration.nix`

```nix
{ config, pkgs, ... }:

let
  pinned = import /etc/nixos/frozen.nix;
in {
  # ...wcześniejsza konfiguracja...

  environment.systemPackages = [
    pinned.nodejs   # <- konkretna wersja z historii nixpkgs
    pinned.go
  ];
}
```

---

## Wszystkie opcje modułu

```nix
services.nix-archiver = {

  # Włącz moduł i zainstaluj binary
  enable = true;

  # Przesłoń pakiet (np. z lokalnych źródeł)
  # package = pkgs.callPackage /ścieżka/do/nix-archiver/default.nix {};

  # Ścieżka do bazy danych sled
  database = "/var/lib/nix-archiver/db";          # domyślnie

  # Ścieżka do bare clone nixpkgs (musisz go stworzyć ręcznie)
  repository = "/var/lib/nix-archiver/nixpkgs";   # domyślnie

  indexer = {
    # Uruchom automatyczne indeksowanie (systemd timer)
    enable = true;

    # Kiedy indeksować (systemd calendar — patrz `man systemd.time`)
    schedule = "weekly";      # domyślnie; inne: "daily", "Sun 03:00"

    # Nie indeksuj historii przed tą datą (format YYYY-MM-DD)
    # null = przetwarzaj aż do napotkania już przetworzonego commitu
    fromDate = "2023-01-01";

    # Liczba wątków (null = auto)
    threads = null;
  };
};
```

---

## Lokalne budowanie (bez fetchTarball)

Jeśli masz sklonowane repo lokalnie, wygodniej jest wskazywać ścieżkę bezpośrednio:

```nix
# configuration.nix
{
  imports = [ /ścieżka/do/nix-archiver/module.nix ];

  services.nix-archiver = {
    enable  = true;
    package = pkgs.callPackage /ścieżka/do/nix-archiver/default.nix {};
  };
}
```

---

## Sprawdzanie statusu

```bash
# Status timera
systemctl status nix-archiver-index.timer

# Ostatnie uruchomienie
journalctl -u nix-archiver-index --no-pager | tail -30

# Statystyki bazy
nix-archiver --database /var/lib/nix-archiver/db stats
```
