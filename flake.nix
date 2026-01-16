{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    nixpkgs-unstable,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system}.extend rust-overlay.overlays.default;
      unstable = nixpkgs-unstable.legacyPackages.${system};
    in {
      devShells.default = pkgs.mkShell {
        buildInputs = [
          # Latest nightly Rust toolchain with miri
          (pkgs.rust-bin.nightly.latest.default.override {
            extensions = ["rust-src" "clippy" "rustfmt" "rust-analyzer" "miri"];
          })

          # Additional tools
          pkgs.pkg-config
          pkgs.openssl
          pkgs.openssl.dev
        ];

        # Environment variables
        RUST_SRC_PATH = "${pkgs.rust-bin.nightly.latest.rust-src}/lib/rustlib/src/rust/library";
      };
    });
}
