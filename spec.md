# Nix-Archiver: AI-Ready Technical Specification

## 1. Project Overview
**Nix-Archiver** to modularne narzędzie w języku Rust, zaprojektowane do rozwiązywania problemu "brakujących wersji" w ekosystemie NixOS. Narzędzie umożliwia deklaratywne przypisywanie pakietów do konkretnych wersji historycznych (np. `nodejs 14.17.0`) poprzez indeksowanie historii Git repozytorium Nixpkgs i generowanie odtwarzalnych wyrażeń Nix.

## 2. Technical Philosophy
- **Modular Workspace**: Architektura oparta na Rust Workspace (osobne crate'y dla Core, DB, Indexer, CLI).
- **Local-First, Cloud-Ready**: Obecna implementacja wykorzystuje lokalną bazę Sled; architektura przygotowana pod migrację na PostgreSQL/Redis w chmurze.
- **Nix-Independent Hashing**: Obliczanie haszy NAR (Nix Archive) bezpośrednio z obiektów Git w Rust (bez zależności od binarnego Nixa podczas indeksowania).
- **Deduplication Strategy**: Dla każdej unikalnej wersji (np. `pkg 1.2.3`), baza przechowuje tylko **najnowszy** (ostatni chronologicznie) commit, co optymalizuje rozmiar bazy i zapewnia najaktualniejsze poprawki definicji pakietu.

---

## 3. Workspace Architecture (Crates)

### `archiver-core`
- **Cel**: Wspólne modele danych i logika generowania kodu Nix.
- **Model danych (`PackageEntry`)**:
    - `attr_name` (String), `version` (String), `commit_sha` (String), `nar_hash` (SRI format), `timestamp` (u64), `is_primary` (bool).
- **Funkcjonalność**: Konwersja wpisów z bazy na bloki `fetchTarball` w formacie Nix.

### `archiver-db`
- **Cel**: Warstwa persistency.
- **Implementacja**: `sled` (Embedded KV store).
- **Logika zapisu**: `insert_if_better(entry)` – nadpisuje istniejącą wersję tylko wtedy, gdy nowy wpis ma nowszy znacznik czasu (timestamp).

### `archiver-index`
- **Cel**: Silnik ETL (Extract, Transform, Load).
- **Zależności**: `git2-rs`, `nix-nar`, `sha2`.
- **Logika**:
    1. Iteracja po historii Git Nixpkgs.
    2. Parsowanie plików `.nix` w poszukiwaniu stringów wersji.
    3. Generowanie hashu NAR z obiektów drzewa Git.
    4. Śledzenie postępu w tabeli `processed_commits`.

### `archiver-cli`
- **Cel**: UI i walidacja konfiguracji użytkownika.
- **Workflow**:
    1. Parsowanie pliku deklaracji użytkownika (`versions.nix`).
    2. Zapytanie do DB; jeśli wersji brak, zwrócenie błędu z sugestią najbliższych dostępnych (Fuzzy matching).
    3. Generowanie pliku `frozen.nix` do importu w systemie.

---

## 4. Logical Constraints & Business Rules
1. **Deterministic Matching**: Jeśli `nodejs 1.3` nigdy nie istniało w Nixpkgs, narzędzie **nie zgaduje**. Musi zwrócić błąd i zasugerować najbliższe wersje (np. 1.2 lub 1.4).
2. **Channel Agnosticism**: Główny nacisk na kanał `nixos-unstable` dla maksymalnej gęstości wersji, z opcjonalnym wsparciem kanałów stabilnych.
3. **Storage Efficiency**: Usuwanie przetworzonych obiektów Git po indeksowaniu. Lokalna baza danych powinna zajmować megabajty (MB), a nie gigabajty (GB).
4. **Cloud Future**: Frontend w Next.js oraz API w Axum (Rust) jako planowany stos technologiczny dla wersji serwerowej.

---

## 5. Implementation Roadmap (TODO)
- [ ] **Phase 1**: Modele w `archiver-core` i walidacja hashowania NAR.
- [ ] **Phase 2**: Integracja `archiver-db` ze Sledem i logiką deduplikacji.
- [ ] **Phase 3**: Walker Git w `archiver-index` przy użyciu `git2-rs`.
- [ ] **Phase 4**: CLI z obsługą błędów i generatorem plików Nix.
- [ ] **Phase 5 (Future)**: Cloud API (Axum), migracja na PostgreSQL, Frontend w Next.js.

---

**Instrukcje dla LLM**: Pomagając w tym projekcie, priorytetyzuj bezpieczeństwo typów w Rust, unikaj wywołań powłoki systemowej (używaj bibliotek) i upewnij się, że logika związana z Nix (NAR, Base32) jest w 100% zgodna z wewnętrznym standardem Nix.