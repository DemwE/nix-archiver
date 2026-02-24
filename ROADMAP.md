# Nix-Archiver: System Integration Roadmap

This document outlines the plan for integrating nix-archiver with the NixOS ecosystem and standard workflows.

---

## 🎯 Vision

Transform nix-archiver from a standalone CLI tool into a fully integrated part of the Nix ecosystem, enabling:
- Declarative version pinning in NixOS configurations
- Seamless flake integration
- Team-wide package version management
- Automated dependency updates with CI/CD

---

## 📊 Integration Levels

### **Level 1: NixOS Module**

**Goal**: Declarative package version management in `configuration.nix`

**Concept**:
```nix
# /etc/nixos/configuration.nix
{ config, pkgs, ... }:

{
  services.nix-archiver = {
    enable = true;
    
    # Local indexing service
    indexer = {
      enable = true;
      repository = "/var/lib/nixpkgs";
      updateInterval = "daily";
      database = "/var/lib/nix-archiver/db";
    };
    
    # Declarative version pinning
    pinnedPackages = {
      nodejs = "20.11.0";
      python = "3.11.7";
      go = "1.21.5";
    };
    
    # Optional: automatic overlays
    generateOverlay = true;
  };
}
```

**Implementation**:
- `modules/nix-archiver.nix` - NixOS module definition
- Automatic overlay generation from pinned packages
- Systemd service for indexer
- Optional caching in `/nix/var/nix-archiver`

**Files to create**:
- `modules/nix-archiver.nix`
- `modules/options.nix`
- `modules/services.nix`

---

### **Level 2: Flake Output Integration**

**Goal**: Export packages as flake outputs

**Concept**:
```nix
# User project's flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nix-archiver.url = "github:user/nix-archiver";
  };
  
  outputs = { self, nixpkgs, nix-archiver }: {
    packages.x86_64-linux = 
      nix-archiver.lib.pinnedPackages {
        inherit nixpkgs;
        packages = {
          nodejs = "20.11.0";
          python3 = "3.11.7";
        };
      };
      
    devShells.x86_64-linux.default = 
      nix-archiver.lib.mkDevShell {
        inherit nixpkgs;
        packages = [ "nodejs@20.11.0" "python@3.11.7" ];
      };
  };
}
```

**Library Functions** (`lib/default.nix`):
- `pinnedPackages` - creates attribute set of pinned versions
- `mkDevShell` - generates dev shell with specific versions
- `mkOverlay` - creates overlay for use in nixpkgs
- `fetchVersion` - fetches specific version on-demand

**Files to create**:
- `lib/default.nix`
- `lib/pinned-packages.nix`
- `lib/dev-shell.nix`
- `lib/overlay.nix`

---

### **Level 3: Home Manager Integration**

**Goal**: Per-user package pinning

**Concept**:
```nix
# ~/.config/nixpkgs/home.nix
{ config, pkgs, ... }:

{
  programs.nix-archiver = {
    enable = true;
    
    # Development environments
    devEnvironments = {
      web = {
        packages = {
          nodejs = "20.11.0";
          yarn = "1.22.19";
        };
      };
      
      python-ml = {
        packages = {
          python3 = "3.11.7";
          poetry = "1.8.0";
        };
      };
    };
    
    # Global pinned packages
    home.packages = [
      (nix-archiver.pin "git" "2.42.0")
      (nix-archiver.pin "vim" "9.0.1600")
    ];
  };
}
```

**Benefits**:
- Per-user isolation
- Easy environment switching
- No sudo/NixOS required

**Files to create**:
- `home-manager/nix-archiver.nix`
- `home-manager/environments.nix`

---

### **Level 4: Enhanced CLI with System Integration**

**New CLI Commands**:

```bash
# Apply to active configuration
nix-archiver apply packages.nix --target system
nix-archiver apply packages.nix --target home-manager
nix-archiver apply packages.nix --target flake

# Generate different formats
nix-archiver generate -i packages.nix -o overlay.nix --format overlay
nix-archiver generate -i packages.nix -o module.nix --format nixos-module
nix-archiver generate -i packages.nix -o flake.nix --format flake

# Sync with active configuration
nix-archiver sync --from /etc/nixos/configuration.nix
nix-archiver diff /etc/nixos/configuration.nix

# Lock file support (like poetry.lock, package-lock.json)
nix-archiver lock packages.nix  # generates packages.lock
nix-archiver update nodejs      # update specific package in lock
nix-archiver check              # verify lock file integrity
```

