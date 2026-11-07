{ config, lib, ... }:
let
  cfg = config.modules.email;
in {
  options.modules.email = {
    enable = lib.mkEnableOption "notmuch/mujmap email";
    name = lib.mkOption {
      type = lib.types.str;
      description = "Real name on the account; set it in the gitignored hosts/local.nix.";
    };
    address = lib.mkOption {
      type = lib.types.str;
      description = "Fastmail address; set it in the gitignored hosts/local.nix.";
    };
  };

  config = lib.mkIf cfg.enable {
    programs.notmuch = {
      enable = true;
      settings = {
        user.name = cfg.name;
        user.primary_email = cfg.address;
        database.path = "${config.home.homeDirectory}/Mail";
      };
    };

    # Periodic sync
    systemd.user.services.mujmap = {
      Service = {
        Type = "simple";
        # The current version of mujmap is broken and does not support
        # bearer tokens.
        ExecStart = "${config.home.homeDirectory}/.cargo/bin/mujmap -C ${config.accounts.email.maildirBasePath}/fastmail -v sync -v";
      };
    };

    systemd.user.timers.mujmap = {
      Timer = {
        OnBootSec = "1m";
        OnUnitActiveSec = "5m";
      };
      Install = {
        WantedBy = [ "timers.target" ];
      };
    };

    accounts.email = {
      maildirBasePath = "Mail";
      accounts.fastmail = {
        primary = true;
        realName = cfg.name;
        address = cfg.address;
        userName = cfg.address;
        passwordCommand = "pass show fastmail.com/mujmap";
        notmuch.enable = true;
        mujmap = {
          enable = true;
          settings = {
            session_url = "https://api.fastmail.com/jmap/session";
            password_command = "pass show fastmail.com/mujmap";
          };
        };
      };
    };

    programs.mujmap = { enable = true; };
  };
}
