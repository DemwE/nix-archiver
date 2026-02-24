{ lib
, rustPlatform
, pkg-config
, openssl
, git
, stdenv
}:

rustPlatform.buildRustPackage rec {
  pname = "nix-archiver";
  version = "0.1.0";

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
    openssl
    git
  ];

  # Use vendored OpenSSL and libgit2 to avoid system dependencies
  CARGO_BUILD_FEATURES = "vendored";

  meta = with lib; {
    description = "Declarative pinning of packages to historical versions in Nixpkgs";
    homepage = "https://github.com/yourusername/nix-archiver";
    license = licenses.mit;
    maintainers = [];
    mainProgram = "nix-archiver";
  };
}
