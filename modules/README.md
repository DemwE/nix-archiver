# NixOS Module Documentation

## Overview

The `nix-archiver` NixOS module provides declarative package version pinning and automatic Nixpkgs repository indexing through systemd integration.

## Features

- **Declarative Package Pinning**: Pin specific package versions in your NixOS configuration
- **Automatic Indexing**: Systemd service that indexes the Nixpkgs repository
- **Periodic Updates**: Systemd timer for automatic database updates
- **Overlay Generation**: Automatically generates a Nix overlay for pinned packages
- **CLI Integration**: Access indexed data through the `nix-archiver` command

## Installation

### 1. Add module to your configuration

Edit `/etc/nixos/configuration.nix`:

```nix
{ config, pkgs, ... }:

{
  imports = [
    /path/to/nix-archiver/modules/nix-archiver.nix
  ];

  services.nix-archiver = {
    enable = true;
    
    pinnedPackages = {
      nodejs = "20.11.0";
    };
  };
}
```

### 2. Rebuild your system

```bash
sudo nixos-rebuild switch
```

## Configuration Options

### `services.nix-archiver.enable`

**Type**: `bool`  
**Default**: `false`

Enable the nix-archiver module.

### `services.nix-archiver.package`

**Type**: `package`  
**Default**: `pkgs.nix-archiver`

The nix-archiver package to use. Override this if you need a custom build.

```nix
services.nix-archiver.package = pkgs.nix-archiver.override {
  # custom overrides
};
```

### `services.nix-archiver.stateDirectory`

**Type**: `path`  
**Default**: `"/var/lib/nix-archiver"`

Directory for storing the database and cloned Nixpkgs repository.

### `services.nix-archiver.logLevel`

**Type**: `enum ["error" "warn" "info" "debug" "trace"]`  
**Default**: `"info"`

Logging verbosity for the indexer service.

### `services.nix-archiver.indexer`

Configuration for the automatic indexing service.

#### `services.nix-archiver.indexer.enable`

**Type**: `bool`  
**Default**: `false`

Enable automatic Nixpkgs indexing via systemd service and timer.

#### `services.nix-archiver.indexer.repository`

**Type**: `path`  
**Default**: `"${stateDirectory}/nixpkgs"`

Path to the Nixpkgs git repository. Will be cloned automatically if it doesn't exist.

#### `services.nix-archiver.indexer.database`

**Type**: `path`  
**Default**: `"${stateDirectory}/db"`

Path to the Sled database directory.

#### `services.nix-archiver.indexer.updateInterval`

**Type**: `str`  
**Default**: `"daily"`

How often to update the index. Can be:
- `"daily"` - Once per day
- `"weekly"` - Once per week
- `"hourly"` - Every hour
- Any systemd calendar format (e.g., `"0/4:00:00"` for every 4 hours)

See `systemd.time(7)` for calendar format documentation.

#### `services.nix-archiver.indexer.maxCommits`

**Type**: `null or int`  
**Default**: `null`

Maximum number of commits to index. Use this to limit initial indexing time.

#### `services.nix-archiver.indexer.batchSize`

**Type**: `int`  
**Default**: `100`

Number of packages to process in each batch. Higher values use more memory but may be faster.

#### `services.nix-archiver.indexer.threads`

**Type**: `null or int`  
**Default**: `null`

Number of threads for parallel processing. `null` means auto-detect based on CPU cores.

### `services.nix-archiver.pinnedPackages`

**Type**: `attrset of str`  
**Default**: `{}`

Attribute set of package names to version strings. These packages will be pinned to the specified versions.

```nix
services.nix-archiver.pinnedPackages = {
  nodejs = "20.11.0";
  python3 = "3.11.7";
  postgresql = "15.5";
};
```

The module will automatically:
1. Generate a `packages.nix` file in the state directory
2. Create an overlay that provides these pinned versions
3. Apply the overlay to `pkgs`

### `services.nix-archiver.generateOverlay`

**Type**: `bool`  
**Default**: `true`

Whether to generate and apply a Nix overlay for pinned packages. When enabled, pinned packages are automatically available in `pkgs`.

## Usage Examples

### Example 1: Minimal Configuration

Pin a single package without automatic indexing:

```nix
services.nix-archiver = {
  enable = true;
  
  pinnedPackages = {
    nodejs = "20.11.0";
  };
  
  # No indexer needed if you already have a database
  indexer.enable = false;
};

environment.systemPackages = [ pkgs.nodejs ];  # Uses version 20.11.0
```

### Example 2: Full Automation

Enable indexer with automatic updates:

```nix
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
    go = "1.21.5";
  };
};
```

### Example 3: Indexer Only

Just run the indexer, use CLI for manual operations:

```nix
services.nix-archiver = {
  enable = true;
  
  indexer = {
    enable = true;
    updateInterval = "weekly";
  };
  
  # No pinned packages
  generateOverlay = false;
};

environment.systemPackages = [ pkgs.nix-archiver ];
```

Then use the CLI:

```bash
nix-archiver search nodejs
nix-archiver search --version "20.*" nodejs
nix-archiver generate -o my-packages.nix
```

