{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.leeward;

  leeward = pkgs.callPackage ./packages.nix { inherit pkgs; };
in {
  options.services.leeward = {
    enable = mkEnableOption "leeward sandbox daemon";

    package = mkOption {
      type = types.package;
      default = leeward.leeward-daemon;
      description = "The leeward package to use.";
    };

    numWorkers = mkOption {
      type = types.int;
      default = 4;
      description = "Number of worker processes in the pool.";
    };

    recycleAfter = mkOption {
      type = types.int;
      default = 100;
      description = "Recycle workers after this many executions.";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.leeward = {
      description = "Leeward sandbox daemon";
      documentation = [ "https://github.com/vektia/leeward" ];
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/leeward-daemon";
        Restart = "on-failure";
        RestartSec = "5s";

        # Security hardening
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ "/run/leeward" ];

        # Runtime directory
        RuntimeDirectory = "leeward";
        RuntimeDirectoryMode = "0755";

        # Logging
        StandardOutput = "journal";
        StandardError = "journal";
        SyslogIdentifier = "leeward-daemon";
      };
    };

    # Create group for socket access
    users.groups.leeward = {};

    # Ensure runtime directory permissions
    systemd.tmpfiles.rules = [
      "d /run/leeward 0755 root leeward - -"
    ];
  };
}
