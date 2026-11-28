{ config, lib, pkgs, ... }:
let
  cfg = config.modules.swaylock;
in {
  options.modules.swaylock = {
    enable = lib.mkEnableOption "swaylock";
    # Nix's swaylock links Nix's PAM, which can't load the host's PAM modules on a
    # non-NixOS system, so every password fails. There, run the distro's swaylock
    # (with its own /etc/pam.d/swaylock) and keep only the settings from here.
    systemBinary = lib.mkEnableOption "the host's /usr/bin/swaylock instead of Nix's";
  };

  config = lib.mkIf cfg.enable {
    # The lock bind exists only where a locker does (the desktop never locks)
    home.file."niri-lock-bind" = {
      text = "binds {\n  Super+Alt+L { spawn \"swaylock\"; }\n}\n";
      target = ".config/niri/mine/lock.kdl";
    };
    programs.swaylock = {
      enable = true;
      package = lib.mkIf cfg.systemBinary
        (pkgs.writeShellScriptBin "swaylock" ''exec /usr/bin/swaylock "$@"'');
      settings = {
        color = "000000";
        show-failed-attempts = true;
        indicator-caps-lock = true;
      };
    };
  };
}