### Example 4: Performance Tuning

For systems with many cores and fast storage:

```nix
services.nix-archiver = {
  enable = true;
  
  indexer = {
    enable = true;
    updateInterval = "0/4:00:00";  # Every 4 hours
    maxCommits = 10000;
    batchSize = 200;
    threads = 16;
  };
  
  logLevel = "debug";
};
```

## Systemd Integration

The module creates the following systemd units:

### `nix-archiver-indexer.service`

A oneshot service that:
1. Clones Nixpkgs repository if missing
2. Updates the repository (`git fetch && git reset --hard origin/master`)
3. Runs the indexer with configured options

### `nix-archiver-indexer.timer`

A timer that triggers the indexer service based on `updateInterval`.

### Checking Service Status

```bash
# Check if the service is running
systemctl status nix-archiver-indexer.service

# Check timer status
systemctl status nix-archiver-indexer.timer

# View logs
journalctl -u nix-archiver-indexer.service

# Follow logs in real-time
journalctl -u nix-archiver-indexer.service -f

# Manually trigger indexing
sudo systemctl start nix-archiver-indexer.service
```

## User and Permissions

The module automatically creates:
- **User**: `nix-archiver` (system user)
- **Group**: `nix-archiver`
- **State Directory**: Owned by `nix-archiver:nix-archiver` with permissions `0755`

The indexer service runs as the `nix-archiver` user.

## CLI Access

When the module is enabled, the `nix-archiver` command is configured to use the correct database path:

```bash
# Search for packages
nix-archiver search nodejs

# Search with version filter
nix-archiver search --version "20.*" nodejs

# Show statistics
nix-archiver stats

# Generate packages.nix
nix-archiver generate -o my-packages.nix
```

## Overlay Mechanism

When `generateOverlay = true` and `pinnedPackages` is not empty:

1. A `packages.nix` file is generated in `${stateDirectory}/packages.nix`:
   ```nix
   {
     nodejs = "20.11.0";
     python3 = "3.11.7";
   }
   ```

2. An overlay is created that runs:
   ```bash
   nix-archiver generate -i ${database} -p ${packagesFile} -o overlay.nix
   ```

3. The overlay is automatically applied to `pkgs` via `nixpkgs.overlays`

4. Pinned packages are available system-wide:
   ```nix
   environment.systemPackages = [ 
     pkgs.nodejs    # version 20.11.0
     pkgs.python3   # version 3.11.7
   ];
   ```

## Troubleshooting

### Indexer service fails to start

Check the logs:
```bash
journalctl -u nix-archiver-indexer.service -n 50
```

Common issues:
- **Permission denied**: Check that state directory is writable by `nix-archiver` user
- **Git clone failed**: Ensure network connectivity
- **Database locked**: Another instance might be running

### Pinned packages not found

1. Verify the indexer has run successfully:
   ```bash
   systemctl status nix-archiver-indexer.service
   ```

2. Check if the version exists:
   ```bash
   nix-archiver search --version "20.11.0" nodejs
   ```

3. Verify overlay is enabled:
   ```nix
   services.nix-archiver.generateOverlay = true;
   ```

4. Rebuild after configuration changes:
   ```bash
   sudo nixos-rebuild switch
   ```

### Large database size

Limit indexed commits:
```nix
services.nix-archiver.indexer.maxCommits = 2000;
```

### Slow indexing

Tune performance settings:
```nix
services.nix-archiver.indexer = {
  batchSize = 150;  # Increase batch size
  threads = 8;      # Use more threads
};
```

### Manual database rebuild

Stop the timer and service:
```bash
sudo systemctl stop nix-archiver-indexer.timer
sudo systemctl stop nix-archiver-indexer.service
```

Remove the database:
```bash
sudo rm -rf /var/lib/nix-archiver/db
```

Start the service:
```bash
sudo systemctl start nix-archiver-indexer.service
sudo systemctl start nix-archiver-indexer.timer
```

## Integration with Other Tools

### Using with Flakes

See [ROADMAP.md](../../ROADMAP.md) - Level 2: Flake Output Integration

### Using with Home Manager

See [ROADMAP.md](../../ROADMAP.md) - Level 3: Home Manager Module

## Security Considerations

- The indexer clones the official Nixpkgs repository from GitHub
- The service runs as an unprivileged system user (`nix-archiver`)
- Database directory has restricted permissions (owner only)
- No network access required after initial repository clone

## Performance Characteristics

- **Initial indexing**: Depends on `maxCommits` setting
  - 1000 commits: ~2-5 minutes
  - 5000 commits: ~10-20 minutes
  - 10000 commits: ~20-40 minutes

- **Database size**: ~50-500MB depending on indexed commits

- **Memory usage**: ~100-500MB during indexing (depends on `batchSize`)

- **Update time**: ~1-2 minutes for incremental updates

## See Also

- [Main README](../../README.md)
- [Project Roadmap](../../ROADMAP.md)
- [Specification](../../spec.md)
- [Example Configurations](../../examples/nixos/)
