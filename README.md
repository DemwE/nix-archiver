# nix-archiver

Narzędzie do indeksowania historii Nixpkgs i przypinania pakietów do konkretnych wersji historycznych w NixOS.

## Czym jest nix-archiver?

`nix-archiver` rozwiązuje problem "brakujących wersji" w NixOS — gdy potrzebujesz konkretnej starszej wersji pakietu (np. `nodejs 18.12.0`), ale nie ma jej już w aktualnym nixpkgs. Narzędzie indeksuje historię Git repozytorium nixpkgs i pozwala znaleźć dokładny commit, w którym dana wersja była dostępna, a następnie wygenerować gotowe wyrażenie Nix.

## Instalacja

### Budowanie ze źródeł

```bash
git clone https://github.com/DemwE/nix-archiver.git
cd nix-archiver
nix-build   # lub: cargo build --release
```

### Integracja z NixOS

Gotowy moduł NixOS bez flaków — wystarczy jeden `imports` w `configuration.nix`.
Szczegóły: [docs/nixos-module.md](docs/nixos-module.md).

## Szybki start

```bash
# 1. Zaindeksuj historię nixpkgs (np. od 2024-01-01)
nix-archiver index --repo /ścieżka/do/nixpkgs --to-date 2024-01-01

# 2. Szukaj pakietów (prefix lub substring)
nix-archiver search nodejs
nix-archiver search python314
nix-archiver search biomejs          # znajdzie vscode-extensions.biomejs.biome

# 3. Konkretna wersja
nix-archiver search nodejs 20.11.0

# 4. Wygeneruj frozen.nix z packages.nix
nix-archiver generate --input packages.nix --output frozen.nix
```

## Dokumentacja

| Dokument | Opis |
|---|---|
| [docs/usage.md](docs/usage.md) | Wszystkie komendy CLI ze szczegółami |
| [docs/architecture.md](docs/architecture.md) | Architektura kodu (crate'y) |
| [docs/nixos-module.md](docs/nixos-module.md) | Integracja z NixOS bez flaków |
| [spec.md](spec.md) | Specyfikacja techniczna implementacji |

## Licencja

MIT — szczegóły w pliku [LICENSE](LICENSE).
