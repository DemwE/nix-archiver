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
- **Database Schema**:
    - **Tree: `packages`** - Stores package entries (key: `"attr_name:version"`, value: serialized `PackageEntry`)
    - **Tree: `processed_commits`** - Tracks successfully indexed commits (key: commit SHA, value: timestamp)
- **Write Logic**: `insert_if_better(entry)` – overwrites existing version only when new entry has newer timestamp.
- **Commit Tracking**:
    - Commits are marked as processed **only after** successful batch completion and database flush
    - This ensures resumability: interrupted indexing can safely restart without data loss
    - Skips already-processed commits on restart for efficiency
- **Features**: 
    - Package entry storage and retrieval
    - Processed commits tracking with atomic batch commits
    - Version listing by package name
    - Flush control for batch operations
- **Status**: ✅ Implemented with tests

### `archiver-index`
- **Purpose**: ETL (Extract, Transform, Load) engine.
- **Dependencies**: `git2-rs`, `regex`, `sha2`, `rayon` (parallel processing), `chrono`.
- **Logic**:
    1. Iterating through Git history of Nixpkgs.
    2. Parsing `.nix` files for version strings (regex-based with validation).
    3. Generating NAR hash from Git blob objects using custom NAR serialization.
    4. Tracking progress in `processed_commits` table.
    5. Parallel batch processing (configurable batch size, default 100) using Rayon thread pool.
- **Performance Features**:
    - **Parallel Processing**: Uses Rayon to process commits across multiple CPU cores
    - **Batch Processing**: Commits are processed in configurable batches (default: 100 commits)
    - **Reduced I/O**: Database flush every 5 batches (500 commits by default) to minimize overhead
    - **Thread Control**: Configurable thread count via CLI (`-j/--threads`)
    - **Batch Size Control**: Configurable batch size via CLI (`-b/--batch-size`)
    - **Progress Tracking**: Real-time speed calculation, ETA estimation, and detailed statistics
    - **NAR Hashing**: Custom implementation following Nix NAR specification, outputs SHA256 in SRI format
- **Version Validation**:
    - Filters out Nix code patterns (interpolations, function calls) from version strings
    - Ensures versions contain at least one digit
    - Validates character set (alphanumeric, dots, hyphens, underscores, plus signs)
- **Resumability**:
    - Commits marked as processed only after successful batch completion and flush
    - Safe restart after interruption - no duplicate processing or data loss
    - Progress logging with commit counts and package statistics
- **Status**: ✅ Fully implemented with parallel processing and NAR hashing

### `archiver-cli`
- **Purpose**: CLI interface and user configuration validation.
- **Commands**:
    - `index` - Index Nixpkgs repository with parallel processing
        - Options: `--repo`, `--from`, `--max-commits`, `--threads`, `--batch-size`
        - Enhanced progress logging with speed, ETA, and statistics
        - Configurable thread count and batch size for performance tuning
    - `search` - Search for package versions with enhanced display
        - **Table Display**: Version, commit SHA, date, and NAR hash
        - **Semantic Sorting**: Versions sorted by semantic versioning (newest first)
        - **Color-Coded Output**: 
            - Newest version: Green with star (★) marker
            - Other versions: White with dimmed styling
            - Headers: Bright cyan
            - Statistics: Yellow labels
            - Relative times: Dimmed (e.g., "2 days ago")
        - **Statistics Summary**: Total versions, newest/oldest timestamps
        - **Fuzzy Matching**: Suggests similar package names on typo
        - **Filtering Options**:
            - `--limit/-n NUM`: Show only NUM newest versions (default: 50)
            - `--all/-a`: Show all versions (override limit)
            - `--major/-m NUM`: Filter by major version (e.g., `-m 14` for 14.x.x)
            - `--pattern/-p REGEX`: Filter by version regex pattern
            - `--since/-s DATE`: Show versions added since date (YYYY-MM-DD)
        - **Example Usage**:
            ```bash
            # Show latest 50 versions (default)
            nix-archiver search nodejs
            
            # Show all versions
            nix-archiver search nodejs --all
            
            # Show only major version 14
            nix-archiver search nodejs --major 14
            
            # Show versions matching pattern
            nix-archiver search nodejs --pattern '^14\.'
            
            # Show versions added since specific date
            nix-archiver search nodejs --since 2023-01-01
            
            # Combine filters: major 14, last 20 versions
            nix-archiver search nodejs -m 14 -n 20
            ```
    - `stats` - Display database statistics with colored output
    - `generate` - Generate `frozen.nix` file from package specification
        - **Input Format**: Nix attribute set with package versions
            ```nix
            {
              nodejs = "20.11.0";
              python = "3.11.7";
              go = "1.21.5";
            }
            ```
        - **Output**: frozen.nix with fetchTarball expressions for each package
        - **Validation**: Checks if versions exist in database, suggests alternatives
        - **Example Usage**:
            ```bash
            # Create package specification
            cat > packages.nix << EOF
            {
              nodejs = "20.11.0";
              go = "1.21.5";
            }
            EOF
            
            # Generate frozen.nix
            nix-archiver generate -i packages.nix -o frozen.nix
            
            # Use in your project
            nix-shell frozen.nix
            ```
