# Advanced nix-archiver configuration with custom settings
# /etc/nixos/configuration.nix

{ config, pkgs, ... }:

{
  imports = [
    /path/to/nix-archiver/modules/nix-archiver.nix
  ];

  services.nix-archiver = {
    enable = true;

    # Use custom package build if needed
    # package = pkgs.nix-archiver.override { /* ... */ };

    # Custom state directory
    stateDirectory = "/srv/nix-archiver";

    # Advanced indexer configuration
    indexer = {
      enable = true;
      repository = "/srv/nix-archiver/nixpkgs";
      database = "/srv/nix-archiver/db";
      
      # Update every 4 hours
      updateInterval = "0/4:00:00";  # systemd calendar format
      
      # Index more commits for longer history
      maxCommits = 10000;
      
      # Performance tuning
      batchSize = 200;  # Process more packages per batch
      threads = 8;      # Use 8 threads
    };

    # Pin multiple packages for reproducible environment
    pinnedPackages = {
      # Development tools
      nodejs = "20.11.0";
      python3 = "3.11.7";
      go = "1.21.5";
      rust = "1.75.0";
      
      # System tools
      git = "2.42.0";
      vim = "9.0.2116";
      tmux = "3.3a";
      
      # Databases
      postgresql = "15.5";
      redis = "7.2.3";
      
      # Web servers
      nginx = "1.24.0";
      caddy = "2.7.6";
    };

    # Enable overlay to make pinned versions available
    generateOverlay = true;

    # Verbose logging for debugging
    logLevel = "debug";
  };

  # Check indexer service status:
  # $ systemctl status nix-archiver-indexer.service
  # $ systemctl status nix-archiver-indexer.timer
  # $ journalctl -u nix-archiver-indexer.service -f
  
  # Manual operations:
  # $ nix-archiver search nodejs  # Search for nodejs versions
  # $ nix-archiver stats          # Show database statistics

  # Use pinned packages in system configuration
  environment.systemPackages = with pkgs; [
    nodejs
    python3
    go
    rust
    git
    vim
    tmux
    postgresql
    redis
    nginx
    caddy
  ];

  # Example: Use pinned PostgreSQL version in service
  services.postgresql = {
    enable = true;
    package = pkgs.postgresql;  # Will use pinned version 15.5
  };
}
