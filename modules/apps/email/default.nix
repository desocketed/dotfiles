{ config, lib, ... }:
let
  cfg = config.modules.email;
in {
  options.modules.email = {
    enable = lib.mkEnableOption "notmuch email";
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

    accounts.email = {
      maildirBasePath = "Mail";
      accounts.fastmail = {
        primary = true;
        realName = cfg.name;
        address = cfg.address;
        userName = cfg.address;
        passwordCommand = "pass show fastmail.com/mujmap";
        notmuch.enable = true;
      };
    };
  };
}