- **Features**:
    - **Semantic Version Sorting**: Proper parsing and comparison of semver versions
    - **Advanced Filtering**: Multiple filter types (major version, regex, date)
    - **Color-Coded Output**: Terminal colors using `colored` crate
    - **Relative Time Formatting**: Human-readable timestamps ("2 days ago", "3 months ago")
    - **Performance Display**: Shows newest version prominently with star marker
    - **Fuzzy Matching**: Suggests similar package names using `strsim`
    - **Error Handling**: Helpful error messages with suggestions
    - **Comprehensive Logging**: INFO/DEBUG/WARN/ERROR levels
    - **Progress Tracking**: Real-time metrics for indexing
    - **Table Formatting**: Clean table output using `tabled` library
- **Logging Features**:
    - Startup information (threads, repository, commit details)
    - Batch progress with speed and ETA
    - Final statistics summary (time, commits, packages, speed, errors)
    - Debug mode with flush timing and thread utilization
- **Dependencies**:
    - `clap` (CLI argument parsing)
    - `tabled` (table formatting)
    - `colored` (terminal colors)
    - `semver` (semantic version parsing)
    - `regex` (pattern filtering)
    - `chrono` (date/time handling)
    - `strsim` (fuzzy matching)
- **Status**: ✅ CLI fully implemented with enhanced display, filtering, sorting, and generate command

---

## 5. Resumable Indexing & Performance Optimization

### Commit Tracking Mechanism
The indexer implements a robust commit tracking system that ensures safe resumability:

1. **Two-Phase Commit**: 
   - Phase 1: Process commits in parallel, extract packages
   - Phase 2: Flush to database, then mark commits as processed
   
2. **Atomicity Guarantee**:
   - Commits are marked as processed **only after** successful database flush
   - If process is interrupted (Ctrl+C, crash), uncommitted batches are reprocessed on restart
   - No risk of partial data or lost packages

3. **Batch Processing**:
   - Default batch size: 100 commits (configurable via `-b/--batch-size`)
   - Flush interval: every 5 batches (500 commits by default)
   - Commits within a batch are marked atomically after flush
   - Can be adjusted based on available memory and I/O characteristics

4. **Skip Logic**:
   - On startup, `is_commit_processed()` checks each commit
   - Already-processed commits are skipped (counted in "skipped" metric)
   - Only new/incomplete commits are processed

### Performance Features

1. **Parallel Processing**:
   - Uses Rayon thread pool for multi-core utilization
   - Default: number of CPU cores
   - Configurable via `-j/--threads` option
   - Recommendation: 1.5-2x CPU cores for I/O-bound operations

2. **Batch Size Tuning**:
   - Default: 100 commits per batch
   - Configurable via `-b/--batch-size` option
   - Smaller batches: more frequent progress updates, better for limited memory
   - Larger batches: better CPU utilization, higher throughput
   - Recommendation: 100-500 for typical use, 500-1000 for high-performance systems

3. **I/O Optimization**:
   - Reduced database flush frequency (every 5 batches)
   - Minimizes disk I/O overhead
   - Significantly improves throughput

4. **Progress Tracking**:
   - Real-time speed calculation (commits/s, packages/s)
   - ETA estimation based on current speed
   - Detailed statistics at completion

5. **Logging Levels**:
   - **INFO**: Progress updates, final statistics
   - **DEBUG**: Flush timing, thread utilization, detailed metrics
   - **WARN/ERROR**: Issues and failures

### Performance Metrics Example
```
⚡ Batch #2 | Commits: 1,000/5,000 (20%) | Packages: 2,567 inserted (4,890 found) | Speed: 95.2 commits/s | ETA: 42s

📊 Final Statistics:
   • Total time:        52.5s
   • Commits processed: 5,000 (4,850 new, 150 skipped)
   • Packages found:    24,567
   • Packages inserted: 12,890 (11,677 duplicates filtered)
   • Average speed:     95.2 commits/s, 245.5 packages/s
   • Errors:            0
```

---

## 6. Logical Constraints & Business Rules
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
- [x] **Phase 5a**: Version validation and filtering (Nix code detection).
- [x] **Phase 5b**: Complete NAR hashing implementation (Nix-independent).
- [x] **Phase 6a**: Enhanced search output with table formatting.
- [x] **Phase 6b**: `generate` command for `frozen.nix` file creation.
- [x] **Phase 6c**: Advanced CLI enhancements (semantic sorting, filtering, colors, statistics).
- [x] **Phase 7**: Parallel processing with Rayon for multi-core utilization.
- [x] **Phase 8a**: Comprehensive logging system with progress tracking and statistics.
- [x] **Phase 8b**: Resumable indexing with atomic commit tracking.
- [ ] **Phase 9 (Future)**: Cloud API (Axum), PostgreSQL migration, Next.js frontend.

---

## 7. Testing Strategy

**Current Coverage**:
- `archiver-core`: 2 unit tests (PackageEntry key, Nix generation)
- `archiver-db`: 2 unit tests (insert/get, deduplication)
- `archiver-index`: 5 unit tests (regex, path extraction, version validation, NAR hash computation, NAR hash padding)
- `archiver-cli`: 1 unit test (CLI parsing)
- **Total**: 10 tests, all passing ✅

**Future Tests**:
- Integration tests with real Nixpkgs repo (small subset)
- Fuzzy matching accuracy tests
- NAR hash validation against Nix binary outputs
- Performance benchmarks (commits/second, packages/second)
- Resumability tests (interrupt and restart scenarios)
- Thread safety tests for parallel processing

---

**Instructions for LLM**: When working on this project, prioritize Rust type safety, avoid shell command invocations (use libraries), and ensure Nix-related logic (NAR, Base32) is 100% compliant with Nix internal standards. All code, comments, and documentation must be written in English.