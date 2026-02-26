# NixOS module for nix-archiver
#
# Import in configuration.nix (no flakes required):
#
#   imports = [
#     (builtins.fetchTarball {
#       url    = "https://github.com/DemwE/nix-archiver/archive/main.tar.gz";
#       sha256 = "0000000000000000000000000000000000000000000000000000";  # fill in
#     } + "/module.nix")
#   ];
#
#   services.nix-archiver = {
#     enable     = true;
#     repository = "/var/lib/nix-archiver/nixpkgs";
#   };

{ config, lib, pkgs, ... }:

let
  cfg = config.services.nix-archiver;

  # Build the package from source if not provided externally.
  # Requires: pkg-config, openssl, git in build environment.
  defaultPackage = pkgs.callPackage (builtins.fetchTarball {
    # replace sha256 after pinning
    url    = "https://github.com/DemwE/nix-archiver/archive/main.tar.gz";
    sha256 = "0000000000000000000000000000000000000000000000000000";
  } + "/default.nix") {};

in {
  options.services.nix-archiver = {

    enable = lib.mkEnableOption "nix-archiver package version archiving tool";

    package = lib.mkOption {
      type        = lib.types.package;
      default     = defaultPackage;
      defaultText = lib.literalExpression "pkgs.callPackage .../default.nix {}";
      description = ''
        The nix-archiver package to use. Override to use a locally built binary:
          services.nix-archiver.package = pkgs.callPackage /path/to/nix-archiver/default.nix {};
      '';
    };

    database = lib.mkOption {
      type        = lib.types.str;
      default     = "/var/lib/nix-archiver/db";
      description = "Path where nix-archiver stores its sled database.";
    };

    repository = lib.mkOption {
      type        = lib.types.str;
      default     = "/var/lib/nix-archiver/nixpkgs";
      description = ''
        Path to a local (bare) clone of the nixpkgs repository that will be
        indexed. nix-archiver does NOT clone it automatically; you must create
        this clone yourself:
          git clone --bare https://github.com/NixOS/nixpkgs.git /var/lib/nix-archiver/nixpkgs
      '';
    };

    indexer = {
      enable = lib.mkEnableOption "automatic periodic re-indexing of the nixpkgs repository";

      schedule = lib.mkOption {
        type        = lib.types.str;
        default     = "weekly";
        example     = "Sun 03:00";
        description = ''
          Systemd calendar expression for when to run the indexer.
          Examples: "daily", "weekly", "Sun 03:00", "*-*-1 00:00:00"
          See systemd.time(7) for the full syntax.
        '';
      };

      fromDate = lib.mkOption {
        type        = lib.types.nullOr lib.types.str;
        default     = null;
        example     = "2023-01-01";
        description = ''
          Only index commits newer than this date (YYYY-MM-DD).
          Null means "from HEAD until a previously processed commit is encountered".
          Set this on the first run to avoid indexing years of history:
            services.nix-archiver.indexer.fromDate = "2023-01-01";
        '';
      };

      threads = lib.mkOption {
        type        = lib.types.nullOr lib.types.int;
        default     = null;
        example     = 4;
        description = "Number of parallel indexing threads. Null = auto (number of CPU cores).";
      };
    };

  };

  config = lib.mkIf cfg.enable {

    # ── Binary available system-wide ──────────────────────────────────────
    environment.systemPackages = [ cfg.package ];

    # ── State directory ───────────────────────────────────────────────────
    systemd.tmpfiles.rules = [
      "d ${cfg.database}    0750 nix-archiver nix-archiver -"
      "d ${cfg.repository}  0750 nix-archiver nix-archiver -"
    ];

    # ── Dedicated system user ─────────────────────────────────────────────
    users.users.nix-archiver = {
      isSystemUser = true;
      group        = "nix-archiver";
      home         = "/var/lib/nix-archiver";
      description  = "nix-archiver service account";
    };
    users.groups.nix-archiver = {};

    # ── Indexer service (oneshot) ─────────────────────────────────────────
    systemd.services.nix-archiver-index = lib.mkIf cfg.indexer.enable {
      description = "nix-archiver: index nixpkgs history";
      after       = [ "network.target" ];
      wants       = [ "network.target" ];

      path = [ cfg.package pkgs.git ];

      script = let
        toDateArg = lib.optionalString
          (cfg.indexer.fromDate != null)
          "--to-date ${cfg.indexer.fromDate}";
        threadArg = lib.optionalString
          (cfg.indexer.threads != null)
          "--threads ${toString cfg.indexer.threads}";
      in ''
        set -euo pipefail

        # Fetch latest commits before indexing
        if [ -d "${cfg.repository}" ]; then
          echo "[nix-archiver] Fetching latest nixpkgs commits..."
          git -C "${cfg.repository}" fetch --quiet origin || true
        else
          echo "[nix-archiver] WARNING: repository not found at ${cfg.repository}" >&2
          echo "[nix-archiver] Create it with:" >&2
          echo "[nix-archiver]   git clone --bare https://github.com/NixOS/nixpkgs.git ${cfg.repository}" >&2
          exit 1
        fi

        echo "[nix-archiver] Starting indexing..."
        nix-archiver \
          --database "${cfg.database}" \
          index \
          --repo "${cfg.repository}" \
          ${toDateArg} \
          ${threadArg}

        echo "[nix-archiver] Indexing complete."
      '';

      serviceConfig = {
        Type            = "oneshot";
        User            = "nix-archiver";
        Group           = "nix-archiver";
        WorkingDirectory = "/var/lib/nix-archiver";
        # Security hardening
        PrivateTmp        = true;
        NoNewPrivileges   = true;
        ProtectSystem     = "strict";
        ReadWritePaths    = [ cfg.database cfg.repository ];
        # Allow long runs (indexing full nixpkgs history can take hours)
        TimeoutSec        = "12h";
      };
    };

    # ── Indexer timer ─────────────────────────────────────────────────────
    systemd.timers.nix-archiver-index = lib.mkIf cfg.indexer.enable {
      description    = "nix-archiver: periodic nixpkgs re-indexing";
      wantedBy       = [ "timers.target" ];
      timerConfig    = {
        OnCalendar    = cfg.indexer.schedule;
        Persistent    = true;   # run if missed while machine was off
        RandomizedDelaySec = "30min";
      };
    };

  };
}
