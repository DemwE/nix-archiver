{ config, lib, pkgs, ... }:

with lib;

{
  options.services.nix-archiver = {
    enable = mkEnableOption "nix-archiver package version pinning service";

    package = mkOption {
      type = types.package;
      default = pkgs.nix-archiver or (pkgs.callPackage ../default.nix {});
      defaultText = "pkgs.nix-archiver";
      description = "The nix-archiver package to use.";
    };

    indexer = {
      enable = mkEnableOption "automatic Nixpkgs repository indexing";

      repository = mkOption {
        type = types.path;
        default = "/var/lib/nix-archiver/nixpkgs";
        description = "Path to the Nixpkgs repository to index.";
      };

      database = mkOption {
        type = types.path;
        default = "/var/lib/nix-archiver/db";
        description = "Path to the nix-archiver database.";
      };

      updateInterval = mkOption {
        type = types.str;
        default = "daily";
        description = ''
          How often to update the index. This is a systemd timer interval.
          Examples: "daily", "weekly", "hourly", "00:00:00" (midnight)
        '';
      };

      maxCommits = mkOption {
        type = types.nullOr types.int;
        default = null;
        description = "Maximum number of commits to process during indexing.";
      };

      batchSize = mkOption {
        type = types.int;
        default = 100;
        description = "Number of commits to process in each batch.";
      };

      threads = mkOption {
        type = types.nullOr types.int;
        default = null;
        description = "Number of threads for parallel processing. Defaults to number of CPU cores.";
      };
    };

    pinnedPackages = mkOption {
      type = types.attrsOf types.str;
      default = {};
      example = literalExpression ''
        {
          nodejs = "20.11.0";
          python = "3.11.7";
          go = "1.21.5";
        }
      '';
      description = ''
        Package versions to pin. These will be made available through
        the generated overlay or as direct package outputs.
      '';
    };

    generateOverlay = mkOption {
      type = types.bool;
      default = true;
      description = ''
        Whether to automatically generate a nixpkgs overlay with
        the pinned packages. When enabled, pinned packages will be
        available in pkgs.
      '';
    };

    stateDirectory = mkOption {
      type = types.path;
      default = "/var/lib/nix-archiver";
      description = "Directory for nix-archiver state and database.";
    };

    logLevel = mkOption {
      type = types.enum [ "error" "warn" "info" "debug" "trace" ];
      default = "info";
      description = "Logging level for nix-archiver services.";
    };
  };
}
