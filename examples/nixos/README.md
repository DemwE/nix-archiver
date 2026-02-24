# NixOS Configuration Examples

This directory contains example NixOS configurations demonstrating different use cases of the nix-archiver module.

## Examples

### [minimal.nix](minimal.nix)
Simplest possible configuration - pin a single package.
- ✓ Pin nodejs to version 20.11.0
- ✗ No automatic indexing
- ✗ No database updates

**Use case**: You already have an indexed database and just want to pin one package.

### [configuration.nix](configuration.nix)
Recommended configuration for most users.
- ✓ Automatic daily indexing
- ✓ Pin multiple packages
- ✓ Configurable performance settings

**Use case**: Production setup with automatic updates and multiple pinned packages.

### [advanced.nix](advanced.nix)
Full-featured configuration showing all available options.
- ✓ Custom state directory
- ✓ Performance tuning (batch size, threads)
- ✓ Frequent updates (every 4 hours)
- ✓ Extended commit history (10,000 commits)
- ✓ Debug logging
- ✓ Multiple pinned packages

**Use case**: High-performance systems, development environments, or when you need fine-grained control.

### [indexer-only.nix](indexer-only.nix)
Run only the indexer service without automatic package pinning.
- ✓ Automatic indexing
- ✗ No pinned packages
- ✗ No overlay generation

**Use case**: Build the database for manual CLI usage, or when you want to generate packages.nix manually.

## Usage

1. Choose the example that fits your needs
2. Copy it to `/etc/nixos/` or import it in your configuration
3. Update the import path:
   ```nix
   imports = [
     /path/to/nix-archiver/modules/nix-archiver.nix
   ];
   ```
4. Customize the settings (repository paths, package versions, etc.)
5. Apply the configuration:
   ```bash
   sudo nixos-rebuild switch
   ```

## Module Documentation

See [../modules/README.md](../modules/README.md) for complete documentation of all options.

## Tips

- Start with `minimal.nix` or `configuration.nix` for your first setup
- Use `indexer-only.nix` if you prefer manual control with the CLI
- Refer to `advanced.nix` for all available configuration options
- Check service status: `systemctl status nix-archiver-indexer.service`
- View logs: `journalctl -u nix-archiver-indexer.service -f`
