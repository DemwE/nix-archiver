# Nix development shell for nix-archiver
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rustfmt
    clippy
    
    # Build dependencies
    pkg-config
    openssl
    
    # Git (for indexing)
    git
    
    # Additional tools
    gnumake
    gcc
  ];

  # Environment variables
  RUST_BACKTRACE = "1";
  OPENSSL_DIR = "${pkgs.openssl.dev}";
  OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
}
