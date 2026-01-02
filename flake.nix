{
  description = "Linux-native sandbox for running untrusted code";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];

      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      nixpkgsFor = forAllSystems (system: import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      });

      # Read version from Cargo.toml
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      version = cargoToml.workspace.package.version;

      buildLeeward = { pkgs, target ? null, static ? false }:
        let
          rustToolchain = if static then
            pkgs.rust-bin.stable.latest.default.override {
              targets = [ "x86_64-unknown-linux-musl" ];
            }
          else
            pkgs.rust-bin.stable.latest.default;

          buildInputs = with pkgs; [
            python3
          ] ++ lib.optionals static [
            pkgs.pkgsStatic.stdenv.cc
          ];

          nativeBuildInputs = with pkgs; [
            rustToolchain
            pkg-config
          ];

        in pkgs.rustPlatform.buildRustPackage {
          pname = "leeward";
          inherit version;

          src = pkgs.lib.cleanSource ./.;

          cargoLock.lockFile = ./Cargo.lock;

          inherit buildInputs nativeBuildInputs;

          CARGO_BUILD_TARGET = if static then "x86_64-unknown-linux-musl" else null;
          CARGO_BUILD_RUSTFLAGS = if static then "-C target-feature=+crt-static" else "";

          postInstall = ''
            mkdir -p $out/bin

            if [ -f target/*/release/leeward-daemon ]; then
              mv target/*/release/leeward-daemon $out/bin/
              mv target/*/release/leeward $out/bin/
            else
              mv target/release/leeward-daemon $out/bin/ || true
              mv target/release/leeward $out/bin/ || true
            fi

            ${if static then "${pkgs.binutils}/bin/strip $out/bin/*" else ""}

            mkdir -p $out/lib/systemd/{system,user}
            cp contrib/leeward.system.service $out/lib/systemd/system/leeward.service
            cp contrib/leeward.user.service $out/lib/systemd/user/leeward.service
          '';

          meta = with pkgs.lib; {
            description = "Linux-native sandbox for running untrusted code";
            homepage = "https://github.com/vektia/leeward";
            license = licenses.asl20;
            platforms = [ "x86_64-linux" "aarch64-linux" ];
            mainProgram = "leeward";
          };
        };

      buildDeb = { pkgs, leeward }:
        let
          arch = if pkgs.stdenv.hostPlatform.system == "x86_64-linux" then "amd64"
                 else if pkgs.stdenv.hostPlatform.system == "aarch64-linux" then "arm64"
                 else throw "Unsupported architecture";
        in pkgs.stdenv.mkDerivation {
          pname = "leeward-deb";
          inherit version;

          dontUnpack = true;
          dontBuild = true;

          nativeBuildInputs = [ pkgs.dpkg ];

          installPhase = ''
            mkdir -p $out
            mkdir -p deb/DEBIAN deb/usr/bin deb/usr/lib/systemd/{system,user}

            cp ${leeward}/bin/* deb/usr/bin/
            cp ${leeward}/lib/systemd/system/*.service deb/usr/lib/systemd/system/
            cp ${leeward}/lib/systemd/user/*.service deb/usr/lib/systemd/user/
            cat > deb/DEBIAN/control <<EOF
            Package: leeward
            Version: ${version}
            Architecture: ${arch}
            Maintainer: Vektia <hello@vektia.com.br>
            Description: Linux-native sandbox for running untrusted code
            Homepage: https://github.com/vektia/leeward
            Section: devel
            Priority: optional
            Depends: python3
            EOF

            cat > deb/DEBIAN/postinst <<'EOF'
            #!/bin/sh
            set -e
            if ! getent passwd leeward >/dev/null; then
                useradd -r -s /usr/sbin/nologin -d /nonexistent leeward
            fi
            mkdir -p /run/leeward
            chown leeward:leeward /run/leeward
            if [ -d /run/systemd/system ]; then
                systemctl daemon-reload
            fi
            EOF
            chmod 755 deb/DEBIAN/postinst

            dpkg-deb --build deb $out/leeward_${version}_${arch}.deb
          '';
        };

    in {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
          leeward = buildLeeward { inherit pkgs; };

        in {
          default = leeward;
          leeward-static = if system == "x86_64-linux"
            then buildLeeward { inherit pkgs; static = true; }
            else null;
          leeward-deb = buildDeb { inherit pkgs leeward; };
        } // pkgs.lib.optionalAttrs (system == "x86_64-linux") {
          leeward-x86_64 = leeward;
        } // pkgs.lib.optionalAttrs (system == "aarch64-linux") {
          leeward-aarch64 = leeward;
        });

      devShells = forAllSystems (system:
        let
          pkgs = nixpkgsFor.${system};
          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "rust-analyzer" ];
          };
        in {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustToolchain
              cargo-watch
              cargo-audit
              pkg-config
              python3
              mold
              clang
            ];

            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
            LEEWARD_SOCKET = "$PWD/.leeward.sock";

            shellHook = "";
          };
        });

      nixosModules.default = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.services.leeward;
        in {
          options.services.leeward = {
            enable = mkEnableOption "Leeward sandbox daemon";

            workers = mkOption {
              type = types.int;
              default = 4;
              description = "Number of pre-forked workers";
            };

            package = mkOption {
              type = types.package;
              default = self.packages.${pkgs.system}.default;
              description = "Leeward package to use";
            };
          };

          config = mkIf cfg.enable {
            systemd.services.leeward = {
              description = "Leeward Sandbox Daemon";
              wantedBy = [ "multi-user.target" ];
              after = [ "network.target" ];

              serviceConfig = {
                Type = "simple";
                ExecStart = "${cfg.package}/bin/leeward-daemon --workers ${toString cfg.workers}";
                Restart = "always";
                RestartSec = 5;
                User = "leeward";
                Group = "leeward";
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = true;
                PrivateTmp = true;
                RuntimeDirectory = "leeward";
                RuntimeDirectoryMode = "0755";
              };
            };

            users.users.leeward = {
              isSystemUser = true;
              group = "leeward";
              description = "Leeward daemon user";
            };

            users.groups.leeward = {};
          };
        };
    };
}