**Features**:
- Multiple output formats (overlay, module, flake)
- Lock file generation and management
- Diff and sync with existing configs
- Validation and integrity checks

**Implementation**:
- New commands in `crates/archiver-cli/src/commands/`
- Lock file parser and generator
- Format converters

---

### **Level 5: Centralized Cache Server**

**Goal**: Shared database for teams/organizations

**Architecture**:
```
┌─────────────┐         ┌─────────────────┐
│   Clients   │────────▶│  nix-archiver   │
│  (CLI/Web)  │◀────────│   API Server    │
└─────────────┘         └────────┬────────┘
                                 │
                    ┌────────────┴────────────┐
                    │                         │
              ┌─────▼─────┐          ┌───────▼──────┐
              │ PostgreSQL│          │Binary Cache  │
              │  Database │          │ (Cachix/S3)  │
              └───────────┘          └──────────────┘
```

**API Endpoints**:
```
GET  /api/packages/{name}/versions
GET  /api/packages/{name}/{version}
POST /api/search
GET  /api/generate?format=overlay&packages=nodejs:20.11.0,python:3.11.7
GET  /api/health
```

**Docker Compose Setup**:
```yaml
services:
  nix-archiver-api:
    image: nix-archiver:latest
    environment:
      DATABASE_URL: postgres://user:pass@db:5432/nix_archiver
      CACHE_URL: https://cache.nixos.org
    ports:
      - "8080:8080"
      
  nix-archiver-indexer:
    image: nix-archiver:latest
    command: ["indexer", "--continuous"]
    environment:
      NIXPKGS_REPO: /data/nixpkgs
      DATABASE_URL: postgres://user:pass@db:5432/nix_archiver
      
  postgres:
    image: postgres:16
    volumes:
      - postgres-data:/var/lib/postgresql/data
```

**Implementation**:
- Axum-based REST API
- PostgreSQL migration from Sled
- Docker images
- Kubernetes manifests

---

### **Level 6: Registry Support**

**Goal**: `nix-archiver://` URL scheme

**Concept**:
```nix
# Instead of:
nodejs = import (fetchTarball {
  url = "https://github.com/NixOS/nixpkgs/archive/abc123.tar.gz";
  sha256 = "...";
}) {};

# Use:
nodejs = builtins.getFlake "nix-archiver:nodejs/20.11.0";
```

**Registry Configuration** (`~/.config/nix-archiver/registry.toml`):
```toml
[registry]
default = "https://archive.nixos.org"
fallback = ["https://cache.nixos.org"]

[mirrors]
primary = "https://your-org.com/nix-archiver"
backup = "https://backup.nixos.org"

[cache]
ttl = "24h"
max_size = "1GB"
```

**Implementation**:
- Custom Nix fetcher plugin
- Registry resolution logic
- Fallback mechanism

---

### **Level 7: CI/CD Integration**

**GitHub Actions**:
```yaml
name: Update Dependencies

on:
  schedule:
    - cron: '0 0 * * 1'  # Weekly
  workflow_dispatch:

jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - uses: nix-archiver/setup-action@v1
        with:
          version: latest
          
      - name: Update packages
        run: |
          nix-archiver update --check-security
          nix-archiver generate -i packages.nix -o frozen.nix
          
      - name: Create PR if changed
        uses: peter-evans/create-pull-request@v5
        with:
          title: "chore: Update pinned packages"
          body: "Automated package update by nix-archiver"
          branch: "update-deps"
```

**GitLab CI**:
```yaml
update-deps:
  image: nixos/nix:latest
  script:
    - nix-archiver search nodejs --format json > versions.json
    - nix-archiver generate -i packages.nix -o frozen.nix --validate
  only:
    - schedules
```

**Pre-commit Hook**:
```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/nix-archiver/pre-commit-hooks
    rev: v1.0.0
    hooks:
      - id: nix-archiver-validate
      - id: nix-archiver-format
```

