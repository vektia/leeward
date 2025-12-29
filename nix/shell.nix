{ pkgs }:

let
  rustToolchain = pkgs.rust-bin.stable.latest.default.override {
    extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
    targets = [ "x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" ];
  };
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustToolchain
    cargo-watch
    cargo-edit
    cargo-expand
    cargo-outdated
    clang
    lld
    mold
    pkg-config
    gnumake
    cmake
    libseccomp
    gdb
    strace
  ];

  shellHook = ''
    export CARGO_HOME="$PWD/.cargo-home"
    export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
    export LIBSECCOMP_LINK_TYPE="dylib"
    export LIBSECCOMP_LIB_PATH="${pkgs.libseccomp}/lib"
  '';

  PKG_CONFIG_PATH = "${pkgs.libseccomp}/lib/pkgconfig";
}