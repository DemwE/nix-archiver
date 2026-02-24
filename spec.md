# Nix-Archiver: AI-Ready Technical Specification

## 1. Project Overview
**Nix-Archiver** is a modular Rust tool designed to solve the "missing versions" problem in the NixOS ecosystem. It enables declarative pinning of packages to specific historical versions (e.g., `nodejs 14.17.0`) by indexing the Git history of the Nixpkgs repository and generating reproducible Nix expressions.

**Language Policy**: This project is maintained entirely in English - all code, comments, documentation, commit messages, and issues should be written in English to ensure accessibility for the global community.

## 2. Technical Philosophy
- **Modular Workspace**: Architecture based on Rust Workspace (separate crates for Core, DB, Indexer, CLI).
- **Local-First, Cloud-Ready**: Current implementation uses local Sled database; architecture prepared for PostgreSQL/Redis migration in the cloud.
- **Nix-Independent Hashing**: Computing NAR (Nix Archive) hashes directly from Git objects in Rust (no dependency on binary Nix during indexing).
- **Deduplication Strategy**: For each unique version (e.g., `pkg 1.2.3`), the database stores only the **latest** (chronologically last) commit, optimizing database size and ensuring the most up-to-date package definition fixes.
- **Nix-First Development**: Reproducible development environment via Nix flake with vendored dependencies (OpenSSL, libgit2) to ensure zero system dependency issues.

---

## 3. Development Environment & Infrastructure

### Nix Flake Configuration
The project provides a complete Nix flake (`flake.nix`) for reproducible development:

- **Rust Toolchain**: Latest stable Rust from `rust-overlay` with rustfmt, clippy, and rust-src
- **Build Dependencies**: pkg-config, OpenSSL, Git, make, gcc
- **Vendored Libraries**: git2 uses `vendored-openssl` and `vendored-libgit2` features to eliminate system dependency issues
- **direnv Integration**: `.envrc` file for automatic environment activation

**Usage**:
```bash
# Enter development shell
nix develop --extra-experimental-features 'nix-command flakes'

# Or with direnv (if installed)
direnv allow
```

### Cargo Workspace Structure
```toml
[workspace]
members = [
    "crates/archiver-core",
    "crates/archiver-db",
    "crates/archiver-index",
    "crates/archiver-cli",
]
```

**Shared Dependencies** (workspace-level):
- Serialization: `serde`, `serde_json`
- Async: `tokio`, `async-trait`
- Error Handling: `anyhow`, `thiserror`
- Database: `sled`
- Git Operations: `git2` (with vendored features)
- Cryptography: `sha2`, `data-encoding` (Nix-style Base32)
- CLI: `clap`, `strsim` (fuzzy matching)
- Testing: `tempfile`, `regex`

---

## 4. Workspace Architecture (Crates)

### `archiver-core`
- **Purpose**: Shared data models and Nix code generation logic.
- **Data Model (`PackageEntry`)**:
    - `attr_name` (String), `version` (String), `commit_sha` (String), `nar_hash` (SRI format), `timestamp` (u64), `is_primary` (bool).
- **Functionality**: Converting database entries to `fetchTarball` blocks in Nix format.
- **Status**: ✅ Implemented with tests

### `archiver-db`
- **Purpose**: Persistence layer with deduplication.
- **Implementation**: `sled` (Embedded KV store).
- **Write Logic**: `insert_if_better(entry)` – overwrites existing version only when new entry has newer timestamp.
- **Features**: 
    - Package entry storage and retrieval
    - Processed commits tracking
    - Version listing by package name
- **Status**: ✅ Implemented with tests

### `archiver-index`
- **Purpose**: ETL (Extract, Transform, Load) engine.
- **Dependencies**: `git2-rs`, `regex`, `sha2`.
- **Logic**:
    1. Iterating through Git history of Nixpkgs.
    2. Parsing `.nix` files for version strings (regex-based).
    3. Generating NAR hash from Git tree objects (TODO: full implementation).
    4. Tracking progress in `processed_commits` table.
- **Status**: ✅ Core implementation done, NAR hashing pending

### `archiver-cli`
- **Purpose**: CLI interface and user configuration validation.
- **Commands**:
    - `index` - Index Nixpkgs repository
    - `search` - Search for package versions
    - `stats` - Display database statistics
    - `generate` - Generate `frozen.nix` file (TODO)
- **Features**:
    - Fuzzy matching for version suggestions
    - Error handling with helpful suggestions
- **Status**: ✅ Basic CLI implemented, `generate` command pending

---

## 5. Logical Constraints & Business Rules
1. **Deterministic Matching**: If `nodejs 1.3` never existed in Nixpkgs, the tool **does not guess**. It must return an error and suggest nearest available versions (e.g., 1.2 or 1.4).
2. **Channel Agnosticism**: Primary focus on `nixos-unstable` channel for maximum version density, with optional support for stable channels.
3. **Storage Efficiency**: Removing processed Git objects after indexing. Local database should occupy megabytes (MB), not gigabytes (GB).
4. **Cloud Future**: Next.js frontend and Axum (Rust) API as planned technology stack for server version.

---

## 6. Implementation Roadmap
- [x] **Phase 1**: Models in `archiver-core` and NAR hashing validation setup.
- [x] **Phase 2**: Integration of `archiver-db` with Sled and deduplication logic.
- [x] **Phase 3**: Git walker in `archiver-index` using `git2-rs`.
- [x] **Phase 4**: CLI with error handling and basic commands.
- [ ] **Phase 5**: Complete NAR hashing implementation (Nix-independent).
- [ ] **Phase 6**: `generate` command for `frozen.nix` file creation.
- [ ] **Phase 7**: Enhanced version detection (pname + version parsing).
- [ ] **Phase 8**: Performance optimizations (parallel processing, caching).
- [ ] **Phase 9 (Future)**: Cloud API (Axum), PostgreSQL migration, Next.js frontend.

---

## 7. Testing Strategy

**Current Coverage**:
- `archiver-core`: 2 unit tests (PackageEntry key, Nix generation)
- `archiver-db`: 2 unit tests (insert/get, deduplication)
- `archiver-index`: 2 unit tests (regex, path extraction)
- `archiver-cli`: 1 unit test (CLI parsing)

**Future Tests**:
- Integration tests with real Nixpkgs repo (small subset)
- Fuzzy matching accuracy tests
- NAR hash validation against Nix binary outputs
- Performance benchmarks (commits/second)

---

**Instructions for LLM**: When working on this project, prioritize Rust type safety, avoid shell command invocations (use libraries), and ensure Nix-related logic (NAR, Base32) is 100% compliant with Nix internal standards. All code, comments, and documentation must be written in English.