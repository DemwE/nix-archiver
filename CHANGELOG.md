# Changelog

All notable changes to nix-archiver will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed - NixOS Installation Configuration

#### Working NixOS Configuration for GitHub Installation
- **OpenSSL Build Fix**: Added `overrideAttrs` configuration to resolve OpenSSL compilation issues
  - Set `OPENSSL_NO_VENDOR = "1"` to use system OpenSSL instead of vendored version
  - Added `openssl` to `buildInputs`
  - Added `pkg-config` and `perl` to `nativeBuildInputs`
- **Branch Correction**: Updated examples to use `master` branch instead of `main`
- **Verified Hash**: Documented working SHA256 hash for current master branch
- **Updated Documentation**:
  - `README.md` - Added working configuration example
  - `INSTALL.md` - Updated Method 3 with technical notes and hash update instructions
  - `SETUP.md` - Added tested configuration as Option A

#### Configuration Example
```nix
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
in { environment.systemPackages = [ nix-archiver ]; }
```

### Added - NixOS Module (Level 1 Integration)

#### Module Implementation
- **NixOS Module** (`modules/nix-archiver.nix`, `modules/options.nix`)
  - Declarative package version pinning via `services.nix-archiver.pinnedPackages`
  - Automatic Nixpkgs indexing with systemd service + timer
  - Configurable update intervals (daily, weekly, hourly, or custom)
  - Automatic overlay generation for pinned packages
  - User and state directory management
  - CLI wrapper with automatic database path configuration

#### Module Options
- `services.nix-archiver.enable` - Enable the module
- `services.nix-archiver.package` - Package override option
- `services.nix-archiver.stateDirectory` - Custom state directory path
- `services.nix-archiver.logLevel` - Logging verbosity (error, warn, info, debug, trace)
- `services.nix-archiver.indexer.*` - Indexer configuration
  - `enable` - Enable automatic indexing
  - `repository` - Nixpkgs repository path
  - `database` - Database path
  - `updateInterval` - Update frequency
  - `maxCommits` - Commit limit for indexing
  - `batchSize` - Parallel processing batch size
  - `threads` - Thread count (null = auto-detect)
- `services.nix-archiver.pinnedPackages` - Attribute set of package versions to pin
- `services.nix-archiver.generateOverlay` - Auto-generate and apply overlay

#### Systemd Integration
- `nix-archiver-indexer.service` - Oneshot service for indexing
  - Automatic git repository clone if missing
  - Repository update via git fetch/reset
  - Indexer execution with configured options
  - Runs as unprivileged `nix-archiver` user
- `nix-archiver-indexer.timer` - Periodic timer
  - Configurable interval (default: daily)
  - Random delay (1 hour) to avoid thundering herd
  - Persistent timer (survives reboots)

#### Documentation
- **Module Documentation** (`modules/README.md`)
  - Complete option reference with examples
  - Usage patterns and best practices
  - Systemd integration details
  - Troubleshooting guide
  - Performance tuning recommendations
  - Security considerations
- **Installation Guide** (`INSTALL.md`)
  - 7 installation methods (Nix profile, nix-env, NixOS, flakes, overlays, cargo, development)
  - Platform-specific instructions (Ubuntu, Fedora, Arch, macOS)
  - Troubleshooting common installation issues
  - Update and uninstall procedures
  - Verification steps
- **Testing Guide** (`TESTING.md`)
  - NixOS VM testing instructions
  - Manual testing checklist
  - Automated test framework examples
  - Common issues and solutions
- **Example Configurations** (`examples/nixos/`)
  - `minimal.nix` - Simplest possible setup
  - `configuration.nix` - Recommended for most users
  - `advanced.nix` - Full-featured with all options
  - `indexer-only.nix` - Database-only setup
  - `README.md` - Examples overview and usage tips

#### Project Documentation Updates
- Updated main `README.md` with NixOS Module section
- Reorganized README with dedicated sections and links
- Updated `ROADMAP.md` with Level 1 completion status
- Phase 11 tasks marked as in-progress
- Added comprehensive documentation cross-references

### Technical Details

#### Files Created
```
modules/
  ├── nix-archiver.nix     # 133 lines - Main module implementation
  ├── options.nix          # 96 lines - Module options schema
  └── README.md            # 450+ lines - Complete documentation

examples/nixos/
  ├── minimal.nix          # Minimal configuration
  ├── configuration.nix    # Recommended configuration  
  ├── advanced.nix         # Advanced configuration
  ├── indexer-only.nix     # Indexer-only configuration
  └── README.md            # Examples documentation
```

#### Features Implemented
- **Declarative Configuration**: Pin package versions in `/etc/nixos/configuration.nix`
- **Automatic Indexing**: Systemd service clones/updates Nixpkgs and indexes commits
- **Overlay Generation**: Creates Nix overlay from `pinnedPackages` using `generate` command
- **State Management**: Automatic user/group creation and directory setup via tmpfiles
- **Performance Tuning**: Configurable batch sizes, thread counts, and commit limits
- **Logging**: Structured logging with configurable verbosity
- **Resilience**: Service runs as unprivileged user, handles missing repositories

#### Integration Capabilities
- Works with existing NixOS configurations
- Compatible with nixpkgs overlays
- Integrates with systemd journal for logging
- Can be used alongside manual CLI operations
- Supports custom state directories for advanced setups

### Testing
- Syntax validation for all `.nix` files ✅
- Module structure verification ✅
- Example configurations validated ✅

### Known Limitations
- No NixOS VM integration tests yet (planned)
- Not yet submitted to nixpkgs upstream (planned)
- Requires manual import in configuration.nix (will be packaged later)

---

## [0.1.0] - Initial Implementation (Phases 1-8b)

### Added
- Core data models (`archiver-core`)
- Sled database with deduplication (`archiver-db`)
- Git repository indexing (`archiver-index`)
- CLI with commands: `index`, `search`, `generate`, `stats` (`archiver-cli`)
- NAR hash computation with SHA256 SRI format
- Parallel processing with configurable threads
- Batch processing for memory efficiency
- Colored table output with alignment
- Version filtering with semver support
- Commit SHA display in frozen files
- Resumable indexing with progress tracking
- Structured logging with env_logger
- Modular CLI architecture (8 files)

### Technical Stack
- Rust 2021
- Dependencies: sled, git2, rayon, clap, tabled, colored, semver, regex, chrono
- 4-crate workspace architecture
- Nix flake for reproducible builds

[Unreleased]: https://github.com/DemwE/nix-archiver/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/DemwE/nix-archiver/releases/tag/v0.1.0
