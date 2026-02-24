{
  description = "Nix-Archiver - AI-Ready deklaratywne przypisywanie pakietów do wersji historycznych";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            
            # Build dependencies
            pkg-config
            openssl
            
            # Git dla indeksowania
            git
            
            # Additional tools
            gnumake
            gcc
          ];

          shellHook = ''
            echo "🚀 Nix-Archiver development environment"
            echo "Rust: $(rustc --version)"
            echo "Cargo: $(cargo --version)"
            
            export RUST_BACKTRACE=1
          '';

          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        };

        # Możliwość zbudowania projektu przez Nix
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "nix-archiver";
          version = "0.1.0";
          
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          
          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          
          buildInputs = with pkgs; [
            openssl
          ];
          
          meta = with pkgs.lib; {
            description = "Deklaratywne przypisywanie pakietów do wersji historycznych w Nixpkgs";
            license = licenses.mit;
          };
        };
      }
    );
}