---

### **Level 8: IDE Extensions**

**VSCode Extension Features**:
- Autocomplete for package versions
- Inline preview of available versions
- Quick actions: "Pin to specific version"
- Syntax highlighting for `packages.nix`
- Integration with Nix language server

**Commands**:
- `Nix-Archiver: Search Package`
- `Nix-Archiver: Pin Current Package`
- `Nix-Archiver: Show Available Versions`
- `Nix-Archiver: Update Lock File`

**Example Snippet**:
```json
{
  "Nix Pin Package": {
    "prefix": "nixpin",
    "body": [
      "${1:package} = \"${2:version}\";"
    ],
    "description": "Pin package to specific version"
  }
}
```

---

### **Level 9: Import from Lock Files**

**Goal**: Automatic conversion from other package managers

**Supported Formats**:

```bash
# From package.json (npm)
nix-archiver import package.json --format npm -o packages.nix

# From Cargo.toml (Rust)
nix-archiver import Cargo.toml --format cargo -o packages.nix

# From requirements.txt (Python)
nix-archiver import requirements.txt --format pip -o packages.nix

# From go.mod (Go)
nix-archiver import go.mod --format go -o packages.nix

# From Gemfile.lock (Ruby)
nix-archiver import Gemfile.lock --format bundler -o packages.nix

# From composer.lock (PHP)
nix-archiver import composer.lock --format composer -o packages.nix
```

**Format Mapping Examples**:

| Source | Format | Nix Output |
|--------|--------|------------|
| `"express": "^4.18.0"` (package.json) | npm | `nodejs-express = "4.18.0";` |
| `serde = "1.0.195"` (Cargo.toml) | cargo | `rustPackages.serde = "1.0.195";` |
| `requests==2.31.0` (requirements.txt) | pip | `python311Packages.requests = "2.31.0";` |
| `github.com/gin-gonic/gin v1.9.1` (go.mod) | go | `goPackages.gin = "1.9.1";` |

**Implementation**:
- Parser for each format
- Version resolver
- Dependency tree builder
- Conflict resolution

---

## 🗺️ Implementation Phases

### **Phase 10: Core System Integration** (3-4 weeks)
**Status**: Planned

**Tasks**:
- [ ] Implement `apply` command with multiple targets
- [ ] Add `sync` and `diff` commands
- [ ] Implement lock file support (packages.lock)
- [ ] Add format converters (overlay, module, flake)
- [ ] Create `lock`, `update`, `check` commands
- [ ] Write comprehensive tests

**Deliverables**:
- Enhanced CLI with new commands
- Lock file format specification
- Documentation and examples

**Dependencies**: None (builds on Phase 6b)

---

### **Phase 11: NixOS Module** (2-3 weeks)
**Status**: Planned

**Tasks**:
- [ ] Design module options schema
- [ ] Implement basic module definition
- [ ] Create systemd service for indexer
- [ ] Add auto-overlay generation
- [ ] Write module documentation
- [ ] Create example configurations
- [ ] Integration tests with NixOS test framework

**Deliverables**:
- `modules/nix-archiver.nix`
- NixOS service configuration
- Documentation
- Test suite

**Dependencies**: Phase 10 (for format converters)

---

### **Phase 12: Flake Library** (1-2 weeks)
**Status**: Planned

**Tasks**:
- [ ] Design library API
- [ ] Implement `pinnedPackages` function
- [ ] Implement `mkDevShell` helper
- [ ] Implement `mkOverlay` helper
- [ ] Implement `fetchVersion` function
- [ ] Create flake.nix for project
- [ ] Write library documentation
- [ ] Create example flakes

**Deliverables**:
- `lib/default.nix` with all functions
- `flake.nix` in project root
- Examples and documentation
- Template repository

**Dependencies**: Phase 10 (for lock file support)

---

### **Phase 13: Home Manager Integration** (2 weeks)
**Status**: Planned

**Tasks**:
- [ ] Design Home Manager module
- [ ] Implement per-user package pinning
- [ ] Create environment management
- [ ] Add activation scripts
- [ ] Write Home Manager documentation
- [ ] Create example configurations
- [ ] Integration tests

