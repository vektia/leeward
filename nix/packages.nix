{ pkgs, lib }:

let
  version = lib.cargoVersion ../Cargo.toml;
  buildDeps = lib.buildDeps;
  src = pkgs.lib.cleanSource ../.;

  rustBuild = {
    inherit version src;
    cargoLock.lockFile = ../Cargo.lock;
    nativeBuildInputs = with pkgs; [ clang mold ] ++ buildDeps;
    buildInputs = buildDeps;
    RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
  };

  leeward-cli = pkgs.rustPlatform.buildRustPackage (rustBuild // {
    pname = "leeward-cli";
    cargoBuildFlags = [ "-p" "leeward-cli" ];
    cargoTestFlags = [ "-p" "leeward-cli" ];
  });

  leeward-daemon = pkgs.rustPlatform.buildRustPackage (rustBuild // {
    pname = "leeward-daemon";
    cargoBuildFlags = [ "-p" "leeward-daemon" ];
    cargoTestFlags = [ "-p" "leeward-daemon" ];
  });

  leeward-ffi = pkgs.rustPlatform.buildRustPackage (rustBuild // {
    pname = "leeward-ffi";
    cargoBuildFlags = [ "-p" "leeward-ffi" ];
    cargoTestFlags = [ "-p" "leeward-ffi" ];
    nativeBuildInputs = with pkgs; [ clang mold cbindgen ] ++ buildDeps;

    postInstall = ''
      mkdir -p $out/lib $out/include
      cp target/release/libleeward.so $out/lib/ 2>/dev/null || true
      cp target/release/libleeward.a $out/lib/ 2>/dev/null || true
      if [ -f include/leeward.h ]; then
        cp include/leeward.h $out/include/
      fi
    '';
  });

in
{
  inherit leeward-cli leeward-daemon leeward-ffi;

  leeward-all = pkgs.symlinkJoin {
    name = "leeward-${version}";
    paths = [ leeward-cli leeward-daemon leeward-ffi ];
  };
}