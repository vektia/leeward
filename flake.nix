{
  description = "Run untrusted Python code safely with native Linux isolation";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        lib = import ./nix/lib.nix { inherit pkgs; };
        packages = import ./nix/packages.nix { inherit pkgs lib; };
      in
      {
        packages = {
          default = packages.leeward-all;
          cli = packages.leeward-cli;
          daemon = packages.leeward-daemon;
          ffi = packages.leeward-ffi;
        };

        devShells.default = import ./nix/shell.nix { inherit pkgs; };
      }
    );
}