**Deliverables**:
- Home Manager module
- Environment switcher
- Documentation
- Examples

**Dependencies**: Phase 12 (for library functions)

---

### **Phase 14: Cloud Server & API** (4-6 weeks)
**Status**: Future

**Tasks**:
- [ ] Design REST API
- [ ] Migrate from Sled to PostgreSQL
- [ ] Implement Axum-based API server
- [ ] Add authentication/authorization
- [ ] Implement caching layer
- [ ] Create Docker images
- [ ] Write Kubernetes manifests
- [ ] Setup CI/CD for server
- [ ] Binary cache integration
- [ ] Create web dashboard

**Deliverables**:
- REST API server
- Docker images
- Deployment configs
- API documentation
- Web interface

**Dependencies**: All previous phases (complete rewrite)

---

### **Phase 15: Advanced Features** (Ongoing)
**Status**: Future

**Tasks**:
- [ ] VSCode extension
- [ ] Import from lock files (npm, cargo, pip, etc.)
- [ ] Registry support with custom URL scheme
- [ ] GitHub/GitLab integration
- [ ] Pre-commit hooks
- [ ] Security vulnerability scanning
- [ ] Dependency graph visualization
- [ ] Automated update PRs

**Deliverables**:
- IDE extensions
- Import tools
- CI/CD integrations
- Additional tooling

**Dependencies**: Phase 14 (for API access)

---

## 🎯 Priority Recommendations

### **Start With** (High Value, Low Effort):

1. **Lock File Support** (Phase 10)
   - Universal, doesn't require Nix changes
   - Enables reproducible builds
   - Foundation for other features
   - **Estimated time**: 1 week

2. **CLI Enhancements** (Phase 10)
   - `apply`, `generate`, `sync` commands
   - Multiple format outputs
   - High user value
   - **Estimated time**: 2 weeks

3. **Flake Library** (Phase 12)
   - Native Nix integration
   - Modern approach
   - Clean API
   - **Estimated time**: 1-2 weeks

### **Then** (Medium Priority):

4. **NixOS Module** (Phase 11)
   - Requires more design
   - System-wide impact
   - **Estimated time**: 2-3 weeks

5. **Home Manager** (Phase 13)
   - Depends on library
   - Per-user focus
   - **Estimated time**: 2 weeks

### **Long Term** (Strategic):

6. **Cloud/API** (Phase 14)
   - Major undertaking
   - Team collaboration features
   - **Estimated time**: 4-6 weeks

7. **Advanced Features** (Phase 15)
   - Nice-to-have
   - Continuous improvement
   - **Estimated time**: Ongoing

---

## 📈 Success Metrics

### **Phase 10-12** (Core Integration):
- [ ] CLI supports all major use cases
- [ ] Lock files work with flakes
- [ ] Library functions tested and documented
- [ ] At least 3 example projects

### **Phase 11-13** (System Integration):
- [ ] NixOS module in nixpkgs
- [ ] Home Manager module released
- [ ] 100+ active users
- [ ] Documentation complete

### **Phase 14+** (Cloud & Scale):
- [ ] API handles 1000+ req/min
- [ ] Database indexed 1M+ packages
- [ ] 10+ organizations using
- [ ] Web dashboard functional

---

## 🤝 Community & Contribution

### **Getting Involved**:
- Review roadmap and provide feedback
- Implement features from priority list
- Write documentation and examples
- Test pre-release versions
- Report bugs and edge cases

### **Communication Channels**:
- GitHub Discussions for planning
- Issues for bugs and feature requests
- Discord/Matrix for real-time chat
- RFC process for major changes

---

## 📚 Related Resources

- [NixOS Module Documentation](https://nixos.org/manual/nixos/stable/index.html#sec-writing-modules)
- [Nix Flakes](https://nixos.wiki/wiki/Flakes)
- [Home Manager](https://github.com/nix-community/home-manager)
- [Nix Language Server](https://github.com/nix-community/nil)

---

## 📝 Notes

- This roadmap is living document - will be updated as project evolves
- Phases may be reordered based on community feedback
- Implementation times are estimates
- Breaking changes will follow semantic versioning

---

**Last Updated**: February 24, 2026
**Status**: Phase 6b Complete, Phase 10 Next
