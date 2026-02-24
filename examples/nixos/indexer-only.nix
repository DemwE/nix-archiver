# nix-archiver indexer-only configuration
# Only runs the indexing service, no automatic package pinning
# /etc/nixos/configuration.nix

{ config, pkgs, ... }:

{
  imports = [
    /path/to/nix-archiver/modules/nix-archiver.nix
  ];

  services.nix-archiver = {
    enable = true;

    # Only enable indexer, no package pinning
    indexer = {
      enable = true;
      repository = "/var/lib/nix-archiver/nixpkgs";
      database = "/var/lib/nix-archiver/db";
      updateInterval = "daily";
    };

    # No pinned packages
    pinnedPackages = {};

    # Don't generate overlay (nothing to pin)
    generateOverlay = false;
  };

  # Install nix-archiver CLI for manual searches
  environment.systemPackages = with pkgs; [
    nix-archiver
  ];

  # After the indexer runs, you can use the CLI:
  # $ nix-archiver search nodejs
  # $ nix-archiver search --version "20.*" nodejs
  # $ nix-archiver stats
  # 
  # Then manually create packages.nix when needed:
  # $ nix-archiver generate -o packages.nix
}
