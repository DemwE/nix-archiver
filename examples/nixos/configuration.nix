# Example NixOS configuration using nix-archiver module
# /etc/nixos/configuration.nix

{ config, pkgs, ... }:

{
  imports = [
    # Import nix-archiver module
    /path/to/nix-archiver/modules/nix-archiver.nix
  ];

  # Enable nix-archiver service
  services.nix-archiver = {
    enable = true;

    # Enable automatic indexing
    indexer = {
      enable = true;
      repository = "/var/lib/nix-archiver/nixpkgs";
      database = "/var/lib/nix-archiver/db";
      updateInterval = "daily";  # Can be: "daily", "weekly", "hourly", or systemd calendar format
      maxCommits = 5000;  # Limit initial indexing
      batchSize = 100;
      threads = 4;  # null for auto-detect
    };

    # Pin specific package versions
    pinnedPackages = {
      nodejs = "20.11.0";
      python = "3.11.7";
      go = "1.21.5";
      git = "2.42.0";
    };

    # Generate overlay (makes pinned packages available in pkgs)
    generateOverlay = true;

    # Logging level
    logLevel = "info";  # "error", "warn", "info", "debug", "trace"
  };

  # Optional: Make pinned packages available system-wide
  environment.systemPackages = with pkgs; [
    # These will use the pinned versions if overlay is enabled
    nodejs
    python
    go
    git
  ];
}
